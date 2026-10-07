# indd-utils

Reads INDD files, the native document format of Adobe® InDesign®, without
InDesign, and converts them to IDML. The crate and the command are both
named `indd`.

Adobe and InDesign are trademarks of Adobe Inc. This project is not
affiliated with or endorsed by Adobe.

## Status

The converter turns INDD and INDT files into IDML packages. Every output
from the test corpus validates against the IDML RelaxNG schemas. It
writes the document
structure, pages, page items, placed graphics, story text with its
formatting, styles, swatches, tables, hyperlinks, fonts and XML
structure. Many attributes are still missing (see
[Current numbers](#current-numbers)), so the output is not yet the same
as InDesign's own IDML export.

The format is undocumented. Every fact the converter relies on was found
by comparing INDD files with IDML files exported from the same
documents, and is recorded with its evidence in [`docs/format/`](docs/format/).
[`CLEANROOM.md`](CLEANROOM.md) describes the method and its limits.

## Use

Build from source with a recent stable Rust:

```sh
cargo build --release
./target/release/indd convert in.indd out.idml
```

| Command | Output |
|---|---|
| `indd convert <in.indd> <out.idml>` | IDML package. Problems that do not stop the conversion are printed to stderr as `warning: …`. |
| `indd info <file.indd>...` | Header, master page and container summary |
| `indd objects <file.indd>` | One line per database object: UID, class, length, first bytes |
| `indd object <file.indd> <uid>` | One object's bytes, to stdout |
| `indd dump <file.indd> <uid>...` | The objects' chunks in hex |

INDT templates are read the same way as INDD files.

## Supported versions

| Versions | Byte order | Distinct samples | State |
|---|---|---|---|
| InDesign 3.0 and 4.0 | big-endian | 2 | Convert. Some object layouts differ from later versions ([`big-endian.md`](docs/format/big-endian.md)). Document preferences, object style text frame settings, the composite font (3.0) and the page number style (3.0) are left out with warnings. |
| InDesign 5.0 and 6.0 | — | none | Not known whether they use the old or the new layouts |
| InDesign 7.0–7.5 | little-endian | 3 | Convert. Document preferences are left out with a warning. |
| InDesign 8.0–21.6 | little-endian | 247 | Convert. Samples exist for 8.0, 9.2, 11.3–13.1 and 15.0–21.6. Tables from 9.2 are not read. |

The byte order flag in the header applies to object data only; the
database pages are little-endian in every file. No sample has another
combination of version and byte order.

## What converts

Each area below links to the document that holds the evidence.

| Area | Converted |
|---|---|
| Document structure | Spreads, master spreads, layers, sections (start, length, continued numbering, Arabic and lower-case Roman page numbers), the story list. [`objects.md`](docs/format/objects.md) |
| Pages and masters | Page bounds, transforms, applied master, margins and columns, page names, ruler guides, master names and prefixes. [`objects.md`](docs/format/objects.md) |
| Page items and graphics | Text frames (with threading), rectangles, ovals, polygons, graphic lines and groups: transforms, paths, fill and stroke, corners, applied object style, text wrap, frame fitting, anchored objects. Placed images, PDF, EPS and SVG with their bounds, clipping path settings and links; graphics pasted without a link keep their data. Transparency: blending, drop shadow, inner shadow and gradient feather where the samples tell the attributes apart. [`objects.md`](docs/format/objects.md), [`attributes.md`](docs/format/attributes.md), [`transparency.md`](docs/format/transparency.md) |
| Text and typography | Story text, paragraph and character ranges with their local formatting (86 text attributes, among them font, size, leading, tracking, indents, spacing, tabs, rules, shading, borders, bullets and numbering, nested styles, span columns, languages), text frame settings, text variables and their instances. [`attributes.md`](docs/format/attributes.md), [`text-variables.md`](docs/format/text-variables.md) |
| Styles | Paragraph, character, object, cell and table styles, with style groups, `BasedOn` and `NextStyle`. [`objects.md`](docs/format/objects.md), [`tables.md`](docs/format/tables.md) |
| Colours and swatches | Process, spot and registration colours, tints, gradients, inks, colour groups, the `None` swatch. [`objects.md`](docs/format/objects.md) |
| Tables | Tables in stories and in cells: rows, columns, headers and footers, cells with spans, cell text, cell and table formatting, applied styles. [`tables.md`](docs/format/tables.md) |
| Links and cross-references | Hyperlinks with text sources, page and URL destinations, bookmarks, cross-reference formats. [`hyperlinks.md`](docs/format/hyperlinks.md), [`cross-references.md`](docs/format/cross-references.md) |
| Fonts | Font families and fonts with their names, styles, types and PostScript names; composite font entries. [`fonts.md`](docs/format/fonts.md) |
| CJK | Kinsoku and mojikumi tables, composite fonts, grid alignment of paragraphs. [`objects.md`](docs/format/objects.md), [`fonts.md`](docs/format/fonts.md) |
| XML | Tags, the XML structure with elements placed in story text, the backing story. [`xml.md`](docs/format/xml.md) |
| Preferences | Page size, facing pages, bleed and intent from the INDD. The other preference values, and the values every InDesign export has on the root styles, are written as observed in all reference IDML files. [`idml-values.md`](docs/format/idml-values.md) |

## What does not convert

Reasons:

- **Not in public samples:** no sample with an IDML has the feature, so
  there is nothing to compare against.
- **Not provable:** the samples have the feature, but every sample has
  the same value, so the fields cannot be told apart.
- **Not started:** the samples have it, and no one has worked on it yet.

| Feature | Reason |
|---|---|
| Cross-reference sources, hyperlink text destinations, page item sources | Not in public samples |
| Footnotes, endnotes, notes, conditional text, index topics, buttons and forms | Not in public samples |
| Ruby, kenten, warichu, tate-chu-yoko | Not in public samples |
| XML attributes, comments, processing instructions, DTDs; XML elements that cross paragraphs | Not in public samples (left out with a warning) |
| Hyperlink sources that cross a style range | Not in public samples with an IDML (left out with a warning) |
| Most transparency effects (glows, bevel, satin, feathers other than gradient), drop shadow offsets | Not provable |
| Table and cell border weights, colours and types; cell edge gap colours | Not provable |
| Font `Status`, variable font design axes | Not provable (`Status` depends on the exporting machine) |
| Many attributes with one value in nearly every sample (`Visible`, `Locked`, `Name` of page items, …) | Not provable |
| Tables from InDesign 9.2 | Not started (different layout) |
| Document preferences from InDesign 3.0–7.5, object style text frame settings from 4.0 | Not started (different layouts) |
| Group transforms: most groups have no transform chunk, and the identity transform written differs from the reference for 275 of 289 groups | Not started |
| Transparency of placed graphics and object styles | Not started |
| Link metadata other than the URI and state | Not started |
| Indexing sort options, TOC styles, trap presets, the language list, named grids, document users, text on a path, QR codes | Not started |

## How fidelity is measured

The local test corpus holds INDD and INDT files made by other people for
other purposes. It is not redistributed. 78 distinct files come with an
IDML that InDesign exported from the same document at the same major
version.

- `tools/compare.py` converts each of the 78 files and compares the
  output with InDesign's IDML. Elements are matched by `Self` (the INDD
  UID), and each attribute value of the reference is counted as equal,
  different, or missing from the output. Story text and the formatting of
  each text range are compared too.
- `tools/compare.py --all` also converts the files without a usable IDML.
- With `--schemas` and `--jing`, every output is validated against the
  IDML RelaxNG schemas with Jing (`tools/validate.sh`).
- `cargo test` runs smoke tests on the open-licensed samples listed in
  `tests/fixtures/manifest.json` (fetch them first with
  `python3 -I tools/fetch_fixtures.py`) and, if `corpus/` exists, tests
  over the corpus.

## Current numbers

From `tools/compare.py --all` with schema validation, over 251 distinct
corpus files: 78 with a reference IDML and 173 without. The InDesign 4.0
fixture, which is not in the corpus, also converts and validates.

| Measure | Result |
|---|---|
| Conversion failures | 0 of 251 files |
| Schema validation failures | 0 of 251 files |
| Story text | 1,141 of 1,218 stories exact, 0 differ, 77 missing |
| Attribute values in elements the converter writes | 75.7 % equal, 0.1 % different, 24.2 % missing |

The 77 missing stories have no story object in the INDD file: 72 of their
UIDs do not exist there, so those IDML files were probably exported from
another save, and the other 5 are objects without a class.

Elements written, and attribute values that equal the reference:

| Element | Written / in reference | Attribute values equal |
|---|---|---|
| `Story` | 1,141 / 1,218 | 75 % |
| Text ranges (by start offset) | 2,156 / 2,159 | 93 % |
| `ParagraphStyle` | 499 / 500 | 82 % |
| `ObjectStyle` | 337 / 340 | 86 % |
| `TableStyle` | 160 / 160 | 98 % |
| `TextFrame` | 1,154 / 1,232 | 38 % |
| `Rectangle` | 761 / 818 | 19 % |
| `Polygon` | 1,066 / 1,089 | 27 % |
| `Image` | 235 / 263 | 9 % |
| `Page` | 325 / 335 | 32 % |
| `Color` | 1,352 / 1,358 | 62 % |
| `GradientStop` | 212 / 216 | 100 % |
| `Cell` | 1,089 / 1,329 | 81 % |
| `Font` | 2,778 / 2,890 | 91 % |
| `Hyperlink` | 43 / 45 | 100 % |
| `CrossReferenceFormat` | 684 / 702 | 100 % |
| `TextVariable` | 777 / 777 | 100 % |

Low rates for page items come mostly from attributes the converter does
not write, such as `Visible`, `Name`, `Locked`, layout constraints and
gradient hilites. Most of them have one value in nearly every sample, so
their INDD fields cannot be located.

The converter printed 2,622 warnings over the corpus. 2,608 of them are
hyperlink sources that cross a style range, in 16 versions of one
template.

## Development

- `python3 -I tools/fetch_fixtures.py` downloads the test fixtures.
- `cargo test`, `cargo clippy --all-targets`, `cargo fmt`.
- `python3 -I tools/compare.py [--all] [--detail TAG --show N]
  [--schemas DIR --jing DIR]` after every change. The schemas and Jing
  are not part of the repository; see `tools/validate.sh`.
- `python3 -I tools/inventory.py corpus/` lists each sample's InDesign
  version and whether it has a matching IDML file.
- Format findings go in `docs/format/`. Read [`CLEANROOM.md`](CLEANROOM.md)
  before contributing.

## Licence

Dual-licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at
your option. Copyright (c) 2026 aiongg and the indd-utils contributors.
The repository contains no sample files. The test fixtures are
downloaded from their sources and keep their own licences; see
`tests/fixtures/README.md`.
