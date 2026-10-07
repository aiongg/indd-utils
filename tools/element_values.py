#!/usr/bin/env python3
"""Find the attribute values that every corpus IDML has on the elements
the converter writes: page items, spreads, pages, layers, swatches, styles
and the singleton elements of designmap.xml.

Reads every IDML in the corpus inventory (distinct files only, the
privately held samples left out, and only files exported by InDesign) and collects, for each element path in
ELEMENTS, each attribute and each `Properties` child of every element on
that path. An element path is a tag (`TextFrame`), or a tag and the tag
of a child without `Self` (`Spread/FlattenerPreference`), as
tools/compare.py counts them.

A value is kept for a range of DOM versions when, in every file of those
versions that has elements on the path, every such element has the value.
Ranges of consecutive versions with the same value are joined; a range
needs at least MIN_FILES files. A range that reaches the newest version
in the corpus is written without an upper bound, so later versions get
its values too.

Paths in SINGLETONS are elements of which every IDML from some version on
has one (`Document/WatermarkPreference`); for them the presence of the
element is kept the same way, and the converter writes the element.
LISTS are elements with `Self` whose whole list (every element, with all
attributes and children, in order) is the same in every file of a range;
the converter writes the list. KEYED values depend only on another
attribute of the element, such as the quotes of a language on its name.
WHEN_WRITTEN values are those of attributes that only some elements have
(`WhenWritten` blocks); the converter writes them where it decides to.

EXPLAINED lists values that are the same in nearly every element; the
others are values the converter reads from the INDD (the reason says
which). For them a value is kept when its share is at least
EXPLAINED_SHARE of the elements of each version, and the exceptions are
counted in the evidence.

Usage: python3 -I tools/element_values.py [--write] [--key PATH ATTR] [--why PATH]
Prints the evidence counts. With --write, regenerates
src/idml/element_values.xml. --key prints the values of one key by
version; --why lists the values of a path that are left out, with the
share of the commonest value (None: element without it) and the
exceptions by version. See docs/format/idml-values.md.
"""

import hashlib
import re
import sys
import xml.etree.ElementTree as ET
import zipfile
from collections import Counter, OrderedDict, defaultdict
from concurrent.futures import ProcessPoolExecutor
from pathlib import Path
from xml.sax.saxutils import quoteattr

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / "src" / "idml" / "element_values.xml"
# A range of versions is kept only when at least this many files show it.
MIN_FILES = 10
# Paths under corpus/ left out: privately held samples.
EXCLUDE = ("own/",)

# Element paths whose observed values the converter writes.
ELEMENTS = [
    "TextFrame", "Rectangle", "Oval", "Polygon", "GraphicLine", "Group",
    "TextFrame/ObjectExportOption", "Rectangle/ObjectExportOption",
    "Oval/ObjectExportOption", "Polygon/ObjectExportOption",
    "GraphicLine/ObjectExportOption", "Group/ObjectExportOption",
    "Spread", "MasterSpread", "Page", "Spread/FlattenerPreference", "Layer",
    "TextFrame/TextFramePreference", "TextFrame/TextFrameFootnoteOptionsObject",
    "Color", "Tint", "Gradient", "Swatch", "Guide", "ObjectStyle/ObjectExportOption",
    "ParagraphStyle", "CharacterStyle",
    "Document/ConditionalTextPreference", "Document/EndnoteOption",
    "Document/TextFrameFootnoteOptionsObject", "Document/LinkedStoryOption",
    "Document/LinkedPageItemOption", "Document/WatermarkPreference",
    "Document/TaggedPDFPreference", "Document/AdjustLayoutPreference",
    "Document/HTMLFXLExportPreference", "Document/PublishExportPreference",
]
# Of those, the elements every IDML has one of (from some version on).
SINGLETONS = {p for p in ELEMENTS if p.startswith("Document/")}
# Elements with Self written as a whole list.
LISTS = ["TrapPreset"]
# Values that depend only on another attribute of the element: tag ->
# (key attribute, attributes). For each key value, an attribute is kept
# when every such element in every IDML has the same value, in at least
# MIN_KEYED_FILES files.
KEYED = {"Language": ("Name", ["SingleQuotes", "DoubleQuotes"])}
MIN_KEYED_FILES = 3
# Attributes that only some elements have, which the converter writes
# where it decides to (for example from DOM 15 on, or when a chunk is
# there): a value is kept when every element that has the attribute has
# the same value. path -> attributes.
WHEN_WRITTEN = {
    "TextFrame/TextFramePreference": [
        "ColumnRuleOffset", "ColumnRuleTopInset", "ColumnRuleBottomInset",
        "ColumnRuleInsetChainOverride", "ColumnRuleStrokeTint",
        "ColumnRuleStrokeType", "ColumnRuleOverprintOverride",
        "FootnotesEnableOverrides", "FootnotesSpanAcrossColumns",
        "VerticalThreshold", "UseFlexibleColumnWidth",
        "MinimumFirstBaselineOffset",
    ],
    "ParagraphStyle": ["EmitCss"],
    "CharacterStyle": ["EmitCss", "SplitDocument"],
}
# Values that are nearly constant; the exceptions are read from the INDD.
# (path, key) -> reason, as recorded in docs/format/idml-values.md.
EXPLAINED = {}
EXPLAINED_SHARE = 0.995
# The root styles, whose values tools/root_values.py collects; they are
# left out of the style paths here.
ROOT_STYLES = {
    "ParagraphStyle/$ID/[No paragraph style]",
    "CharacterStyle/$ID/[No character style]",
    "ObjectStyle/$ID/[None]",
    "CellStyle/$ID/[None]",
    "TableStyle/$ID/[No table style]",
}
# Attributes that are names or references of the element itself, or
# written by the converter from the document structure.
SKIP = {"Self", "Name"}
# Values the IDML schema the output is checked against does not allow
# (InDesign 8 and 9 wrote them): (tag, attribute).
SCHEMA_INVALID = {("ObjectExportOption", "CustomImageSizeOption")}


def exported_by_indesign(idml):
    """Whether designmap.xml has the `product` that InDesign writes in its
    `aid` processing instruction. One corpus IDML lacks it and differs
    from InDesign's exports in other ways (it was written by a script)."""
    head = zipfile.ZipFile(idml).read("designmap.xml")[:400]
    return re.search(rb'<\?aid [^>]*product="', head) is not None


def corpus_idmls():
    """(DOM major, IDML path) of every distinct corpus IDML."""
    lines = (ROOT / "corpus" / "inventory.tsv").read_text().splitlines()
    cols = lines[0].split("\t")
    seen, out = set(), []
    for line in lines[1:]:
        row = dict(zip(cols, line.split("\t")))
        dom = row.get("idml_dom", "")
        if not dom or not dom[0].isdigit() or row["path"].startswith(EXCLUDE):
            continue
        idml = ROOT / "corpus" / (row.get("idml") or str(Path(row["path"]).with_suffix(".idml")))
        digest = hashlib.md5(idml.read_bytes()).hexdigest()
        if digest not in seen and exported_by_indesign(idml):
            seen.add(digest)
            out.append((int(dom.split(".")[0]), str(idml)))
    return out


def prop_text(p):
    return re.sub(r">\s+<", "><", ET.tostring(p, encoding="unicode").strip())


def flatten(el):
    """Attributes as '@name' and Properties children as 'P/tag'."""
    out = {"@" + k: v for k, v in el.attrib.items()}
    for c in el:
        if c.tag == "Properties":
            for p in c:
                out["P/" + p.tag] = prop_text(p)
    return out


def scan(arg):
    """Per path: the flattened elements of one IDML; per list tag: the
    serialized list."""
    dom, idml = arg
    z = zipfile.ZipFile(idml)
    found = defaultdict(list)
    lists = defaultdict(list)
    wanted = set(ELEMENTS)
    for name in z.namelist():
        if not name.endswith(".xml"):
            continue
        root = ET.fromstring(z.read(name))
        for el in root.iter():
            if el.tag in LISTS:
                lists[el.tag].append(prop_text(el))
            if el.tag in KEYED:
                lists["keyed:" + el.tag].append(dict(el.attrib))
            if el.get("Self") is None:
                continue
            if el.tag in wanted and el.get("Self") not in ROOT_STYLES:
                found[el.tag].append(flatten(el))
            for ch in el:
                path = f"{el.tag}/{ch.tag}"
                if ch.get("Self") is None and path in wanted:
                    found[path].append(flatten(ch))
    return dom, idml, dict(found), dict(lists)


def ranges(per_dom, files_per_dom, newest):
    """Join consecutive versions with the same value. per_dom: {dom:
    value or None (varies)}; versions without files are skipped. Returns
    [(first, last or None, value, files)]."""
    out = []
    for d in sorted(per_dom):
        v = per_dom[d]
        if out and out[-1][2] == v and v is not None:
            first, _, _, n = out[-1]
            out[-1] = (first, d, v, n + files_per_dom[d])
        else:
            out.append((d, d, v, files_per_dom[d]))
    kept = []
    for first, last, v, n in out:
        if v is None or n < MIN_FILES:
            continue
        kept.append((first, None if last == newest else last, v, n))
    return kept


def analyse(scans):
    doms = sorted({d for d, _, _, _ in scans})
    newest = doms[-1]
    # (path, key) -> dom -> Counter of per-file status
    status = defaultdict(lambda: defaultdict(Counter))
    elements = defaultdict(lambda: defaultdict(Counter))  # (path,key)->dom->value counts
    present = defaultdict(lambda: defaultdict(int))  # path -> dom -> files with it
    files = Counter(d for d, _, _, _ in scans)
    counts = Counter()  # path -> elements
    order = defaultdict(dict)
    for dom, _idml, found, _lists in scans:
        for path, els in found.items():
            present[path][dom] += 1
            counts[path] += len(els)
            keys = {}
            for e in els:
                for k in e:
                    keys.setdefault(k, None)
                    order[path].setdefault(k, len(order[path]))
            for k in keys:
                vals = [e.get(k) for e in els]
                elements[(path, k)][dom].update(vals)
                if None in vals:
                    status[(path, k)][dom]["partial"] += 1
                elif len(set(vals)) > 1:
                    status[(path, k)][dom]["varies"] += 1
                else:
                    status[(path, k)][dom]["=" + vals[0]] += 1
    kept = defaultdict(list)  # path -> [(key, first, last, value, files)]
    written = defaultdict(list)  # the same, for WHEN_WRITTEN
    left_out = defaultdict(list)
    for (path, key), by_dom in status.items():
        if key[1:] in SKIP and key.startswith("@"):
            continue
        if (path.split("/")[-1], key[1:]) in SCHEMA_INVALID:
            continue
        per_dom = {}
        varies = False
        for d in doms:
            n = present[path].get(d, 0)
            s = by_dom.get(d, Counter())
            if not n or not s:
                continue
            if key[1:] in WHEN_WRITTEN.get(path, ()) and key.startswith("@"):
                c = {v for v in elements[(path, key)][d] if v is not None}
                if len(c) == 1:
                    per_dom[d] = next(iter(c))
                elif c:
                    varies = True
                continue
            if (path, key) in EXPLAINED:
                c = elements[(path, key)][d]
                total = sum(c.values())
                v, k = c.most_common(1)[0]
                if v is None or k / total < EXPLAINED_SHARE or sum(s.values()) != n:
                    varies = True
                per_dom[d] = v
            elif len(s) == 1 and sum(s.values()) == n and next(iter(s)).startswith("="):
                per_dom[d] = next(iter(s))[1:]
            else:
                varies = True
        if varies:
            left_out[path].append(key)
            continue
        rs = ranges(per_dom, {d: present[path].get(d, 0) for d in doms}, newest)
        target = written if key[1:] in WHEN_WRITTEN.get(path, ()) else kept
        for first, last, v, n in rs:
            target[path].append((key, first, last, v, n))
    presence = {}
    for path in SINGLETONS:
        per_dom = {d: (True if present[path].get(d, 0) == files[d] else None) for d in doms}
        presence[path] = ranges(per_dom, files, newest)
    return doms, files, kept, left_out, presence, order, elements, counts, written


def analyse_lists(scans, doms):
    newest = doms[-1]
    files = Counter(d for d, _, _, _ in scans)
    out = {}
    for tag in LISTS:
        per_dom = {}
        for d in doms:
            lists = {tuple(ls.get(tag, ())) for dd, _, _, ls in scans if dd == d}
            per_dom[d] = next(iter(lists)) if len(lists) == 1 and () not in lists else None
        out[tag] = ranges(per_dom, files, newest)
    return out


def node_xml(path_tag, attrs, props, indent):
    a = "".join(f" {k}={quoteattr(v)}" for k, v in attrs)
    if not props:
        return f"{indent}<{path_tag}{a} />\n"
    s = f"{indent}<{path_tag}{a}>\n{indent}\t<Properties>\n"
    s += "".join(f"{indent}\t\t{p}\n" for p in props)
    return s + f"{indent}\t</Properties>\n{indent}</{path_tag}>\n"


HEADER = """<?xml version="1.0" encoding="UTF-8"?>
<!--
Values that every IDML exported by InDesign in the corpus has on the
elements the converter writes, by DOM version. A block applies to
documents from MinimumVersion to MaximumVersion (no bound: all). Each
element names a path: a tag, or a tag and a child tag (Spread with child
FlattenerPreference). Values the converter reads from the INDD take
precedence. Present="true" marks an element every IDML of those versions
has. List blocks hold whole lists of elements. Generated by
tools/element_values.py; evidence in docs/format/idml-values.md.
-->
"""


def analyse_keyed(scans):
    """{tag: [(key value, [(attr, value)], files)]} for KEYED."""
    out = {}
    for tag, (key, attrs) in KEYED.items():
        seen = defaultdict(lambda: defaultdict(set))  # key value -> attr -> values
        files = defaultdict(set)
        for _dom, idml, _found, ls in scans:
            for a in ls.get("keyed:" + tag, []):
                k = a.get(key)
                files[k].add(idml)
                for name in attrs:
                    seen[k][name].add(a.get(name))
        rows = []
        for k in sorted(seen, key=lambda v: (v is None, v)):
            if k is None or len(files[k]) < MIN_KEYED_FILES:
                continue
            kept = [(n, next(iter(v))) for n, v in seen[k].items() if len(v) == 1 and None not in v]
            if kept:
                rows.append((k, kept, len(files[k])))
        out[tag] = rows
    return out


def write(kept, presence, lists, order, keyed, written):
    s = HEADER + "<ElementValues>\n"
    s += blocks_xml("Values", kept, presence, order)
    s += blocks_xml("WhenWritten", written, {}, order)
    for tag, rs in lists.items():
        for first, last, lst, _n in rs:
            gate = f' MinimumVersion="{first}"' + (f' MaximumVersion="{last}"' if last else "")
            s += f'\t<List Tag="{tag}"{gate}>\n'
            s += "".join(f"\t\t{e}\n" for e in lst)
            s += "\t</List>\n"
    for tag, rows in keyed.items():
        key = KEYED[tag][0]
        s += f'\t<Keyed Tag="{tag}" Key="{key}">\n'
        for k, attrs, _n in rows:
            a = "".join(f" {n}={quoteattr(v)}" for n, v in [(key, k)] + attrs)
            s += f"\t\t<{tag}{a} />\n"
        s += "\t</Keyed>\n"
    OUT.write_text(s + "</ElementValues>\n")


def blocks_xml(block_tag, kept, presence, order):
    blocks = defaultdict(lambda: defaultdict(list))  # (first,last) -> path -> [(key,v)]
    for path, items in kept.items():
        for key, first, last, v, _n in items:
            blocks[(first, last)][path].append((key, v))
    for path, rs in presence.items():
        for first, last, _v, _n in rs:
            blocks[(first, last)].setdefault(path, [])
    s = ""
    for (first, last) in sorted(blocks, key=lambda r: (r[0], r[1] or 999)):
        gate = f' MinimumVersion="{first}"' + (f' MaximumVersion="{last}"' if last else "")
        s += f"\t<{block_tag}{gate}>\n"
        by_parent = OrderedDict()
        for path in ELEMENTS:
            if path not in blocks[(first, last)]:
                continue
            items = sorted(blocks[(first, last)][path], key=lambda kv: order[path].get(kv[0], 0))
            parent, _, child = path.partition("/")
            by_parent.setdefault(parent, {"self": None, "children": []})
            pres = path in presence and any(f == first and l == last for f, l, _, _ in presence[path])
            attrs = [(k[1:], v) for k, v in items if k.startswith("@")]
            if pres:
                attrs = [("Present", "true")] + attrs
            props = [v for k, v in items if k.startswith("P/")]
            if child:
                by_parent[parent]["children"].append((child, attrs, props))
            else:
                by_parent[parent]["self"] = (attrs, props)
        for parent, n in by_parent.items():
            attrs, props = n["self"] or ([], [])
            if not n["children"]:
                s += node_xml(parent, attrs, props, "\t\t")
                continue
            a = "".join(f" {k}={quoteattr(v)}" for k, v in attrs)
            s += f"\t\t<{parent}{a}>\n"
            if props:
                s += "\t\t\t<Properties>\n" + "".join(f"\t\t\t\t{p}\n" for p in props)
                s += "\t\t\t</Properties>\n"
            for child, ca, cp in n["children"]:
                s += node_xml(child, ca, cp, "\t\t\t")
            s += f"\t\t</{parent}>\n"
        s += f"\t</{block_tag}>\n"
    return s


def main():
    idmls = corpus_idmls()
    with ProcessPoolExecutor() as pool:
        scans = list(pool.map(scan, idmls, chunksize=4))
    doms, files, kept, left_out, presence, order, elements, counts, written = analyse(scans)
    print(f"{len(idmls)} distinct IDML files; files per DOM version: "
          + ", ".join(f"{d}: {files[d]}" for d in doms))
    if "--key" in sys.argv:
        i = sys.argv.index("--key")
        path, key = sys.argv[i + 1], sys.argv[i + 2]
        for d in doms:
            print(d, elements[(path, key)].get(d, Counter()).most_common(4))
        return
    if "--why" in sys.argv:
        path = sys.argv[sys.argv.index("--why") + 1]
        for key in sorted(left_out.get(path, [])):
            total = Counter()
            for c in elements[(path, key)].values():
                total.update(c)
            n = sum(total.values())
            v, k = total.most_common(1)[0]
            odd = [f"{d}:{sum(c.values()) - c.most_common(1)[0][1]}/{sum(c.values())}"
                   for d, c in sorted(elements[(path, key)].items())
                   if len(c) > 1]
            print(f"  {key[1:] if key[0] == '@' else key}: {k}/{n} {k / n:.4f} {str(v)[:40]!r}  {' '.join(odd)}")
        return
    for path, items in written.items():
        print(f"{path}: when written: " + ", ".join(
            f"{k[1:]}={v!r} {f}{'' if l is None else f'-{l}'}" for k, f, l, v, _ in items))
    for path in ELEMENTS:
        items = kept.get(path, [])
        by_range = Counter((f, l, n) for _, f, l, _, n in items)
        line = "; ".join(f"{k} from {f}{'' if l is None else f' to {l}'} ({n} files)"
                         for (f, l, n), k in sorted(by_range.items(), key=lambda x: (x[0][0], x[0][1] or 99)))
        extra = ""
        if path in presence:
            extra = " present: " + ", ".join(
                f"{f}{'+' if l is None else f'-{l}'} ({n} files)" for f, l, _, n in presence[path])
        print(f"{path} ({counts[path]} elements): kept {line}{extra}")
        if left_out.get(path):
            print(f"    left out ({len(left_out[path])}): {', '.join(sorted(k[1:] if k[0] == '@' else k for k in left_out[path]))}")
    lists = analyse_lists(scans, doms)
    for tag, rs in lists.items():
        print(f"list {tag}: " + ", ".join(
            f"{len(v)} elements, {f}{'+' if l is None else f'-{l}'} ({n} files)" for f, l, v, n in rs))
    keyed = analyse_keyed(scans)
    for tag, rows in keyed.items():
        n_attrs = Counter(n for _, attrs, _ in rows for n, _ in attrs)
        print(f"keyed {tag} by {KEYED[tag][0]}: {len(rows)} values, kept {dict(n_attrs)}")
    if "--write" in sys.argv:
        write(kept, presence, lists, order, keyed, written)
        print(f"wrote {OUT.relative_to(ROOT)}")


if __name__ == "__main__":
    main()
