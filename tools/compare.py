#!/usr/bin/env python3
"""Compare converter output with reference IDML files.

For every INDD file in the corpus that has a reference IDML from the same
major version (the `idml` column of corpus/inventory.tsv, written by
tools/inventory.py), convert it with `target/release/indd` and compare the
result with the reference. Files with the same SHA-256 are converted
once. Reports, per element type: how many referenced elements we produce
(matched by Self), and per attribute how often our value matches. Also
reports story text agreement.

Usage: python3 -I tools/compare.py [--limit N] [--detail TAG]... [--file SUBSTR]
                                   [--schemas DIR --jing DIR] [--bin PATH]
                                   [--all] [--exclude PREFIX]... [--jobs N]
                                   [--shortfalls N] [--trusted] [--stale N]
                                   [--gaps N] [--out DIR]
Run from the repository root after `cargo build --release`. With --schemas
and --jing, also validates every output with tools/validate.sh, in batches
of VALIDATE_BATCH files (one Jing run per schema for a whole batch). --bin runs
another converter binary, for example a copy of the previous build.

--all also converts every other INDD and INDT file under corpus/ (any
version, either byte order, without a usable IDML), validates the output
if schemas are given, and reports failures for those files separately.
--exclude leaves out files whose path under corpus/ starts with PREFIX.
Converter warnings are counted by kind over all converted files.

Pairs are trustworthy or stale (the IDML shows a different save than the
INDD; rule and evidence in docs/measurement.md). Totals are printed for all
pairs and for trustworthy pairs; --trusted limits the element tables,
shortfalls and details to trustworthy pairs, --stale N lists stale pairs.

Last comes the headline (docs/measurement.md): value coverage, the share
of reference values the conversion reproduces, over trustworthy pairs and
over all pairs; document scores (pairs with coverage >= 99 % and >= 99.9 %);
and the biggest gaps (--gaps N). pairs.tsv, gaps.tsv and gaps-all.tsv go to
--out (default target/compare), with values.tsv and values-all.tsv, which
list every key with its values and how many are reproduced.

--shortfalls N lists the N element types and attributes that fall short
(missing elements; wrong or missing attribute values; differing story
text) in the most documents.

Embedded file data (`Contents`) is compared by digest, so it is reported as
`md5:<hex> <length>` rather than as the full text.
"""

import argparse
import fnmatch
import functools
import hashlib
import os
import re
import subprocess
import sys
import tempfile
import xml.etree.ElementTree as ET
import zipfile
from collections import Counter, defaultdict
from concurrent.futures import ThreadPoolExecutor
from datetime import datetime, timedelta, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
BIN = ROOT / "target" / "release" / "indd"
# Converted files validated together in one tools/validate.sh run.
VALIDATE_BATCH = 600


def inventory():
    """Rows of corpus/inventory.tsv (tools/inventory.py), as dicts."""
    lines = (ROOT / "corpus" / "inventory.tsv").read_text().splitlines()
    cols = lines[0].split("\t")
    return [dict(zip(cols, line.split("\t"))) for line in lines[1:]]


def digest(indd, known):
    """SHA-256 of a corpus file, from the inventory if it lists the file."""
    path = str(indd.relative_to(ROOT / "corpus"))
    if path in known:
        return known[path]
    return hashlib.sha256(indd.read_bytes()).hexdigest()


def pairs(limit, substr, exclude, seen, known):
    """Corpus files with a same-version reference IDML, without duplicates.
    Adds the digest of each file to `seen`."""
    out = []
    for row in inventory():
        path, order, ver, dom = row["path"], row["order"], row["version"], row["idml_dom"]
        if order != "LE" or not dom or ver.split(".")[0] != dom.split(".")[0]:
            continue
        if substr and substr not in path or any(path.startswith(e) for e in exclude):
            continue
        indd = ROOT / "corpus" / path
        d = digest(indd, known)
        if d in seen:
            continue
        seen.add(d)
        idml = row.get("idml") or str(Path(path).with_suffix(".idml"))
        out.append((indd, ROOT / "corpus" / idml))
    return out[:limit] if limit else out


def unpaired(substr, exclude, seen, known):
    """Every other INDD and INDT file under corpus/, without duplicates."""
    out = []
    for indd in sorted((ROOT / "corpus").rglob("*")):
        if indd.suffix.lower() not in (".indd", ".indt") or not indd.is_file():
            continue
        path = str(indd.relative_to(ROOT / "corpus"))
        if substr and substr not in path or any(path.startswith(e) for e in exclude):
            continue
        d = digest(indd, known)
        if d in seen:
            continue
        seen.add(d)
        out.append(indd)
    return out


def warning_kind(msg):
    """A warning with its numbers and quoted names replaced, for counting."""
    msg = re.sub(r'"[^"]*"', '"…"', msg)
    msg = re.sub(r"\b0x[0-9a-fA-F]+\b", "#", msg)
    return re.sub(r"\b\d+(\.\d+)?\b", "#", msg)


XMP_NS = {"rdf": "http://www.w3.org/1999/02/22-rdf-syntax-ns#",
          "xmp": "http://ns.adobe.com/xap/1.0/"}
# A pair is stale if the INDD was saved more than this long after the
# IDML's metadata date (docs/measurement.md).
STALE_AFTER = timedelta(hours=1)


def modify_date(packet):
    """xmp:ModifyDate of the document in an XMP packet, or None."""
    if not packet:
        return None
    start = packet.find(b"<x:xmpmeta")
    end = packet.rfind(b"</x:xmpmeta>")
    if start < 0 or end < 0:
        return None
    try:
        meta = ET.fromstring(packet[start:end + len(b"</x:xmpmeta>")])
    except ET.ParseError:
        return None
    key = "{%s}ModifyDate" % XMP_NS["xmp"]
    for desc in meta.findall("rdf:RDF/rdf:Description", XMP_NS):
        text = desc.get(key) or desc.findtext(key)
        if text:
            try:
                d = datetime.fromisoformat(text.strip().replace("Z", "+00:00"))
            except ValueError:
                return None
            # Without a time zone, compare clock times.
            return d.replace(tzinfo=d.tzinfo or timezone.utc)
    return None


def idml_modify_date(idml):
    """xmp:ModifyDate in an IDML package's META-INF/metadata.xml, or None."""
    try:
        with zipfile.ZipFile(idml) as z:
            return modify_date(z.read("META-INF/metadata.xml"))
    except (KeyError, zipfile.BadZipFile, OSError):
        return None


def stale_reasons(indd_date, idml_date, indd_uids, ref_el, ref_st, our_st):
    """Why a pair's IDML may not show the same save as its INDD (empty if
    there is no sign of that). See docs/measurement.md for the evidence."""
    out = []
    if indd_date is None or idml_date is None:
        out.append("no ModifyDate")
    elif indd_date < idml_date:
        out.append("IDML modified after the INDD")
    elif indd_date - idml_date > STALE_AFTER:
        out.append("INDD saved over an hour after the IDML")
    ref_uids = {int(s[1:], 16) for _, s in ref_el if re.fullmatch(r"u[0-9a-f]+", s)}
    if indd_uids is not None and ref_uids - indd_uids:
        out.append("IDML objects not in the INDD")
    if set(our_st) - set(ref_st):
        out.append("INDD stories not in the IDML")
    return out


def validate(outs, args):
    """Schema errors of outputs, as {path: [messages]} (valid outputs are
    left out). Runs tools/validate.sh once over all of them; it batches the
    Jing runs."""
    errs = defaultdict(list)
    if not outs:
        return errs
    env = dict(os.environ, VALIDATE_JOBS=str(max(1, args.jobs)))
    v = subprocess.run(
        [ROOT / "tools" / "validate.sh", *map(str, outs), args.schemas, args.jing],
        capture_output=True, text=True, env=env)
    for line in v.stdout.splitlines():
        path, _, msg = line.partition("\t")
        errs[path].append(msg)
    if v.returncode not in (0, 1) or v.returncode == 1 and not errs or "jing" in errs:
        raise SystemExit(f"tools/validate.sh failed: {v.stderr.strip()} {errs.get('jing')}")
    return errs


def text_ranges(story):
    """Map text offset -> attributes of the paragraph and character range
    starting there, with keys prefixed PSR. and CSR."""
    out = {}
    pos = 0
    for psr in story.findall("ParagraphStyleRange"):
        pa = {"PSR." + k: v for k, v in props(psr).items()}
        for csr in psr.findall("CharacterStyleRange"):
            start = pos
            for el in csr:
                if el.tag == "Content":
                    pos += len((el.text or "").encode("utf-16-le")) // 2
                elif el.tag != "Properties":
                    pos += 1
            if pos > start:
                ca = {"CSR." + k: v for k, v in props(csr).items()}
                out[start] = {**pa, **ca}
    return out


def load(path):
    """Map Self -> element, plus story texts, from an IDML package."""
    z = zipfile.ZipFile(path)
    elements = {}
    stories = {}
    ranges = {}
    for name in z.namelist():
        if not name.endswith(".xml"):
            continue
        try:
            root = ET.fromstring(z.read(name))
        except ET.ParseError as e:
            raise SystemExit(f"{path}: {name}: {e}")
        for el in root.iter():
            s = el.get("Self")
            if s is not None:
                elements.setdefault((el.tag, s), el)
                # Children without Self (TextFramePreference, ...) are
                # compared as "<parent tag>/<child tag>" of the parent.
                for ch in el:
                    if ch.get("Self") is not None or ch.tag == "Properties":
                        continue
                    if len(ch.attrib):
                        elements.setdefault((f"{el.tag}/{ch.tag}", s), ch)
                    else:
                        # A container without attributes (TransparencySetting):
                        # compare its children as "<parent>/<child>/<grandchild>".
                        for g in ch:
                            if g.get("Self") is None and len(g.attrib):
                                elements.setdefault((f"{el.tag}/{ch.tag}/{g.tag}", s), g)
        if name == "Resources/Preferences.xml":
            for el in root:
                elements.setdefault((el.tag, "Preferences"), el)
        if name.startswith("Stories/"):
            for st in root.iter("Story"):
                stories[st.get("Self")] = story_text(st)
                ranges[st.get("Self")] = text_ranges(st)
    return elements, stories, ranges


def story_text(story):
    parts = []
    for el in story.iter():
        if el.tag == "Content":
            parts.append(el.text or "")
        elif el.tag == "Br":
            parts.append("\n")
    return "".join(parts)


def norm(v):
    """Normalise numbers so 1 == 1.0 and tiny float noise is ignored."""
    def fix(tok):
        try:
            f = float(tok)
        except ValueError:
            return tok
        return f"{f:.6g}"
    return " ".join(fix(t) for t in v.split(" "))


def canon(el):
    """A structured property (TabList, AllNestedStyles, ...) as one string:
    each leaf as tag=text with numbers normalised, in document order."""
    if len(el) == 0 and el.get("type") != "list":
        return f"{el.tag}={norm(el.text or '')}"
    return el.tag + "[" + ",".join(canon(c) for c in el) + "]"


def props(el):
    """Attributes plus <Properties> children (as P.Name)."""
    a = dict(el.attrib)
    p = el.find("Properties")
    if p is not None:
        for c in p:
            if c.get("type") == "list":
                a["P." + c.tag] = canon(c)
            elif len(c) == 0:
                text = c.text or ""
                extra = sorted((k, v) for k, v in c.attrib.items() if k != "type")
                if extra:
                    text = " ".join(f"{k}={v}" for k, v in extra)
                if c.tag == "Contents":
                    n = norm(text)
                    text = f"md5:{hashlib.md5(n.encode()).hexdigest()} {len(n)}"
                a["P." + c.tag] = text
    return a


# Values left out of value coverage because they cannot be derived from an
# INDD file: (tag pattern, attribute or P.<Properties child>, reason), tag
# patterns as in fnmatch. docs/measurement.md explains each entry.
EXCLUDED = [
    ("Font", "Status", "whether the font is installed on the exporting computer"),
    ("Document/{http://ns.adobe.com/AdobeInDesign/idml/1.0/packaging}*", "src",
     "file names of the package parts, chosen by the IDML writer"),
    ("DocumentUser", "P.UserColor",
     "colour the exporting InDesign gives each user, not the colour in the INDD"),
]


@functools.cache
def excluded(tag, key):
    return any(k == key and fnmatch.fnmatchcase(tag, t) for t, k, _ in EXCLUDED)


def tree(el):
    """An element with its attributes, text and descendants as one string,
    numbers normalised: the value of a structured <Properties> child such as
    PathGeometry."""
    attrs = ",".join(f"{k}={norm(v)}" for k, v in sorted(el.attrib.items()) if k != "type")
    kids = ",".join(tree(c) for c in el)
    return f"{el.tag}({attrs})[{kids}]{norm((el.text or '').strip())}"


def values(tag, el):
    """The values of an element that value coverage counts: attributes and
    <Properties> children, without Self and without EXCLUDED values."""
    a = props(el)
    p = el.find("Properties")
    if p is not None:
        for c in p:
            if c.get("type") != "list" and len(c):
                a["P." + c.tag] = tree(c)
    return {k: v for k, v in a.items() if k != "Self" and not excluded(tag, k)}


# Gap keys for values that could not be compared one by one.
NO_ELEMENT = "(element not produced)"
NO_TEXT = "(story text differs)"
ELEMENT = "(element)"


class Stats:
    """Comparison counts over a set of pairs."""

    def __init__(self):
        self.docs = 0
        # Value coverage: reference values, and those reproduced exactly.
        self.values = 0
        self.reproduced = 0
        # (tag, key) -> values wrong / missing, and documents affected.
        self.gap_wrong = Counter()
        self.gap_missing = Counter()
        self.gap_docs = Counter()
        self.gap_total = Counter()
        # (pair, reproduced, values) for each pair.
        self.pairs = []
        self.found = Counter()
        self.total = Counter()
        self.attr_ok = defaultdict(Counter)
        self.story_ok = Counter()
        self.short_docs = Counter()  # (kind, tag, attribute) -> documents affected

    def add(self, other):
        self.docs += other.docs
        self.values += other.values
        self.reproduced += other.reproduced
        for c in ("gap_wrong", "gap_missing", "gap_docs", "gap_total"):
            getattr(self, c).update(getattr(other, c))
        self.pairs += other.pairs
        self.found.update(other.found)
        self.total.update(other.total)
        for k, c in other.attr_ok.items():
            self.attr_ok[k].update(c)
        self.story_ok.update(other.story_ok)
        self.short_docs.update(other.short_docs)

    def coverage(self):
        return self.reproduced / self.values if self.values else 1.0

    def quantile(self, q):
        """Value coverage of the pair at quantile q (0 = lowest)."""
        cov = sorted(ok / n if n else 1.0 for _, ok, n in self.pairs)
        return cov[min(len(cov) - 1, int(q * len(cov)))] if cov else 1.0

    def document_score(self, at_least):
        """Pairs whose value coverage is at least `at_least`."""
        return sum(1 for _, ok, n in self.pairs if n == 0 or ok / n >= at_least)

    def gaps(self):
        """(tag, key), documents, values wrong, values missing; most
        documents first, then most values."""
        rows = [(k, d, self.gap_wrong[k], self.gap_missing[k]) for k, d in self.gap_docs.items()]
        return sorted(rows, key=lambda r: (-r[1], -(r[2] + r[3]), r[0]))

    def story_line(self):
        st = sum(self.story_ok.values())
        return (f"{self.story_ok['ok']}/{st} exact, {self.story_ok['wrong']} differ, "
                f"{self.story_ok['missing']} missing")


def compare_pair(name, ref, ours, examples, show):
    """Stats of one pair: `ref` and `ours` are load() results. Adds up to
    `show` mismatches per attribute to `examples`."""
    ref_el, ref_st, ref_rg = ref
    our_el, our_st, our_rg = ours
    p = Stats()
    p.docs = 1
    short = set()
    for (tag, s), el in ref_el.items():
        p.total[tag] += 1
        mine = our_el.get((tag, s))
        if mine is None:
            short.add(("element", tag, ""))
            continue
        p.found[tag] += 1
        mine = props(mine)
        for k, v in props(el).items():
            if k == "Self":
                continue
            if k not in mine:
                p.attr_ok[(tag, k)]["missing"] += 1
                short.add(("attribute", tag, k))
            elif norm(mine[k]) == norm(v):
                p.attr_ok[(tag, k)]["ok"] += 1
            else:
                p.attr_ok[(tag, k)]["wrong"] += 1
                short.add(("attribute", tag, k))
                if len(examples[(tag, k)]) < show:
                    examples[(tag, k)].append((name, s, v, mine[k]))
    for sid, rgs in ref_rg.items():
        if our_st.get(sid) != ref_st.get(sid):
            continue
        mine = our_rg.get(sid, {})
        for start, attrs in rgs.items():
            p.total["TextRange"] += 1
            if start not in mine:
                short.add(("element", "TextRange", ""))
                continue
            p.found["TextRange"] += 1
            for k, v in attrs.items():
                o = mine[start].get(k)
                if o is None:
                    p.attr_ok[("TextRange", k)]["missing"] += 1
                    short.add(("attribute", "TextRange", k))
                elif norm(o) == norm(v):
                    p.attr_ok[("TextRange", k)]["ok"] += 1
                else:
                    p.attr_ok[("TextRange", k)]["wrong"] += 1
                    short.add(("attribute", "TextRange", k))
                    if len(examples[("TextRange", k)]) < show:
                        examples[("TextRange", k)].append((name, sid, start, v, o))
    for sid, text in ref_st.items():
        if sid not in our_st:
            p.story_ok["missing"] += 1
            short.add(("element", "Story", ""))
        elif our_st[sid] == text:
            p.story_ok["ok"] += 1
        else:
            p.story_ok["wrong"] += 1
            short.add(("text", "Story", ""))
            if len(examples[("Story", "text")]) < max(show, 3):
                examples[("Story", "text")].append((name, sid, text[:80], our_st[sid][:80]))
    p.short_docs.update(short)
    value_coverage(p, ref, ours)
    return p


def value_coverage(p, ref, ours):
    """Count reference values and the reproduced ones into `p`: element
    presence, attribute and <Properties> values, story text, and the start
    and attributes of each text range."""
    ref_el, ref_st, ref_rg = ref
    our_el, our_st, our_rg = ours
    total = Counter()
    wrong = Counter()
    missing = Counter()

    def compare(tag, theirs, mine):
        for k, v in theirs.items():
            key = (tag, k)
            total[key] += 1
            o = mine.get(k)
            if o is None:
                missing[key] += 1
            elif norm(o) != norm(v):
                wrong[key] += 1

    for (tag, s), el in ref_el.items():
        theirs = values(tag, el)
        mine = our_el.get((tag, s))
        if mine is None:
            n = 1 + len(theirs)
            total[(tag, NO_ELEMENT)] += n
            missing[(tag, NO_ELEMENT)] += n
            continue
        total[(tag, ELEMENT)] += 1
        compare(tag, theirs, values(tag, mine))
    for sid, text in ref_st.items():
        key = ("Story", "(text)")
        total[key] += 1
        if sid not in our_st:
            missing[key] += 1
        elif our_st[sid] != text:
            wrong[key] += 1
    for sid, rgs in ref_rg.items():
        if our_st.get(sid) != ref_st.get(sid):
            n = sum(1 + len(a) for a in rgs.values())
            total[("TextRange", NO_TEXT)] += n
            missing[("TextRange", NO_TEXT)] += n
            continue
        mine = our_rg.get(sid, {})
        for start, attrs in rgs.items():
            if start not in mine:
                n = 1 + len(attrs)
                total[("TextRange", NO_ELEMENT)] += n
                missing[("TextRange", NO_ELEMENT)] += n
                continue
            total[("TextRange", ELEMENT)] += 1
            compare("TextRange", attrs, mine[start])
    n = sum(total.values())
    bad = sum(wrong.values()) + sum(missing.values())
    p.values += n
    p.reproduced += n - bad
    p.gap_total.update(total)
    p.gap_wrong.update(wrong)
    p.gap_missing.update(missing)
    p.gap_docs.update(k for k in total if wrong[k] or missing[k])


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--limit", type=int, default=0)
    ap.add_argument("--detail", action="append", default=[],
                    help="print attribute table for this tag (repeatable)")
    ap.add_argument("--file", help="only pairs whose path contains this")
    ap.add_argument("--show", type=int, default=0, help="show N mismatches per attribute")
    ap.add_argument("--schemas", help="IDML RelaxNG schema directory (validate output)")
    ap.add_argument("--jing", help="directory with jing.jar, isorelax.jar, saxon.jar")
    ap.add_argument("--bin", default=str(BIN), help="converter binary")
    ap.add_argument("--all", action="store_true",
                    help="also convert and validate files without a reference IDML")
    ap.add_argument("--exclude", action="append", default=[],
                    help="leave out files whose path under corpus/ starts with this (repeatable)")
    ap.add_argument("--warnings", type=int, default=15,
                    help="number of warning kinds to list")
    ap.add_argument("--shortfalls", type=int, default=0,
                    help="rank the N biggest shortfalls by documents affected")
    ap.add_argument("--trusted", action="store_true",
                    help="element tables, shortfalls and details over trustworthy pairs only")
    ap.add_argument("--stale", type=int, default=0,
                    help="list N stale pairs with their reasons")
    ap.add_argument("--gaps", type=int, default=30,
                    help="print the N biggest value coverage gaps")
    ap.add_argument("--out", default=str(ROOT / "target" / "compare"),
                    help="directory for pairs.tsv and gaps*.tsv (default target/compare)")
    ap.add_argument("--jobs", type=int, default=os.cpu_count() or 1,
                    help="conversions and schema validations run in parallel")
    args = ap.parse_args()
    check = bool(args.schemas and args.jing)

    every = Stats()     # all pairs
    trusted = Stats()   # pairs without a sign of staleness
    stale = []          # (name, reasons)
    pair_rows = []      # (name, Stats, stale reasons) per pair
    examples = defaultdict(list)
    failures = []
    warnings = Counter()
    warned_files = Counter()
    seen = set()
    known = {r["path"]: r["sha256"] for r in inventory() if r.get("sha256")}
    todo = pairs(args.limit, args.file, args.exclude, seen, known)
    others = unpaired(args.file, args.exclude, seen, known) if args.all else []
    other_failures = []
    pool = ThreadPoolExecutor(max(1, args.jobs))

    def convert(indd, out):
        """Convert one file; runs in the pool."""
        return subprocess.run([args.bin, "convert", indd, out], capture_output=True, text=True)

    def convert_pair(indd, idml, out):
        """Convert one paired file and read what stale_reasons needs: the
        INDD's UIDs and the ModifyDate of both files; runs in the pool."""
        r = convert(indd, out)
        x = subprocess.run([args.bin, "xmp", indd], capture_output=True)
        u = subprocess.run([args.bin, "uids", indd], capture_output=True, text=True)
        uids = ({int(line.split("\t")[0]) for line in u.stdout.splitlines()}
                if u.returncode == 0 else None)
        dates = (modify_date(x.stdout) if x.returncode == 0 else None, idml_modify_date(idml))
        return r, dates, uids

    def count_warnings(r):
        kinds = set()
        for line in r.stderr.splitlines():
            if line.startswith("warning: "):
                kind = warning_kind(line[len("warning: "):])
                warnings[kind] += 1
                kinds.add(kind)
        for kind in kinds:
            warned_files[kind] += 1

    other_invalid = []
    invalid = []
    with tempfile.TemporaryDirectory() as tmp:
        jobs = [pool.submit(convert, indd, Path(tmp) / f"other{n}.idml")
                for n, indd in enumerate(others)]
        jobs += [pool.submit(convert_pair, indd, idml, Path(tmp) / f"pair{n}.idml")
                 for n, (indd, idml) in enumerate(todo)]
        # Converted files waiting for validation: (output, name, list of
        # invalid files it belongs to). They are validated in batches, which
        # bounds the space the unpacked outputs take.
        to_check = []

        def flush(at_least):
            if len(to_check) < at_least:
                return
            errs = validate([out for out, _, _ in to_check], args)
            for out, name, bad in to_check:
                if errs.get(str(out)):
                    bad.append((name, errs[str(out)]))
                out.unlink(missing_ok=True)
            to_check.clear()
        for n, indd in enumerate(others):
            r = jobs[n].result()
            count_warnings(r)
            out = Path(tmp) / f"other{n}.idml"
            if r.returncode != 0:
                other_failures.append((indd.name, r.stderr.strip()))
            elif check:
                to_check.append((out, indd.name, other_invalid))
                flush(VALIDATE_BATCH)
                continue
            out.unlink(missing_ok=True)
        for n, (indd, idml) in enumerate(todo):
            out = Path(tmp) / f"pair{n}.idml"
            r, (indd_date, idml_date), indd_uids = jobs[len(others) + n].result()
            count_warnings(r)
            if r.returncode != 0:
                failures.append((indd.name, r.stderr.strip()))
                continue
            if check:
                to_check.append((out, indd.name, invalid))
            ref = load(idml)
            ours = load(out)
            reasons = stale_reasons(indd_date, idml_date, indd_uids, ref[0], ref[1], ours[1])
            p = compare_pair(indd.name, ref, ours, examples, args.show)
            rel = str(indd.relative_to(ROOT / "corpus"))
            p.pairs = [(rel, p.reproduced, p.values)]
            pair_rows.append((rel, p, reasons))
            every.add(p)
            if reasons:
                stale.append((rel, reasons))
            else:
                trusted.add(p)
            if check:
                flush(VALIDATE_BATCH)
            else:
                out.unlink(missing_ok=True)
        pool.shutdown()
        flush(1)

    print(f"conversion failures: {len(failures)} of {len(todo)} paired files")
    for name, err in failures[:10]:
        print(f"  {name}: {err}")
    if check:
        print(f"schema validation failures: {len(invalid)}")
        for name, errs in invalid[:10]:
            print(f"  {name}: {len(errs)} errors, {errs[:3]}")
    if args.all:
        print(f"files without a reference: {len(others)}, "
              f"conversion failures: {len(other_failures)}")
        for name, err in other_failures[:10]:
            print(f"  {name}: {err}")
        if check:
            print(f"files without a reference: schema validation failures: {len(other_invalid)}")
            for name, errs in other_invalid[:10]:
                print(f"  {name}: {len(errs)} errors, {errs[:3]}")
    print(f"warnings: {sum(warnings.values())} (count, files, kind)")
    for kind, count in warnings.most_common(args.warnings):
        print(f"  {count:6} {warned_files[kind]:4}  {kind[:150]}")
    print(f"story text: {every.story_line()}")
    reasons = Counter(r for _, rs in stale for r in rs)
    print(f"trustworthy pairs: {trusted.docs} of {every.docs}; stale: {len(stale)} "
          f"({', '.join(f'{r}: {n}' for r, n in reasons.most_common())})")
    print(f"trustworthy pairs: story text: {trusted.story_line()}")
    for name, rs in stale[:args.stale]:
        print(f"  stale: {name}: {'; '.join(rs)}")
    st = trusted if args.trusted else every
    which = "trustworthy pairs" if args.trusted else "all pairs"
    print(f"\nelements, {which} (produced / in reference):")
    for tag, n in st.total.most_common():
        print(f"  {tag:34} {st.found[tag]:6} / {n:6}")
    if args.shortfalls:
        print(f"\nshortfalls by documents affected (of {st.docs} {which}):")
        def instances(kind, tag, k):
            """How many elements, values or stories fall short."""
            if kind == "attribute":
                return st.attr_ok[(tag, k)]["wrong"] + st.attr_ok[(tag, k)]["missing"]
            if kind == "element":
                return st.total[tag] - st.found[tag]
            return st.story_ok["wrong"]
        ranked = sorted(st.short_docs.items(), key=lambda kv: (-kv[1], -instances(*kv[0]), kv[0]))
        for (kind, tag, k), n in ranked[:args.shortfalls]:
            what = f"{st.story_ok['wrong']} stories differ"
            if kind == "attribute":
                c = st.attr_ok[(tag, k)]
                what = f"{c['wrong']} wrong, {c['missing']} missing of {sum(c.values())}"
            elif kind == "element":
                what = f"{st.total[tag] - st.found[tag]} of {st.total[tag]} not produced"
            print(f"  {n:5}  {tag + (' ' + k if k else ''):56} {what}")
    for tag in args.detail:
        print(f"\nattributes of {tag}, {which} (ok / wrong / missing):")
        rows = [(k, c) for (t, k), c in st.attr_ok.items() if t == tag]
        for k, c in sorted(rows, key=lambda kc: -sum(kc[1].values())):
            print(f"  {k:40} {c['ok']:6} {c['wrong']:6} {c['missing']:6}")
            for ex in examples[(tag, k)]:
                print(f"      {ex}")
    for ex in examples[("Story", "text")]:
        print("  story mismatch:", ex)
    headline(args, every, trusted, pair_rows)


def gap_name(key):
    tag, k = key
    return f"{tag} {k}"


def write_gaps(path, st):
    with open(path, "w") as f:
        f.write("tag\tkey\tdocuments\twrong\tmissing\tvalues\n")
        for key, docs, w, m in st.gaps():
            f.write(f"{key[0]}\t{key[1]}\t{docs}\t{w}\t{m}\t{st.gap_total[key]}\n")


def write_values(path, st):
    """Every key with its values and how many are reproduced, for checking
    that a change reproduces no fewer values of any key."""
    with open(path, "w") as f:
        f.write("tag\tkey\tvalues\treproduced\twrong\tmissing\n")
        for key in sorted(st.gap_total):
            w, m = st.gap_wrong[key], st.gap_missing[key]
            f.write(f"{key[0]}\t{key[1]}\t{st.gap_total[key]}\t{st.gap_total[key] - w - m}"
                    f"\t{w}\t{m}\n")


def headline(args, every, trusted, pair_rows):
    """Print value coverage, document scores and the biggest gaps; write
    pairs.tsv, gaps.tsv (trustworthy pairs) and gaps-all.tsv to --out."""
    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    with open(out / "pairs.tsv", "w") as f:
        f.write("path\ttrusted\tcoverage\treproduced\tvalues\tstale reasons\n")
        for name, p, reasons in sorted(pair_rows, key=lambda r: r[1].coverage()):
            f.write(f"{name}\t{not reasons}\t{p.coverage():.5f}\t{p.reproduced}\t{p.values}"
                    f"\t{'; '.join(reasons)}\n")
    write_gaps(out / "gaps.tsv", trusted)
    write_gaps(out / "gaps-all.tsv", every)
    write_values(out / "values.tsv", trusted)
    write_values(out / "values-all.tsv", every)
    print("\nheadline (docs/measurement.md):")
    for label, st in (("trustworthy pairs", trusted), ("all pairs", every)):
        n = st.docs or 1
        print(f"  {label}: value coverage {st.coverage():.2%} "
              f"({st.reproduced} of {st.values} values); documents at >= 99 %: "
              f"{st.document_score(0.99)}/{st.docs} ({st.document_score(0.99) / n:.1%}), "
              f">= 99.9 %: {st.document_score(0.999)}/{st.docs} "
              f"({st.document_score(0.999) / n:.1%})")
        print(f"    per pair: lowest {st.quantile(0):.1%}, 10th percentile {st.quantile(0.1):.1%}, "
              f"median {st.quantile(0.5):.1%}, 90th percentile {st.quantile(0.9):.1%}, "
              f"highest {st.quantile(1):.1%}")
    rows = trusted.gaps()
    print(f"\nvalue gaps, trustworthy pairs, by documents affected "
          f"(documents, values wrong, missing, of values; all in {out / 'gaps.tsv'}):")
    for key, docs, w, m in rows[:args.gaps]:
        print(f"  {docs:5} {w:7} {m:7} {trusted.gap_total[key]:8}  {gap_name(key)[:90]}")
    print("\nvalue gaps, trustworthy pairs, by values (documents, values wrong, missing, of values):")
    for key, docs, w, m in sorted(rows, key=lambda r: -(r[2] + r[3]))[:args.gaps // 2]:
        print(f"  {docs:5} {w:7} {m:7} {trusted.gap_total[key]:8}  {gap_name(key)[:90]}")


if __name__ == "__main__":
    main()
