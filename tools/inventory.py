#!/usr/bin/env python3
"""Inventory the local test corpus.

Walks a corpus directory, reads the header of every .indd and .indt file,
finds the IDML and PDF of the same document, and reads the IDML's
DOMVersion. Prints one tab-separated row per INDD file and a summary on
stderr.

Usage: python3 -I tools/inventory.py corpus/ [--exclude PREFIX]... > corpus/inventory.tsv

Pairing. An IDML or PDF belongs to an INDD file when its name, without the
extension (and without a further `.indd` or `.indt`, as in `a.indt.idml`),
equals the INDD file's name without the extension, ignoring case:

- In the same folder.
- Otherwise, if a folder above the INDD file holds a `SOURCE.md` (one
  downloaded source), anywhere under that folder: the candidate whose
  folder shares the longest path with the INDD file's folder. A candidate
  that sits next to another INDD file of the same name belongs to that file
  and is not used.

`corpus/pairs-extra.tsv` (local, optional) adds pairings that the sources
state but the names do not show: lines of `indd path<TAB>idml or pdf path`,
relative to the corpus directory.

Files are identified by SHA-256. `dup_of` names the first INDD file (in path
order) with the same digest, so tools can skip duplicates.
"""

import argparse
import hashlib
import re
import struct
import sys
import zipfile
from collections import Counter, defaultdict
from pathlib import Path

INDD_GUID = bytes.fromhex("0606edf5d81d46e5bd31efe7fe74b71d")
CREATOR_RE = re.compile(rb"<(?:xmp|xap):CreatorTool>(Adobe InDesign[^<]*)<")
DOM_RE = re.compile(rb'DOMVersion="([^"]+)"')
INDD_EXT = (".indd", ".indt")
COUNTED_EXT = (".indd", ".indt", ".indb", ".idml", ".pdf")


def indd_header(path: Path) -> dict:
    with path.open("rb") as f:
        head = f.read(0x80)
    if len(head) >= 0x25 and head[:16] == INDD_GUID:
        base = 0
    elif len(head) >= 0x71 and head[0x5C:0x64] == b"DOCUMENT":
        # InDesign 1.x layout (docs/format/header.md).
        base = 0x4C
    else:
        return {"valid": False}
    order = head[base + 0x18]
    endian = {1: "<", 2: ">"}.get(order)
    major = minor = None
    if endian:
        major, minor = struct.unpack_from(endian + "II", head, base + 0x1D)
    return {
        "valid": base == 0,
        "kind": head[base + 0x10:base + 0x18].decode("ascii", "replace"),
        "order": {1: "LE", 2: "BE"}.get(order, f"?{order}"),
        "major": major,
        "minor": minor,
    }


def sha256(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as f:
        while chunk := f.read(1 << 20):
            h.update(chunk)
    return h.hexdigest()


def indd_creator(path: Path) -> str:
    # Embedded images carry their own XMP, so take only InDesign creators.
    found = CREATOR_RE.findall(path.read_bytes())
    return Counter(found).most_common(1)[0][0].decode() if found else ""


def idml_dom_version(path: Path) -> str:
    try:
        with zipfile.ZipFile(path) as z:
            m = DOM_RE.search(z.read("designmap.xml"))
            return m.group(1).decode() if m else "?"
    except (zipfile.BadZipFile, KeyError, OSError) as e:
        return f"error:{type(e).__name__}"


def key(path: Path) -> str:
    """Name without the extension, and without a further .indd/.indt."""
    stem = path.stem
    if Path(stem).suffix.lower() in INDD_EXT:
        stem = Path(stem).stem
    return stem.lower()


def source_dir(path: Path, root: Path):
    """The nearest folder above `path` (inside `root`) with a SOURCE.md."""
    for d in path.parents:
        if d == root or root not in d.parents:
            return None
        if (d / "SOURCE.md").is_file():
            return d


def common_depth(a: Path, b: Path) -> int:
    n = 0
    for x, y in zip(a.parts, b.parts):
        if x != y:
            break
        n += 1
    return n


def find_partner(indd: Path, root: Path, by_key: dict, indd_dirs: set):
    """The IDML or PDF (from `by_key`) of the same document as `indd`."""
    cands = by_key.get(key(indd), [])
    same = [p for p in cands if p.parent == indd.parent]
    if same:
        return min(same)
    src = source_dir(indd, root)
    if src is None:
        return None
    cands = [p for p in cands
             if src in p.parents and (p.parent, key(p)) not in indd_dirs]
    if not cands:
        return None
    best = max(common_depth(p.parent, indd.parent) for p in cands)
    best_cands = sorted(p for p in cands if common_depth(p.parent, indd.parent) == best)
    return best_cands[0] if len(best_cands) == 1 else None


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("root")
    ap.add_argument("--exclude", action="append", default=[],
                    help="leave out paths under root that start with this (repeatable)")
    args = ap.parse_args()
    root = Path(args.root).resolve()

    def rel(p: Path) -> str:
        return str(p.relative_to(root))

    files = sorted(
        p for p in root.rglob("*")
        if p.suffix.lower() in COUNTED_EXT and p.is_file()
        and not any(rel(p).startswith(e) for e in args.exclude))
    by_ext = defaultdict(list)
    for p in files:
        by_ext[p.suffix.lower()].append(p)
    idml_by_key = defaultdict(list)
    for p in by_ext[".idml"]:
        idml_by_key[key(p)].append(p)
    pdf_by_key = defaultdict(list)
    for p in by_ext[".pdf"]:
        pdf_by_key[key(p)].append(p)
    indds = [p for p in files if p.suffix.lower() in INDD_EXT]
    indd_dirs = {(p.parent, key(p)) for p in indds}

    extra = defaultdict(dict)
    extra_file = root / "pairs-extra.tsv"
    if extra_file.is_file():
        for line in extra_file.read_text().splitlines():
            if line.strip() and not line.startswith("#"):
                a, b = line.split("\t")[:2]
                extra[a][Path(b).suffix.lower()] = b

    digests = {p: sha256(p) for p in files}
    first = {}
    rows = []
    for indd in indds:
        h = indd_header(indd)
        path = rel(indd)
        idml = find_partner(indd, root, idml_by_key, indd_dirs)
        pdf = find_partner(indd, root, pdf_by_key, indd_dirs)
        idml = root / extra[path][".idml"] if ".idml" in extra[path] else idml
        pdf = root / extra[path][".pdf"] if ".pdf" in extra[path] else pdf
        digest = digests[indd]
        dup_of = first.setdefault(digest, path)
        rows.append({
            "path": path,
            "bytes": indd.stat().st_size,
            "valid": h["valid"],
            "order": h.get("order", ""),
            "version": f"{h['major']}.{h['minor']}" if h.get("major") is not None else "",
            "creator": indd_creator(indd),
            "idml_dom": idml_dom_version(idml) if idml else "",
            "idml": rel(idml) if idml else "",
            "pdf": rel(pdf) if pdf else "",
            "sha256": digest,
            "dup_of": "" if dup_of == path else dup_of,
        })

    cols = ["path", "bytes", "valid", "order", "version", "creator", "idml_dom",
            "idml", "pdf", "sha256", "dup_of"]
    print("\t".join(cols))
    for r in rows:
        print("\t".join(str(r[c]) for c in cols))

    err = sys.stderr
    print("files by extension (all / distinct by SHA-256):", file=err)
    for ext in COUNTED_EXT:
        ps = by_ext[ext]
        print(f"  {ext:6} {len(ps):5} / {len({digests[p] for p in ps}):5}", file=err)
    distinct = [r for r in rows if not r["dup_of"]]
    paired = [r for r in distinct if r["idml_dom"]]
    with_pdf = [r for r in distinct if r["pdf"]]
    print(f"{len(rows)} INDD/INDT files, {len(distinct)} distinct; of these "
          f"{len(paired)} with an IDML, {len(with_pdf)} with a PDF", file=err)
    print("by header version (distinct / with IDML / with PDF):", file=err)
    allv = Counter(r["version"] for r in distinct)
    pv = Counter(r["version"] for r in paired)
    fv = Counter(r["version"] for r in with_pdf)
    for v in sorted(allv, key=lambda s: [int(x) for x in s.split(".")] if s else [-1]):
        print(f"  {v or 'unknown':>8}  {allv[v]:4} / {pv[v]:4} / {fv[v]:4}", file=err)
    print("byte order:", dict(Counter(r["order"] for r in distinct)), file=err)


if __name__ == "__main__":
    main()
