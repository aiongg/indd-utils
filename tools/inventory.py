#!/usr/bin/env python3
"""Inventory the local test corpus.

Walks a corpus directory, reads the header of every .indd file, finds a
sibling .idml with the same stem, and reads its DOMVersion. Prints one
tab-separated row per INDD file and a summary on stderr.

Usage: python3 -I tools/inventory.py corpus/ > corpus/inventory.tsv
"""

import re
import struct
import sys
import zipfile
from collections import Counter
from pathlib import Path

INDD_GUID = bytes.fromhex("0606edf5d81d46e5bd31efe7fe74b71d")
CREATOR_RE = re.compile(rb"<(?:xmp|xap):CreatorTool>(Adobe InDesign[^<]*)<")
DOM_RE = re.compile(rb'DOMVersion="([^"]+)"')


def indd_header(path: Path) -> dict:
    with path.open("rb") as f:
        head = f.read(0x40)
    if len(head) < 0x25 or head[:16] != INDD_GUID:
        return {"valid": False}
    order = head[0x18]
    endian = {1: "<", 2: ">"}.get(order)
    major = minor = None
    if endian:
        major, minor = struct.unpack_from(endian + "II", head, 0x1D)
    return {
        "valid": True,
        "kind": head[0x10:0x18].decode("ascii", "replace"),
        "order": {1: "LE", 2: "BE"}.get(order, f"?{order}"),
        "major": major,
        "minor": minor,
    }


def indd_creator(path: Path) -> str:
    # Embedded images carry their own XMP, so take only InDesign creators.
    found = CREATOR_RE.findall(path.read_bytes())
    return Counter(found).most_common(1)[0][0].decode() if found else ""


def idml_dom_version(path: Path) -> str:
    try:
        with zipfile.ZipFile(path) as z:
            m = DOM_RE.search(z.read("designmap.xml"))
            return m.group(1).decode() if m else "?"
    except (zipfile.BadZipFile, KeyError) as e:
        return f"error:{type(e).__name__}"


def main() -> None:
    root = Path(sys.argv[1])
    rows = []
    for indd in sorted(root.rglob("*")):
        if indd.suffix.lower() not in (".indd", ".indt") or not indd.is_file():
            continue
        h = indd_header(indd)
        idml = next(
            (p for p in indd.parent.iterdir()
             if p.suffix.lower() == ".idml" and p.stem == indd.stem),
            None,
        )
        rows.append({
            "path": str(indd.relative_to(root)),
            "bytes": indd.stat().st_size,
            "valid": h["valid"],
            "order": h.get("order", ""),
            "version": f"{h['major']}.{h['minor']}" if h.get("major") is not None else "",
            "creator": indd_creator(indd),
            "idml_dom": idml_dom_version(idml) if idml else "",
        })

    cols = ["path", "bytes", "valid", "order", "version", "creator", "idml_dom"]
    print("\t".join(cols))
    for r in rows:
        print("\t".join(str(r[c]) for c in cols))

    paired = [r for r in rows if r["idml_dom"]]
    print(f"{len(rows)} INDD files, {len(paired)} with a sibling IDML", file=sys.stderr)
    print("by header version (all / paired):", file=sys.stderr)
    allv = Counter(r["version"] for r in rows)
    pv = Counter(r["version"] for r in paired)
    for v in sorted(allv, key=lambda s: [int(x) for x in s.split(".")] if s else [-1]):
        print(f"  {v or 'invalid':>8}  {allv[v]:4} / {pv[v]:4}", file=sys.stderr)
    print("byte order:", dict(Counter(r["order"] for r in rows)), file=sys.stderr)


if __name__ == "__main__":
    main()
