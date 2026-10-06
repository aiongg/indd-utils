# indd-utils

Rust reader for INDD files (Adobe InDesign's native format) and converter
to IDML. Crate and command: `indd`. Repo: `aiongg/indd-utils` (private for
now). Licence: MIT.

## Clean-room rules (read `CLEANROOM.md`)

- Never commit anything from `corpus/`. It holds third-party sample files
  without redistribution rights. Only files listed in
  `tests/fixtures/README.md` may be committed.
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
- Do not name the third-party corpus sources in committed files. The
  provenance record is `corpus/SOURCES.md` (local).

## Layout

- `src/header.rs`, `src/container.rs`: decoded layers (see `docs/format/`).
- `tests/fixtures.rs`: smoke tests on committed open-licensed samples.
- `tests/corpus.rs`: tests over the local `corpus/`; they skip if absent.
- `tools/inventory.py`: corpus inventory (`python3 -I tools/inventory.py corpus/`).
- Analysis scratch scripts go in the session scratchpad, not the repo.
  Anything worth keeping becomes an `indd` subcommand.

## Corpus

`corpus/` (git-ignored, ~3.4 GB): 352 INDD files, 236 with a sibling IDML.
`corpus/inventory.tsv` lists version and pairing per file. Priority:
InDesign 18–21, little-endian. Pairs whose IDML DOMVersion is older than
the INDD version were probably re-saved after export; trust them less.

## Commands

- `cargo test` (all tests), `cargo clippy --all-targets`, `cargo fmt`.
- `cargo run -q -- info <file>`.
