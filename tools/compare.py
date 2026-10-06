#!/usr/bin/env python3
"""Compare converter output with reference IDML files.

For every INDD file in the corpus that has a sibling IDML from the same
major version, convert it with `target/release/indd` and compare the result
with the sibling. Reports, per element type: how many referenced elements
we produce (matched by Self), and per attribute how often our value
matches. Also reports story text agreement.

Usage: python3 -I tools/compare.py [--limit N] [--detail TAG] [--file SUBSTR]
Run from the repository root after `cargo build --release`.
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


def load(path):
    """Map Self -> element, plus story texts, from an IDML package."""
    z = zipfile.ZipFile(path)
    elements = {}
    stories = {}
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
        if name.startswith("Stories/"):
            for st in root.iter("Story"):
                stories[st.get("Self")] = story_text(st)
    return elements, stories


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


def props(el):
    """Attributes plus <Properties> children (as P.Name)."""
    a = dict(el.attrib)
    p = el.find("Properties")
    if p is not None:
        for c in p:
            if len(c) == 0:
                a["P." + c.tag] = c.text or ""
    return a


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--limit", type=int, default=0)
    ap.add_argument("--detail", help="print attribute table for this tag")
    ap.add_argument("--file", help="only pairs whose path contains this")
    ap.add_argument("--show", type=int, default=0, help="show N mismatches per attribute")
    args = ap.parse_args()

    found = Counter()
    total = Counter()
    attr_ok = defaultdict(Counter)
    examples = defaultdict(list)
    story_ok = Counter()
    failures = []
    with tempfile.TemporaryDirectory() as tmp:
        out = Path(tmp) / "out.idml"
        for indd, idml in pairs(args.limit, args.file):
            r = subprocess.run([BIN, "convert", indd, out], capture_output=True, text=True)
            if r.returncode != 0:
                failures.append((indd.name, r.stderr.strip()))
                continue
            ref_el, ref_st = load(idml)
            our_el, our_st = load(out)
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
    st = sum(story_ok.values())
    print(f"story text: {story_ok['ok']}/{st} exact, {story_ok['wrong']} differ, {story_ok['missing']} missing")
    print("\nelements (produced / in reference):")
    for tag, n in total.most_common():
        print(f"  {tag:34} {found[tag]:6} / {n:6}")
    if args.detail:
        print(f"\nattributes of {args.detail} (ok / wrong / missing):")
        rows = [(k, c) for (t, k), c in attr_ok.items() if t == args.detail]
        for k, c in sorted(rows, key=lambda kc: -sum(kc[1].values())):
            print(f"  {k:40} {c['ok']:6} {c['wrong']:6} {c['missing']:6}")
            for ex in examples[(args.detail, k)]:
                print(f"      {ex}")
    for ex in examples[("Story", "text")]:
        print("  story mismatch:", ex)


if __name__ == "__main__":
    main()
