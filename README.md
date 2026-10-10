# indd-utils

Reads INDD files, the native document format of Adobe® InDesign®, without
InDesign, and converts them to IDML. The crate and the command are both
named `indd`.

Adobe and InDesign are trademarks of Adobe Inc. This project is not
affiliated with or endorsed by Adobe.

## Status

The converter turns INDD and INDT files into IDML packages. It writes
the document structure, pages, page items, placed graphics, story text
with its formatting, footnotes, endnotes, notes and tracked changes,
styles, swatches, tables, hyperlinks and cross-references, fonts and XML
structure. Its output validates against the IDML RelaxNG schemas, except
for endnote markup, which it writes as InDesign does and which the
published schema rejects ([`docs/measurement.md`](docs/measurement.md)).
Some attributes are still missing (see
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
| `indd dump [--full] <file.indd> <uid>...` | The objects' chunks in hex |
| `indd uids <file.indd>` | One line per object: UID and class, without reading object data |
| `indd xmp <file.indd>` | The document's XMP packet |
| `indd audit [--tsv] <file.indd>` | What the converter does not read in the document |

INDT templates are read the same way as INDD files.

As a library:

```rust
let conversion = indd::convert_file("brochure.indd")?;
std::fs::write("brochure.idml", &conversion.idml)?;
for warning in &conversion.warnings {
    eprintln!("warning: {warning}");
}
```

`indd::convert(bytes, name)` converts bytes in memory and
`indd::convert_into(bytes, name, writer)` writes the package to a
writer. A conversion keeps no global state, so several documents can be
converted on different threads at once.

The library also builds for WebAssembly (`wasm32-unknown-unknown`):
`indd::convert` needs no file system or other host function, and gives
the same package as a native build (`tools/check_wasm.sh`).


## Supported versions

| Versions | Distinct samples | State |
|---|---:|---|
| InDesign 1.x | 8 | Not supported (different container); reported as such. |
| InDesign 3.0 and 4.0 | 67 | Convert. Some object layouts differ from later versions ([`big-endian.md`](docs/format/big-endian.md)). Document preferences, object style text frame settings, and the page number style (3.0) are left out with warnings. |
| InDesign 5.0 and 6.0 | 181 | Convert. Document preferences are left out with a warning. |
| InDesign 7.0–7.5 | 566 | Convert. Document preferences are left out with a warning. |
| InDesign 8.0–21.6 | 3,438 | Convert. Tables from 9.2 are not read. |

Files in both byte orders convert. The byte order applies to object
data only; the database pages are little-endian in every file.

## What converts

Each area below links to the document that holds the evidence.

| Area | Converted |
|---|---|
| Document structure | Spreads, master spreads, layers, sections (start, length, continued numbering, Arabic and lower-case Roman page numbers), the story list. [`objects.md`](docs/format/objects.md) |
| Pages and masters | Page bounds, transforms, applied master, margins and columns, page names, ruler guides, master names and prefixes, primary text frames. [`objects.md`](docs/format/objects.md) |
| Page items and graphics | Text frames (with threading), rectangles, ovals, polygons, graphic lines and groups: transforms, paths, fill and stroke, corners, applied object style, text wrap, frame fitting, anchored objects, text on a path. EPS text. Placed images, PDF, EPS, SVG and InDesign pages with their bounds, clipping path settings and links; graphics pasted without a link keep their data. Transparency: blending, drop shadow, inner shadow, outer and inner glow and gradient feather where the samples tell the attributes apart. [`objects.md`](docs/format/objects.md), [`attributes.md`](docs/format/attributes.md), [`transparency.md`](docs/format/transparency.md) |
| Text and typography | Story text, paragraph and character ranges with their local formatting (about 200 text attributes, among them font, size, leading, tracking, indents, spacing, hyphenation, keeps, tabs, rules, underline and strikethrough, shading, borders, bullets and numbering lists, nested styles, span columns, directions, OpenType and CJK settings, languages), story settings, text frame settings, text variables and their instances, footnotes and endnotes with their options, notes, tracked changes, ruby (text, type, alignment, position, offsets, scaling and spacing), kenten kind and size, warichu, tate-chu-yoko. [`attributes.md`](docs/format/attributes.md), [`text-variables.md`](docs/format/text-variables.md) |
| Styles | Paragraph, character, object, cell and table styles, with style groups, `BasedOn` and `NextStyle`. [`objects.md`](docs/format/objects.md), [`tables.md`](docs/format/tables.md) |
| Colours and swatches | Process, spot and registration colours, tints, gradients, inks, colour groups, the `None` swatch. [`objects.md`](docs/format/objects.md) |
| Tables | Tables in stories and in cells: rows, columns, headers and footers, cells with spans, cell text, cell and table formatting, applied styles. [`tables.md`](docs/format/tables.md) |
| Links and cross-references | Hyperlinks with text, paragraph and page item sources; text, page, URL and external page destinations; bookmarks; cross-reference sources and formats. [`hyperlinks.md`](docs/format/hyperlinks.md), [`cross-references.md`](docs/format/cross-references.md) |
| Fonts | Font families and fonts with their names (including built-in and native-script names), styles, types, PostScript names and Typekit IDs; missing fonts; composite font entries. [`fonts.md`](docs/format/fonts.md) |
| CJK | Kinsoku and mojikumi tables (with the spacing of custom mojikumi tables), composite fonts, grid alignment of paragraphs, layout grids of pages, named grids, frame grids and their stories. [`objects.md`](docs/format/objects.md), [`fonts.md`](docs/format/fonts.md) |
| XML | Tags, the XML structure with elements placed in story text, the backing story. [`xml.md`](docs/format/xml.md) |
| Index | The index with its topics, page references in the text, index options and header groups, index sort groups. [`index.md`](docs/format/index.md), [`preferences.md`](docs/format/preferences.md) |
| Document lists | TOC styles, trap presets, numbering lists, the language list, named grids, document users, the document label. [`objects.md`](docs/format/objects.md), [`idml-values.md`](docs/format/idml-values.md) |
| Preferences | Document setup, view, guides, grids, text, margins, pasteboard, colour settings, default styles and grids, print and booklet print settings, chapter numbering, dictionary, EPUB version and identifier, from the INDD. The other preference values, and the values every InDesign export has on the root styles, are written as observed in all reference IDML files. [`preferences.md`](docs/format/preferences.md), [`idml-values.md`](docs/format/idml-values.md) |

## What does not convert

Reasons:

- **Not in public samples:** no sample with an IDML has the feature, so
  there is nothing to compare against.
- **Not provable:** the samples have the feature, but every sample has
  the same value, so the fields cannot be told apart.
- **Not started:** the samples have it, and no one has worked on it yet.

| Feature | Reason |
|---|---|
| Conditional text, buttons and forms | Not in public samples |
| Ruby and kenten fonts, colours, weights and Y scales; kenten placement and characters; warichu characters before and after a break | Not provable (every sample has the default value) |
| XML attributes, comments, processing instructions, DTDs; XML elements that cross paragraphs | Not in public samples (left out with a warning) |
| Hyperlink sources that cross a style range | Not in public samples with an IDML (left out with a warning) |
| Most transparency effects (bevel, satin, feathers other than gradient, some glow values), drop shadow offsets | Not provable |
| Which table border attribute is which side (all four sides are equal in every sample) | Not provable |
| Font `Status`, variable font design axes | Not provable (`Status` depends on the exporting machine) |
| Many attributes with one value in nearly every sample (`Visible`, `Locked`, `Name` of page items, …) | Not provable |
| Tables from InDesign 9.2 | Not started (different layout) |
| Document preferences from InDesign 3.0–7.5, object style text frame settings from 4.0 | Not started (different layouts) |
| Transparency of placed graphics and object styles | Not started |
| Index title in languages other than Japanese or Chinese without stored index options, endnote title without stored endnote options; `BitmapPrinting` of the print settings | Not provable (they follow the exporting application or computer) |
| QR codes | Not started |

## How fidelity is measured

The local test corpus holds INDD and INDT files made by other people for
other purposes. It is not redistributed; its openly licensed part can be
downloaded with `tools/fetch_corpus.py`. 803 distinct files come with an
IDML that InDesign exported from the same document at the same major
version. In 606 of these pairs the IDML shows the same save as the INDD
(trustworthy pairs); the others were saved again after the export
([`docs/measurement.md`](docs/measurement.md)).

- `tools/compare.py` converts each paired file and compares the output
  with InDesign's IDML. Elements are matched by `Self` (the INDD UID).
  The headline is value coverage: the share of the reference's values
  that the output reproduces.
- `tools/compare.py --all` also converts the files without a usable IDML.
- With `--schemas` and `--jing`, every output is validated against the
  IDML RelaxNG schemas with Jing (`tools/validate.sh`).
- `cargo test` runs unit tests, smoke tests on the open-licensed samples
  listed in `tests/fixtures/manifest.json` (fetch them first with
  `python3 -I tools/fetch_fixtures.py`) and, if `corpus/` exists, tests
  over the corpus.

## Current numbers

<!-- numbers:start -->
From `tools/compare.py --all` with schema validation, without the
privately held samples:

| Measure | Result |
|---|---|
| Conversion failures | 0 of 1,460 paired files; 0 of 4,303 other files |
| Rejected files | 0 of 1,460 paired files; 22 of 4,303 other files (11 not INDD files, 8 InDesign 1.x files, 2 truncated, 1 without an object database) |
| Schema validation failures | 0 paired files, 11 other files |
| Value coverage, trustworthy pairs | 97.47 % (28,118,816 of 28,848,732 values) |
| Value coverage, all pairs | 96.24 % |
| Story text, trustworthy pairs | 37,112 of 37,150 stories exact, 20 differ, 18 missing |
<!-- numbers:end -->


The biggest remaining gaps are elements the converter does not write
yet (hyperlinks in some documents, page references, footnote options of
text frames, graphic layers) and attributes that have one value
in nearly every sample, so that their INDD fields cannot be located.
`compare.py` lists them, and writes per-key tables to `target/compare/`.

## Development

- [`CONTRIBUTING.md`](CONTRIBUTING.md): the clean-room rules, the checks
  to run, and how to add an attribute, element or constant value end to
  end. Read [`CLEANROOM.md`](CLEANROOM.md) first.
- [`ARCHITECTURE.md`](ARCHITECTURE.md): layers, data flow, module map,
  and how the generated value files work.
- `python3 -I tools/fetch_fixtures.py` downloads the test fixtures.
- `cargo test`, `cargo clippy --all-targets`, `cargo fmt`.
- `python3 -I tools/compare.py [--all] [--detail TAG --show N]
  [--schemas DIR --jing DIR]` after every change. The schemas and Jing
  are not part of the repository; see `tools/validate.sh`.
- `python3 -I tools/diff_outputs.py [OLD [NEW]]` compares the output of
  two revisions over the corpus; a refactoring must change none.
- `python3 -I tools/readme_numbers.py --schemas DIR --jing DIR` runs the
  full measurement and updates [Current numbers](#current-numbers).

- `python3 -I tools/inventory.py corpus/` lists each sample's InDesign
  version and whether it has a matching IDML file.

## Licence

Dual-licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at
your option. Copyright (c) 2026 aiongg and the indd-utils contributors.
The repository contains no sample files. The test fixtures are
downloaded from their sources and keep their own licences; see
`tests/fixtures/README.md`.
