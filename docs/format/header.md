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

## Open questions

- Bytes 0x19–0x1C and 0x25 onward are not yet identified.
- Whether any file uses a byte-order value other than 1 or 2.
