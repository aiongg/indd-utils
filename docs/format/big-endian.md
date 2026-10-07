# Big-endian files and InDesign 3.0–4.0

Two sample files store their objects big-endian (byte order flag 2, see
`header.md`): `opf-neddy-flyer` (InDesign 3.0) and `xmp-toolkit-bluesquare`
(InDesign 4.0), both in `tests/fixtures/`. They are also the only samples
from before InDesign 7.0, so some of their object layouts differ from the
rest of the corpus for reasons of version rather than byte order. Both
kinds of difference are described here.

Neither file has an IDML. The facts below rest on the files' internal
consistency, on the flyer's print PDF (in the fixture directory), and on
comparison with the layouts in the other documents of `docs/format/`.
Implemented in `src/object.rs` (byte order) and `src/model/`.

## Byte order

| Part | Byte order in big-endian files | Evidence |
|---|---|---|
| Master page fields at 0x108 and 0x118 | Little-endian | `container.md` |
| Database pages: trailers, logical page tables, tree pages, slotted page directories and record headers | Little-endian | Both files open with the reader of `database.md`, which checks every rule while it reads. The tree entry counts equal the master page counts: 1,590 entries for 1,401 objects (3.0) and 338 for 186 objects (4.0). |
| Object data | Big-endian | See below |
| XMP object's packet length | Big-endian | `container.md` |

So the byte order flag applies to object data only.

**Object data.** All integers and f64 values in object data are
big-endian, including chunk headers, text segment headers and UTF-16 code
units:

- Chunks: 1,397 of the 1,401 objects in the 3.0 file and 185 of the 186
  in the 4.0 file are sequences of chunks with big-endian IDs and lengths
  that end exactly at the object's end. None parses with little-endian
  headers. The others are the XMP packet in both files and, in the 3.0
  file, two class 0x129 objects (raw data; one starts with a JPEG header)
  and one unclassed object.
- Text: UTF-16 code units are big-endian. The 3.0 file names the font
  family `小塚明朝 Std` and the 4.0 file `小塚明朝 Pro`, each with seven
  styles; read as UTF-16LE these names are not text.
- Numbers: page bounds, matrices and colour values read as big-endian
  give the values in the tables below (for example the US Letter page
  size of the flyer's PDF).

**In-object strings.** In little-endian data a string is often preceded
by a flag byte (1 = built-in key, `$ID/` in IDML): flag, 2, u8, u16
length, segments (`objects.md`). In big-endian data the flag and the 2
are swapped: 2, flag, u8, u16 length, segments. So the flag and the 2
form a u16 with the 2 in the high byte. Example, the name of colour 11
in the 4.0 file (chunk 0x1F10):

```
02 00 00 00 05 40 05 42 6C 61 63 6B   2, flag 0, 0, length 5, segment 0x4005 "Black"
```

Where code skips the byte before a string instead of reading it as a
flag, that byte is the 2 in big-endian data, and the string reader
starts at the flag. `Cursor::string` handles both cases.

Evidence: scanning every chunk for this pattern (2, two bytes, a non-zero
u16 length, then segments that decode exactly to that length) finds 661
strings in the 3.0 file and 462 in the 4.0 file. The byte after the 2 is
0, 1 or 2 in 610 and 420 of them. In the other 51 and 42 it is 3; all of
those are in chunks 0x191, 0xA4C and 0x11605, which the converter does
not read. The byte after that is 0 in all but 13 strings of each file.

## Layouts that differ in InDesign 3.0 and 4.0

| Object | Little-endian corpus (InDesign 7.0–21) | InDesign 3.0 and 4.0 files | Converter |
|---|---|---|---|
| Page (0x50F) transform and bounds | Chunks 0x5CC and 0x5DD | Chunks 0x151 (matrix, 48 bytes) and 0x154 (four f64, 32 bytes, same order as 0x5DD); no 0x5CC or 0x5DD | Reads 0x151 and 0x154 when 0x5CC and 0x5DD are missing |
| Page chunk 0x140F | u32, u16, matrix | u32, u16 only (6 bytes) | No matrix: `MasterPageTransform` is the identity |
| Font record (`fonts.md`) | PostScript name as u16 *n*, *n* bytes | PostScript name as u8 and an in-object string; 3.0 also has no version field | Tries the current layout, then the 4.0 and 3.0 layouts, which must end exactly at the chunk's end |
| XML tag name (chunk 0xBF2F) | u32 length, segments | An in-object string with flag 2, then four zero bytes | Reads the string when the length and segments do not parse |
| XML node part 0xBF0D (`xml.md`) | 22 bytes between the content UID and the parent key | 20 bytes (4.0), 4 bytes (3.0) | Uses the size for which the child count ends the part |
| Story strands (chunk 0x223) | Every strand has object data | 3.0: the last strand of each story has no object data | Skips strands without data |
| Ruler guide (chunk 0x3308, `objects.md`) | 52 bytes | 40 bytes, the first 40 bytes of the 52-byte layout | Reads it without the guide type; `GuideType` is left out |
| Object style text frame settings (chunk 0x1B924) | 222, 162, 142 or 106 bytes | 104 bytes (4.0); the 3.0 file has no object styles | Left out with a warning |
| Document preferences (chunk 0x533) | 146 bytes or more, except in three InDesign 7.x files | 118 bytes | Left out with a warning |
| Composite font (chunk 0xCB02) | Starts with the flagged name | 4.0: the same. 3.0: four zero bytes, then the name | 3.0: left out with a warning |
| Section page number style (`objects.md`) | 0x4C15 or 0x4C17 | 4.0: 0x4C15. 3.0: 0x4C06 | 3.0: `PageNumberStyle` left out with a warning |

Evidence for each row:

- **Pages.** Each file has three pages (one document page, two master
  pages), and all six have 0x151 and 0x154 and neither 0x5CC nor 0x5DD.
  The bounds are (0, 0, 612, 792) on all six, and the flyer's PDF page
  is 612 × 792 points. The matrices are translations: the document page
  and the right master page by (0, −396), the left master page by
  (−612, −396), so the two master pages lie side by side. Only the
  document pages have 0x140F, of 6 bytes, naming the master spread. In the little-endian corpus the converter reads
  0x5CC and 0x5DD from every page and converts every file.
- **Font records.** The 3.0 file has 18 font families with 48 fonts. All
  18 parse to the end of the chunk with the 3.0 layout and none with the
  4.0 layout. The 4.0 file has 4 families with 38 fonts, and all 4 parse
  to the end with the 4.0 layout and none with the 3.0 layout. The
  PostScript names read this way have the usual form (`Times-Roman`,
  `KozMinStd-ExtraLight`, `MyriadPro-LightCond`), and the versions in the
  4.0 file read as font version strings (`5.0d10e1`, `OTF 1.013;PS
  001.000;...`).
- **XML.** Each file has one tag (`Root`) and two XML nodes, the document
  node and the root element. With the sizes given, every field is
  consistent: the root element's tag UID is the tag object (0x6B in 3.0,
  0x8C in 4.0), its parent key is the document node's own key, and the
  document node's one child key is the root element's own key. In the
  4.0 file the 20 bytes are the same in both nodes; in the 3.0 file the
  4 bytes are zero in both. The InDesign 7.0 file stores the tag name as
  a u32 length and segments, like later files.
- **Story strands.** The 3.0 file has 33 stories, 32 with 9 strands and
  one with 8. In each, the last strand UID has no object data. The 4.0
  file has one story, whose 8 strands all have data, and so do all
  187,452 strands listed in the little-endian corpus files.
- **Guides.** 2 guides in the 3.0 file and 12 in the 4.0 file have a
  40-byte record, as do the 32 guides of the InDesign 7.0 file
  (`objects.md`). In all 46, offset 14 holds f64 0.05 and offset 22 u32 6,
  the values every 52-byte record has there; offset 8 names a page or
  the spread, and offset 12 is 0 or 1. The positions are on or
  inside the page: for example 304.7 (vertical) and 396 (horizontal) on
  the flyer's 612 × 792 page.
- **Text frame settings.** All 4 object styles of the 4.0 file have a
  104-byte chunk 0x1B924. The offsets of `objects.md` do not apply, and
  one file is not enough to find new ones.
- **Document preferences.** Both files have a 118-byte chunk 0x533. Its
  layout differs from the one in `objects.md` (page width at offset 0):
  offsets 0–15 are zero, and offsets 16 and 24 hold f64 612 and 792.
  Two files with the same page size are not enough to map the fields.
  Three files from InDesign 7.0 and 7.5 have a 126-byte chunk, also left
  out with a warning; all other little-endian files that have the chunk
  have 146 bytes or more.
- **Composite fonts.** One composite font in each file
  (`[No composite font]`). In the 4.0 file and in the InDesign 7.0 file
  the chunk starts with the flagged name; in the 3.0 file four zero
  bytes come first. One sample is not enough to rely on that.
- **Sections.** Each file has one section. The 4.0 file stores 0x4C15,
  as all 423 sections of the distinct little-endian files do; the 3.0
  file stores 0x4C06. The flyer's PDF has no page labels, so the meaning
  of 0x4C06 is not shown.

## Open questions

- Whether files from InDesign 5.0 and 6.0, which the corpus does not
  have, use the old or the new layouts above.
- The fields of the 104-byte text frame settings and the old document
  preferences.
