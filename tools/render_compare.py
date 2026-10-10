#!/usr/bin/env python3
"""Compare renders of corpus documents that come with an InDesign PDF.

For every INDD file in the corpus with a reference IDML and a PDF that show
the same save (a same-save triple; rule in docs/measurement.md), compare
three PDFs page by page:

  R  the InDesign PDF from the corpus
  A  DesignCraft's PDF of the reference IDML
  B  DesignCraft's PDF of the converter's IDML

A vs B measures the converter: fonts, linked files and renderer are the
same on both sides, so any difference comes from the IDML. R vs A
measures DesignCraft. DesignCraft is an input: pass its command-line
program with --designcraft PATH (or the DESIGNCRAFT_CLI environment
variable).

Usage: python3 -I tools/render_compare.py --designcraft PATH [--bin PATH]
           [--jobs N] [--limit N] [--file SUBSTR] [--exclude PREFIX]...
           [--versions 13-21] [--max-pages N] [--link-fonts] [--out DIR]
           [--gaps GAPS_TSV]
       python3 -I tools/render_compare.py --images ID:PAGE [ID:PAGE...] [--out DIR]

Run from the repository root after `cargo build --release`. Needs
poppler-utils (pdfinfo, pdftoppm, pdftotext, pdffonts) and the Python
standard library only.

--exclude leaves out files whose path under corpus/ starts with PREFIX
(default own/; --exclude '' leaves out nothing). --versions limits the
measured triples to INDD major versions in that range; other triples are
listed in triples.tsv but not measured. --limit judges only the first N
triples. --max-pages limits the page-by-page comparison of each document
(default 100). --link-fonts gives DesignCraft the font files that a
package keeps beside the IDML, in a `Document Fonts` folder (DesignCraft
reads only that folder).

Outputs go to --out (default target/render-compare): triples.tsv,
docs.tsv, pages.tsv, lines.tsv, dropped.tsv, issues.tsv, summary.json, and
work/ID/ with the converted IDML, A.pdf, B.pdf and the renderer's results.
--images ID:PAGE writes side-by-side (R | A | B) and A/B difference images
of pages of an earlier run to images/, and does nothing else. --gaps names
the gaps.tsv of a compare.py run (default target/compare/gaps.tsv); the
summary lists its document count next to each key found on a first
differing page.
"""

import argparse
import hashlib
import json
import os
import re
import statistics
import struct
import subprocess
import sys
import tempfile
import time
import unicodedata
import xml.etree.ElementTree as ET
import zipfile
import zlib
from collections import Counter, defaultdict
from concurrent.futures import ProcessPoolExecutor, as_completed
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import compare  # noqa: E402

ROOT = compare.ROOT
CORPUS = ROOT / "corpus"
DPI = 50
PX = DPI / 72.0
RENDER_TIMEOUT = 600
CONVERT_TIMEOUT = 120
TOOL_TIMEOUT = 600
RTL = re.compile("[\u0590-\u08ff\ufb1d-\ufdff\ufe70-\ufefc]")
FONT_EXT = (".otf", ".ttf", ".ttc", ".otc", ".pfb", ".pfm", ".afm", ".dfont")


def run(cmd, timeout, text=False):
    """Run a command; (exit code or "timeout", seconds, stdout, stderr)."""
    t = time.time()
    try:
        p = subprocess.run(cmd, capture_output=True, timeout=timeout)
    except subprocess.TimeoutExpired:
        return "timeout", timeout, b"" if not text else "", ""
    out = p.stdout.decode("utf-8", "replace") if text else p.stdout
    return p.returncode, round(time.time() - t, 2), out, p.stderr.decode("utf-8", "replace")


# ---------------------------------------------------------------- triples

def major(version):
    try:
        return int(version.split(".")[0])
    except ValueError:
        return None


def candidates(args):
    """Inventory rows with an IDML and a PDF, without duplicates."""
    lo, hi = (int(x) for x in args.versions.split("-"))
    out = []
    for r in compare.inventory():
        path = r["path"]
        if not r.get("idml") or not r.get("pdf") or r.get("dup_of"):
            continue
        if args.file and args.file not in path:
            continue
        if any(e and path.startswith(e) for e in args.exclude):
            continue
        skip = []
        v, dom = major(r["version"]), major(r.get("idml_dom") or "")
        if r["order"] != "LE":
            skip.append("not little-endian")
        if v is None or not lo <= v <= hi:
            skip.append(f"version outside {args.versions}")
        if v != dom:
            skip.append("IDML of another major version")
        sha = r.get("sha256") or hashlib.sha256((CORPUS / path).read_bytes()).hexdigest()
        out.append({"id": sha[:10], "path": path, "idml": r["idml"], "pdf": r["pdf"],
                    "version": r["version"], "skip": skip})
    return out[:args.limit] if args.limit else out


def pdf_info(pdf):
    """(pages, Creator) from pdfinfo."""
    code, _, out, _ = run(["pdfinfo", str(pdf)], 120, text=True)
    pages = re.search(r"^Pages:\s+(\d+)", out, re.M)
    creator = re.search(r"^Creator:\s+(.*)$", out, re.M)
    return (int(pages.group(1)) if pages else None), (creator.group(1).strip() if creator else "")


def idml_pages(idml):
    """Number of <Page> elements in an IDML package's spreads."""
    try:
        with zipfile.ZipFile(idml) as z:
            return sum(len(re.findall(rb"<Page\s", z.read(n))) for n in z.namelist()
                       if n.startswith("Spreads/") and n.endswith(".xml"))
    except (zipfile.BadZipFile, OSError, KeyError):
        return None


def judge(cfg, t):
    """Same-save verdict of one triple. Converts the INDD (the IDML's
    stale rule needs the converter's stories); a measured triple keeps its
    conversion in work/ID/ours.idml."""
    indd, idml, pdf = CORPUS / t["path"], CORPUS / t["idml"], CORPUS / t["pdf"]
    pages, creator = pdf_info(pdf)
    _, _, pdf_xmp, _ = run(["pdfinfo", "-meta", str(pdf)], 120)
    code, _, indd_xmp, _ = run([cfg["bin"], "xmp", str(indd)], 120)
    if code != 0:
        indd_xmp = b""
    n_idml = idml_pages(idml)
    reasons = compare.pdf_stale_reasons(indd_xmp, pdf_xmp, pages, n_idml, creator)
    res = dict(t, pdf_pages=pages, idml_pages=n_idml, creator=creator, reasons=reasons)
    if t["skip"]:
        return res
    work = Path(cfg["out"]) / "work" / t["id"]
    work.mkdir(parents=True, exist_ok=True)
    ours = work / "ours.idml"
    ours.unlink(missing_ok=True)
    code, secs, _, err = run([cfg["bin"], "convert", str(indd), str(ours)], CONVERT_TIMEOUT)
    res["convert"] = {"exit": code, "secs": secs,
                      "warnings": sum(1 for x in err.splitlines() if x.startswith("warning: ")),
                      "error": compare.error_message(err) if code != 0 else ""}
    if code != 0:
        res["reasons"] = reasons + ["conversion failed"]
        return res
    u = subprocess.run([cfg["bin"], "uids", str(indd)], capture_output=True, text=True)
    uids = None
    if u.returncode == 0:
        uids = {}
        for line in u.stdout.splitlines():
            uid, cls = line.split("\t")
            uids[int(uid)] = None if cls == "-" else int(cls, 16)
    try:
        ref, mine = compare.load(idml), compare.load(ours)
    except SystemExit as e:
        res["reasons"] = reasons + [f"IDML unreadable: {e}"[:200]]
        return res
    stale = compare.stale_reasons(compare.modify_date(indd_xmp), compare.idml_modify_date(idml),
                                  uids, ref[0], ref[1], mine[1])
    res["reasons"] = reasons + [f"IDML stale: {s}" for s in stale]
    st = compare.Stats()
    compare.value_coverage(st, ref, mine)
    res["gap_keys"] = sorted(compare.gap_name(k) for k in st.gap_total
                             if st.gap_wrong[k] + st.gap_missing[k])
    return res


# ---------------------------------------------------------------- rendering

def package(src_idml, dest, doc, link_fonts):
    """A folder of symbolic links to everything beside the reference IDML,
    with `doc` linked as doc.idml, so that A and B find the same linked
    files and fonts."""
    dest.mkdir(parents=True, exist_ok=True)
    for e in src_idml.parent.iterdir():
        link = dest / e.name
        if e.name != src_idml.name and not link.is_symlink() and not link.exists():
            link.symlink_to(e)
    fonts = [e for e in src_idml.parent.iterdir() if e.suffix.lower() in FONT_EXT and e.is_file()]
    has_folder = any(e.name.lower() == "document fonts" for e in dest.iterdir())
    if link_fonts and fonts and not has_folder:
        (dest / "Document Fonts").mkdir()
        for f in fonts:
            (dest / "Document Fonts" / f.name).symlink_to(f)
    d = dest / "doc.idml"
    if d.is_symlink() or d.exists():
        d.unlink()
    d.symlink_to(doc)
    return d


def designcraft(cfg, idml, pdf):
    """Render an IDML to PDF; exit code, time, font list, preflight items
    and warnings."""
    pdf.unlink(missing_ok=True)
    code, secs, out, err = run([cfg["designcraft"], "run", "--in", str(idml), "--cmd", "font.list",
                                "--cmd", "preflight.run", "--export", str(pdf)], RENDER_TIMEOUT,
                               text=True)
    lines = [x for x in out.splitlines() if x.strip()]
    res = {"exit": code, "secs": secs, "fonts": None, "preflight": None,
           "warnings": [x[len("warning: "):] for x in err.splitlines() if x.startswith("warning: ")],
           "stderr": err[-2000:] if code != 0 else ""}
    try:
        res["fonts"] = json.loads(lines[0])
        res["preflight"] = json.loads(lines[1])
    except (IndexError, ValueError):
        pass
    return res


def boxes(pdf):
    """Per page (1-based): {"media", "crop", "trim"} boxes as (x0, y0, x1, y1)."""
    code, _, out, _ = run(["pdfinfo", "-box", "-f", "1", "-l", "1000000", str(pdf)], 300, text=True)
    pages = defaultdict(dict)
    box = r"(MediaBox|CropBox|TrimBox):\s+([-\d.]+)\s+([-\d.]+)\s+([-\d.]+)\s+([-\d.]+)"
    for m in re.finditer(r"^Page\s+(\d+)\s+" + box, out, re.M):
        pages[int(m.group(1))][m.group(2)[:-3].lower()] = tuple(float(m.group(i)) for i in range(3, 7))
    if not pages:  # pdfinfo leaves out "Page N" for a single page
        for m in re.finditer("^" + box, out, re.M):
            pages[1][m.group(1)[:-3].lower()] = tuple(float(m.group(i)) for i in range(2, 6))
    n = re.search(r"^Pages:\s+(\d+)", out, re.M)
    return (int(n.group(1)) if n else 0), dict(pages)


def trim_offset(b):
    """(trim box, offset of the trim box's top-left corner in the crop box)
    in points; pdftoppm and pdftotext -cropbox work in the crop box."""
    crop = b.get("crop") or b.get("media")
    trim = b.get("trim") or crop
    return trim, (trim[0] - crop[0], crop[3] - trim[3])


def raster(pdf, outdir, last, first=1, dpi=DPI):
    """Greyscale page images; {page: path}."""
    outdir.mkdir(parents=True, exist_ok=True)
    run(["pdftoppm", "-r", str(dpi), "-gray", "-f", str(first), "-l", str(last), str(pdf),
         str(outdir / "p")], TOOL_TIMEOUT)
    out = {}
    for f in outdir.iterdir():
        m = re.fullmatch(r"p-(\d+)\.pgm", f.name)
        if m:
            out[int(m.group(1))] = f
    return out


def read_pgm(path):
    """(width, height, bytes) of a binary 8-bit PGM file."""
    data = path.read_bytes()
    fields, pos = [], 0
    while len(fields) < 4:
        m = re.compile(rb"\s*(#[^\n]*\n\s*)*(\S+)").match(data, pos)
        fields.append(m.group(2))
        pos = m.end()
    if fields[0] != b"P5" or int(fields[3]) > 255:
        raise ValueError(f"{path}: not an 8-bit binary PGM")
    w, h = int(fields[1]), int(fields[2])
    return w, h, data[pos + 1:pos + 1 + w * h]


def crop(img, left, top, w, h):
    iw, ih, px = img
    left, top = max(0, min(iw, left)), max(0, min(ih, top))
    w, h = max(0, min(iw - left, w)), max(0, min(ih - top, h))
    return w, h, b"".join(px[(top + y) * iw + left:(top + y) * iw + left + w] for y in range(h))


def page_image(path, b):
    """A page image cropped to the trim box."""
    img = read_pgm(path)
    trim, (dx, dy) = trim_offset(b)
    return crop(img, round(dx * PX), round(dy * PX), round((trim[2] - trim[0]) * PX),
                round((trim[3] - trim[1]) * PX))


def halve(img):
    """2 x 2 mean, so that one-pixel shifts of anti-aliased edges count little."""
    w, h, px = img
    w2, h2 = w // 2, h // 2
    rows = []
    for y in range(h2):
        r0 = px[2 * y * w:2 * y * w + 2 * w2]
        r1 = px[(2 * y + 1) * w:(2 * y + 1) * w + 2 * w2]
        s = [a + b for a, b in zip(r0, r1)]
        rows.append(bytes((s[i] + s[i + 1] + 2) >> 2 for i in range(0, 2 * w2, 2)))
    return w2, h2, rows


INK = bytes(49 if v < 200 else 48 for v in range(256))  # "1" for ink, "0" for paper


def ink_masks(rows, w):
    """Per row, the ink pixels (value < 200) as bits of an integer."""
    return [int(r.translate(INK), 2) if w else 0 for r in rows]


def dilate(masks, w, k=2):
    full = (1 << w) - 1
    horiz = []
    for m in masks:
        d = m
        for s in range(1, k + 1):
            d |= (m << s) | (m >> s)
        horiz.append(d & full)
    out = []
    for y in range(len(horiz)):
        d = 0
        for v in horiz[max(0, y - k):y + k + 1]:
            d |= v
        out.append(d)
    return out


def pix_stats(a, b):
    """Pixel measures of page image `b` against `a` (both full resolution):
    identical bytes; mean absolute difference (0-1) and share of pixels
    that differ by more than 48 of 255 after a 2 x 2 mean; ink (< 200) in
    `a` with no ink within 2 pixels in `b` (missing ink) and the reverse
    (extra ink), as shares of the pixels."""
    if a is None or b is None:
        return {"pix": "missing"}
    if max(abs(a[0] - b[0]), abs(a[1] - b[1])) > 3:
        return {"pix": "size"}
    if a == b:
        return {"pix": "ok", "identical": True, "mad": 0.0, "diff": 0.0, "missing_ink": 0.0,
                "extra_ink": 0.0}
    w, h = min(a[0], b[0]), min(a[1], b[1])
    a, b = halve(crop(a, 0, 0, w, h)), halve(crop(b, 0, 0, w, h))
    w, h = a[0], a[1]
    n = (w * h) or 1
    total = big = 0
    for ra, rb in zip(a[2], b[2]):
        d = [abs(x - y) for x, y in zip(ra, rb)]
        total += sum(d)
        big += sum(1 for x in d if x > 48)
    ia, ib = ink_masks(a[2], w), ink_masks(b[2], w)
    da, db = dilate(ia, w), dilate(ib, w)
    full = (1 << w) - 1
    missing = sum((x & ~y & full).bit_count() for x, y in zip(ia, db))
    extra = sum((x & ~y & full).bit_count() for x, y in zip(ib, da))
    return {"pix": "ok", "identical": False, "mad": total / n / 255, "diff": big / n,
            "missing_ink": missing / n, "extra_ink": extra / n}


# ---------------------------------------------------------------- text

def norm(s):
    s = unicodedata.normalize("NFKC", s)
    return re.sub("[\\s\u00ad\ufeff\u200b]", "", s).casefold()


def norm_story(s):
    """Story text as a search key: also without hyphens, which line breaks
    add."""
    return re.sub("[\\-\u2010\u2011]", "", norm(s))


XHTML = "{http://www.w3.org/1999/xhtml}"
BAD_XML = re.compile("[\x00-\x08\x0b\x0c\x0e-\x1f\ufffe\uffff]")


def text_pages(pdf):
    """Per page: lines of words from pdftotext -bbox-layout, in points
    from the crop box's top-left corner. A line is
    {"words": [(x0, x1, text)], "x", "xend", "top", "height", "key", "text", ...};
    to_trim adds the same positions in the trim box (tx, txend, ty, twords)."""
    code, _, out, _ = run(["pdftotext", "-q", "-cropbox", "-bbox-layout", str(pdf), "-"],
                          TOOL_TIMEOUT)
    if code != 0 or not out:
        return []
    out = BAD_XML.sub("", out.decode("utf-8", "replace"))
    try:
        root = ET.fromstring(out)
    except ET.ParseError:
        return []
    pages = []
    for pg in root.iter(XHTML + "page"):
        lines = []
        for ln in pg.iter(XHTML + "line"):
            words = [(float(w.get("xMin")), float(w.get("xMax")), w.text or "")
                     for w in ln.iter(XHTML + "word") if (w.text or "").strip()]
            if not words:
                continue
            text = " ".join(w[2] for w in words)
            key = norm(text)
            if not key:
                continue
            x, top = float(ln.get("xMin")), float(ln.get("yMin"))
            lines.append({"words": words, "x": x, "xend": words[-1][1], "top": top,
                          "height": float(ln.get("yMax")) - top, "key": key, "text": text,
                          "rtl": bool(RTL.search(text)), "tx": x, "ty": top,
                          "txend": words[-1][1], "twords": words,
                          "h": round(float(ln.get("yMax")) - top, 1)})
        pages.append(lines)
    return pages


def to_trim(lines, b):
    _, (dx, dy) = trim_offset(b)
    for ln in lines:
        ln["tx"], ln["ty"], ln["txend"] = ln["x"] - dx, ln["top"] - dy, ln["xend"] - dx
        ln["twords"] = [(a - dx, b - dx, t) for a, b, t in ln["words"]]
    return lines


def match(ref, oth):
    """Greedy nearest match of lines with identical text: (pairs, unmatched
    reference lines, unmatched other lines)."""
    by = defaultdict(list)
    for j, ln in enumerate(oth):
        by[ln["key"]].append(j)
    used, pairs, unref = set(), [], []
    for i, ln in enumerate(ref):
        cands = [j for j in by.get(ln["key"], []) if j not in used]
        if not cands:
            unref.append(i)
            continue
        j = min(cands, key=lambda j: abs(oth[j]["ty"] - ln["ty"]) + abs(oth[j]["tx"] - ln["tx"]))
        used.add(j)
        pairs.append((i, j))
    return pairs, unref, [j for j in range(len(oth)) if j not in used]


def char_overlap(ref, oth):
    """Share of the reference page's characters (without spaces) that the
    other page has too, as a multiset overlap; None for an empty page."""
    a = Counter("".join(ln["key"] for ln in ref))
    b = Counter("".join(ln["key"] for ln in oth))
    n = sum(a.values())
    return (sum((a & b).values()) / n if n else None), n


def line_stats(ref, oth, offsets=None):
    """Line match of a page. A matched line moved when its left edge or the
    top of its box differs by more than 1 pt. Line boxes come from the
    fonts' ascent and descent as each PDF declares them, so between R and A
    the top of the box is first corrected by `offsets`: the document's
    median difference for lines of the same box heights."""
    pairs, unref, unoth = match(ref, oth)
    moved = []
    for i, j in pairs:
        dy = oth[j]["ty"] - ref[i]["ty"]
        if offsets is not None:
            dy -= offsets.get((ref[i]["h"], oth[j]["h"]), 0.0)
        if abs(dy) > 1 or abs(oth[j]["tx"] - ref[i]["tx"]) > 1:
            moved.append((i, j))
    ov, n = char_overlap(ref, oth)
    return {"lines": len(ref), "matched": len(pairs), "moved": len(moved), "chars": n,
            "overlap": ov,
            "ex_moved": [(ref[i]["text"][:50], round(ref[i]["tx"], 1), round(ref[i]["ty"], 1),
                          round(oth[j]["tx"], 1), round(oth[j]["ty"], 1)) for i, j in moved[:3]],
            "ex_unmatched": [ref[i]["text"][:50] for i in unref[:3]]}


def box_offsets(pairs_by_page):
    """Median vertical difference of matched lines per (reference box
    height, other box height) over a document."""
    groups = defaultdict(list)
    for ref, oth, pairs in pairs_by_page:
        for i, j in pairs:
            groups[(ref[i]["h"], oth[j]["h"])].append(oth[j]["ty"] - ref[i]["ty"])
    return {k: statistics.median(v) for k, v in groups.items()}


def line_checks(lines):
    """Overflow and spread lines of one page (docs/measurement.md, "Render
    comparison"). Right-to-left lines are left out."""
    lines = [ln for ln in lines if not ln["rtl"]]
    edges = Counter(round(ln["txend"] * 2) / 2 for ln in lines)
    freq = [e for e, n in edges.items() if n >= 3]
    cols = Counter((round(ln["tx"]), round(ln["txend"])) for ln in lines)
    cols = [c for c, n in cols.items() if n >= 3]
    over, spread = [], []

    def em(ln):
        return 0.8 * max(ln["height"], 1)
    by_y = sorted(lines, key=lambda ln: (round(ln["ty"] / 0.6), ln["tx"]))
    for a, b in zip(by_y, by_y[1:]):
        if abs(a["ty"] - b["ty"]) > 0.6 or len(a["twords"]) + len(b["twords"]) > 6:
            continue
        if b["tx"] - a["txend"] < 3 * em(a):
            continue
        for left, right in cols:
            if abs(a["tx"] - left) <= 1 and abs(b["txend"] - right) <= 1:
                spread.append({"key": a["key"] + b["key"], "text": a["text"] + " … " + b["text"],
                               "x": a["tx"], "xend": b["txend"], "y": a["ty"],
                               "words": len(a["twords"]) + len(b["twords"])})
                break
    for ln in lines:
        x, xe = ln["tx"], ln["txend"]
        if any(abs(xe - e) <= 1 for e in freq):
            gaps = [b[0] - a[1] for a, b in zip(ln["twords"], ln["twords"][1:])]
            if 2 <= len(ln["twords"]) <= 6 and gaps and min(gaps) >= em(ln):
                spread.append({"key": ln["key"], "text": ln["text"], "x": x, "xend": xe,
                               "y": ln["ty"], "words": len(ln["twords"])})
            continue
        for e in freq:
            if x <= e - 50 and e + 2 <= xe <= e + 80:
                # The usual left edge of lines ending on e: a positive indent
                # means the line starts after a prefix (number, bullet, tab).
                lefts = Counter(round(m["tx"]) for m in lines if abs(m["txend"] - e) <= 1)
                left = lefts.most_common(1)[0][0] if lefts else x
                over.append({"key": ln["key"], "text": ln["text"], "x": x, "xend": xe, "edge": e,
                             "indent": x - left, "y": ln["ty"], "words": len(ln["twords"])})
                break
    return over, spread


def script_of(pages):
    """The writing system of most letters in a document's text."""
    c = Counter()
    for lines in pages[:20]:
        for ln in lines:
            for ch in ln["key"]:
                if ch.isalpha():
                    name = unicodedata.name(ch, "")
                    if "HIRAGANA" in name or "KATAKANA" in name:
                        c["Japanese"] += 1
                    elif name.startswith("CJK"):
                        c["Han"] += 1
                    elif name.startswith("HANGUL"):
                        c["Korean"] += 1
                    else:
                        c[name.split(" ")[0].capitalize() or "other"] += 1
    if sum(c.values()) < 20:
        return "no text"
    if c["Han"] and c["Japanese"] * 20 >= c["Han"]:
        return "Japanese"
    return c.most_common(1)[0][0]


# ---------------------------------------------------------------- fonts

def family_key(s):
    return re.sub(r"[^a-z0-9]", "", s.lower())


def ps_family(fontname):
    """The family part of a PDF font name ("ABCDEF+MinionPro-Regular")."""
    f = fontname.split("+", 1)[-1]
    return family_key(f.split("-")[0].split(",")[0])


def page_fonts(pdf, page):
    code, _, out, _ = run(["pdffonts", "-f", str(page), "-l", str(page), str(pdf)], 120, text=True)
    lines = out.splitlines()
    try:
        start = next(i for i, x in enumerate(lines) if x.startswith("---")) + 1
    except StopIteration:
        return []
    return [x.split()[0] for x in lines[start:] if x.split() and x.split()[0] != "[none]"]


def font_ok(families, fontname):
    """Whether DesignCraft has the font a PDF font name belongs to:
    `families` maps family keys to True when every face of the family
    matched exactly."""
    k = ps_family(fontname)
    if len(k) < 3:
        return False
    for fam, ok in families.items():
        if fam and (k.startswith(fam) or fam.startswith(k)):
            return ok
    return False


# ---------------------------------------------------------------- IDML

def local(tag):
    return tag.rsplit("}", 1)[-1]


def page_parts(z):
    """Per document page, in order: the package parts whose items can draw
    on it: its spread and its master spread."""
    dm = ET.fromstring(z.read("designmap.xml"))
    spreads = [el.get("src") for el in dm if local(el.tag) == "Spread"]
    masters = {}
    for el in dm:
        if local(el.tag) == "MasterSpread":
            src = el.get("src")
            root = ET.fromstring(z.read(src))
            for ms in root.iter("MasterSpread"):
                masters[ms.get("Self")] = src
    out = []
    for src in spreads:
        root = ET.fromstring(z.read(src))
        for pg in root.iter("Page"):
            parts = [src]
            m = masters.get(pg.get("AppliedMaster"))
            if m:
                parts.append(m)
            out.append(parts)
    return out


STYLE_REFS = ("AppliedParagraphStyle", "AppliedCharacterStyle", "AppliedObjectStyle",
              "AppliedTableStyle", "AppliedCellStyle", "AppliedTOCStyle")


def part_items(z, part, cache):
    """(Self identifiers, applied styles, parent stories) of the elements of
    one package part."""
    if part not in cache:
        selfs, styles, stories = set(), set(), set()
        if part in z.namelist():
            for el in ET.fromstring(z.read(part)).iter():
                if el.get("Self"):
                    selfs.add(el.get("Self"))
                styles.update(el.get(k) for k in STYLE_REFS if el.get(k))
                if el.get("ParentStory"):
                    stories.add(el.get("ParentStory"))
        cache[part] = (selfs, styles, stories)
    return cache[part]


def based_on(z):
    """Style Self -> the style it is based on."""
    out = {}
    if "Resources/Styles.xml" in z.namelist():
        for el in ET.fromstring(z.read("Resources/Styles.xml")).iter():
            b = el.findtext("Properties/BasedOn")
            if el.get("Self") and b:
                out[el.get("Self")] = b
    return out


def page_selfs(z, parts, cache, based):
    """`Self` identifiers of everything that can draw on a page: the items
    of its parts, the stories of their text frames (and of frames anchored
    in those stories), and the styles they apply with their BasedOn chains.
    Returns (item and story selfs, story ids, style selfs)."""
    selfs, styles, stories = set(), set(), set()
    todo, seen = list(parts), set()
    while todo:
        part = todo.pop()
        if part in seen:
            continue
        seen.add(part)
        items, applied, parents = part_items(z, part, cache)
        selfs |= items
        styles |= applied
        for ps in parents - stories:
            stories.add(ps)
            todo.append(f"Stories/Story_{ps}.xml")
    chain = set()
    for st in styles:
        for _ in range(50):
            if not st or st in chain:
                break
            chain.add(st)
            st = based.get(st)
    return selfs, stories, chain


def page_gaps(ref_idml, ours_idml, page, identical):
    """compare.py's gap keys and extra-value keys of the elements that can
    draw on a page (page_selfs): the styles, and the items and stories that
    draw on none of the pixel-identical pages `identical`; if those have
    none, all of them. A style can change one page and not another, so
    styles are always kept. Returns ({gap name: values wrong or missing},
    {name: extra values}, scope)."""
    with zipfile.ZipFile(ref_idml) as z:
        pp = page_parts(z)
        if page > len(pp):
            return {}, {}, ""
        cache, based = {}, based_on(z)
        selfs, sids, styles = page_selfs(z, pp[page - 1], cache, based)
        other, other_sids = set(), set()
        for n in identical:
            if n <= len(pp):
                a, b, _ = page_selfs(z, pp[n - 1], cache, based)
                other |= a
                other_sids |= b
    ref, ours = compare.load(ref_idml), compare.load(ours_idml)

    def keys(selfs, sids):
        def sub(d):
            el, st, rg = d
            return ({k: v for k, v in el.items() if k[1] in selfs},
                    {k: v for k, v in st.items() if k in sids},
                    {k: v for k, v in rg.items() if k in sids})
        st = compare.Stats()
        compare.value_coverage(st, sub(ref), sub(ours))
        gaps = {compare.gap_name(k): st.gap_wrong[k] + st.gap_missing[k] for k in st.gap_total
                if st.gap_wrong[k] + st.gap_missing[k]}
        return gaps, {compare.gap_name(k): n for k, n in st.extra_values.items() if n}
    gaps, extras = keys((selfs - other) | styles, sids - other_sids)
    if gaps or extras:
        return gaps, extras, "only on the page"
    return (*keys(selfs | styles, sids), "on the page")


def style_table(z):
    """Paragraph and character styles: Self -> attributes plus simple
    Properties children."""
    tables = {"ParagraphStyle": {}, "CharacterStyle": {}}
    root = ET.fromstring(z.read("Resources/Styles.xml"))
    for el in root.iter():
        if el.tag in tables and el.get("Self"):
            d = dict(el.attrib)
            for k in ("BasedOn", "Leading", "AppliedFont"):
                v = el.findtext(f"Properties/{k}")
                if v is not None:
                    d[k] = v
            tables[el.tag][el.get("Self")] = d
    return tables


def resolve(name, table, key):
    for _ in range(20):
        st = table.get(name)
        if st is None:
            return None
        if key in st:
            return st[key]
        name = st.get("BasedOn")
    return None


def number(v):
    try:
        return float(v)
    except (TypeError, ValueError):
        return None


def frame_heights(z):
    """Story id -> list of (frame height, top + bottom inset) of the text
    frames on spreads."""
    out = defaultdict(list)
    for n in z.namelist():
        if not n.startswith("Spreads/"):
            continue
        for tf in ET.fromstring(z.read(n)).iter("TextFrame"):
            ys = [number(p.get("Anchor", "").split(" ")[-1]) for p in tf.iter("PathPointType")]
            ys = [y for y in ys if y is not None]
            if not ys:
                continue
            t = [number(x) or 0.0 for x in tf.get("ItemTransform", "1 0 0 1 0 0").split()]
            sy = (t[2] ** 2 + t[3] ** 2) ** 0.5 if len(t) == 6 else 1.0
            tb = 0.0
            pref = tf.find("TextFramePreference")
            if pref is not None:
                if pref.get("InsetSpacing"):
                    tb = 2 * (number(pref.get("InsetSpacing")) or 0.0)
                else:
                    v = [number(x.text) for x in pref.iterfind("Properties/InsetSpacing/ListItem")]
                    if len(v) == 4 and None not in v:
                        tb = v[0] + v[2]
            out[tf.get("ParentStory")].append(((max(ys) - min(ys)) * sy, tb))
    return out


def dropped_text(ref_idml, r_pages, a_pages):
    """Stories in one text frame that R shows and A does not, with the
    frame's inner height and the first character's size and leading."""
    r_text = ["".join(norm_story(ln["text"]) for ln in lines) for lines in r_pages]
    a_text = "".join(r for lines in a_pages for r in (norm_story(ln["text"]) for ln in lines))
    out = []
    with zipfile.ZipFile(ref_idml) as z:
        styles = style_table(z)
        frames = frame_heights(z)
        for n in z.namelist():
            if not n.startswith("Stories/"):
                continue
            root = ET.fromstring(z.read(n))
            for story in root.iter("Story"):
                sid = story.get("Self")
                if len(frames.get(sid, [])) != 1:
                    continue
                text = compare.story_text(story)
                if RTL.search(text):
                    continue
                key = norm_story(text)[:24]
                if len(key) < 4:
                    continue
                pages = [i + 1 for i, t in enumerate(r_text) if key in t]
                if not pages or key in a_text:
                    continue
                psr = story.find(".//ParagraphStyleRange")
                csr = story.find(".//CharacterStyleRange")
                pa = psr.attrib if psr is not None else {}
                ca = dict(csr.attrib) if csr is not None else {}
                if csr is not None:
                    for k in ("Leading", "AppliedFont"):
                        v = csr.findtext(f"Properties/{k}")
                        if v is not None:
                            ca[k] = v
                pst, cst = pa.get("AppliedParagraphStyle"), ca.get("AppliedCharacterStyle")

                def get(k):
                    for v in (ca.get(k), resolve(cst, styles["CharacterStyle"], k),
                              pa.get(k), resolve(pst, styles["ParagraphStyle"], k)):
                        if v is not None:
                            return v
                    return None
                size = number(get("PointSize")) or 12.0
                lead = number(get("Leading"))
                leading = lead if lead is not None else size * (number(get("AutoLeading")) or 120) / 100
                fonts = {x.text for x in story.iter("AppliedFont") if x.text}
                if get("AppliedFont"):
                    fonts.add(get("AppliedFont"))
                h, tb = frames[sid][0]
                out.append({"story": sid, "key": key, "pages_R": pages, "frame_h": h - tb,
                            "size": size, "leading": leading, "fonts": sorted(fonts)})
    return out


# ---------------------------------------------------------------- one document

def ra_flags(p):
    out = []
    if p.get("size_RA"):
        out.append("page size differs")
    px = p.get("RA_pix") or {}
    if px.get("pix") == "missing":
        out.append("page missing")
    if px.get("pix") == "ok":
        if px["missing_ink"] > 0.02:
            out.append("ink missing")
        if px["extra_ink"] > 0.02:
            out.append("ink extra")
    t = p.get("RA_text")
    if t and t["chars"] >= 20:
        if t["overlap"] is not None and t["overlap"] < 0.9:
            out.append("text missing")
        if t["lines"] >= 3 and t["matched"] / t["lines"] < 0.8:
            out.append("line breaks differ")
        if t["matched"] >= 3 and t["moved"] / t["matched"] > 0.2:
            out.append("lines moved")
    return out


def ab_flags(p):
    out = []
    px = p.get("AB_pix") or {}
    if px.get("pix") in ("missing", "size"):
        out.append("page missing" if px["pix"] == "missing" else "page size differs")
    if px.get("pix") == "ok" and not px["identical"]:
        out.append("pixels differ")
        if px["missing_ink"] > 0.005:
            out.append("ink missing")
        if px["extra_ink"] > 0.005:
            out.append("ink extra")
    t = p.get("AB_text")
    if t and t["chars"]:
        if t["overlap"] is not None and t["overlap"] < 0.99:
            out.append("text differs")
        if t["moved"]:
            out.append("lines moved")
        if t["matched"] < t["lines"]:
            out.append("lines differ")
    return out


def measure(cfg, t):
    """Render A and B and compare R, A and B page by page."""
    work = Path(cfg["out"]) / "work" / t["id"]
    ref_idml, r_pdf = CORPUS / t["idml"], CORPUS / t["pdf"]
    res = {"id": t["id"]}
    a_doc = package(ref_idml, work / "pkgA", ref_idml, cfg["link_fonts"])
    res["A"] = designcraft(cfg, a_doc, work / "A.pdf")
    b_doc = package(ref_idml, work / "pkgB", work / "ours.idml", cfg["link_fonts"])
    res["B"] = designcraft(cfg, b_doc, work / "B.pdf")
    (work / "run.json").write_text(json.dumps(res, indent=1))
    pdfs = {"R": r_pdf, "A": work / "A.pdf", "B": work / "B.pdf"}
    info = {k: boxes(p) for k, p in pdfs.items() if p.exists()}
    res["pages"] = {k: v[0] for k, v in info.items()}
    n = min(res["pages"].get("R", 0), cfg["max_pages"])

    fl = (res["A"].get("fonts") or [])
    families = {}
    for f in fl:
        k = family_key(f.get("family", ""))
        families[k] = families.get(k, True) and f.get("matchStatus") == "exact"
    res["fonts_missing"] = sorted({f["family"] for f in fl if f.get("matchStatus") == "missing"})
    res["fonts_substituted"] = sorted({f"{f['family']} {f.get('style', '')}" for f in fl
                                       if f.get("matchStatus") not in ("exact", "missing")})
    res["font_complete"] = res["A"]["exit"] == 0 and all(families.values())

    text = {k: text_pages(p) for k, p in pdfs.items() if k in info}
    res["script"] = script_of(text.get("R", []))
    for k in text:
        for i, lines in enumerate(text[k]):
            b = info[k][1].get(i + 1)
            if b:
                to_trim(lines, b)
    rows = []
    with tempfile.TemporaryDirectory(dir=cfg["tmp"]) as tmp:
        imgs = {k: raster(p, Path(tmp) / k, n) for k, p in pdfs.items() if k in info and n}
        for pno in range(1, n + 1):
            p = {"page": pno}
            bx = {k: info[k][1].get(pno) for k in info if pno <= info[k][0]}
            for k, b in bx.items():
                if b:
                    trim, _ = trim_offset(b)
                    p[f"size_{k}"] = (round(trim[2] - trim[0], 1), round(trim[3] - trim[1], 1))
            for x, y in (("R", "A"), ("A", "B")):
                sx, sy = p.get(f"size_{x}"), p.get(f"size_{y}")
                p[f"size_{x}{y}"] = bool(sx and sy and (abs(sx[0] - sy[0]) > 1 or abs(sx[1] - sy[1]) > 1))
            im = {}
            for k in bx:
                f = imgs.get(k, {}).get(pno)
                im[k] = page_image(f, bx[k]) if f and bx[k] else None
            p["RA_pix"] = pix_stats(im.get("R"), im.get("A"))
            p["AB_pix"] = pix_stats(im.get("A"), im.get("B"))
            rows.append(p)
            for f in (imgs.get(k, {}).get(pno) for k in imgs):
                if f:
                    f.unlink()

    def lines(k, pno):
        pages = text.get(k, [])
        return pages[pno - 1] if pno <= len(pages) else None
    ra_pairs = []
    for p in rows:
        r, a, b = lines("R", p["page"]), lines("A", p["page"]), lines("B", p["page"])
        if r is not None and a is not None:
            ra_pairs.append((r, a, match(r, a)[0]))
        if a is not None and b is not None:
            p["AB_text"] = line_stats(a, b)
    offsets = box_offsets(ra_pairs)
    all_ok = res["font_complete"]
    checks = []
    for p in rows:
        pno = p["page"]
        r, a = lines("R", pno), lines("A", pno)
        if r is not None and a is not None:
            p["RA_text"] = line_stats(r, a, offsets)
        p["fontok"] = all_ok or (res["A"]["exit"] == 0 and all(
            font_ok(families, f) for f in page_fonts(r_pdf, pno)))
        p["RA_flags"], p["AB_flags"] = ra_flags(p), ab_flags(p)
        found = {}
        for k in ("R", "A", "B"):
            ls = lines(k, pno)
            if ls is not None:
                found[k] = line_checks(ls)
        for k, (over, spread) in found.items():
            for kind, items in (("overflow", over), ("spread", spread)):
                in_r = {x["key"] for x in (found.get("R", ([], []))[0 if kind == "overflow" else 1])}
                for x in items:
                    checks.append(dict(x, page=pno, pdf=k, check=kind,
                                       in_R=(x["key"] in in_r) if k != "R" else ""))
    res["page_rows"] = rows
    res["checks"] = checks
    res["dropped"] = []
    if text.get("R") and text.get("A") and res["A"]["exit"] == 0:
        try:
            res["dropped"] = dropped_text(ref_idml, text["R"], text["A"])
        except (ET.ParseError, KeyError, zipfile.BadZipFile) as e:
            res["dropped_error"] = str(e)[:200]

    differ = [p["page"] for p in rows if (p["AB_pix"] or {}).get("identical") is not True]
    res["ab_first"] = differ[0] if differ else None
    res["ab_after"] = len(differ) - 1 if differ else 0
    res["ab_gaps"], res["ab_extras"] = {}, {}
    if differ and res["B"]["exit"] == 0 and (work / "ours.idml").exists():
        try:
            same = [p["page"] for p in rows if (p["AB_pix"] or {}).get("identical") is True]
            res["ab_gaps"], res["ab_extras"], res["ab_scope"] = page_gaps(
                ref_idml, work / "ours.idml", differ[0], same)
        except (ET.ParseError, KeyError, zipfile.BadZipFile, SystemExit) as e:
            res["ab_gaps_error"] = str(e)[:200]
    return res


def process(cfg, t):
    """Judge one triple and, if it is a same-save triple to measure, measure it."""
    try:
        res = judge(cfg, t)
        if res["skip"] or res["reasons"]:
            work = Path(cfg["out"]) / "work" / t["id"] / "ours.idml"
            work.unlink(missing_ok=True)
            return res, None
        return res, measure(cfg, t)
    except Exception as e:  # report the triple and go on with the others
        import traceback
        return dict(t, reasons=[f"tool error: {type(e).__name__}: {e}"[:300]],
                    traceback=traceback.format_exc()[-2000:]), None


# ---------------------------------------------------------------- reports

def fmt(v, nd=4):
    if v is None:
        return ""
    if isinstance(v, bool):
        return "yes" if v else "no"
    if isinstance(v, float):
        return f"{v:.{nd}f}"
    if isinstance(v, (list, tuple)):
        return ",".join(fmt(x, nd) for x in v)
    return str(v).replace("\t", " ").replace("\n", " ")


def write_tsv(path, cols, rows):
    with open(path, "w") as f:
        f.write("\t".join(cols) + "\n")
        for r in rows:
            f.write("\t".join(fmt(r.get(c)) for c in cols) + "\n")


def doc_issues(m):
    """DesignCraft (R vs A) issue kinds of one document: kind -> (pages,
    items)."""
    out = defaultdict(lambda: [set(), 0])

    def add(kind, page, n=1):
        out[kind][0].add(page)
        out[kind][1] += n
    if m["A"]["exit"] != 0:
        add("render failed" if m["A"]["exit"] != "timeout" else "render timed out", 0)
        return out
    if m["pages"].get("A") != m["pages"].get("R"):
        add("page count differs", 0)
    for p in m["page_rows"]:
        for f in p["RA_flags"]:
            add(f, p["page"])
    for c in m["checks"]:
        if c["pdf"] == "A" and not c["in_R"]:
            add("line past the column edge" if c["check"] == "overflow" else "spread line",
                c["page"])
    for d in m["dropped"]:
        short = d["frame_h"] < 1.05 * d["size"]
        add("story not drawn, frame shorter than the point size" if short else
            "story not drawn", d["pages_R"][0])
    for item in (m["A"].get("preflight") or {}).get("issues", []):
        page = item.get("page")  # 0-based; none for an item off the pages
        add(f"preflight: {item.get('kind')}", page + 1 if isinstance(page, int) else 0)
    return out


def images(args):
    """Side-by-side and A/B difference images of pages of an earlier run."""
    out = Path(args.out)
    tri = {}
    with open(out / "triples.tsv") as f:
        cols = f.readline().rstrip("\n").split("\t")
        for line in f:
            r = dict(zip(cols, line.rstrip("\n").split("\t")))
            tri[r["id"]] = r
    (out / "images").mkdir(exist_ok=True)
    dpi = 72
    with tempfile.TemporaryDirectory() as tmp:
        for spec in args.images:
            tid, page = spec.split(":")
            page = int(page)
            work = out / "work" / tid
            pdfs = {"R": CORPUS / tri[tid]["pdf"], "A": work / "A.pdf", "B": work / "B.pdf"}
            im = {}
            for k, pdf in pdfs.items():
                if not pdf.exists():
                    continue
                n, bx = boxes(pdf)
                f = raster(pdf, Path(tmp) / f"{tid}-{page}-{k}", page, page, dpi).get(page)
                if f and bx.get(page):
                    trim, (dx, dy) = trim_offset(bx[page])
                    s = dpi / 72
                    im[k] = crop(read_pgm(f), round(dx * s), round(dy * s),
                                 round((trim[2] - trim[0]) * s), round((trim[3] - trim[1]) * s))
            h = max(i[1] for i in im.values())
            gap = 8
            w = sum(i[0] for i in im.values()) + gap * (len(im) - 1)
            rows = []
            for y in range(h):
                parts = []
                for k in ("R", "A", "B"):
                    if k in im:
                        iw, ih, px = im[k]
                        parts.append(px[y * iw:(y + 1) * iw] if y < ih else b"\xff" * iw)
                rows.append((b"\x80" * gap).join(parts))
            write_png(out / "images" / f"{tid}-p{page}.png", w, h, rows, grey=True)
            if "A" in im and "B" in im:
                aw, ah, apx = im["A"]
                bw, bh, bpx = im["B"]
                w2, h2 = min(aw, bw), min(ah, bh)
                rows = []
                for y in range(h2):
                    ra, rb = apx[y * aw:y * aw + w2], bpx[y * bw:y * bw + w2]
                    row = bytearray()
                    for x, z in zip(ra, rb):
                        if x == z:
                            v = 255 - (255 - x) // 3
                            row += bytes((v, v, v))
                        elif x < z:      # darker in A: A only
                            row += bytes((220, 30, 30))
                        else:            # darker in B: B only
                            row += bytes((30, 60, 220))
                    rows.append(bytes(row))
                write_png(out / "images" / f"{tid}-p{page}-ab.png", w2, h2, rows, grey=False)
            print(f"{tid} page {page}: images/{tid}-p{page}.png"
                  + (f", images/{tid}-p{page}-ab.png" if "A" in im and "B" in im else ""))


def write_png(path, w, h, rows, grey):
    def chunk(kind, data):
        c = kind + data
        return struct.pack(">I", len(data)) + c + struct.pack(">I", zlib.crc32(c) & 0xffffffff)
    raw = b"".join(b"\x00" + r for r in rows)
    path.write_bytes(b"\x89PNG\r\n\x1a\n"
                     + chunk(b"IHDR", struct.pack(">IIBBBBB", w, h, 8, 0 if grey else 2, 0, 0, 0))
                     + chunk(b"IDAT", zlib.compress(raw, 6)) + chunk(b"IEND", b""))


def dc_warning_kind(msg):
    """A DesignCraft warning with its names, paths and numbers replaced."""
    msg = re.sub(r"`[^`]*`", "`…`", msg)
    msg = re.sub(r"'[^']*'", "'…'", msg)
    return compare.warning_kind(re.sub(r"\([^)]*\)", "(…)", msg))


def compare_gap_docs(path):
    """Documents affected per gap key in a gaps.tsv of compare.py, if it
    exists."""
    path = Path(path)
    if not path.exists():
        return {}
    out = {}
    for line in path.read_text().splitlines()[1:]:
        f = line.split("\t")
        out[f"{f[0]} {f[1]}"] = int(f[2])
    return out


def report(args, cfg, judged, measured, secs):
    out = Path(args.out)
    tcols = ["id", "verdict", "reasons", "version", "pdf_pages", "idml_pages", "creator",
             "path", "idml", "pdf"]
    for r in judged:
        r["verdict"] = ("not measured" if r.get("skip") else
                        "same save" if not r["reasons"] else "rejected")
        r["reasons"] = "; ".join(r.get("skip", []) + r["reasons"]) if isinstance(r["reasons"], list) \
            else r["reasons"]
    write_tsv(out / "triples.tsv", tcols, judged)
    by_id = {r["id"]: r for r in judged}

    pages, docs, lines, dropped, issues = [], [], [], [], []
    conv_docs = []
    ra_all, ra_complete = defaultdict(lambda: [0, 0, 0]), defaultdict(lambda: [0, 0, 0])
    dc_warn = Counter()
    times = []
    for m in measured:
        t = by_id[m["id"]]
        for p in m["page_rows"]:
            ra, ab = p.get("RA_pix") or {}, p.get("AB_pix") or {}
            rt, bt = p.get("RA_text") or {}, p.get("AB_text") or {}
            pages.append({"id": m["id"], "page": p["page"], "size_R": p.get("size_R"),
                          "size_A": p.get("size_A"), "size_B": p.get("size_B"),
                          "fontok": p["fontok"], "RA_mad": ra.get("mad"),
                          "RA_missing_ink": ra.get("missing_ink"), "RA_extra_ink": ra.get("extra_ink"),
                          "RA_lines": rt.get("lines"), "RA_matched": rt.get("matched"),
                          "RA_moved": rt.get("moved"), "RA_char_overlap": rt.get("overlap"),
                          "AB_identical": ab.get("identical"), "AB_mad": ab.get("mad"),
                          "AB_missing_ink": ab.get("missing_ink"), "AB_extra_ink": ab.get("extra_ink"),
                          "AB_lines": bt.get("lines"), "AB_matched": bt.get("matched"),
                          "AB_moved": bt.get("moved"), "AB_char_overlap": bt.get("overlap"),
                          "RA_flags": p["RA_flags"], "AB_flags": p["AB_flags"]})
        for c in m["checks"]:
            lines.append(dict(c, id=m["id"], text=c["text"][:80]))
        for d in m["dropped"]:
            dropped.append(dict(d, id=m["id"], shown_R=True, shown_A=False,
                                ratio=d["frame_h"] / d["size"] if d["size"] else None))
        iss = doc_issues(m)
        for kind, (pgs, n) in sorted(iss.items()):
            issues.append({"id": m["id"], "version": t["version"], "script": m["script"],
                           "font_complete": m["font_complete"], "issue": kind,
                           "pages": sorted(x for x in pgs if x), "items": n})
            for tab in ((ra_all, ra_complete) if m["font_complete"] else (ra_all,)):
                tab[kind][0] += 1
                tab[kind][1] += len([x for x in pgs if x])
                tab[kind][2] += n
        for w in set(dc_warning_kind(x) for x in m["A"]["warnings"]):
            dc_warn[w] += 1
        for k in ("A", "B"):
            if isinstance(m[k]["secs"], (int, float)):
                times.append(m[k]["secs"])
        rows = m["page_rows"]
        ident = sum(1 for p in rows if (p["AB_pix"] or {}).get("identical") is True)
        top = sorted(m["ab_gaps"].items(), key=lambda kv: -kv[1])
        docs.append({"id": m["id"], "version": t["version"], "script": m["script"],
                     "pages_R": m["pages"].get("R"), "pages_A": m["pages"].get("A"),
                     "pages_B": m["pages"].get("B"), "pages_compared": len(rows),
                     "convert_exit": t["convert"]["exit"], "convert_secs": t["convert"]["secs"],
                     "A_exit": m["A"]["exit"], "A_secs": m["A"]["secs"],
                     "B_exit": m["B"]["exit"], "B_secs": m["B"]["secs"],
                     "font_complete": m["font_complete"],
                     "fonts_missing": ";".join(m["fonts_missing"]),
                     "fonts_substituted": ";".join(m["fonts_substituted"]),
                     "links_missing": sum(1 for i in (m["A"].get("preflight") or {}).get("issues", [])
                                          if i.get("kind") == "missingLink"),
                     "overset_A": sum(1 for i in (m["A"].get("preflight") or {}).get("issues", [])
                                      if i.get("kind") == "overset"),
                     "overset_B": sum(1 for i in (m["B"].get("preflight") or {}).get("issues", [])
                                      if i.get("kind") == "overset"),
                     "AB_identical_pages": ident, "AB_first_page": m["ab_first"],
                     "AB_pages_after": m["ab_after"],
                     "AB_first_page_gaps": "; ".join(f"{k} ({n})" for k, n in top),
                     "AB_gaps_scope": m.get("ab_scope", ""),
                     "RA_issues": "; ".join(f"{k}: {len(v[0])}" for k, v in sorted(iss.items())),
                     "path": t["path"]})
        conv_docs.append((m, len(rows), ident))

    write_tsv(out / "pages.tsv", list(pages[0].keys()) if pages else ["id"], pages)
    write_tsv(out / "docs.tsv", list(docs[0].keys()) if docs else ["id"], docs)
    write_tsv(out / "lines.tsv", ["id", "page", "pdf", "check", "in_R", "x", "xend", "edge",
                                  "indent", "words", "y", "text"], lines)
    write_tsv(out / "dropped.tsv", ["id", "story", "key", "shown_R", "shown_A", "pages_R",
                                    "frame_h", "size", "ratio", "leading", "fonts"], dropped)
    write_tsv(out / "issues.tsv", ["id", "version", "script", "font_complete", "issue", "pages",
                                   "items"], issues)

    gap_docs = compare_gap_docs(args.gaps)
    first_keys, first_extra = Counter(), Counter()
    # Gap keys of the documents that render pixel-identical: a key that
    # many of them have too is unlikely to change a rendering.
    same_keys = Counter(k for m, n, ident in conv_docs if n and ident == n
                        for k in by_id[m["id"]].get("gap_keys", []))
    no_key = 0
    for m, n, ident in conv_docs:
        if m["ab_first"] is None:
            continue
        first_keys.update(m["ab_gaps"].keys())
        first_extra.update(m["ab_extras"].keys())
        if not m["ab_gaps"] and not m["ab_extras"]:
            no_key += 1
    rejected = Counter(x for r in judged if r["verdict"] == "rejected"
                       for x in r["reasons"].split("; "))
    compared = [(m, n, i) for m, n, i in conv_docs if m["B"]["exit"] == 0]
    complete = [m for m in measured if m["font_complete"]]

    def table(tab):
        return {k: {"documents": v[0], "pages": v[1], "items": v[2]}
                for k, v in sorted(tab.items(), key=lambda kv: (-kv[1][0], kv[0]))}
    summary = {
        "options": {"exclude": args.exclude, "versions": args.versions, "limit": args.limit,
                    "file": args.file, "max_pages": args.max_pages, "link_fonts": args.link_fonts},
        "tools": cfg["tools"],
        "seconds": round(secs),
        "triples": {
            "candidates": len(judged),
            "measured_versions": sum(1 for r in judged if r["verdict"] != "not measured"),
            "same_save": len(measured),
            "rejected_by": dict(rejected.most_common()),
        },
        "converter": {
            "documents": len(measured),
            "conversion_failures": sum(1 for r in judged if r.get("convert", {}).get("exit") not in (0, None)),
            "B_render_failures": sum(1 for m in measured if m["B"]["exit"] != 0),
            "documents_compared": len(compared),
            "documents_identical": sum(1 for m, n, i in compared if n and i == n),
            "pages_compared": sum(n for _, n, _ in compared),
            "pages_identical": sum(i for _, _, i in compared),
            "documents_differing": sum(1 for m, n, i in compared if i < n),
            "pages_after_first_difference": sum(m["ab_after"] for m, _, _ in compared),
            "first_difference_without_gap": no_key,
            "first_difference_gaps": [{"key": k, "documents": n,
                                       "identical_documents_with_gap": same_keys[k],
                                       "compare_documents": gap_docs.get(k)}
                                      for k, n in first_keys.most_common()],
            "first_difference_extras": [{"key": k, "documents": n} for k, n in first_extra.most_common()],
        },
        "designcraft": {
            "renders": 2 * len(measured),
            "render_failures": sum(1 for m in measured for k in ("A", "B") if m[k]["exit"] not in (0, "timeout")),
            "render_timeouts": sum(1 for m in measured for k in ("A", "B") if m[k]["exit"] == "timeout"),
            "render_seconds_median": statistics.median(times) if times else None,
            "render_seconds_max": max(times) if times else None,
            "font_complete_documents": len(complete),
            "font_complete_without_text": sum(1 for m in complete if m["script"] == "no text"),
            "font_complete_pages": sum(len(m["page_rows"]) for m in complete),
            "issues_font_complete": table(ra_complete),
            "issues_all": table(ra_all),
            "warnings": dict(dc_warn.most_common(30)),
        },
    }
    (out / "summary.json").write_text(json.dumps(summary, indent=1, ensure_ascii=False) + "\n")
    tr, c, d = summary["triples"], summary["converter"], summary["designcraft"]
    print(f"triples: {tr['candidates']} with IDML and PDF; {tr['measured_versions']} in the measured "
          f"versions; {tr['same_save']} same-save, measured")
    for k, n in rejected.most_common():
        print(f"  rejected: {n:4}  {k}")
    print(f"\nconverter (A vs B): {c['documents_identical']} of {c['documents_compared']} documents and "
          f"{c['pages_identical']} of {c['pages_compared']} pages pixel-identical; "
          f"conversion failures {c['conversion_failures']}, B render failures {c['B_render_failures']}")
    print(f"  first differing page in {c['documents_differing']} documents; "
          f"{c['pages_after_first_difference']} differing pages after it; "
          f"{c['first_difference_without_gap']} without a value gap on that page")
    print("  value gaps on the first differing page (documents; pixel-identical documents with the "
          "same gap; documents in compare.py's gaps.tsv):")
    for g in c["first_difference_gaps"][:40]:
        print(f"    {g['documents']:4} {g['identical_documents_with_gap']:4} "
              f"{g['compare_documents'] or '':>5}  {g['key'][:100]}")
    print(f"\nDesignCraft (R vs A): {d['renders']} renders, {d['render_failures']} failed, "
          f"{d['render_timeouts']} timed out; median {d['render_seconds_median']} s, "
          f"max {d['render_seconds_max']} s")
    print(f"  font-complete documents: {d['font_complete_documents']} of {len(measured)} "
          f"({d['font_complete_pages']} pages)")
    print("  issues on font-complete documents (documents, pages, items):")
    for k, v in d["issues_font_complete"].items():
        print(f"    {v['documents']:4} {v['pages']:5} {v['items']:6}  {k}")
    print("  issues on all documents (documents, pages, items):")
    for k, v in d["issues_all"].items():
        print(f"    {v['documents']:4} {v['pages']:5} {v['items']:6}  {k}")
    print(f"\n{round(secs)} s; tables in {out}")


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--designcraft", default=os.environ.get("DESIGNCRAFT_CLI"),
                    help="DesignCraft command-line program (or DESIGNCRAFT_CLI)")
    ap.add_argument("--bin", default=str(compare.BIN), help="converter binary")
    ap.add_argument("--jobs", type=int, default=os.cpu_count() or 1)
    ap.add_argument("--limit", type=int, default=0, help="judge only the first N triples")
    ap.add_argument("--file", help="only triples whose INDD path contains this")
    ap.add_argument("--exclude", action="append", default=None,
                    help="leave out paths under corpus/ starting with this (default own/)")
    ap.add_argument("--versions", default="13-21", help="INDD major versions to measure")
    ap.add_argument("--max-pages", type=int, default=100)
    ap.add_argument("--link-fonts", action="store_true",
                    help="give DesignCraft the font files beside the IDML as Document Fonts")
    ap.add_argument("--out", default=str(ROOT / "target" / "render-compare"))
    ap.add_argument("--gaps", default=str(ROOT / "target" / "compare" / "gaps.tsv"),
                    help="compare.py's gaps.tsv, to rank first-difference keys against")
    ap.add_argument("--images", nargs="+", metavar="ID:PAGE",
                    help="write images of these pages of an earlier run, and nothing else")
    args = ap.parse_args()
    if args.exclude is None:
        args.exclude = ["own/"]
    out = Path(args.out)
    if args.images:
        images(args)
        return
    if not args.designcraft:
        ap.error("--designcraft PATH (or DESIGNCRAFT_CLI) is required")
    out.mkdir(parents=True, exist_ok=True)
    tmp = out / "tmp"
    tmp.mkdir(exist_ok=True)
    dc = Path(args.designcraft).resolve()
    _, _, version, _ = run([str(dc), "--version"], 60, text=True)
    cfg = {"bin": str(Path(args.bin).resolve()), "designcraft": str(dc), "out": str(out.resolve()),
           "tmp": str(tmp.resolve()), "link_fonts": args.link_fonts, "max_pages": args.max_pages,
           "tools": {"converter": args.bin,
                     "repository": subprocess.run(["git", "-C", str(ROOT), "describe", "--always",
                                                   "--dirty"], capture_output=True,
                                                  text=True).stdout.strip(),
                     "designcraft": version.strip(),
                     "designcraft_sha256": hashlib.sha256(dc.read_bytes()).hexdigest()[:16]}}
    start = time.time()
    todo = candidates(args)
    results = {}
    with ProcessPoolExecutor(max(1, args.jobs)) as pool:
        # Largest files first, so that the longest renders do not come last.
        order = sorted(todo, key=lambda t: -(CORPUS / t["path"]).stat().st_size)
        jobs = {pool.submit(process, cfg, t): t["id"] for t in order}
        for n, job in enumerate(as_completed(jobs), 1):
            res, m = job.result()
            results[jobs[job]] = (res, m)
            if "traceback" in res:
                print(f"tool error on {res['id']}:\n{res['traceback']}", file=sys.stderr)
            if n % 25 == 0:
                print(f"  {n}/{len(todo)} triples", file=sys.stderr)
    judged = [results[t["id"]][0] for t in todo]
    measured = [results[t["id"]][1] for t in todo if results[t["id"]][1] is not None]
    report(args, cfg, judged, measured, time.time() - start)


if __name__ == "__main__":
    main()
