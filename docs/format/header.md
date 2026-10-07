# INDD file header

Offsets are from the start of the file.

| Offset | Size | Meaning | Evidence |
|---|---|---|---|
| 0x00 | 16 | Fixed signature `06 06 ED F5 D8 1D 46 E5 BD 31 EF E7 FE 74 B7 1D` | Identical in all 352 corpus files |
| 0x10 | 8 | ASCII `DOCUMENT` | All 352 files |
| 0x18 | 1 | Byte order: `01` = little-endian, `02` = big-endian. The XMP Toolkit calls it the object stream byte order. | 351 corpus files are `01`. The `02` files (`opf-neddy-flyer`, InDesign 3.0, and `xmp-toolkit-bluesquare`, 4.0) store the version below big-endian. The flag applies to object data only (`big-endian.md`). |
| 0x1D | 4 | Major version, u32 in the byte order above | Matches the major version in the document's XMP `CreatorTool` in all 352 files |
| 0x21 | 4 | Minor version, u32 | Matches the minor version in `CreatorTool` (for example 18.5, 21.3) in all files where `CreatorTool` states one, except two where the header is one higher (header 18.2 / XMP 18.1, header 21.1 / XMP 21.0). The header probably records the last save and the XMP an earlier one. |

Major versions seen: 3, 7–9, 11–13, 15–21.

## InDesign 1.0 and 1.5 files

Eight distinct files whose header (below) says 1.0 or 1.5 do not start
with the signature. The ASCII `DOCUMENT` is at 0x5C instead, and the
fields after it follow the layout above shifted by 0x4C: byte order 1 at
0x64, `70 0F 00 00` at 0x65, major version at 0x69 and minor at 0x6D
(1.0 or 1.5, matching the version in each file name).
The 76 bytes before 0x4C hold small u32 values and are not identified.
The files are not made of 4096-byte pages (sizes 41,984 to 123,904
bytes, multiples of 1,024 but not of 4,096), so the container below does
not apply. The converter reports them as not supported. Files from 2.0
on start with the signature.

## Other observations

These are observations, not yet explanations. The page layout, master page
selection and contiguous objects are described in `container.md`.

- The two master pages (0x0000 and 0x1000) carry the same signature, tag,
  byte order and version.
- In the little-endian files, the first 0x40 bytes of the two master pages
  differ, if at all, only at 0x37 and 0x3A.
- 0x19–0x1C read `70 0F 00 00` in every little-endian file sampled.
- 0x25 (u32): small values, 1 to 16 in the sample.
- 0x29 (u64): large values that differ between unrelated files.

## Open questions

- Meaning of 0x19–0x1C, 0x25, 0x29 and 0x31–0x107.
- Whether any file uses a byte-order value other than 1 or 2.
