# Contributing

This project learns the INDD format only from sample files, under the
clean-room rules in [`CLEANROOM.md`](CLEANROOM.md). Read that file first.
[`ARCHITECTURE.md`](ARCHITECTURE.md) explains where the code for each
step lives.

## Clean-room rules

Every contributor follows these rules. A change that breaks one is not
accepted.

- **No Adobe software.** Do not install or run InDesign, and do not use
  the InDesign SDK, its documentation or its binaries.
- **No knowledge from elsewhere.** Do not use what you know or remember
  of InDesign internals (class names, IDs, persistence details). Use only
  what the samples show and the references `CLEANROOM.md` permits.
- **No made-to-order samples.** Do not create INDD files for this project
  or ask anyone to make them.
- **Every fact has evidence.** A fact about the INDD format goes in
  `docs/format/` with what supports it: which files, how many, what was
  compared. Code relies only on documented facts.
- **No sample files in the repository.** Never commit INDD, IDML, PDF,
  images or archives, whatever their licence. Open-licensed test fixtures
  are downloaded by `tools/fetch_fixtures.py`; other samples stay in the
  git-ignored `corpus/`.
- **No golden files from samples.** Tests compare with the reference IDML
  at run time; they do not store output derived from third-party files.
- **Do not name the corpus sources** in committed files. The local
  `corpus/SOURCES.md` records where the samples came from.
- **Privately held samples stay private.** Leave them out of committed
  numbers (`--exclude own/` in the tools) and never describe their
  content.
- **Treat sample files as untrusted.** Run Python tools with `-I`, keep
  downloads in their own directory, and keep scratch scripts out of it.

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

There is no hosted CI. Run the checks on your own machine once your change
is complete, before you push it:

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

| Kind of change | Required result |
|---|---|
| Refactoring | `diff_outputs.py`: 0 differing outputs, 0 changed failures and warnings |
| Behaviour change | `compare.py`: no new conversion or schema validation failures; value coverage does not fall; the diff of `diff_outputs.py` is explained in the commit message |

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

## Commits

- Use conventional commit messages: `feat:`, `fix:`, `refactor:`,
  `test:`, `docs:`, `chore:`.
- One change per commit. A refactoring commit changes no output.
- For a behaviour change, say in the body what changed in the output and
  why, with the numbers from `compare.py`.
- Stage only the files you changed; never commit `corpus/`, fixture
  files, schemas, Jing, or tool output.
- Do not add hosted CI workflows.
