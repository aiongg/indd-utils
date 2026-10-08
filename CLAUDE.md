# indd-utils

Rust reader for INDD files (Adobe InDesign's native format) and converter
to IDML. Crate and command: `indd`. Repo: `aiongg/indd-utils` (private for
now). Licence: MIT OR Apache-2.0.

## Clean-room rules (read `CLEANROOM.md`)

- Never commit downloaded sample files of any kind (INDD, IDML, PDF,
  images, archives), whatever their licence. They can be downloaded
  elsewhere; the repo records only how to get them. `corpus/` is
  git-ignored and stays local.
- Every fact about the INDD format goes in `docs/format/` with the evidence
  that supports it (which files, how many, what was compared). Code relies
  only on documented facts.
- Do not use knowledge of InDesign internals from memory (InDesign SDK
  class names, IDs, persistence details). If it can't be shown from the
  corpus or a permitted reference in `CLEANROOM.md`, it doesn't go in.
- Do not install or run InDesign, and do not ask anyone to create INDD
  files for this project.
- No golden output files derived from third-party samples. Corpus tests
  compare against the sibling IDML at run time.
- Openly licensed corpus sources may be listed in a committed fetch
  manifest (pinned URL, hash, licence) so contributors can rebuild that
  part of the corpus. Do not name any other source in committed files.
  The full provenance record is `corpus/SOURCES.md` (local).

## Layout

- `src/header.rs`, `src/container.rs`: decoded layers (see `docs/format/`).
- `src/audit.rs`: records what a conversion reads, for `indd audit`.
- `src/model/`: the document model. `reader` (typed object access),
  `document` (`Reader::document`), and one module per area: `spread`,
  `item`, `story`, `style`, `settings`, `table`, `color`, `font`, …
- `src/idml/`: the IDML writer. `mod.rs` has `write`; one module per
  package part (`designmap`, `resources`, `styles`, `spread`, `story`),
  plus `attrs` (attribute tables), `format`, `pages`, `values`.
- `docs/measurement.md`: how `compare.py` and the audit measure the
  converter (stale pairs, value coverage, exclusions).
- `tests/fixtures.rs`: smoke tests on open-licensed samples listed in
  `tests/fixtures/manifest.json`; they skip if `tests/fixtures/files/` is
  absent.
- `tools/fetch_fixtures.py`: downloads the fixtures into the git-ignored
  `tests/fixtures/files/` and checks their SHA-256.
- `tests/corpus.rs`: tests over the local `corpus/`; they skip if absent.
- `src/hostile.rs`: hostile-input tests. Damaged synthetic documents and
  fixtures must convert or fail without a panic or a hang.
  `INDD_HOSTILE_SCALE=40 cargo test --lib hostile` runs a larger sweep.

- `tools/inventory.py`: corpus inventory (`python3 -I tools/inventory.py corpus/`).
- `tools/root_values.py [--write]`: root style values that every corpus IDML
  has; regenerates `src/idml/root_values.xml` (see `docs/format/idml-values.md`).
- Analysis scratch scripts go in the session scratchpad, not the repo.
  Anything worth keeping becomes an `indd` subcommand.

## Corpus

`corpus/` (git-ignored, ~3.4 GB): 357 third-party INDD files, 240 with a
sibling IDML, plus privately held samples in `corpus/own/`.
`corpus/inventory.tsv` lists version and pairing per file. Priority:
InDesign 18–21, little-endian. Some IDMLs show a different save than their
INDD; compare.py marks those pairs stale and reports trustworthy pairs
separately (rule and evidence: `docs/measurement.md`).

## Commands

- `cargo test` (all tests), `cargo clippy --all-targets`, `cargo fmt`.
- `python3 -I tools/fetch_fixtures.py`: fetch the fixture files before
  `cargo test`. To add a fixture, add its pinned URL, size and SHA-256 to
  `tests/fixtures/manifest.json` and its licence and source to
  `tests/fixtures/README.md`.
- `cargo run -q -- info <file>`, `indd convert in.indd out.idml`,
  `indd objects <file>`, `indd dump <file> <uid>...`, `indd uids <file>`,
  `indd xmp <file>`.
- `python3 -I tools/compare.py [--detail TAG --show N]`: convert every
  corpus pair and compare with the reference IDML. This is the main
  measure of progress; run it after every change. It reports all pairs and
  the trustworthy ones (`--stale N` lists stale pairs, `--trusted` limits
  the tables to trustworthy pairs).
- Headline numbers (value coverage, extra values, document scores, ranked
  gaps; defined in `docs/measurement.md`): `cargo build --release &&
  python3 -I tools/compare.py --exclude own/`. It prints them last and
  writes `pairs.tsv`, `gaps*.tsv`, `values*.tsv` and `extras*.tsv` to
  `target/compare/`.
- `python3 -I tools/diff_outputs.py [OLD [NEW]]`: build two revisions
  (default `HEAD` and the working tree `.`) under `~/.cache/indd-diff/`,
  convert every corpus file with both and report which outputs differ
  (package entries compared byte for byte), plus differences in failures
  and warnings. Takes about a minute. A refactoring must show 0 differing
  outputs. Full list: `~/.cache/indd-diff/diff.tsv`.
- `indd audit <file>`: what the converter does not read in one document
  (classes, chunks, attribute IDs, unknown codes, strand kinds) and its
  warnings. `python3 -I tools/audit_corpus.py --exclude own/` ranks these
  over the whole corpus by files affected (tables in `target/audit/`). Use
  it to pick what to decode next.
- `compare.py --all` also converts every other INDD/INDT under `corpus/`
  (any version, either byte order) and reports their failures; every run
  counts converter warnings by kind (`--warnings N` lists N kinds).
  `--exclude PREFIX` leaves out files whose path under `corpus/` starts
  with PREFIX; committed numbers must not include the privately held samples
  (`--exclude own/`).
- Schema validation: `tools/validate.sh a.idml [b.idml...] <schemas> <jing>`,
  or `compare.py --schemas <dir> --jing <dir>`. validate.sh validates many
  packages in one run (one JVM per schema and 400 parts, `VALIDATE_JOBS`
  in parallel); compare.py validates its outputs 600 at a time. Run
  `compare.py --all --schemas … --jing …` after changes to the IDML
  writer: it takes about 5 minutes. The IDML RelaxNG schemas (generated by
  InDesign, published in the metanorma/idml repo) and Jing (Maven Central)
  are kept outside the repo, for example in the session scratchpad; do not
  commit them.
