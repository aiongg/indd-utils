# Architecture

This document explains how the converter is built: its layers, how data
flows from INDD bytes to an IDML package, where each kind of code lives,
where the evidence for format facts is kept, and how the generated value
files work. For how to change the code, see [`CONTRIBUTING.md`](CONTRIBUTING.md).

## Layers

Each layer uses only the layers above it in this table.

| Layer | Module | Input → output | Knows about |
|---|---|---|---|
| Header | `src/header.rs` | first bytes → `Header` (byte order, version) | the fixed header |
| Container | `src/container.rs` | file bytes → pages, master page, contiguous objects | the page layout |
| Database | `src/database.rs` | pages → object bytes by UID, class of each UID | the object trees and data pages |
| Objects | `src/object.rs` | object bytes → chunks; `Cursor` decodes numbers, strings and text | the byte order and string tag (`Encoding`) |
| Model | `src/model/` | objects → `Document` | INDD classes, chunks and attribute IDs |
| Writer | `src/idml/` | `Document` → IDML package (ZIP) | IDML elements, attributes and naming rules |
| Audit | `src/audit.rs` | what the model read → `Audit` report | nothing of the format; records reads |

The model holds decoded INDD data. The writer only maps model types to
IDML: it does not read INDD bytes. When the writer needs a new INDD
value, decode it in the model and add a field or a `Value` variant.

## Data flow

`indd::convert_into` (`src/lib.rs`) runs these steps:

1. `Container::parse` checks the header and splits the file into pages.
2. `Container::database` reads the object trees into a `Database`. The
   database gets the file's `Encoding` from the header.
3. `model::Reader::document` reads the document object (UID 1), then
   every object of the class list whose class it knows, and builds a
   `Document`. Each object comes from `Database::get` and carries the
   `Encoding`, so every decoder reads it in the file's byte order.
4. `idml::write` writes the package parts (`designmap.xml`, spreads,
   stories, resources, styles) and stores them in a ZIP file.
5. The warnings of the model and of the writer are returned with the
   package.

`indd::convert` returns the package as bytes; `indd::convert_file`
reads a file first. The CLI (`src/main.rs`) uses `convert_into` to write
to a file.

### No global state

A conversion keeps all its state in its own values:

- The byte order and the string tag are an `Encoding`, held by the
  `Database`, each `Object` and each `Cursor`.
- The `Reader` holds its object cache, warnings and the state it collects
  while reading (XML nodes, frame orientations, the group path).
- `indd audit` gives the `Database` an `audit::Recorder`; objects and
  attribute lists carry it and report their reads to it. A normal
  conversion has no recorder.

The only statics are caches of the constant value files (`OnceLock`).
Documents can therefore be converted on several threads at once; the
fixture test `conversions_on_several_threads_are_independent` checks it.

### Errors and warnings

| Problem | Result |
|---|---|
| The file is not an INDD file, is truncated, or its database structures are damaged | `Err(Error)`: no package |
| One object cannot be read (a story, spread, page item, style, colour, …) | The object is left out with a warning |
| An attribute list cannot be parsed | The list is left out with a warning |
| A value has no IDML equivalent, or one the samples do not show | The value is left out, with a warning where a reader would miss it; unknown codes are reported to `indd audit` |

The model also guards against damaged structures that would recurse
without end or allocate too much: groups that contain themselves, style
group cycles, table sizes larger than their cell data.

## Module map

### `src/model/`

| Module | Reads |
|---|---|
| `reader.rs` | `Reader`: object cache, chunk access, child lists, warnings |
| `document.rs` | `Document`; `Reader::document` and one function per class or area |
| `ids.rs` | class, chunk and strand IDs and enumeration codes |
| `attrs.rs` | attribute lists (`Attrs`, `Value`), including structured values: tab lists, nested styles, bullet characters, opacity stops |
| `spread.rs` | spreads, pages, guides, layers, sections |
| `item.rs` | page items: transforms, paths, frames, graphics, links, text wrap |
| `story.rs` | stories: text runs, anchored objects, tables in text, hyperlink sources, XML markers |
| `style.rs` | paragraph, character, object and TOC styles, style groups, anchored object, text frame and story settings |
| `table.rs` | tables, rows, columns, cells, cell and table styles |
| `color.rs`, `font.rs`, `cjk.rs` | swatches and inks; font families; composite fonts and kinsoku and mojikumi tables |
| `hyperlink.rs`, `xref.rs`, `variable.rs`, `xml.rs` | hyperlinks and bookmarks; cross-reference formats; text variables; the XML structure |
| `settings.rs`, `prefs.rs` | document-level lists and preferences |
| `strings.rs` | searches for names at positions that are not decoded |

### `src/idml/`

| Module | Writes |
|---|---|
| `mod.rs` | `Writer` and `write`: the package parts in order |
| `designmap.rs` | `designmap.xml`: document settings, languages, layers, sections, hyperlinks, TOC styles, text variables |
| `spread.rs` | spreads and master spreads: pages, guides, page items |
| `story.rs` | stories, tables in stories, the XML backing story |
| `styles.rs` | `Resources/Styles.xml` |
| `resources.rs` | `Resources/Graphic.xml`, `Fonts.xml`, `Preferences.xml` |
| `pages.rs` | page numbering and page names |
| `attrs.rs` | attribute tables: INDD attribute ID → IDML name and value kind |
| `kind.rs` | `Kind`: how each kind of value becomes IDML text |
| `transparency.rs` | transparency effects of page items |
| `values.rs` | lookups in the generated value files |
| `format.rs` | number, matrix, base64 and name formats |
| `xml.rs`, `zip.rs` | the XML and ZIP writers |

## Where evidence lives

| What | Where |
|---|---|
| How the format was learned, and the rules for new facts | [`CLEANROOM.md`](CLEANROOM.md) |
| Every fact about the INDD format, with the files and counts that support it | [`docs/format/`](docs/format/), one file per area |
| Values written because every InDesign export has them | [`docs/format/idml-values.md`](docs/format/idml-values.md) |
| How fidelity is measured (stale pairs, value coverage) | [`docs/measurement.md`](docs/measurement.md) |
| The samples | `corpus/`, local and git-ignored; `corpus/inventory.tsv` lists version and pairing |

Code comments cite the `docs/format/` section a decoder relies on. Code
uses only facts recorded there.

## Generated value files

IDML consumers apply their own defaults to attributes that are missing.
To avoid that, the writer also writes values that every InDesign export
in the corpus has, even when the INDD has no field for them. These
values are facts about InDesign's IDML output, so their evidence is the
corpus IDML files alone ([`idml-values.md`](docs/format/idml-values.md)).

| File | Generator | Holds | Read by (`src/idml/values.rs`) |
|---|---|---|---|
| `src/idml/element_values.xml` | `tools/element_values.py` | values of page items, spreads, pages, swatches, styles, document elements and preferences; lists such as trap presets; values keyed on another attribute | `element`, `element_attrs`, `when_written`, `present`, `list`, `keyed` |
| `src/idml/root_values.xml` | `tools/root_values.py` | values of the five root styles | `root_style` |
| `src/idml/object_style_values.xml` | `tools/root_values.py` | the object style `[None]` | `object_style` |
| `src/idml/preference_values.xml` | `tools/root_values.py` | the elements of `Resources/Preferences.xml` | `preferences` |

How they work:

- A generator reads every corpus IDML and keeps a value when every
  element on the path has it, in every file of a range of DOM versions.
  Blocks in the file carry that range (`MinimumVersion`,
  `MaximumVersion`); the writer asks for the document's version.
- The files are committed. Regenerate them with `--write` after the
  corpus changes, and review the diff: a value that varies in new
  samples disappears.
- `root_values.py` is pinned to an earlier part of the corpus (`SOURCE`
  in the tool); see `idml-values.md` for why.

**Precedence.** A value read from the INDD always wins over an observed
value. The XML writer (`src/idml/xml.rs`) keeps the attributes of an
open start tag until it closes: `Xml::attr` sets a value, replacing an
observed one in place, and `Xml::attrs_missing` adds observed values
only for attributes the element does not have. For value trees
(`values::Node`), `Node::set` replaces and `Node::merge` adds missing
values.

A few observed values are still written as literals in the writer (for
example the story attributes in `idml/story.rs` and font `PlatformName`).
Each cites `idml-values.md`.

## Tests and tools

| Check | What it covers |
|---|---|
| Unit tests (`cargo test`) | decoders on synthetic bytes (`database::synthetic` builds objects and databases), writer functions on small models |
| `tests/fixtures.rs` | conversion of open-licensed fixtures (`tools/fetch_fixtures.py`), both byte orders, thread independence, audit consistency |
| `tests/corpus.rs` | every corpus file parses and converts; skipped without `corpus/` |
| `tools/compare.py` | fidelity against the reference IDML files, schema validation |
| `tools/diff_outputs.py` | byte-for-byte output of two revisions over the corpus |
| `indd audit`, `tools/audit_corpus.py` | what the converter does not read yet |
