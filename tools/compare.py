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
                                   [--shortfalls N]
Run from the repository root after `cargo build --release`. With --schemas
and --jing, also validates every output with tools/validate.sh, in batches
of VALIDATE_BATCH files (one Jing run per schema for a whole batch). --bin runs
another converter binary, for example a copy of the previous build.

--all also converts every other INDD and INDT file under corpus/ (any
version, either byte order, without a usable IDML), validates the output
if schemas are given, and reports failures for those files separately.
--exclude leaves out files whose path under corpus/ starts with PREFIX.
Converter warnings are counted by kind over all converted files.

--shortfalls N lists the N element types and attributes that fall short
(missing elements; wrong or missing attribute values; differing story
text) in the most documents.

Embedded file data (`Contents`) is compared by digest, so it is reported as
`md5:<hex> <length>` rather than as the full text.
"""

import argparse
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
    ap.add_argument("--jobs", type=int, default=os.cpu_count() or 1,
                    help="conversions and schema validations run in parallel")
    args = ap.parse_args()
    check = bool(args.schemas and args.jing)

    found = Counter()
    total = Counter()
    attr_ok = defaultdict(Counter)
    examples = defaultdict(list)
    story_ok = Counter()
    short_docs = Counter()  # (kind, tag, attribute) -> documents affected
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
        jobs += [pool.submit(convert, indd, Path(tmp) / f"pair{n}.idml")
                 for n, (indd, _) in enumerate(todo)]
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
            r = jobs[len(others) + n].result()
            count_warnings(r)
            if r.returncode != 0:
                failures.append((indd.name, r.stderr.strip()))
                continue
            if check:
                to_check.append((out, indd.name, invalid))
            ref_el, ref_st, ref_rg = load(idml)
            our_el, our_st, our_rg = load(out)
            short = set()
            for (tag, s), el in ref_el.items():
                total[tag] += 1
                mine = our_el.get((tag, s))
                if mine is None:
                    short.add(("element", tag, ""))
                    continue
                found[tag] += 1
                ours = props(mine)
                for k, v in props(el).items():
                    if k == "Self":
                        continue
                    if k not in ours:
                        attr_ok[(tag, k)]["missing"] += 1
                        short.add(("attribute", tag, k))
                    elif norm(ours[k]) == norm(v):
                        attr_ok[(tag, k)]["ok"] += 1
                    else:
                        attr_ok[(tag, k)]["wrong"] += 1
                        short.add(("attribute", tag, k))
                        if len(examples[(tag, k)]) < args.show:
                            examples[(tag, k)].append((indd.name, s, v, ours[k]))
            for sid, rgs in ref_rg.items():
                if our_st.get(sid) != ref_st.get(sid):
                    continue
                mine = our_rg.get(sid, {})
                for start, attrs in rgs.items():
                    total["TextRange"] += 1
                    if start not in mine:
                        short.add(("element", "TextRange", ""))
                        continue
                    found["TextRange"] += 1
                    for k, v in attrs.items():
                        o = mine[start].get(k)
                        if o is None:
                            attr_ok[("TextRange", k)]["missing"] += 1
                            short.add(("attribute", "TextRange", k))
                        elif norm(o) == norm(v):
                            attr_ok[("TextRange", k)]["ok"] += 1
                        else:
                            attr_ok[("TextRange", k)]["wrong"] += 1
                            short.add(("attribute", "TextRange", k))
                            if len(examples[("TextRange", k)]) < args.show:
                                examples[("TextRange", k)].append((indd.name, sid, start, v, o))
            for sid, text in ref_st.items():
                if sid not in our_st:
                    story_ok["missing"] += 1
                    short.add(("element", "Story", ""))
                elif our_st[sid] == text:
                    story_ok["ok"] += 1
                else:
                    story_ok["wrong"] += 1
                    short.add(("text", "Story", ""))
                    if len(examples[("Story", "text")]) < max(args.show, 3):
                        examples[("Story", "text")].append((indd.name, sid, text[:80], our_st[sid][:80]))
            short_docs.update(short)
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
    st = sum(story_ok.values())
    print(f"story text: {story_ok['ok']}/{st} exact, {story_ok['wrong']} differ, {story_ok['missing']} missing")
    print("\nelements (produced / in reference):")
    for tag, n in total.most_common():
        print(f"  {tag:34} {found[tag]:6} / {n:6}")
    if args.shortfalls:
        n_docs = len(todo) - len(failures)
        print(f"\nshortfalls by documents affected (of {n_docs}):")
        def instances(kind, tag, k):
            """How many elements, values or stories fall short."""
            if kind == "attribute":
                return attr_ok[(tag, k)]["wrong"] + attr_ok[(tag, k)]["missing"]
            if kind == "element":
                return total[tag] - found[tag]
            return story_ok["wrong"]
        ranked = sorted(short_docs.items(), key=lambda kv: (-kv[1], -instances(*kv[0]), kv[0]))
        for (kind, tag, k), n in ranked[:args.shortfalls]:
            what = f"{story_ok['wrong']} stories differ"
            if kind == "attribute":
                c = attr_ok[(tag, k)]
                what = f"{c['wrong']} wrong, {c['missing']} missing of {sum(c.values())}"
            elif kind == "element":
                what = f"{total[tag] - found[tag]} of {total[tag]} not produced"
            print(f"  {n:5}  {tag + (' ' + k if k else ''):56} {what}")
    for tag in args.detail:
        print(f"\nattributes of {tag} (ok / wrong / missing):")
        rows = [(k, c) for (t, k), c in attr_ok.items() if t == tag]
        for k, c in sorted(rows, key=lambda kc: -sum(kc[1].values())):
            print(f"  {k:40} {c['ok']:6} {c['wrong']:6} {c['missing']:6}")
            for ex in examples[(tag, k)]:
                print(f"      {ex}")
    for ex in examples[("Story", "text")]:
        print("  story mismatch:", ex)


if __name__ == "__main__":
    main()
