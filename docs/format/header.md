# INDD file header

Offsets are from the start of the file.

| Offset | Size | Meaning | Evidence |
|---|---|---|---|
| 0x00 | 16 | Fixed signature `06 06 ED F5 D8 1D 46 E5 BD 31 EF E7 FE 74 B7 1D` | Identical in all 352 corpus files |
| 0x10 | 8 | ASCII `DOCUMENT` | All 352 files |
| 0x18 | 1 | Byte order: `01` = little-endian, `02` = big-endian | 351 files are `01`. The one `02` file (`opf-neddy-flyer`, InDesign 3.0) stores the version below big-endian. |
| 0x1D | 4 | Major version, u32 in the byte order above | Matches the major version in the document's XMP `CreatorTool` in all 352 files |
| 0x21 | 4 | Minor version, u32 | Matches the minor version in `CreatorTool` (for example 18.5, 21.3) in all files where `CreatorTool` states one, except two where the header is one higher (header 18.2 / XMP 18.1, header 21.1 / XMP 21.0). The header probably records the last save and the XMP an earlier one. |

Major versions seen: 3, 7–9, 11–13, 15–21.

## File layout observations

These are observations, not yet explanations.

- Every file's size is a multiple of 4096 bytes (352 of 352).
- The block at 0x1000 starts with the same signature, `DOCUMENT` tag, byte
  order and version as the block at 0x0000. The two blocks are referred to
  below as A (0x0000) and B (0x1000).
- In the little-endian files, A and B differ, if at all, only at 0x37 and
  0x3A. When they differ, A has `02` at 0x37 and `80` at 0x3A, and B has
  `01` and `00`. When they match, both have `01` at 0x37 (or `00` in some
  older files). Possible explanation, untested: two alternating copies of a
  master block, where the higher value at 0x37 marks the current one.
- 0x19–0x1C read `70 0F 00 00` in every little-endian file sampled.
- 0x25 (u32): small values, 1 to 16 in the sample. Files in one series of
  templates have different values, so it may count something that changes
  between saves.
- 0x29 (u64): large values that differ between unrelated files and are
  equal in copies of the same document in different folders.

## Open questions

- Meaning of 0x19–0x1C, 0x25, 0x29 and 0x31–0x3F.
- Whether any file uses a byte-order value other than 1 or 2.
- What starts at 0x2000.
