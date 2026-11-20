# Contributing

The shared rules in [`AGENTS.md`](AGENTS.md) apply to every contribution.
[`CLEANROOM.md`](CLEANROOM.md) records how the INDD format was learned from
sample files. [`ARCHITECTURE.md`](ARCHITECTURE.md) explains where the code for each
step lives.

## Setting up

1. Install a recent stable Rust and Python 3.
2. `python3 -I tools/fetch_fixtures.py` downloads the test fixtures and
   checks their SHA-256.
3. `python3 -I tools/fetch_corpus.py` downloads the openly licensed part
   of the corpus (about 5.5 GB, listed with source and licence in
   `tools/corpus-manifest.json`) into `corpus/open/` and checks every
   file's SHA-256. `--source SLUG` fetches one source, `--list` lists
   them. Put other samples in `corpus/` too, and run
   `python3 -I tools/inventory.py corpus/` to list their versions and
   pairs. A pair is an INDD file with an IDML exported from the same
   document.
4. For schema validation, get the IDML RelaxNG schemas and Jing and keep
   them outside the repository (see `tools/validate.sh`).
5. For the WebAssembly check, run `rustup target add
   wasm32-unknown-unknown` and install Node.js.


## Checks

There is no hosted CI. Run the checks for the code you changed on your own
machine, and say in the PR what ran:

```sh
cargo fmt
cargo clippy --all-targets
cargo test --release
cargo doc --no-deps
tools/check_wasm.sh                           # WebAssembly build, same output as native
python3 -I tools/diff_outputs.py              # output of HEAD vs the working tree

cargo build --release && python3 -I tools/compare.py --exclude own/
```

After a change to the IDML writer, also validate every output:
`python3 -I tools/compare.py --all --exclude own/ --schemas <dir> --jing <dir>`
(about 5 minutes on the full corpus).

A refactoring should leave `diff_outputs.py` with 0 differing outputs. A behaviour change should add no conversion or schema validation failures and not lower value coverage.

To see how a change affects rendering, compare renders of the documents
that come with an InDesign PDF. This needs poppler-utils (`pdfinfo`,
`pdftoppm`, `pdftotext`, `pdffonts`) and a build of DesignCraft's
command-line program:

```sh
cargo build --release && python3 -I tools/render_compare.py --designcraft PATH/designcraft-cli
```

It reports how many documents and pages render pixel-identical from the
converter's IDML and from the reference IDML, and the first differing
page of each other document (`docs/measurement.md`, "Render
comparison").

## Adding something end to end

The same six steps apply whether you add a text attribute, a page item
setting, an element or a constant value.

### 1. Find it

- `indd audit <file>` lists what the converter does not read in one
  document: classes, chunks, attribute IDs, unknown codes.
- `python3 -I tools/audit_corpus.py --exclude own/` ranks those over the
  corpus by files affected (tables in `target/audit/`).
- `compare.py` prints the biggest value gaps and writes `gaps.tsv` and
  `values.tsv` to `target/compare/`; `--detail TAG --show N` shows the
  differing values of one element.

### 2. Prove it

- Pair an INDD object with its IDML element: the element's `Self` is the
  object's UID in hexadecimal (`Self="u1a2"` is UID 0x1A2).
- Look at the bytes with `indd dump <file> <uid>...` and `indd objects`.
- Write a scratch script (outside the repository) that compares the
  candidate field with the IDML value over **every** pair, and count the
  matches. Use trustworthy pairs (`docs/measurement.md`).
- A field whose value is the same in every sample cannot be told apart
  from its neighbours. Do not map it.

### 3. Document it

Add the fact to the matching file in `docs/format/`: the layout, the
mapping, and the evidence (files, counts such as "171 of 171").

### 4. Decode it

All INDD decoding is in `src/model/`.

| What | Where |
|---|---|
| A class or chunk ID | `src/model/ids.rs` (or the area module's `class` / `chunk`) |
| A value in an attribute list | nothing for a number; for a structured value, a `Layout` in `text_layout` and a `Value` variant (`src/model/attrs.rs`) |
| A setting in its own chunk | a field on the model struct, read in the area module (`item.rs`, `style.rs`, …) with `self.chunk` and `self.cursor` |
| A new class | a struct, a reader function, and an arm in `Reader::class_object` (`src/model/document.rs`) |

Read numbers and strings with a `Cursor` from `self.cursor(&data)` (or
`obj.cursor(data)`), never with `from_le_bytes`: object data is in the
file's byte order. Add a unit test that builds the bytes in code; the
test-only `database::synthetic` module builds objects, flagged strings
and whole databases, in either byte order.

### 5. Write it

The writer (`src/idml/`) maps model types to IDML and never reads INDD
bytes.

| What | Where |
|---|---|
| An attribute in a text, page item, cell or table list | a row (ID, IDML name, `Kind`) in the tables of `src/idml/attrs.rs` |
| A new kind of value | a `Kind` variant and its arm in `Writer::value` (`src/idml/kind.rs`) |
| A setting or element | the writer function for its element, at the position the schema requires |

Write values read from the INDD with `Xml::attr`. They take precedence
over observed values in any order.

### 6. Measure it

Run `compare.py` and `diff_outputs.py`. Check that value coverage rose,
that no extra values appeared (`extras.tsv`), and that every output
still validates. Put the numbers in the commit message.

### Constant values

A value that every InDesign export has, but that has no INDD field, comes
from the generated value files, not from a literal in the writer:

1. Add the element path to `ELEMENTS` (or `LISTS`, `KEYED`,
   `WHEN_WRITTEN`) in `tools/element_values.py`.
2. Run `python3 -I tools/element_values.py --write` and review the diff
   of `src/idml/element_values.xml`.
3. Read the values with `values::element` (or `element_attrs`,
   `when_written`, `list`, `keyed`) where the writer writes the element.
4. Record the evidence in `docs/format/idml-values.md`.

## Tools

- `python3 -I tools/check.py LABEL [--base REV]` runs the build, tests, clippy, the measurement with schema validation, the output diff against REV (default `origin/main`), the README numbers, the render comparison and the privacy checks, and freezes the results to `~/.cache/indd-frozen-LABEL/`. It reuses earlier conversions and renders (`~/.cache/indd-check/`). Local settings are in `.thoughts/tools/check.json`; details in `docs/measurement.md`, "Check runs".
- `tools/check_wasm.sh [file.indd...]` builds the library for `wasm32-unknown-unknown` (`rustup target add wasm32-unknown-unknown`), checks that a conversion imports no host function and compares its output with the native output. It needs Node.js.
- `python3 -I tools/readme_numbers.py --schemas <dir> --jing <dir>` rewrites the README's "Current numbers" table (between `<!-- numbers:start -->` and `<!-- numbers:end -->`) from `target/compare/summary.json`; `--check` only reports whether the README is current.
- `python3 -I tools/diff_outputs.py [OLD [NEW]]` builds two revisions (default `HEAD` and the working tree) under `~/.cache/indd-diff/`, converts every corpus file with both and reports differing outputs, failures and warnings; the full list is `~/.cache/indd-diff/diff.tsv`.
- `python3 -I tools/render_compare.py --designcraft PATH [--jobs N] [--file SUBSTR]` renders the reference IDML and the converter's IDML with `designcraft-cli` (from `storytold/designcraft`) and compares both with the same-save PDF; output in `target/render-compare/`.
- `python3 -I tools/compare.py [--detail TAG --show N] [--all] [--exclude PREFIX] [--schemas <dir> --jing <dir>]` converts every corpus pair and compares with the reference IDML; it writes `pairs.tsv`, `gaps*.tsv`, `values*.tsv`, `extras*.tsv` and `summary.json` to `target/compare/`. `--stale N` lists stale pairs, `--trusted` limits the tables to trustworthy pairs, `--warnings N` lists warning kinds.
- `tools/validate.sh a.idml [b.idml...] <schemas> <jing>` validates packages against the IDML RelaxNG schemas (`VALIDATE_JOBS` runs them in parallel). Keep the schemas and Jing outside the repository.
- `indd audit <file>` and `python3 -I tools/audit_corpus.py --exclude own/` list what the converter does not read; `tools/root_values.py [--write]` regenerates `src/idml/root_values.xml`.
- `INDD_HOSTILE_SCALE=40 cargo test --lib hostile` runs a larger hostile-input sweep.
- `cargo run -q -- info <file>`, `indd convert in.indd out.idml`, `indd objects <file>`, `indd dump <file> <uid>...`, `indd uids <file>` and `indd xmp <file>` inspect and convert one file.
