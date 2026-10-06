# indd-utils

Reads INDD files, the native document format of Adobe® InDesign®, without
InDesign. The goal is conversion to IDML, with other outputs (text, JSON,
metadata) to follow.

Adobe and InDesign are trademarks of Adobe Inc. This project is not
affiliated with or endorsed by Adobe.

## Status

Early. The file header (signature, byte order, InDesign version) is decoded.
Nothing else is yet.

## Use

```sh
cargo install --git https://github.com/aiongg/indd-utils
indd info file.indd
```

## Development

- `cargo test` runs unit tests and smoke tests on the open-licensed samples
  in `tests/fixtures/`.
- Tests in `tests/corpus.rs` also run over a local `corpus/` directory of
  sample files if one exists. That directory is git-ignored.
- `python3 -I tools/inventory.py corpus/` lists each sample's InDesign
  version and whether it has a matching IDML file.
- Format findings go in `docs/format/`. Read `CLEANROOM.md` before
  contributing.

## Licence

MIT. Sample files in `tests/fixtures/` keep their own licences; see the
README there.
