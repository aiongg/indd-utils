#!/usr/bin/env python3
"""Compare converter output with reference IDML files.

For every INDD file in the corpus that has a sibling IDML from the same
major version, convert it with `target/release/indd` and compare the result
with the sibling. Reports, per element type: how many referenced elements
we produce (matched by Self), and per attribute how often our value
matches. Also reports story text agreement.

Usage: python3 -I tools/compare.py [--limit N] [--detail TAG]... [--file SUBSTR]
                                   [--schemas DIR --jing DIR] [--bin PATH]
Run from the repository root after `cargo build --release`. With --schemas
and --jing, also validates every output with tools/validate.sh. --bin runs
another converter binary, for example a copy of the previous build.

Embedded file data (`Contents`) is compared by digest, so it is reported as
`md5:<hex> <length>` rather than as the full text.
"""

import argparse
import hashlib
import re
import subprocess
import sys
import tempfile
import xml.etree.ElementTree as ET
import zipfile
from collections import Counter, defaultdict
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
BIN = ROOT / "target" / "release" / "indd"


def pairs(limit, substr):
    rows = (ROOT / "corpus" / "inventory.tsv").read_text().splitlines()[1:]
    seen = set()
    out = []
    for row in rows:
        path, _size, _valid, order, ver, _creator, dom = row.split("\t")
        if order != "LE" or not dom or ver.split(".")[0] != dom.split(".")[0]:
            continue
        if substr and substr not in path:
            continue
        indd = ROOT / "corpus" / path
        digest = hashlib.md5(indd.read_bytes()).hexdigest()
        if digest in seen:
            continue
        seen.add(digest)
        out.append((indd, indd.with_suffix(".idml")))
    return out[:limit] if limit else out


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
                    if ch.get("Self") is None and ch.tag not in ("Properties",) and len(ch.attrib):
                        elements.setdefault((f"{el.tag}/{ch.tag}", s), ch)
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
    args = ap.parse_args()

    found = Counter()
    total = Counter()
    attr_ok = defaultdict(Counter)
    examples = defaultdict(list)
    story_ok = Counter()
    failures = []
    invalid = []
    with tempfile.TemporaryDirectory() as tmp:
        out = Path(tmp) / "out.idml"
        for indd, idml in pairs(args.limit, args.file):
            r = subprocess.run([args.bin, "convert", indd, out], capture_output=True, text=True)
            if r.returncode != 0:
                failures.append((indd.name, r.stderr.strip()))
                continue
            if args.schemas and args.jing:
                v = subprocess.run(
                    [ROOT / "tools" / "validate.sh", out, args.schemas, args.jing],
                    capture_output=True, text=True)
                if v.returncode != 0:
                    invalid.append((indd.name, v.stdout.strip().splitlines()[:3]))
            ref_el, ref_st, ref_rg = load(idml)
            our_el, our_st, our_rg = load(out)
            for (tag, s), el in ref_el.items():
                total[tag] += 1
                mine = our_el.get((tag, s))
                if mine is None:
                    continue
                found[tag] += 1
                ours = props(mine)
                for k, v in props(el).items():
                    if k == "Self":
                        continue
                    if k not in ours:
                        attr_ok[(tag, k)]["missing"] += 1
                    elif norm(ours[k]) == norm(v):
                        attr_ok[(tag, k)]["ok"] += 1
                    else:
                        attr_ok[(tag, k)]["wrong"] += 1
                        if len(examples[(tag, k)]) < args.show:
                            examples[(tag, k)].append((indd.name, s, v, ours[k]))
            for sid, rgs in ref_rg.items():
                if our_st.get(sid) != ref_st.get(sid):
                    continue
                mine = our_rg.get(sid, {})
                for start, attrs in rgs.items():
                    total["TextRange"] += 1
                    if start not in mine:
                        continue
                    found["TextRange"] += 1
                    for k, v in attrs.items():
                        o = mine[start].get(k)
                        if o is None:
                            attr_ok[("TextRange", k)]["missing"] += 1
                        elif norm(o) == norm(v):
                            attr_ok[("TextRange", k)]["ok"] += 1
                        else:
                            attr_ok[("TextRange", k)]["wrong"] += 1
                            if len(examples[("TextRange", k)]) < args.show:
                                examples[("TextRange", k)].append((indd.name, sid, start, v, o))
            for sid, text in ref_st.items():
                if sid not in our_st:
                    story_ok["missing"] += 1
                elif our_st[sid] == text:
                    story_ok["ok"] += 1
                else:
                    story_ok["wrong"] += 1
                    if len(examples[("Story", "text")]) < max(args.show, 3):
                        examples[("Story", "text")].append((indd.name, sid, text[:80], our_st[sid][:80]))

    print(f"conversion failures: {len(failures)}")
    for name, err in failures[:10]:
        print(f"  {name}: {err}")
    if args.schemas:
        print(f"schema validation failures: {len(invalid)}")
        for name, errs in invalid[:10]:
            print(f"  {name}: {errs}")
    st = sum(story_ok.values())
    print(f"story text: {story_ok['ok']}/{st} exact, {story_ok['wrong']} differ, {story_ok['missing']} missing")
    print("\nelements (produced / in reference):")
    for tag, n in total.most_common():
        print(f"  {tag:34} {found[tag]:6} / {n:6}")
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
