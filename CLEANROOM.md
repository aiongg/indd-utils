# How this converter is developed

The INDD format is undocumented. This project learns it only by examining
INDD files and comparing them with IDML files exported from the same
documents.

## Rules

- **No Adobe software.** Contributors who analyse the format have not
  installed InDesign or accepted Adobe's terms of use.
- **Only files made by others, for other purposes.** Sample documents come
  from published templates, test suites and archives. We do not ask anyone
  to create documents for this project.
- **No Adobe binaries, and no code under incompatible licences.** We do not
  read, decompile or use InDesign binaries, the InDesign SDK, or code from
  other INDD readers whose licence is incompatible with this project's.
- **Published source under compatible licences may be used**, with
  attribution. The list is in "Permitted references" below.
- **The IDML side follows Adobe's published IDML specification.**
- **Only redistributable samples are committed.** See
  `tests/fixtures/README.md`. Other samples stay in the git-ignored
  `corpus/` directory.

## Where format knowledge is recorded

Each fact about the INDD format is written in `docs/format/` together with
the observation that supports it, before or alongside the code that relies
on it.

## Permitted references

| Reference | Licence | Used for |
|---|---|---|
| [Adobe XMP Toolkit SDK](https://github.com/adobe/XMP-Toolkit-SDK), `InDesign_Handler.cpp` | BSD 3-Clause (`third_party/xmp-toolkit-sdk/LICENSE`) | Master pages and contiguous objects (`docs/format/container.md`) |
| Adobe IDML specification | Published specification | IDML output |
