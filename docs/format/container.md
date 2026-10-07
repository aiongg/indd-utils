# INDD container layout

Source: Adobe XMP Toolkit SDK, `XMPFiles/source/FileHandlers/InDesign_Handler.cpp`
(BSD 3-Clause, commit `7093513bd3ca`; licence in
`third_party/xmp-toolkit-sdk/LICENSE`). Every fact below was also checked
against the local corpus (352 files, InDesign 3.0–21.6) and the fixtures.

## Overall layout

| Part | Location |
|---|---|
| Master page 0 | 0x0000–0x0FFF |
| Master page 1 | 0x1000–0x1FFF |
| Database pages | from 0x2000, up to `db_pages × 4096` |
| Contiguous objects | from `db_pages × 4096`, back to back |
| Padding | to the next 4096-byte boundary |

The file size is always a multiple of 4096.

## Master pages

Both master pages start with the header described in `header.md`. The one
with the higher sequence number is active.

| Offset | Size | Field | Notes |
|---|---|---|---|
| 0x108 | 8 | Sequence number, u64 **little-endian** | Little-endian even in big-endian files (checked on 2 big-endian files). The two master pages always differ by 1 in the corpus. |
| 0x118 | 4 | Database page count, u32 **little-endian** | Includes the two master pages |

The XMP Toolkit names the byte at 0x18 the "object stream endian" flag.
The version at 0x1D follows that byte order (`header.md`), but the fields
above do not.

## Contiguous objects

Each contiguous object is a copy of one database object, stored as a
continuous byte stream:

| Part | Size |
|---|---|
| Header marker | 32 |
| Data | `length` bytes |
| Trailer marker | 32 |

Marker layout, all integers little-endian:

| Offset | Size | Field |
|---|---|---|
| 0 | 16 | GUID: header `DE 39 39 79 51 88 4B 6C 8E 63 EE F8 AE E0 DD 38`, trailer `FD CE DB 70 F7 86 4B 4F A4 D3 C7 28 B3 41 71 06` |
| 16 | 4 | Object UID |
| 20 | 4 | Object class ID |
| 24 | 4 | Data length |
| 28 | 4 | Checksum (the XMP Toolkit writes `FFFFFFFF`) |

The list ends at the first position that does not start with the header
GUID.

### Observed in the corpus

- Every file has exactly one contiguous object: the XMP metadata packet.
- Its class ID field is `0xC0000000` (327 files) or `0xE0000000` (25 files).
- The XMP object's data is a u32 packet length (in the file's byte order),
  equal to the data length minus 4, followed by the packet starting with
  `<?xpacket begin=`.
- In 2 of 1,504 distinct files with the signature (both InDesign 9.2),
  the packet length is smaller than the data length
  minus 4 (164,680 of 171,636 and 160,539 of 554,659 bytes). The packet
  ends with `<?xpacket end="w"?>` exactly at the stated length; the bytes
  after it are the end of another, longer XMP packet (a second
  `</x:xmpmeta>` and `<?xpacket end="w"?>`, which end the data). In one of
  them the header says big-endian and the length is little-endian. The
  converter takes the stated length. All 1,504 files have an XMP object.
- In 13 files, non-zero bytes follow the last object before the end of the
  file. In the one examined, they are space characters, which matches the
  padding at the end of an XMP packet from an earlier, longer save.

## Not yet known

- How database pages are organised and how objects are found in them.
- What the object UID and class ID values identify.
