# INDD objects

How the byte stream of a database object (see `database.md`) is
structured, and what the classes and chunks used by the converter mean.
Implemented in `src/object.rs` and `src/model/`.

## How this was learned

IDML exported by InDesign names most objects `Self="u<hex>"`, and that hex
number is the object's UID in the INDD file. So each corpus pair gives, for
every such IDML element, the INDD object, its class, and the attribute
values its chunks must encode. Facts below were found by comparing the two,
and are checked by `tools/compare.py`, which converts every pair (78 unique
pairs, InDesign 7.5–21) and compares the output with the reference IDML
attribute by attribute.

## Chunks

Most objects (99.9 % of 773,753 in the little-endian corpus) are a sequence
of chunks:

| Size | Field |
|---|---|
| 4 | Chunk ID |
| 4 | Data length *n* |
| *n* | Data |

The rest are plain byte streams: objects of class 0x129 (raw data, below)
and the XMP packet (UID 0x80000001). In the 245 distinct little-endian
files, all 1,171 class 0x129 objects are plain byte streams. Chunk IDs are in the same number space as
class IDs; a chunk often lists objects of the class with the same number
(for example chunk 0x501 of the document lists spreads, class 0x501).

## Primitive encodings

Object data is in the byte order of the header's flag (`header.md`).
All samples except two are little-endian, and the examples here are
little-endian. Big-endian data and the layouts of InDesign 3.0 and 4.0
are in `big-endian.md`.

- **UID lists:** u32 count, then that many u32 UIDs.
- **Matrices:** six f64, in IDML `ItemTransform` order (a b c d tx ty).
- **Text segments:** a u16 header, flags in bits 14–15 and a count in bits
  0–13. 0x4000: *count* single-byte characters follow. 0x8000: *count*
  UTF-16 code units follow. A text of *n* code units is a sequence of
  segments adding up to *n*. Example: "WOMEN’S\r" is
  `05 40 "WOMEN" 01 80 19 20 02 40 "S\r"`.
- **In-object strings:** u8 2, u8 (usually 0; a script code in some
  strings, below), u16 length in code units, then segments. 4,135 occurrences in three sample files.
  In files from InDesign 2.0 the first byte is 1: in the four distinct 2.0
  files, the pattern tag, 0, length *n*, 0, segment header 0x4000 + *n*
  occurs 154 to 353 times per file with tag 1 and never with tag 2. In
  the 83 distinct 3.0, 4.0 and 5.0 little-endian files the same pattern
  has tag 2 (76 to 4,296 times per file); tag 1 matches once in each of
  two of them.

## Document (class 0xE01, always UID 1)

| Chunk | Contents |
|---|---|
| 0x501 | UID list: spreads, in order |
| 0x1401 | UID list: master spreads |
| 0x301 | UID list: layers (the first is an internal layer that holds pages) |
| 0x313 | u32: active layer (IDML `ActiveLayer`) |
| 0x222 | Two UID lists: the stories, then the XML backing story; IDML `StoryList` is both (`xml.md`) |
| 0x4C01 | UID list: sections |
| 0xA443 | Document users: u32 count, then per user a flag byte, the name as an in-object string, u32 colour |
| 0x1D8 | Save history: u32 count, then one record per session (below) |
| 0x1630B | Label: u16 0x7B7B, u32 count *n*, then *n* pairs of key and value, each a flag byte (1 for a built-in key) and an in-object string |

**Document label.** IDML writes `Document/Properties/Label` with one
`KeyValuePair` (`Key`, `Value`) per stored pair, in stored order; the
`$ID/` rule of the flag applies. Without the chunk there is no `Label`.
Evidence: 436 of the 438 trustworthy pairs whose IDML has a document
label match (one differs, one has no chunk).

**Script byte of the user names.** The second byte of each user name's
in-object string (the byte after the string tag) is a script code: 0
in documents in Latin script, 1 in all 9 trustworthy pairs with
Japanese user or assignment names, 7 in a Russian, 25 in a Simplified
Chinese and 29 in a Czech document. The same codes are font writing
scripts (1 for Japanese fonts, 25 for Simplified Chinese fonts). All
users of a document have the same byte, except in one pair with users
of bytes 0 and 7. The converter uses the byte of the last user to
choose font family names (`fonts.md`, native family names).

Other in-object strings carry the same kind of byte, so a search for a
string must accept any second byte. In the 149 pairs typeset in
Japanese it is 1 for most user-made names, and 3, 5, 7, 0x19 and 0x1D
also occur, in style names, style GUIDs, layer names and ruby font
styles. Layer names and style GUIDs (below) rely on this.

**Document users.** IDML lists one `DocumentUser` per user in chunk
0xA443, `Self` being `dDocumentUser` and the index in hexadecimal (491 of
495 trustworthy pairs have as many users as the chunk; the other 4 are
DOM 7 and 16 files with one user more in the IDML). A user with flag 0
has its stored name as `UserName` (1,724 of 1,724). Flag 2 marks the
placeholder for an unknown user, stored with a name such as
`Unknown User Name` or the same words in the language of the computer
that saved it; IDML writes `$ID/Unknown User Name` for 495 of the 586
such users, and the stored name for the other 91. The schema requires
`UserName`, so the converter writes `$ID/Unknown User Name` for every
flag 2 user. The colour UID names an interface colour, but the IDML
`UserColor` does not follow from it (users stored with 0.6, 0.4, 0 are
`BrickRed` in 4 files and `Gold` in 1), so it is left out of the
conversion and of the measurement (`measurement.md`).

**Save history (chunk 0x1D8).** A u32 count, then one record per
session that saved the document, oldest first:

| Field | Contents |
|---|---|
| u16 | kind (not used) |
| u16 | platform (not used) |
| u16, u16 | not identified |
| u16 | language code (below) |
| flag byte, in-object string | application version, such as `16.1.0.20` |
| u16 | build (not used) |
| u32, u32 | FILETIME of the session, high part first (not used) |

Evidence: the layout parses every record of 4,431 of the 4,439
distinct valid corpus files outside the privately held samples (4,272
little-endian, 159 big-endian), from InDesign 3.0 to 21. Of the other
8, six are InDesign 2.x files whose strings have another tag
(in-object strings, above) and two (one 3.x, one 13.x) have no chunk. All 803
pairs parse.

**Language code.** The u16 of the last record is 0x0100 in 4,018 of
those files, 0x0101 in 335, 0x0103 in 53 and 0x0400 in 25. It follows
the language edition of the application of that session:

- All 149 pairs typeset in Japanese that the corpus gained in 2026-10
  have 0x0101; so do 11 earlier pairs, among them the 7 whose default
  story tag name (preferences chunk 0xBF4F) is Japanese or Chinese.
  No document with a tag name in another language (Korean, German,
  French, Russian, …) has 0x0101 (654 pairs before 2026-10).
- The values IDML writes in the exporting InDesign's language follow
  it (below): in 0x0101 documents the flattener resolution, the index
  title and separator and the endnote separator are those of a
  Japanese edition.
- 0x0103 occurs in documents in Latin and Hebrew script; it gives the
  same values as 0x0100 in all 12 pairs that have it.

The converter reads the code of the last record only, and only uses
whether it is 0x0101. The record count is checked against the chunk
size; a chunk that does not parse gives no code.

**Values that follow the language of the last session.** Over all 803
pairs (160 with code 0x0101):

| Value | 0x0101 | Other codes |
|---|---|---|
| Spread `FlattenerPreference` resolutions, no chunk 0x10833 | 400, 400 | 300, 150 |
| `IndexOptions` `BetweenEntriesSeparator`, no chunk 0x13010 (`preferences.md`) | `、` in 157 of 159 | `; ` in 632 of 632 |
| `IndexOptions` `Title`, no chunk 0x13010 | `索引` in 158 of 159 | `Index` in 624 of 632, and `Indice`, `Rejstřík`, `Índice`, `Указатель` |
| `EndnoteOption` `EndnoteSeparatorText` and `EndnoteMarkerPositioning`, no chunk 0x2261E (`footnotes.md`) | U+3000 and `RubyMarker` in 17 of 18 | tab and `SuperscriptMarker` in 123 of 123 |
| `EndnoteOption` `EndnoteTitle`, no chunk 0x2261E | `文末脚注` (15, DOM 13), `後注` (2, DOM 19 and 20), `Endnotes` (1) | `Endnotes` in 93 of 123, and 4 other titles |
| `Assignment` `Name` | `$ID/UnassignedInCopy` in 157 of 160 | `$ID/UnassignedInCopy` in 612 of 643 |

One trustworthy pair from version 16.1 has code 0x0101 but the values
of another language in every row (flattener 300 and 150 on its 9
spreads, `Index`, `; `, tab, `Endnotes`). Its IDML is dated 55.5 hours
after the last session in the INDD, while the IDMLs of 9 other 0x0101
documents are dated 0 to 40 seconds after it (plus their time zone).
So that IDML was exported in a session the INDD does not record, by an
application of another language.

The converter uses the code where it decides the value: the flattener
resolutions, the index separator, the index title `索引` for 0x0101,
and the endnote separator and marker position. It writes no index
title for other codes (the code does not tell the Latin-script
languages apart), no endnote title without the chunk, and no
assignment `Name`: names that are not `$ID/UnassignedInCopy` occur
with every code.

## Spreads (0x501) and master spreads (0x1401)

| Chunk | Contents |
|---|---|
| 0x503 | u32 self, u32 0, UID list of spread layers |
| 0x56E | Matrix: `ItemTransform` |
| 0x1B8 | u32: `BindingLocation` |
| 0x1A8 | u16: 0 = `AllowPageShuffle="true"`, 1 = `false` |
| 0x10833 | Flattener settings (below) |
| 0x14580 | Tab orders: u32 count, then per page u32 page UID and a UID list (the page's `TabOrder`) |
| 0x140D | Master spreads: u16, 0 = `ShowMasterItems="false"` |
| 0x140A | Master spreads: u32 story of the primary text frame (below) |

Evidence, over the 5,129 spreads and 914 master spreads of the 495
trustworthy pairs:

- `AllowPageShuffle`: 0x1A8 is 0 in 5,064 spreads, all `true`, and 1 in
  53, all `false`. The 12 spreads without the chunk (in 7 documents) are
  all `false`.
- `ShowMasterItems` of master spreads: 913 have no chunk 0x140D and are
  `true`; the one with the chunk (0) is `false`.
- `PrimaryTextFrame` of master spreads, over all pairs of the corpus of
  2026-10: IDML writes `n` when chunk 0x140A is absent (1,471 master
  spreads) or holds 0 (6). Otherwise it names the text
  frame on that master spread whose `ParentStory` is the chunk's story
  and whose `PreviousTextFrame` is `n` (50 of 50; in 40 of them the
  story has a second frame on the spread, which is not the primary
  one). IDML of DOM 7 has no `PrimaryTextFrame` (9 master spreads). The
  attribute is the last one of `MasterSpread`. The converter writes it
  from DOM 8 and leaves it out when the spread has no such frame: 1,521
  of 1,521 values over all pairs with the same major version.
- `TabOrder` of pages: the list for the page in its spread's chunk 0x14580
  equals the IDML list for 9,367 of 9,367 pages (8 with a tab order);
  pages not in the chunk have an empty `TabOrder`.

**Flattener settings.** Every IDML spread has a `FlattenerPreference`
child. Chunk 0x10833 (52 bytes) is in 20 spreads of 5 trustworthy
documents, always with the same bytes: f64 0.5 at offset 8, f64 400 at
offsets 20 and 28, 2 at 36 and 800 at 44. Their IDML has
`LineArtAndTextResolution` and `GradientAndMeshResolution` 400 and
`RasterVectorBalance` 50. The converter writes the f64 at 20 and 28 as
the two resolutions; which is which is not known, as they are equal in
every sample. Without the chunk, the IDML has 400 and 400 when the
last session of the save history (document, above) has code 0x0101,
and 300 and 150 otherwise: 7,481 of the 7,490 spreads of all 803 pairs
match, the other 9 being the spreads of the pair exported in a
session the INDD does not record. No stored resolution decides it: the
bytes of 400 as f64 occur in 1 of the 10 such documents of the 654
pairs before 2026-10, and no byte of the spread chunks separates the
two groups. The converter writes the resolutions by this rule, and
none when the history cannot be read. The other four values are the
same in every IDML (`idml-values.md`).

**Spread layers (0x301)** hold the items of one document layer on one
spread. Chunk 0x302: u32 document layer UID, u16 1 for the layer's guide
part. Chunk 0x303: u32 spread, u32 spread, UID list of children. Pages are
children of the spread layer for the internal pages layer.

## Guides (0x3301)

Ruler guides are objects of class 0x3301. They are children of a spread
layer (listed in its chunk 0x303), like page items, and their chunk 0x15B
names the spread and the spread layer. The spread layer's document layer
is IDML `ItemLayer`. IDML writes a guide as a `Guide` element inside a
`Page` element. Chunk 0x3308 (52 bytes):

| Offset | Contents | IDML |
|---|---|---|
| 0 | f64 position in spread coordinates: y for a horizontal guide, x for a vertical one | `Location` (below) |
| 8 | u32 owner: a page of the spread, or the spread itself | the `Page` element the guide is in |
| 12 | u16 orientation: 1 horizontal, 0 vertical | `Orientation` |
| 14 | f64, 0.05 in every sample | `ViewThreshold="5"` in every sample |
| 22 | u32, 6 in every sample | `GuideColor` `Cyan` in every sample |
| 26 | u16 fit to page: 1 true, 0 false | `FitToPage` |
| 28 | f64, not identified | |
| 36 | u32, not identified (24, 25, 30 or 31) | |
| 40 | u32 guide type: 0 ruler, 1 liquid | `GuideType` (`Ruler`, `Liquid`) |
| 44 | f64, 1 in every sample | |

**Evidence.** The 137 distinct little-endian files hold 847 guides; 815
have this 52-byte record. The other 32 are in one InDesign 7.0 file with
no IDML and have a 40-byte record, the first 40 bytes of the layout
above; the InDesign 3.0 and 4.0 files have it too (`big-endian.md`). The
converter reads it and leaves out `GuideType`. In the corpus pairs, 71 IDML guides in 33 distinct files have an
INDD object with the same UID. Two more pairs have the same number of
guides on both sides but different UIDs (the IDML was exported from
another save) and are not used. For the 71:

- `Orientation`: 71 of 71 (50 horizontal, 21 vertical).
- `FitToPage`: 71 of 71 (49 true, 22 false).
- `GuideType`: 71 of 71 (59 ruler, 12 liquid).
- `ItemLayer`: 71 of 71.
- Owner: the 49 guides owned by a page are in that page's element, the
  22 owned by the spread are in the spread's first page (71 of 71). All
  22 spread guides have `FitToPage="false"` and all 49 page guides
  `FitToPage="true"`.
- The view threshold and colour have one value each in the INDD and in
  IDML, so the fields are not proven. The converter writes
  `ViewThreshold="5"` only for 0.05 and `GuideColor` `Cyan` only for 6.

**Location.** IDML measures `Location` from the ruler origin in
`ViewPreference` (`RulerOrigin`). With `SpreadOrigin`, it is the stored
position minus the top edge (horizontal guides) or the left edge
(vertical guides) of the spread's pages, each page's edge being its
`GeometricBounds` corner mapped by its `ItemTransform`. This holds for
66 of 66 guides in files with `SpreadOrigin`, including pages whose
bounds do not start at 0 and two-page spreads. One file has
`SpineOrigin`: its vertical guides on a two-page spread are measured
from the spine (x = 0) instead (2 guides), its other 3 guides agree with
the rule above.

The ruler origin was not found in the INDD: 235 of the 240 corpus IDML
files have `SpreadOrigin`, one `SpineOrigin` and four `PageOrigin`, and
no field separates them in the objects searched (every class with at most
40 objects in a file, apart from page items and text). The converter writes
`ViewPreference` with `RulerOrigin="SpreadOrigin"` and measures guide
locations from the spread, so the two agree.

**Spread guides outside the pages.** IDML leaves out a guide owned by
the spread when its position lies outside the bounding box of the
spread's pages (each page's corners mapped by its `ItemTransform`; the
bleed does not count): for a horizontal guide when its y is above the
top or below the bottom, for a vertical one the same with x. Over the
489 trustworthy pairs before 2026-10: 84 of 85 such guides are left out
(positions such as −15, −66 or 1215 on pages 1200 wide), all 1,198
spread guides inside are written, and all 5,740 page guides are written
wherever they are. With the bleed added, 30 of the left-out guides
would be inside. The exception is a horizontal guide at −9 on a master
spread, which IDML writes; its object has chunks 0x1424 and 0x1425,
which no left-out guide has. The converter leaves out such guides
(with a tolerance of 1e-6).

**Other settings.** A guide has chunks 0x2C2D (`Locked`) and 0x1424
(`OverriddenPageItemProps`) as page items do (`Page item settings`),
and the f64 at offset 44 of the 52-byte record is `GuideZone`. Over the
7,003 guides of the trustworthy pairs: `Locked` 7,003 of 7,003 (3
locked), `GuideZone` 6,988 of the 6,988 with a 52-byte record (464 with
0), `OverriddenPageItemProps` 6,988 of 6,988; the 15 guides with a
40-byte record have neither attribute in IDML.

`PageIndex` counts from the spine of the spread: 0 for a guide of the
spread itself; for a page guide, the pages left of the spine are −1,
−2, … (counting outwards) and those right of it 1, 2, …, the spine being
after `BindingLocation` pages; on master spreads it is at the left edge.
This gives 6,998 of 7,003 (1,215 spread guides 0; 2,266 of 2,269 page
guides on document spreads, 473 of them −1 and 12 −2;
3,517 of 3,519 on master spreads).

## Layers (0x302)

Chunk 0x304:

| Offset | Contents | IDML |
|---|---|---|
| 0 | u16 | `Locked` (1 = true) |
| 2 | u16 | `Visible` |
| 4 | u16 | `Printable` |
| 6 | u16 | `LockGuides` |
| 8 | u16, 1 in every sample | |
| 10 | u32 UID of an interface colour | `LayerColor` (below) |
| 14 | u16 | `UI` |
| 16 | u16, 1 in every sample | |
| 18 | u8 flag, in-object string | `Name` |
| after the name | u16 | `IgnoreWrap` |

The internal layer is the first UID of the document's layer list
(chunk 0x301 of UID 1) and has no IDML element: in 489 of 489
trustworthy pairs that layer is not written. It is usually named
`Internal_pages_layer_name` (467), but also `Pages` (20) or has an empty
name (2), so the name does not identify it. In all 1,053 layers of the trustworthy pairs the name starts at
offset 19, and each field above equals the IDML value for 1,053 of 1,053
layers (127 locked, 55 hidden, 59 not printable, 4 with locked guides,
1 without `UI`, 2 ignoring text wrap). In the corpus of 2026-10, the
string at offset 19 gives the IDML `Name` of 1,683 of 1,683 layers in
all pairs, whatever its second byte (in the trustworthy pairs, 188
layers have a byte other than 0, all with the IDML name). The converter reads the name at offset 19 when byte
19 is the string tag, and otherwise searches for a string from offset
18. `ShowGuides` and `Expendable` are
true in every layer; the two fields that are 1 in every sample may hold
them. The converter reads the fields only when the name starts at offset
19 (in all little-endian corpus pairs, 1,465 layers).

## Pasted smooth shades (0x5533)

Objects of class 0x5533 hold shadings. Chunk 0x5531: a flag byte (1 =
built-in key) and the name, then a u32. Chunk 0x5532 holds the shading;
a constant shade has 92 bytes, with u32 28 (the length of what follows)
at offset 60, then a u32 and three f64. IDML writes it as a
`PastedSmoothShade` with `ContentsType="ConstantShade"` and `Contents`
the base64 of the same u32 and three f64 in big-endian order
(`AAAAAUBv4AAA…` for 1 and 255, 0, 0).

Every trustworthy pair has one constant shade in IDML, and it is the
class 0x5533 object of lowest UID with a constant shade (495 of 495);
in 84 pairs the INDD has two more that IDML leaves out. The converter
writes that one shade, and matches the IDML on all values in 495 of
495 (name `$ID/` from flag 1 and an empty name; two contents values,
255 and 1). The other shades in IDML are `AxialShade`s whose contents
re-encode a longer structure in the same way; they are not decoded.

## Index sort options

Chunk 0x1307E of the preferences object (class 0x2202) holds the index
groups. A "string" is a flag byte and an in-object string; a "u32
string" is a u32 length in UTF-16 units and text segments.

```
u32 group count
group:
  string     group name (kIndexGroup_Symbol, kWRIndexGroup_GreekAlphabet, ...)
  u8         include
  u8         (not identified)
  u16        header variant
  u16        (not identified)
  u32        variant count
  variant:
    string      internal name
    string      UI string
    u32 string  document string
    u8          1 = visible
    u8          (0)
    u32         section count
    section:
      u32 string  sorting string
      u32 string  document header string
      string      UI header string
      u16         language
```

Every chunk of the 654 pairs parses to its end with this layout. The
variants and sections have the same fields as the list that IDML writes
under `IndexHeaderSetting` (`preferences.md`), which the converter takes
from chunk 0x1300E. The converter uses the name, include byte and header
variant of each group.

IDML writes an `IndexingSortOption` per group, in the stored order:
`Self` `dIndexingSortOptionn` and the name, `Name` `$ID/` and the name,
`Include`, `Priority` (the position, from 0) and `HeaderType`:

| Group | Variant | `HeaderType` |
|---|---|---|
| `kIndexGroup_Alphabet` | 0, 3, 5 | `BasicLatin`, `Spanish`, `Czech` |
| `kWRIndexGroup_CyrillicAlphabet` | 2 | `Russian` |
| `kIndexGroup_Kana` | 0 | `HiraganaAll` |
| `kIndexGroup_Chinese` | 0 | `ChinesePinyin` |
| `kIndexGroup_Korean` | 0 | `KoreanConsonant` |
| Symbol, numeric, Greek, Arabic and Hebrew groups | 0 | `Nothing` |

The converter leaves `HeaderType` out for other combinations. Evidence:
the 495 trustworthy pairs have 11 different lists (8 or 10 groups, in
different orders, with different groups included); the converted groups
match the IDML in all 4,934 groups on every attribute. Over all 654
pairs the Alphabet variant is 0 in 652 files, 3 and 5 in one each, and
the Cyrillic variant is 2 in all.

## Numbering lists (0x1A483)

Chunk 0x1A49B of the preferences object: u32 UID of the default list,
u16 count *n*, *n* list UIDs, i32 −1. Each list has chunk 0x1A49A: a
flag byte (1 for a built-in key; 0 and 2 plain) and the name, then u8
`ContinueNumbersAcrossStories` and, two bytes after the name, u8
`ContinueNumbersAcrossDocuments` (1 `true`); the rest is not identified.

IDML writes one `NumberingList` per list in `designmap.xml`, the default
(`$ID/[Default]`) first, then the *n* lists in stored order, with
`Self` `NumberingList/` and the name. The paragraph attribute 0x1A417
(`AppliedNumberingList`, a `Properties` child of type `object`) is the
UID of a list. Evidence: 705 of 705 lists of the trustworthy pairs
(953 of 953 in all pairs), names, both flags and order;
`AppliedNumberingList` 519 of 519 paragraph styles and 489 of 489 text
defaults.

## Assignments (0x1BE01)

Every document has one object of class 0x1BE01, listed in chunk 0x1BE13
of the document; IDML writes it as an `Assignment` with `Self` its UID
(495 of 495 trustworthy pairs). Chunk 0x1BE1B starts with a flag byte
and the name in the language the document was made in (`Unassigned
InCopy Content`, `Contenu InCopy non affecté`, …), then two empty
strings and fields that are the same in every pair. IDML names it
`$ID/UnassignedInCopy` in 476 pairs and gives the stored name in 19, in
files of several languages and versions, so the name follows the
computer that exported the IDML; the converter leaves `Name` out (it is
optional in the schema) and the measurement too. The language code of
the save history does not decide it either (document, above). The other
attributes are the same in every IDML (`idml-values.md`).

## Named grids (0xCD12)

Chunk 0xCD28: u32, a flag byte (1 = built-in key) and the name
(`[Page Grid]`, IDML `$ID/[Page Grid]`). IDML writes a `NamedGrid` per
object, in UID order, with a `GridDataInformation`:

- A grid made by the user has its own chunk 0xCD02, with the layout of
  the page chunk (layout grid, below), and IDML writes those settings.
- The built-in `[Page Grid]` has only chunk 0xCD28. Its
  `GridDataInformation` equals the document's layout grid preference
  (`LayoutGridDataInformation`, chunk 0xCD2F, `preferences.md`) in every
  attribute that preference has: 803 of 803 pairs of the corpus of
  2026-10 (606 trustworthy). It does not always equal the pages' grid:
  4 of the 574 trustworthy pairs whose pages all have the same grid
  differ.

The converter writes each named grid with its own settings, or with
the layout grid preference. Over all pairs: 876 of 876 named grids, and
every attribute of their `GridDataInformation` (`AppliedFont` 875: one
family that IDML names in another form, `fonts.md`); 38 pairs have
grids made by the user (2 or 3 named grids).

## Table of contents styles (0x11605)

Chunk 0x11605: a flag byte (1 = built-in key) and the name, three u32
(the third the UID of the title's paragraph style), a flag byte and the
title, a flag byte and a string not identified, then u16 fields: at 2
`NumberedParagraphs` (0 `IncludeFullParagraph`, 2 `ExcludeNumbers`), at
4 `MakeAnchor` and at 6 `RemoveForcedLineBreak` (1 = true; the chunk of
older files ends before the fields IDML does not have yet), then the
entries. IDML writes one `TOCStyle` per object in `Styles.xml`.

A story made by a table of contents has chunk 0x8C40, the UID of an
object of class 0x8C20 whose chunk 0x11613 is the TOC style; IDML writes
that style as the story's `AppliedTOCStyle`, and `n` for other stories.

Evidence over the trustworthy pairs: the 511 TOC styles the converter
writes match on `Name`, `Title` and `TitleStyle` (511 of 511),
`NumberedParagraphs` (511, one `ExcludeNumbers`), `MakeAnchor` (445 of
445 from DOM 9 on, 4 true) and `RemoveForcedLineBreak` (340 of 340 from
DOM 13 on, 4 true); `AppliedTOCStyle` matches for 18,923 of 18,923
stories (21 made by a table of contents). `CreateBookmarks` (false in 1
of 676 IDML styles) and `IncludeBookDocuments` (true in 8) were not
found, and the entries (`TOCStyleEntry`, 130 in all corpus IDML files)
are not decoded: the converter writes the styles without them.

## Languages (0x2D07)

Chunk 0x2D0F: a flag byte and the name (`English: USA`), a flag byte and
the primary name (`English`), a flag byte and the secondary name (`USA`,
often empty), u16 `Id`, then two vendors, each a flag byte, a u32 and a
string: first the spelling vendor, then the hyphenation vendor
(`Hunspell`, `Proximity`, `Duden`, `WinSoft`); last the locale
(`en_US`).

IDML lists the language objects in UID order as `Language` elements at
the start of `designmap.xml`: `Name`, `PrimaryLanguageName` and
`SublanguageName` are `$ID/` and the stored names, except that the
language named `Neutral` is `$ID/[No Language]` in all three. Evidence
over the trustworthy pairs: in 488 of 495 the IDML list equals the INDD
objects in that order; in the other 7 the INDD has 4 to 61 language
objects and the IDML lists only some of them (1 or 2), for no reason
found. For the 1,249 IDML languages the names and `Id` are equal in
1,249. `HyphenationVendor` is the second vendor and `SpellingVendor` the
first when their flag is 1 (1,146 and 1,166 of 1,249); otherwise the
stored string is empty or names a vendor the IDML does not show, and
IDML gives `$ID/`, `$ID/InDihyph` or a vendor such as `Hunspell` that
depends on the computer that exported it, so the converter leaves the
attribute out. The quotes (`SingleQuotes`, `DoubleQuotes`) are not in the
chunk; they are written from observation (`idml-values.md`).

## Interface colours (0x1F11)

Layers, pages and XML tags name their colour by the UID of an object of
class 0x1F11. Chunk 0x1F01 of that object: u32 5 (RGB), u16 3, three f64
fractions. IDML writes the colour as one of its named colours, or, for
other colours, as a list of three numbers (0 to 255):

| Red, green, blue | IDML | Red, green, blue | IDML |
|---|---|---|---|
| 0.31, 0.6, 1 | `LightBlue` | 1, 0.6, 0 | `Gold` |
| 1, 0, 0 | `Red` | 1, 0.4, 0 | `Orange` |
| 0.31, 1, 0.31 | `Green` | 0, 0.33, 0 | `DarkGreen` |
| 0, 0, 1 | `Blue` | 0.6, 0.6, 1 | `Lavender` |
| 1, 1, 0.31 | `Yellow` | 0.67, 0.64, 0.71 | `Charcoal` |
| 1, 0.31, 1 | `Magenta` | 0.6, 0.2, 1 | `Violet` |
| 0, 1, 1 | `Cyan` | 1, 1, 1 | `White` |
| 0.5, 0.5, 0.5 | `Gray` | 1, 0.6, 0.8 | `Pink` |
| 0, 0, 0 | `Black` | 0.61, 0.87, 0.61 | `GridGreen` |
| 0.6, 0, 0 | `BrickRed` | 0, 0, 0.53 | `DarkBlue` |
| 0.6, 0.8, 0 | `GrassGreen` | 0.81, 0.51, 0.71 | `Lipstick` |
| 1, 0.71, 0.42 | `GridOrange` | 0.97, 0.35, 0.42 | `Fiesta` |
| 0, 0.6, 0.6 | `Teal` | 0.73, 0.73, 0.73 | `LightGray` |
| 0.48, 0.73, 0.85 | `GridBlue` | 1, 0.6, 0.6 | `Peach` |
| 0.6, 0, 0.2 | `Burgundy` | | |

Evidence: in the layers and pages of the trustworthy pairs every colour
object with these components has this name (1,107 layers and pages; from
491 `LightBlue` down to 1 `Teal`). The last four are from the guide,
grid and pasteboard colours of the preferences (`preferences.md`), where
every colour with these components has this name. Two other colours occur, 1, 0.4863,
0.651 (3 layers) and 0.9412 three times (1 layer); IDML writes them as
the lists 255, 124, 166 and 240, 240, 240, the components times 255.
The converter writes such a list for a colour whose components are whole
multiples of 1/255 and that is not in the table, and leaves the colour
out otherwise. XML tags use the table but no list, as before.

## Pages (0x50F)

| Chunk | Contents |
|---|---|
| 0x140F | u32 applied master spread, u16 (unknown), matrix `MasterPageTransform` |
| 0x5CC | Matrix: `ItemTransform` |
| 0x5DD | Four f64: left, top, right, bottom. IDML `GeometricBounds` is top, left, bottom, right. |

Files from InDesign 3.0 and 4.0 store the page transform and bounds in
chunks 0x151 and 0x154, and chunk 0x140F has no matrix
(`big-endian.md`).

**Page settings.**

| Chunk | Contents | IDML | Without the chunk |
|---|---|---|---|
| 0x1404 | u32 count; if not 0, a UID list of master page items and a UID list of their overrides | `OverrideList`: each item and its override (`n` for 0) | empty |
| 0xCD04 | 6 bytes, the last u16 | `UseMasterGrid` (1 = true) | |
| 0x563 | u32, u32 code: 1 `Recenter`, 2 `ObjectBased`, 3 `Scale`, 4 `GuideBased`, 5 `UseMaster` | `LayoutRule` | `Off` |
| 0x5FF | u32: 0 `Nothing`, 1 `UseMasterColor`, else an interface colour | `PageColor` | `UseMasterColor` |

Evidence over the 9,366 pages (document and master pages) of the
trustworthy pairs that the converter writes: `OverrideList`,
`UseMasterGrid` and `PageColor` 9,366 of 9,366 each, `LayoutRule` 9,347
of the 9,347 pages that have it. Of the 9,367 IDML pages, 1,352 have
overrides and 2,445 `UseMasterGrid="false"`; 5,111 have no chunk 0x563
and `Off`, 4,117 `UseMaster`, 92 `ObjectBased`, 24 `GuideBased`, 3
`Recenter` and 1 `Scale`; 9,260 have no chunk 0x5FF, 44 code 1, 5 code 0
and 58 an interface colour. `LayoutRule` is in IDML from DOM 8 on (no
page of the DOM 7 files has it).
| 0x51A | Margins: four f64 (left, top, right, bottom), u16 own-margins flag |
| 0x528 | Columns: u32 count *n*, *n* f64 column edges, f64 gutter, u16 own-columns flag, u16, u16 column direction (1 vertical, 0 horizontal) |
| 0xCD02 | Layout grid (`GridDataInformation`, below) |

**Master spread colour.** IDML gives each `MasterSpread` a `PageColor`
property equal to the `PageColor` of its pages, which is the same for all
pages of a master in all 1,359 master spreads of the 654 pairs
(`UseMasterColor` in 1,326, a named colour or `Nothing` in the others).
The converter writes the pages' colour. Over all pairs this reproduces
1,352 of 1,352 compared values.

**Margins and columns.** IDML writes them as `MarginPreference`. The
column edges are `ColumnsPositions` (left and right edge of each column,
so `ColumnCount` is *n*/2), the gutter is `ColumnGutter`. A flag of 1
means the page has its own values. With 0, or without the chunk, the
page shows the values in effect on its master page: the page at the same
position, counted from the left by `ItemTransform`, in the applied
master spread, or that spread's only page. That master page can itself
take them from its own master. The stored values of such a page are
often stale.

Evidence: all 325 IDML pages of the same-version pairs that have an INDD
page with the same UID (document and master pages):

- Margins: 156 pages with flag 1 match their own values; 169 take them
  from a master page (146 directly, 23 through two or three masters) and
  match. 325 of 325. Without the rule, 72 pages with flag 0 differ.
- Columns, gutter and positions: 156 with flag 1, 169 from a master page,
  325 of 325. Without the rule, 83 differ.
- The order of the margins: the four values differ in many pages (for
  example 56.69, 42.52, 56.69, 56.69 with IDML `Top="42.51968503937008"`).

In the 250 distinct little-endian files, 2,246 pages hold 0x51A in 2,160
(34 bytes each, flag 0 or 1) and 0x528 in 2,211 (always 18 bytes after
the positions). The last u16 of 0x528 is the column direction: IDML
`ColumnDirection` is `Vertical` where it is 1 and `Horizontal` where it
is 0, taking it from the master page with the columns (above). Evidence:
9,274 of 9,274 pages of the trustworthy pairs (14 vertical, in 3
documents), 12,224 of 12,224 over all pairs. A page whose flag is 0
takes the direction from its master page: 44 such pages store 1 but have
horizontal columns in IDML, like their master pages.

**Layout grid (chunk 0xCD02).** u32 font family UID, a flag byte (1 =
built-in key, `$ID/`), the font style as an in-object string, five f64,
four u32. IDML writes `GridDataInformation`:

| Field | IDML | Evidence |
|---|---|---|
| Font family | `AppliedFont` (Properties, the family name) | 283 of 325 (below) |
| Font style | `FontStyle` | 325 of 325 (`Regular`, `Roman`) |
| f64 1 to 5 | `PointSize`, `CharacterAki`, `LineAki`, `HorizontalScale` (×100), `VerticalScale` (×100) | see below |
| u32 1 to 4 | `LineAlignment`, `GridAlignment`, `CharacterAlignment`; one not identified | see below |

In the first corpus the numbers were 12, 0, 9, 1, 1 and the codes 3, 0,
3, 1 in all 2,246 pages of the 250 distinct little-endian files, and all
832 `GridDataInformation` elements of the corpus IDML files had
`PointSize="12"`, `CharacterAki="0"`, `LineAki="9"`, scales of 100,
`LineAlignment="LeftOrTopLineJustify"` and `AlignEmCenter` for both
alignments. The numbers 12 and 9 occur once, so those fields were
located. The corpus of 2026-10 confirms them: pages with other sizes
(such as 9.2126 and 6.9094) are in many pairs typeset in Japanese, and
`PointSize` and `LineAki` match in 13,458 of 13,458 pages of all pairs.
Named grids made by the user (above) locate `CharacterAki`: 37 of them
over all pairs have a second f64 other than 0 (such as −0.354), and
their IDML has the same value in 37 of 37. With the
flag, `FontStyle` matches in all 13,458 pages (44 trustworthy pages have
flag 1 and IDML `$ID/` before the style). The scales and the codes
still have one value only: the converter writes the scales only when
they are 1, and the three alignments only when the codes are 3, 0, 3,
1. The other 42 pages are in one pair whose IDML
names the family `Minion Pro (OTF)` where the INDD family name is
`Minion Pro`; its `FontFamily` element has the same difference
(`fonts.md`).

**Applied master of a master page.** Pages of master spreads use the same
chunk 0x140F. When it is absent or names UID 0, IDML writes
`AppliedMaster="n"` and `MasterPageTransform="1 0 0 1 0 0"` (220 of 220
master pages in the corpus pairs; 2 of them have the chunk with UID 0 and
an identity matrix). In 147 master pages of 73 distinct INDD files without
an IDML, chunk 0x140F names another master spread of the same document,
never the page's own spread. No pair shows the IDML for this case; the
converter writes it as `AppliedMaster`, as for document pages.

## Page items

Frames, shapes and lines are all class 0x6201; groups are 0x401.

| Chunk | Contents |
|---|---|
| 0x151 | Matrix: `ItemTransform` |
| 0x40D | Groups without chunk 0x151: the matrix (`ItemTransform`) |
| 0x162B | Path geometry (below) |
| 0x15B | Hierarchy: u32 spread, u32 parent, UID list of children |
| 0x6E03 | Attribute list: local formatting (see `attributes.md`) |
| 0x1B916 | u32: applied object style |

**Content type.** A rectangle, oval or polygon with a placed graphic is
`ContentType="GraphicType"`. Without one, chunk 0x1623 (u16) decides: 1
is `GraphicType` (an empty graphic frame), 0 or no chunk `Unassigned`.
In 200 of the trustworthy pairs, the 757 empty shapes with
`GraphicType` all have 1, and the 15,768 with `Unassigned` have 0
(13,226) or no chunk.

Groups store their matrix in chunk 0x40D when they have no chunk 0x151:
with it, the `ItemTransform` of all 5,670 groups of the trustworthy
pairs (8,788 of 8,789 over all pairs) matches; without it, 5,277 of them were
written as the identity, though they have a translation in IDML.

**Path geometry:** u32 path count; per path: u32 point count, points, u16
1 if the path is open. Each point is a u32 type, then f64 values: type 2 =
anchor (x, y) only; types 0 and 1 = left direction, anchor, right
direction. All 3,191 path chunks in the samples parse exactly, and the
points of every page item path match the IDML `PathPointType` values.

**IDML element type.** A frame is a `TextFrame` if a child of class
0x263 exists. Otherwise chunk 0x6204 (u16, then u32) gives a shape code:
1 `GraphicLine`, 2 or 3 `Rectangle`, 4 or 5 `Oval`, 0 and 6 to 8
`Polygon`; code 9 means none. With code 9 or without the chunk the
converter uses the path: two points and open → `GraphicLine`; four
corner points on two distinct x and y values → `Rectangle`; four points
with direction points, all of point type 0 → `Oval`; anything else →
`Polygon`. Path points of type 1 have direction points too, and a closed
four-point path with one or more of them is a `Polygon` in IDML (1,672
shapes in the trustworthy pairs).

Evidence over the 62,908 rectangles, ovals, polygons and lines of the
trustworthy pairs: the code gives the IDML element for 22,579 of 22,587
shapes that have one (7 polygons have code 2 or 3, 1 rectangle code 4);
the path for 40,070 of 40,321 without one. Most of the others (215) are
polygons whose four points of type 0 form an ellipse, as ovals do.

**Text frames:** the frame's child of class 0x263 (multi-column frame) has
a child of class 0x227 (frame column). Chunk 0x220 of the column holds the
frame list (class 0x228). Its chunk 0x205: u32 story, then a UID list of
the columns of all threaded frames in order, which gives
`PreviousTextFrame` and `NextTextFrame`.

## Page item settings

Every page item element in IDML (`TextFrame`, `Rectangle`, `Oval`,
`Polygon`, `GraphicLine`, `Group`) has the attributes below. Each is
stored in a chunk of the item (class 0x6201 or 0x401), or in its
attribute list (chunk 0x6E03, `attributes.md`); without the chunk the
IDML value is the default in the table.

| Chunk | Contents | IDML | Without the chunk |
|---|---|---|---|
| 0x2C10 (groups: 0x418) | u8 1 if a built-in key, then an in-object string | `Name`: the string, or `$ID/` and the key | `$ID/` |
| 0x2C32 | u16, 0 = hidden | `Visible` | `true` |
| 0x2C2D | u32, 1 = locked | `Locked` | `false` |
| 0x142D | u32, 1 = overrides not allowed | `AllowOverrides` on items of master spreads (the spread in chunk 0x15B is a master spread): `false` for 1, else `true`; no item of a document spread and no placed graphic has it (11,791 of 11,791 trustworthy items) | `true` on master spreads |
| 0x21D4E | u32 count *n*, *n* pairs of u32 | `ParentInterfaceChangeCount`: the numbers | empty |
| 0x21D50 | same | `TargetInterfaceChangeCount` | empty |
| 0x21D53 | same | `LastUpdatedInterfaceChangeCount` | empty |
| 0x1424 | u32 master page item, UID list | `OverriddenPageItemProps`: the list in decimal; empty if the master item is 0 | empty |
| 0x22228 | u8 flags | `HorizontalLayoutConstraints` (bits 4–6), `VerticalLayoutConstraints` (bits 0–2): per bit `FixedDimension` if set, else `FlexibleDimension` | as 0x22 |

| Attribute | IDML | Without the attribute |
|---|---|---|
| 0x5520, 0x551F, 0x551E | `GradientFillStart`, `GradientFillLength`, `GradientFillAngle` | `0 0`, `0`, `0` |
| 0x5526, 0x5525, 0x5524 | `GradientStrokeStart`, `GradientStrokeLength`, `GradientStrokeAngle` | same |
| 0x5522, 0x5523 | `GradientFillHiliteLength`, `GradientFillHiliteAngle` | `0` |
| 0x5528, 0x5529 | `GradientStrokeHiliteLength`, `GradientStrokeHiliteAngle` | `0` |

Rules the IDML follows, over the 92,409 page item elements of the 495
trustworthy pairs:

- `Locked` is on the 36,817 items that are children of a spread or
  anchored in text, and on none of the 55,592 items inside a group or
  another page item.
- `ItemLayer` is on the 35,276 items that are children of a `Spread`
  or `MasterSpread` (35,276 of 35,276), and on none of the items inside
  a group (48,289 of 48,291), inside a rectangle, oval or polygon (2,925
  of 2,925) or anchored in text (1,999 of 1,999). The layer of a nested
  item always equals its parent's. Counted over the 489 trustworthy pairs
  of the corpus without the privately held samples.
- The change counts, `OverriddenPageItemProps` and the layout
  constraints are in IDML from DOM 8 on: every page item of the
  distinct corpus IDML files from DOM 8 on has them, none of the DOM 7
  files does (145 text frames, 335 rectangles, 61 groups).
- A group has no gradient attributes of its own (no group in the pairs
  has 0x551E to 0x5529). IDML gives it, for each gradient attribute, the
  value all its child items have, and leaves the attribute out when they
  differ: 58,068 of the 58,080 gradient values of the 5,808 groups.

Evidence: the INDD values, decoded as above, against the IDML of the
same item, over the page items of the trustworthy pairs that have an
INDD object:

| IDML attribute | Equal |
|---|---|
| `Name` | 92,264 of 92,402 |
| `Visible` | 92,402 of 92,402 |
| `Locked` | 36,810 of 36,810 |
| The three change counts and `OverriddenPageItemProps` | 91,812 of 91,812 each |
| Both layout constraints | 91,812 of 91,812 each |
| Gradient start and length, fill and stroke (four attributes), items other than groups | 86,594 of 86,594 each |
| Gradient angles and highlights (six attributes) | all, 92,396 to 92,402 per attribute (groups by the rule above) |

The 138 names that differ belong to groups that are states of a button
(their parent has class 0x1450D); they have no name chunk, and IDML
names them `$ID/$$$/StateType/...`. The converter does not write such
groups.

Values other than the default: 206 items with a name given by the
user, 176 hidden, 1,241 with change counts, 1,113 with overridden
properties. Flags 0x22 are in 66,820 items, 0x55 (`FixedDimension
FlexibleDimension FixedDimension` both ways) in 643 and 0x02
(`FlexibleDimension` three times horizontally) in 13; with another bit
set the converter leaves both attributes out. In 14,918 items with
chunk 0x1424 the master item is 0; in 197 of them the list still holds
IDs, and IDML writes the attribute empty.

**Stroke weight without an attribute.** An item whose attribute list has
no stroke weight (attribute 0x6E65) and whose object style is `[None]`
has `StrokeWeight="1"` in IDML: in the trustworthy pairs, 31,252 of
31,252 polygons, 5,294 of 5,294 rectangles, 702 of 702 ovals, 214 of
214 graphic lines and 4,248 of 4,263 text frames (15 have 0.28). With
another object style the attribute is absent in IDML (the style supplies
it), apart from 160 text frames with `[Normal Text Frame]` and 10 lines
with styles of their own, which have 1. Groups vary (1,166 without the
attribute, 625 with 1 and 30 with other values, in 200 of the pairs)
and get none. The converter
writes 1 for items other than groups with the style `[None]`.

## Object export options

Page items (class 0x6201 and groups, 0x401) may have chunk 0x1E206, the
alternative text and tagging settings that IDML writes in
`ObjectExportOption`:

| Size | Contents | IDML `ObjectExportOption` |
|---|---|---|
| 4 | u32 alternative text source: 0 `SourceCustom`, 5 `SourceXMLStructure`, 8 `SourceDecorativeImage` | `AltTextSourceType` |
| flagged string | custom alternative text | `CustomAltText` |
| flagged string | | `Properties/AltMetadataProperty NamespacePrefix` |
| flagged string | | `Properties/AltMetadataProperty PropertyPath` |
| 4 | u32 actual text source: 0 `SourceCustom`, 5 `SourceXMLStructure`, 6 `SourceXMPAltText` | `ActualTextSourceType` |
| flagged string | custom actual text | `CustomActualText` |
| flagged string ×2 | | `Properties/ActualMetadataProperty` (as above) |
| 4 | u32 tagging: 0 `TagFromStructure`, 1 `TagArtifact` | `ApplyTagType` |
| rest | 64 bytes (DOM 8–9), 83 (10–20), 88 (21) | not decoded |

A flagged string is a u8 (1: a built-in key, written `$ID/` and the
string; 2: a plain string) and an in-object string. An empty key is
`$ID/`.

Evidence: 69,814 page items with the chunk in the trustworthy pairs; all
eight values match in 69,814 of 69,814 (other than the default: 30
custom and 199 decorative alternative text sources, 197 custom and 14
XMP actual text sources, 205 artifacts, 31 custom alternative texts, 197
custom actual texts). The 22,005 items without the chunk all have the
values `SourceXMLStructure`, `$ID/` and `TagFromStructure`. The chunk
parses in all 363,145 page items of the distinct little-endian corpus
files. The converter leaves out an attribute whose code is not in the
table. Which other attributes `ObjectExportOption` has depends on the
version (`idml-values.md`, export options of page items).

## Text wrap

Page items (class 0x6201 and groups, 0x401) and placed graphics have
chunk 0x3703 (44 bytes):

| Offset | Contents |
|---|---|
| 0 | u32 wrap mode: 0 `None`, 1 `JumpObjectTextWrap`, 3 `BoundingBoxTextWrap`, 6 `Contour` |
| 4 | u32 UID of the wrap path object (class 0x3702), or 0 |
| 8 | Four f64 offsets: left, top, right, bottom (IDML `TextWrapOffset`) |
| 40 | u32, not fully identified; 1 in all but one element of the pairs |

The wrap path object holds the contour as path geometry (chunk 0x162B,
as for page items) and names the item in chunk 0x3709.

Placed graphics also have chunk 0x373D. Its first u32 is the contour
type: 5 = `SameAsClipping`.

**Evidence from the pairs.** For every element with a `Self` in the
corpus pairs, the IDML `TextWrapPreference` was compared with chunk
0x3703 of the INDD object with that UID. Same-version pairs first, then
the pairs whose IDML is from an older version:

| INDD | IDML | Same version | Older IDML |
|---|---|---|---|
| Mode 0 | `TextWrapMode="None"` | 2,625 of 2,625 | 615 of 615 |
| No chunk 0x3703 | `TextWrapMode="None"` | 1,468 of 1,468 | 873 of 873 |
| Mode 1 | `TextWrapMode="JumpObjectTextWrap"` | 2 of 2 (one file) | 1, matched by geometry (below) |
| Mode 3 | `TextWrapMode="BoundingBoxTextWrap"` | 1 of 1 (an image) | none |
| Mode 6 | `TextWrapMode="Contour"` | 3 of 3 (one file) | 1 of 1 (an image) |
| Offsets as left, top, right, bottom | `TextWrapOffset` | 2,631 of 2,631 | 616 of 616 |
| u32 at 40 = 1 | `Inverse="false"`, `ApplyToMasterPageOnly="false"`, `TextWrapSide="BothSides"` | 2,630 of 2,630 | 616 of 616 |
| 0x373D type 5 | `ContourType="SameAsClipping"` | 224 of 225 (1 has no `ContourOption`) | 75 of 75 |

**Order of the offsets.** Most offsets in the pairs are 0, so the order
rests on a few elements:

- Offset 24 (third) is `Right`: the bounding-box image has 2.83 there
  and 0 elsewhere; its IDML has `Right="2.834645669291339"` and 0 for
  the other sides.
- Offset 32 (fourth) is `Bottom`: both jump-object frames have 14.17
  there and 0 elsewhere; their IDML has `Bottom="14.173228346456694"`
  and 0 for the other sides.
- Offset 16 (second) is `Top`. In a pair whose IDML is from an older
  version (IDML 17.0, INDD 18.1), the INDD has one jump-object frame with
  11.34 at offset 16 and 5.67 at offset 32. Its UID differs from the IDML
  element's, but its path and `ItemTransform` are identical to those of
  the IDML's only jump-object frame, which has `Top="11.338582677165356"`
  and `Bottom="5.669291338582678"`. A sample and its print PDF agree; see
  below.
- Offset 8 (first) is therefore `Left`. It is 0 in every sample that
  has a reference.
- The two contour frames have the same value at all four offsets, as in
  their IDML.

**Evidence from a sample and its print PDF.** The sample has frames
with mode 1. Offset 8 is 0 in all of them; offset 16 is 0, negative or
positive. With jump-object wrap, no text is set beside a frame, and text
above the frame must end above the top of the wrap area. Text line
positions in the PDF were taken from pdftotext word boxes and compared
with the frame bounds:

- Where offset 16 is 0, the body line above a frame ends at most 1.5 pt
  below the frame's top edge.
- Where offset 16 is negative, the body line above may end inside the
  frame, never further below its top edge than the offset's size plus
  1.5 pt. Where the next line position would end further down than
  that, it is left empty.
- A line that lies wholly between a frame's top and bottom edges falls
  inside the band that a large negative offset opens.

So the value at offset 16 moves the top edge of the wrap area, and a
negative value lets text into the frame from above.

**Other values.** One image in the same-version pairs has 0x40001 at
offset 40 and `TextWrapSide="SideAwayFromSpine"`; no other value is
shown, so the converter writes the side and inverse settings only when
the u32 at 40 is 1. Modes other than 0, 1, 3 and 6 have no IDML
evidence; the converter leaves out `TextWrapPreference` for them.

## Frame fitting

The frame fitting settings are attributes in the page item attribute list
(chunk 0x6E03, `attributes.md`). Object styles hold all of them in chunk
0x1B956 (a u16 count, then records of the same layout).

| ID | IDML `FrameFittingOption` attribute | Encoding |
|---|---|---|
| 0x6E83 | `AutoFit` | u32, 0 in every sample (`false`) |
| 0x6E7E | `LeftCrop` | f64 |
| 0x6E7F | `TopCrop` | f64 |
| 0x6E80 | `RightCrop` | f64 |
| 0x6E81 | `BottomCrop` | f64 |
| 0x6E7C | `FittingOnEmptyFrame` | u32: 0 `None`, 1 `ContentToFrame`, 2 `Proportionally`, 3 `FillProportionally` |
| 0x6E7D | `FittingAlignment` | u32: 0 `TopLeftAnchor`, 4 `CenterAnchor` |

Evidence, from the same-version pairs:

- **Object styles.** All 337 object styles paired by name hold all seven
  IDs in chunk 0x1B956, and all seven match the style's IDML
  `FrameFittingOption` in 337 of 337 (`FittingAlignment` 4 in 313
  styles, 0 in 24).
- **Crops.** Each crop ID is told apart by non-zero values: wherever IDML
  writes the attribute, the frame's own value, or its style's if the
  frame has none, matches: left 429, top 397, right 433, bottom 451, no
  mismatch. Non-zero values: left 235, top 204, right 239, bottom 258.
- **Fitting on an empty frame.** 301 of 301; codes 0, 1, 2 and 3 occur
  (1,154, 9, 77 and 16 values).
- **Alignment.** 200 of 205. The other 5 are frames in one file with the
  style `[None]`, no local value and a style value of 0, where IDML says
  `CenterAnchor`; not explained. Code 0 is shown in 9 frames, code 4 in
  object styles. Other codes are left out.
- **AutoFit.** 0 and `false` wherever IDML writes it (200 of 200); other
  values are left out.

**Where IDML writes it.** Only on rectangles, ovals and polygons, never
on text frames (1,154) or lines (136), even when their list has these
attributes. Let D be the item's local attributes whose value differs
from its object style's. IDML writes D if it is not empty; otherwise all
seven of the style's values, unless the style is the root `[None]`, in
which case it writes no element. This gives the IDML element exactly
for 1,849 of 1,864 frames (2,582 of 2,599 with the pairs whose IDML is
from an older version); in the other 15, IDML writes all seven values
although D is not empty. The converter follows the rule, and writes the
seven values of every object style.

## Clipping path settings

Images, PDF and EPS graphics may have chunk 0x2C1A (37 bytes):

| Offset | Contents | IDML `ClippingPathSettings` |
|---|---|---|
| 0 | u32, 0 in every sample | `ClippingType="None"` |
| 4 | f64 | `Tolerance` |
| 12 | f64 | `InsetFrame` |
| 20 | u8 | `Threshold` |
| 23 | i16 | `Index` |
| 25 | u8: 2 true, 0 false | `UseHighResolutionImage` |

Evidence: 75 of 235 images in the same-version pairs have the chunk, and
all 75 match on these fields (tolerance 2 or 0, threshold 25 or 0). The
byte at 25 changes together with the threshold and tolerance in the same
5 images, so it is identified only as the field that is left. PDF (153
of 206) and EPS (6 of 18) graphics have the chunk with the same values as
images that use the defaults. Graphics without the chunk (160 images, 53
PDF, 12 EPS) have `UseHighResolutionImage="true"`, `Threshold="25"`,
`Tolerance="2"`, `InsetFrame="0"` and `Index="-1"` in IDML, and the
converter writes those values for them. The converter leaves out the
element for a type other than 0.

`InvertPath="false"`, `IncludeInsideEdges="false"`,
`RestrictToFrame="false"` and `AppliedPathName="$ID/"` have no
identified field. All 496 images, 386 PDF and 49 EPS graphics with the
element in the corpus IDML files have these values, and the converter
writes them (`idml-values.md`).

**ImageIOPreference.** Chunk 0x1714 of an image:

| Offset | Size | IDML `ImageIOPreference` |
|---|---|---|
| 0 | u32 | 72 in all 1,300 images with the chunk |
| 4 | u16 | `ApplyPhotoshopClippingPath`: 1 `true`, 0 `false` |
| 6 | u16 | 1 in all (`AllowAutoEmbedding="true"` in all IDML) |
| 8 | flagged string | `AlphaChannelName` (an empty key is `$ID/`) |

Without the chunk, IDML has `ApplyPhotoshopClippingPath="true"` and
`AlphaChannelName="$ID/"`. Evidence: 3,607 of 3,607 images of the
trustworthy pairs for both attributes (14 `false`, 92 `$ID/kNoneName`,
2 plain names). Chunk 0x8C39 (u16 at 0), which looked like the clipping
flag in fewer samples, gives the wrong value in 6 images and is missing
in 549. `AllowAutoEmbedding="true"` is written from observation.

## Placed graphics

Images (0x1702), PDF (0x2501), EPS (0x6601) and SVG (0x6639) are children
of a frame (in its 0x15B list) and share these chunks:

| Chunk | Contents |
|---|---|
| 0x151 | Matrix: `ItemTransform` (437/437 match) |
| 0x1633 | Four f64: `GraphicBounds` left, top, right, bottom |
| 0x8CBC | u32, u32, u32 link UID |

**Links (0x8C42).** Chunk 0x8C9B:

| Offset | Size | Contents | IDML `Link` |
|---|---|---|---|
| 0 | 4 | 0 in all links | |
| 4 | 4 | 0x101 in all links | (`LinkClientID="257"`) |
| 8 | 4 | UID of the link resource (class 0x8C41) | |
| 12 | 4 | 1 or 0 | `LinkResourceModified`: 1 `false`, 0 `true` |
| 16 | 4 | UID of the graphic | |
| 20 | 2 | 1 or 0 | `ShowInUI`: 1 `true`, 0 `false` |
| 22 | 6 | three u16, 1 in all links | |
| 28 | 4 | 0 in all links | |
| 32 | 4 + text | import stamp: u32 length, then text segments | `LinkImportStamp` |
| then | 8 | modification time of the file (FILETIME) | `LinkImportModificationTime` |
| then | 8 | time the file was placed or updated (FILETIME) | `LinkImportTime` |

A FILETIME here is the count of 100 ns intervals since 1601-01-01 UTC,
stored as two u32: the high half first, then the low half (each in the
file's byte order). Evidence: the stamp text is `file <n> <size>`, and
`<n>` equals the first time value in 4,686 of 4,686 links with a stamp.
A link with an empty stamp (length 0) has neither `LinkImportStamp` nor
`LinkImportModificationTime` in IDML, but has `LinkImportTime` (13 of
13).

Chunk 0x1B6 of a link (u32, on 59 links) is `PDFIdentifier`, which IDML
writes from DOM 21 on; links without the chunk have `PDFIdentifier="0"`
(81 of 81 DOM 21 links).

**Link resources (0x8C41)**: chunk 0x8C92 is a flag byte, u32 length,
then the URI as bytes (`LinkResourceURI`).

After the URI, chunk 0x8C92 continues:

| Offset after the URI | Field |
|---|---|
| 0 | u8 1, u8 1, u16 0 (all resources) |
| 4 | u32 0 or 1, not identified |
| 8 | u32 0, 2 or 3 |
| 12 | u32 UID of a raw data object (class 0x129), or 0 |
| 16 | stamp: u32 length, text segments (as in the link) |
| then | modification time (FILETIME, as in the link) |
| then | file size: u32 high half, u32 low half; `LinkResourceSize` is `<high>~<low>` in lower-case hexadecimal without leading zeros (`0~42ebd`) |
| then | u8, 1 in all resources |
| then | in-object string: the format name; `LinkResourceFormat` is `$ID/` and the name, and the graphic's `ImageTypeName` is the same |

Evidence over the 4,699 links of the trustworthy pairs (4,697 with a
resource object), with the converter's output compared with the
reference:

| IDML | Equal |
|---|---|
| `LinkResourceModified` | all 4,699 (5 with 0 = `true`) |
| `ShowInUI` | all 4,699 (2 with 0 = `false`) |
| `LinkImportStamp` | 4,105 of 4,105 links with the same URI in the INDD and the IDML |
| `LinkResourceSize`, `LinkResourceFormat` | 4,118 of 4,118 links with the same URI |
| `PDFIdentifier` | 81 of 81 |

The resource's stamp differs from the link's in 184 links; IDML then has
the link's stamp and the resource's size. All 63,470 link chunks and
their resource chunks in the distinct little-endian corpus files parse
with this layout.

**Links resolved to another file at export.** In 19 trustworthy
documents, 529 links differ from the INDD: the IDML names the same file
in another folder (344) or a newer file (185). InDesign wrote the
location and state of the file it found on the exporting computer; the
INDD keeps the state of its last save. These values cannot come from the
INDD, and `compare.py` leaves such links out (`docs/measurement.md`).

**Link times.** IDML writes `LinkImportTime` and
`LinkImportModificationTime` as `YYYY-MM-DDTHH:MM:SS` (seconds
truncated) in the local time of the computer that exported the IDML:

- IDML equals the INDD's UTC time plus a whole number of hours in 9,009
  of 9,385 time values; the other 376 belong to links resolved to
  another file at export or have no stamp.
- The offset differs between documents (−8 to +13 hours) and, in 71 of
  284 documents, between links of the same document in step with the
  season (for example 1 and 2 hours, −5 and −4, +10 and +11), with the
  larger offset on summer dates of that hemisphere: the daylight-saving
  rule of the exporting computer's time zone, which the INDD does not
  store.

The converter estimates the offset from the dates of the XMP packet that
have a UTC offset (`xmp:CreateDate`, `xmp:MetadataDate`,
`xmp:ModifyDate`, every `stEvt:when`): it takes the offset of the date
nearest to the link time in day of the year (difference of the days of
the year, not wrapping at the year end; ties: the latest date), and UTC
when there is no such date. Over the trustworthy pairs this gives
`LinkImportTime` for 3,886 of 4,446 links and
`LinkImportModificationTime` for 3,316 of 4,434 (links with the same URI:
3,748 of 4,118 and 3,019 of 4,105). Other rules give less: UTC about
155 values of 9,385, the offset of `xmp:ModifyDate` 6,488, the same rule
with the day difference wrapping at the year end 3,840 import times.
`compare.py` leaves both times out of the value coverage
(`docs/measurement.md`).

**Embedded links.** A link is embedded (`StoredState="Embedded"`) when its
resource names a raw data object at offset 12. That object holds the
linked file's bytes, which IDML writes as the graphic's `Contents`.

Evidence:

- In the 245 distinct little-endian files there are 883 link resources.
  The field at offset 8 is 2 in 123 of them, and exactly those 123 name a
  raw data object at offset 12. The other 760 have 0 at offset 12 (742
  with 0 at offset 8, 18 with 3; no pair shows what 3 means).
- In the corpus pairs, all 10 resources of embedded links (12 links) name
  a raw data object, and none of the 369 resources of normal links do.
- For all 12 embedded links, the raw data object's bytes equal the
  base64-decoded `Contents` of the IDML graphic (JPEG, PNG, PDF, EPS and
  SVG files, 2 KB to 11 MB).

**Settings shared with page items.** Images, PDF, EPS and SVG graphics
carry the page item setting chunks (page item settings, above): 0x2C10
name, 0x2C32 visibility, 0x21D4E, 0x21D50 and 0x21D53 change counts,
0x1424 overridden properties, 0x22228 layout constraints, and 0x1B916
object style (`AppliedObjectStyle`). Many graphics (2,396 images) lack
0x2C10 and 0x2C32; the defaults of the page item table then apply.
Graphics never have `Locked`. Evidence over the 5,131 graphics of the
trustworthy pairs: `Visible`, `Name` and `AppliedObjectStyle` 5,131 of
5,131; the change counts, `OverriddenPageItemProps` and both layout
constraints 5,079 of 5,079 from DOM 8 on, and absent in all 52 DOM 7
graphics. Images also have the five fill gradient attributes
(`GradientFillStart`, `…Length`, `…Angle`, `…HiliteLength`,
`…HiliteAngle`) from their attribute list (chunk 0x6E03) with the
defaults of page items, and `FillColor` when the list has attribute
0x6E68 (77 of 77 images; no image without it has `FillColor`). PDF, EPS
and SVG graphics have no gradient attributes.

**`ImageTypeName`** is `$ID/` and the format name of the link resource
(links, above), on graphics with a link only: 4,696 of 4,699 linked
graphics; no graphic without a link has it (308 images, 111 PDF, 12
EPS).

**Image properties (chunk 0x1708 of an image).** u32 count, then per
record u32 key, u32 data length *n*, a u8 (meaning unknown), *n* bytes;
then 4 zero bytes:

| Key | Data | Meaning |
|---|---|---|
| 0x64 | u32 | width in pixels |
| 0x65 | u32 | height in pixels |
| 0x67 | u32 | colour space: 1 grey, 2 RGB, 4 CMYK |
| 0x6C | u32, 16.16 fixed point | horizontal resolution (ppi) |
| 0x6D | u32, 16.16 fixed point | vertical resolution |
| 0x6F | colour table | present only for indexed colour |

In all 3,607 images of the trustworthy pairs, `GraphicBounds` width and
height equal pixels × 72 / resolution. All 29,055 image property chunks
of the distinct little-endian corpus files parse. IDML writes these
values only on images with a link (3,299 of 3,299; no image without a
link has them):

| IDML | Rule | Equal |
|---|---|---|
| `Space` | 1 `$ID/#Links_Grayscale`, 2 `$ID/#Links_RGB`, 2 with key 0x6F `$ID/#Links_Indexed RGB`, 4 `$ID/#Links_CMYK` | 3,299 of 3,299 |
| `ActualPpi` | both resolutions rounded half up to integers (72.009 → 72) | 3,299 of 3,299 |
| `EffectivePpi` | each `ActualPpi` value divided by the image's scale on the spread, rounded half up | 3,299 of 3,299 |

The scale on the spread: multiply the `ItemTransform` of the image with
those of all its ancestors (frames, groups, the spread; for an item
anchored in text, its own chain only) to get *a b c d tx ty*. The
horizontal scale is √(a² + b²); the vertical scale is |ad − bc| divided
by the horizontal scale. Using √(c² + d²) for the vertical scale fails on
87 skewed images; rounding half to even fails on one image whose value
is exactly 76.5 (IDML: 77).

**Profile** (`Properties/Profile` of `Image`, chunk 0x7C0F): u32 at 0 is
3 → `$ID/Embedded`, 1 → `$ID/Use Document Default`; 0 or no chunk →
`$ID/None`. A profile name follows as an in-object string. 3,607 of
3,607 images (751, 1,652 and 1,204).

**Vector colour policies of PDF and EPS graphics (chunk 0x7C42).** 16
bytes, four u32. Offset 4 is `RGBVectorPolicy`, offset 12
`CMYKVectorPolicy`: 1 `IgnoreAll`, 3 `HonorAllProfiles`. Offset 0 is 1
and offset 8 is 3 in all 334 graphics with the chunk. Without the chunk
the policies follow the document's colour policies (chunk 0x7C44 of the
preferences, `preferences.md`): `RGBVectorPolicy` is `IgnoreAll` when
`RGBPolicy` is `ColorPolicyOff`, else `HonorAllProfiles`;
`CMYKVectorPolicy` is `IgnoreAll` for `ColorPolicyOff` and
`CombinationOfPreserveAndSafeCmyk`, `HonorAllProfiles` for
`PreserveEmbeddedProfiles` and `ConvertToWorkingSpace`. Evidence: with
the chunk 334 of 334 graphics; without it 1,005 of 1,005 graphics in 128
documents (5 combinations of document policies).

**PDF placement (chunk 0x251B of a PDF).**

| Offset | Size | IDML `PDFAttribute` |
|---|---|---|
| 0 | u32 | `PageNumber` |
| 6 | u8 | `TransparentBackground`: 1 `true`, 0 `false` |
| 8 | u32 | `PDFCrop`: 0 `CropContentVisibleLayers`, 2 `CropArt`, 3 `CropPDF`, 4 `CropTrim`, 6 `CropMedia`, 7 `CropContentAllLayers` |

Evidence: 492 of 492 PDFs with the chunk (28 bytes in 486, 64 in 6),
all three fields. Without the chunk, IDML has `PageNumber="1"`,
`TransparentBackground="true"` and `PDFCrop="CropContentVisibleLayers"`
in 507 of 509 PDFs; the other 2 (one document) have `CropArt`, not
explained. Every PDF has `PDFAttribute` (1,001 of 1,001).

**Graphic layers (chunk 0x177A of images, PDFs and imported pages).**

| Offset | Size | Contents |
|---|---|---|
| 0 | 2 | 1 if an image has a `GraphicLayerOption` |
| 2 | 8 | unknown |
| 10 | 4 | layer count *n* |
| 14 | | *n* layer records, in IDML order |

A layer record is a flagged string (`Name`; a key is written with
`$ID/`), u32 layer ID (`Id`), u32 original and u32 current visibility (1
= visible: `OriginalVisibility`, `CurrentVisibility`), i32 ID of the
parent layer (−1 for a top-level layer), and u32 flags: 0x01
`SeparatorLayer`, 0x04 `FXLayer`, 0x08 `Locked`. A layer whose parent
ID is *p* is a child element of the `GraphicLayer` with `Id` *p*.
`Self` is the graphic's `Self`, `GraphicLayerOption1`, and `i` and the
layer ID in hexadecimal for each layer from the top level down
(`u5dbcGraphicLayerOption1i92i8e` is layer 0x8E inside layer 0x92).

Evidence: all 1,064 `GraphicLayer` elements of the trustworthy pairs
(679 PDF, 291 image and 94 imported page layers, 262 of them nested):
name, `Id`, `Self`, both visibilities and nesting 1,064 of 1,064; flag
0x04 ↔ `FXLayer="true"` 11 of 11; flag 0x08 ↔ `Locked="true"` 37 of 37;
flag 0x01 ↔ `SeparatorLayer="true"` 1 of 1. Flags 0x400 (10 group
layers) and 0x800 (5) have no IDML counterpart. `AdjustmentLayer`,
`HasViewState`, `ViewState`, `HasExportState`, `ExportState`,
`HasPrintState` and `PrintState` are `false` in all 1,064.
`GraphicLayerOption` with `UpdateLinkOption="KeepOverrides"` is on every
PDF and imported page (1,039 of 1,039, also when the layer count is 0)
and on the images whose chunk has 1 at offset 0 (11 of 11; the 1,212
images with 0 there have no element). All 38,087 layer chunks of the
distinct little-endian corpus files from InDesign 4.0 on parse; 172 in
InDesign 3.0 files do not, and the converter leaves their layers out
with a warning.

**Layer comps.** Images with a `GraphicLayerOption` also have
`LayerCompOption AppliedLayerComp`: chunk 0x9209 of the image, i32 at
offset 4 (−1, −2 or a comp number). 11 of 11.

## Text on a path (class 0xB320)

A shape or text frame with text on its path names a text path object
in chunk 0xB30A (600, 392 or 336 bytes). Items without text on their
path can have the chunk too, with 0 in the first u32.

| Offset in 0xB30A | Contents | IDML `TextPath` |
|---|---|---|
| 0 | u32 text path UID (class 0xB320), 0 for none | `Self` |
| 4 | u32, the same UID | |
| 8 | u16 1, u16 4, u32 3 in all 41 | `PathAlignment="CenterPathAlignment"`, `TextAlignment="BaselineTextAlignment"`, `PathEffect="RainbowPathEffect"`, `PathSpacing="0"` (which field is which is not known) |
| 16 | u8: 1 `Flipped`, 0 `NotFlipped` | `FlipPathEffect` |
| 18 | f64 | `StartBracket` |
| 26 | f64 | `EndBracket` |

The text path object has chunk 0xB334: u32 (its own UID), u32 0, then a
UID list with one multi-column frame (class 0x263). That frame's 0x15B
child list holds a column of class 0xB318, which gives the story and
the thread as the column of a text frame does (text frames, below):
`ParentStory`, `PreviousTextFrame` and `NextTextFrame` (`n` in every
sample).

IDML writes `TextPath` as a child of its shape or frame, directly after
`Properties`, with the attributes in the order of the table (then the
two thread attributes). The converter writes the four constant values
only when the bytes at 8 are those of every sample.

Evidence: 42 `TextPath` elements with an INDD object in the pairs (36
in trustworthy pairs; parents: polygons, an oval and text frames), all
values equal in 42 of 42, both `FlipPathEffect` values occurring.

## EPS text (class 0x660B)

Text kept from a placed EPS or PDF graphic as text that cannot be
edited. IDML writes it as an `EPSText` page item. It is listed in the
0x15B child list of a spread layer or a group (483 of 634 are in
groups), and has the usual page item chunks (0x151 transform, 0x6E03
attributes, 0x1B916 object style, 0x3703 text wrap, 0x2C10 name, …) but
no path.

| Chunk | Contents | IDML |
|---|---|---|
| 0x151 | Matrix | `ItemTransform` |
| 0x154 | Four f64: left, top, right, bottom | `Properties/PathBoundingBox` `Left`, `Top`, `Right`, `Bottom` |
| 0x6612 | 60 bytes: four f64 in the same order, then f64 72 and zeros (not identified) | `Properties/EPSTextAttributeBounds` `Top`, `Left`, `Bottom`, `Right`, from the four f64 |
| 0x6611 | Text record (below) | `Properties/EPSTextData` |

**Text record (chunk 0x6611).** u32 version (0x00010002 in all 634), a
flagged in-object string (the font name), nine f64 (a matrix of font
size, 0, 0, font size, x, y, then three numbers), a flagged in-object
string (the text) and a tail of 210 bytes: u32 at 0 (16 in all), zeros
to 72, u32 at 72 (varies), u16 at 76 (0 or 1), u32 at 78 (14 in all),
zeros to 210.

IDML `EPSTextData` is this record in big-endian byte order, without the
u32 at 72 and at 78 of the tail, in base64 in lines of 76 characters:
each number big-endian, each string as in big-endian data (tag 2, flag,
the byte after the tag, u16 length, each segment header big-endian,
byte segments copied and UTF-16 code units big-endian), and the zero
areas copied. So `EPSTextData` is 8 bytes shorter than the chunk.

IDML writes `Properties` (`PathBoundingBox`, `EPSTextData`,
`EPSTextAttributeBounds`) and then `TextWrapPreference`; `EPSText` has
no `ContentType`, no path and no export options. Its other attributes
are those of other page items (page item settings, above); the values
that every `EPSText` has come from `element_values.xml`.

Evidence: 634 `EPSText` elements in 14 pairs (633 in 13 trustworthy
pairs), versions 8.1 to 20.5. The transform, both bounds, the text data
and the text wrap equal the IDML in 634 of 634. `StrokeType` (634),
`OverprintFill` (33) and `StrokeDashAndGap` (7) are not reproduced yet:
IDML writes `StrokeType="StrokeStyle/$ID/Solid"` on 627 items whose
attribute list has no stroke type.

## Placed InDesign pages (class 0x6607)

A placed page of another InDesign document is a child of a frame, like
an image, and has the chunks of placed graphics (transform 0x151,
bounds 0x1633, link 0x8CBC, text wrap). IDML writes it as
`ImportedPage`. Chunk 0x2505 (68 bytes) holds the page: its u32 at 0
plus 1 is `PageNumber`.

The converter writes `ImportedPage` with `PageNumber`, the attributes
of placed graphics (`ImageTypeName` from the link format, `$ID/InDesign
Format Name`), `Properties/GraphicBounds`, `TextWrapPreference` and
`Link`. Not decoded, and left out: `ImportedPageCrop` (41 of 42 are
`CropContent`, 1 `CropBleed`; the byte at 18 of 0x2505 is 0 in 28 and
1 in 13 `CropContent` items, so it is not that field),
`MetadataPacketPreference` (an XMP packet of an object of class 0x12609
that chunk 0x12629 names), `ClippingPathSettings`, `GraphicLayerOption`
and `PDFAttribute`.

Evidence: 42 `ImportedPage` elements in 8 pairs (38 in 7 trustworthy
pairs). `ItemTransform`, `GraphicBounds` and `PageNumber` equal the
IDML in 42 of 42.

## Graphics pasted without a link

A graphic with no link (its chunk 0x8CBC names no link) can hold its file
itself:

| Class | Chunk | Contents |
|---|---|---|
| Image (0x1702) | 0x8C23 | u32 raw data object (0 = none) |
| PDF (0x2501) | 0x2521 | u32 raw data object (0 = none) |

Evidence from the corpus pairs: the raw data object equals the IDML
`Contents` for 124 of 124 PDFs and 2 of 2 images without a link. One
more image without a link has 0x8C23 = 0; its IDML `Contents` equals the
preview data described below, which the converter does not use. In the
whole corpus, two linked and embedded PDFs also have a 0x2521 object,
different from the link resource's; with no pair to compare, the
converter uses the link resource's.

## Graphic previews

Placed graphics also have chunk 0x170D, the UID of an object of class
0x1708, whose chunk 0x119 names a raw data object. In the 245 distinct
little-endian files, these objects are TIFF (613), JPEG (411), PNG (307),
GIF (27) and one other. In the same-version pairs they equal the IDML
`Contents` for only 3 of 109 embedded or pasted graphics, so they are
most likely screen previews. The package does not include them: IDML has
no place for a preview. `indd::convert` returns the previews of graphics
without `Contents` in `Conversion::previews`, so an application can show
one when the linked file is missing.

## Embedded data in IDML

The reference IDML files write `Contents` inside the graphic's
`Properties` as base64 (with `=` padding) in lines of 76 characters joined
by a line feed, with no line feed at the end. The text is split into
CDATA sections of 262,144 characters. The converter writes the same form.

## Stories (0x201)

Chunk 0x223: u32 length, u16, u32 first strand, UID list of more strands,
then other fields. Each strand has chunk 0x261: u16 count, then (u32
length, u32 data object UID) pairs. Each data object has chunk 0x262:

| Field | Contents |
|---|---|
| u32 | Kind: 0x202 text, 0x203 character style runs, 0x204 paragraph style runs |
| u32 | Owning strand |
| u16 | Run count |
| runs | Each: u32 record size, then the record |
| u32 | Total length |

Each record starts with a u32 run length in UTF-16 code units. Text
records continue with text segments. Style records continue with the
style UID.

**Story settings.** Three chunks of the story object (trustworthy
pairs, 18,610 stories):

| Chunk | Layout | IDML | Without the chunk | Evidence |
|---|---|---|---|---|
| 0xA44C | flag byte (1 for a built-in key), in-object string | `Story` `StoryTitle` (`$ID/` and the key for flag 1) | `$ID/` | 18,610 of 18,610 (30 with a name) |
| 0x50F96 | u16: 1 | `StoryPreference` `StoryDirection="RightToLeftDirection"` | `LeftToRightDirection` | 18,610 of 18,610 (780 right to left, 10 documents) |
| 0x2EE (16 bytes) | u16 at 0; f64 at 2; u16 at 10 (not identified); u16 at 12 (1 `true`); u16 at 14 | `StoryPreference` `StoryOrientation` (u16 at 0: 0 `Horizontal`, 1 `Vertical`), `OpticalMarginSize`, `OpticalMarginAlignment`, `FrameType` (u16 at 14: 0 `TextFrameType`, 1 `FrameGridType`) | 12, `false`, `Horizontal`, `TextFrameType` | alignment 18,610 of 18,610; size 18,601 of 18,610; orientation and frame type below |

The 9 other sizes are stories in DOM 16 and 20 documents with 9.2126 in
IDML and no chunk; not explained. Another value of the direction or
alignment u16 leaves the attribute out.

Orientation and frame type, over the
28,669 stories of all corpus pairs (stale pairs included), with chunk
0x2EE as (u16 at 0, u16 at 14):

| Chunk | Stories | IDML `FrameType`, `StoryOrientation` |
|---|---:|---|
| none | 25,206 | `TextFrameType`, `Horizontal` (all) |
| (0, 0) | 3,359 | `TextFrameType`, `Horizontal` (all) |
| (0, 1) | 4 | `FrameGridType`, `Horizontal` (all) |
| (1, 0) | 47 | `TextFrameType`, `Vertical` (all) |
| (1, 1) | 53 | `FrameGridType`, `Vertical` (all) |

The vertical and frame grid stories are in documents typeset in
Japanese (DOM 12, 13 and 18). Another frame type value leaves
`FrameType` out. For another orientation value the converter uses the
orientation of the story's frames (text orientation, below).

**Frame grids.** Over all pairs of the corpus of 2026-10:

- Chunk 0xCD41 of a text frame's multi-column frame object (class
  0x263, the child that the frame's chunk 0x15B lists) is `00 00` for a
  plain frame (23,537 of 23,537) and starts with u16 1 for a frame grid
  (248 of 248; 100, 146, 192 or 238 bytes). IDML writes an empty
  `GridDataInformation` in the `TextFrame` for exactly these 248 frames,
  after `Properties`, with `GridView="GridViewEnum"`,
  `CharacterCountLocation="BottomAlign"` and `CharacterCountSize` equal
  to the `CharacterCountSize` of the document's story grid preference
  (chunk 0xCD2E, `preferences.md`): 248 of 248 in 39 documents. The rest
  of the chunk is not decoded.
- The stories of these frames are the frame grid stories of chunk 0x2EE
  (above): 57 of 57. They have chunk 0xCD02 in the story object, with
  the layout of the page chunk (layout grid, below), and IDML writes a
  `GridDataInformation` in the `Story`, after `StoryPreference` and
  before `InCopyExportOption`, with the same values: 57 of 57 for every
  attribute. 223 other stories have the chunk but are not frame grid
  stories, and IDML writes no `GridDataInformation` for them. One frame
  grid story in the trustworthy pairs has the IDML element but no
  chunk.

The converter writes both elements by these rules.

**Owned items (run kind 0x209).** Each record: u32 run length, u16
count, then that many (u32 class, u32 UID) pairs, the objects owned by
the text position at the start of the run. An item anchored in text has
the character U+FFFC at that position and is owned through an object of
class 0x262, whose chunk 0x2C8 (u32, u32, UID list) lists the anchored page
item. IDML writes the page item element in place of the U+FFFC.
A text variable instance (class 0xCA64) is owned by a U+0018 at its
position; see `text-variables.md`. Footnotes (class 0x24F) and endnotes (class
0x2805) are owned by a U+0004 (`footnotes.md`), text and paragraph
destinations through class 0x1353C by a U+FEFF (`hyperlinks.md`).

**Text owners.** The text-owner strand (run kind 0x2A4, `tables.md`)
gives each stretch of a story's text an owner. In the 495 trustworthy
pairs of the research run the owners were the story itself (19,418
stretches), a table (class 0xB608, 11,844), a deleted text (class 0xA40A,
1,889), a footnote (class 0x24F, 502) and a note (class 0xA429, 186). The
cell field is 1 for footnotes and 0 for the story, deleted texts and
notes. Each footnote, note and deleted-text stretch ends with U+000D
(2,577 of 2,577), which IDML does not write, as for a table cell.

**Ruby strand (run kind 0x420D).** A story can list a strand of class
0x420C (chunk 0x4216: u32 data object, u32 story) whose data objects
(class 0x42B9) have chunk 0x262 with run kind 0x420D. The runs cover the
whole story text. After the run length a record has u16 1 if attributes
follow (0 otherwise), a u16 count and text attribute records
(`attributes.md`). Over the little-endian corpus: 911 stories, 11,514
runs, 7,442 with attributes. The records hold only ruby attributes, a
copy of those on the character runs. IDML does not use the copy: all
185 IDML ruby ranges of the pairs have the character run's values, and
where the two differ (5 ranges, whose strand string lacks the U+3000
separators; 16 ranges with 0x423E only on the character run) IDML
follows the character run. IDs found only in the strand, such as 0x4232
in 6,176 records, are not in IDML. The converter skips this strand.

**Notes (class 0xA429).** A note is owned by a U+FEFF (186 of 186), and
IDML writes a `Note` element in its place, inside the character range;
the `Note` holds the note's text stretch as paragraph and character
ranges. Chunk 0xA412: u32 length *n* and *n* UTF-16 units of text
segments (the user name), then two FILETIME values (u32 high part, u32
low part; 100 ns since 1601-01-01 UTC), creation and modification, then
u16 (0 in all 186). Attributes, in this order:

| Attribute | Value |
|---|---|
| `Collapsed` | `false` in all; the trailing u16 may hold it, but it never varies |
| `CreationDate`, `ModificationDate` | the two times as local time, by the link time rule below |
| `UserName` | the user name; `$ID/Unknown User Name` when the first document user of that name has flag 2 |
| `AppliedDocumentUser` | `dDocumentUser<index hex>` of the first document user (document chunk 0xA443) with that name; `n` if none has it |

Check: over all pairs the converter's notes equal the IDML notes on these
five attributes in 211 of 211 (compared as multisets per pair, since
`Note` has no `Self`).

**Tracked changes.** Deleted text is an object of class 0xA40A, owned by
the character that follows the deletion (any character; 1,696 of 1,889
in the research run were U+F035). Its text is the stretch the text-owner
strand gives to its UID, without the final U+000D. IDML writes it as
`<Change ChangeType="DeletedText">` inside the character range of the
owning character, before that character and before any element that
replaces it, holding the deleted text as paragraph and character ranges
(one `Change` per object: 1,889 of 1,889 in 21 pairs).

A story strand of class 0xA465 has runs of kind 0xA466 that cover the
whole story text (positions count characters). Each record: u32 run
length, u16 entry count, then per entry a flag byte (0), a string (the
user name, in-object form), u16 kind, FILETIME (u32 high part, u32 low
part) and u16 (0). Kind 1 is a deletion at the start of the run, with the
user and time of that deleted text; kind 2 marks the run's text as
inserted, which IDML writes as `<Change ChangeType="InsertedText">`
around it (2,073 of 2,073 entries, one element each). Records of
deleted-text stretches have no entries. `Change` attributes, in this
order: `Date`, `ChangeType`, `UserName`, `AppliedDocumentUser`, the last
two by the rule for notes.

The converter writes deleted text before the owning character and wraps
inserted text in a `Change` inside each character range it covers; a
table, an anchored item (the schema allows no page item in a `Change`) or
an XML marker ends it, and it is opened again after them, so a
change that crosses character ranges or elements is written as several
`Change` elements. Over all pairs, 6,024 of the 6,027 reference `Change`
elements have an equal `Change` in the output (`Date`, `ChangeType`,
`UserName`, `AppliedDocumentUser`, compared as multisets per pair); the
output has 92 more, from such splits. Where a paragraph-level hyperlink
source starts at an inserted character, InDesign writes an empty
`Change` before the source (1,668 of the 1,681 empty changes in the
research run); the converter writes the inserted text inside the
`Change` instead.

**Dates of notes and changes.** IDML writes `CreationDate`,
`ModificationDate` and `Date` as local time of the exporting computer,
like link times: the converter applies the link time rule below (the
offset of the XMP date nearest in day of the year). With it, 211 of 211
notes and 6,024 of 6,027 changes have an equal element in the output;
the other 3 are deleted texts, not examined further. The 21 pairs with tracked changes come from at most two
computers in one time zone, so the rule is checked on one time zone
only.

**Index markers (class 0x13006).** An index marker is owned by a U+FEFF,
which is not text in IDML: IDML writes a `PageReference` in its place.
Its layout and attributes are in `index.md`.

**Anchored object settings (chunk 0x2800).** The anchor object (class
0x262) and object styles have a 62-byte chunk 0x2800:

| Offset | Contents | IDML `AnchoredObjectSetting` |
|---|---|---|
| 0 | f64 | `AnchorYoffset` |
| 32 | f64, the negative of the value at 0 in every sample | |
| 52 | u16: 0 `TopAlign`, 1 `CenterAlign`, 2 `BottomAlign` | `VerticalAlignment` |

Evidence: 42 anchored items in the pairs (including those whose IDML is
from an older version), compared with their IDML values or, where the
item has none, its object style's. `AnchorYoffset`: 42 of 42, 11 of
them non-zero in 2 files (−3.54 and −111.46). `VerticalAlignment`: 42 of
42; 4 items in 2 files have 0 and `TopAlign`, the others 2 and
`BottomAlign`. In the 337 object styles, 312 have 2 and `BottomAlign`,
25 have 1 and `CenterAlign`, and all have 0 at offset 0 and
`AnchorYoffset="0"`. IDML writes an item's `AnchoredObjectSetting` with
the values that differ from its object style (14 of the 42 items); the
converter does the same.

Two groups of u16 fields change together in every sample, so each group
is written as a whole, for the combinations observed only:

| u16 at 46, 50, 56 | `AnchorPoint`, `PinPosition` |
|---|---|
| 0, 2, 1 | `BottomRightAnchor`, `true` |
| 2, 0, 0 | `TopLeftAnchor`, `false` |

| u16 at 40, 48 | `AnchoredPosition`, `HorizontalAlignment` |
|---|---|
| 0, 2 | `InlinePosition`, `LeftAlign` |
| 2, 1 | `AboveLine`, `CenterAlign` |

Evidence: 2,858 object styles in the 654 pairs whose IDML style has the
same name (2,628 and 230 for the first table; 2,821 and 37 for the
second) and the preferences of 527 files (`preferences.md`). Which
field holds which attribute is not known.

**Positions count characters.** A text record's run length counts
UTF-16 code units, but every other position counts characters, a
surrogate pair being one: the lengths in a strand's chunk 0x261 (also
for the text strand), and the run lengths of style runs, owned items and
text owners. Evidence: the 6 stories of the little-endian corpus whose
text has a character outside the Basic Multilingual Plane (emoji, all in
one pair). In all 6 the text records add up to the number of UTF-16
units, and the other strands and all chunk 0x261 lengths to the number
of characters; with that reading, each emoji gets the character style
range IDML gives it (6 of 6). The converter applies the same reading to
hyperlink range trees, for which no sample with such a character exists.

U+FEFF characters can be XML markers, which IDML does not write
(`xml.md`).

INDD stores a forced line break as U+000A; IDML writes it as U+2028. The
last paragraph return of a story is not written to IDML.

## Styles (0x205)

Paragraph and character styles share the class. Chunk 0x230: u32 next
style (0 = itself), u32 based-on style, fields not yet identified, u16 1
for paragraph styles or 0 for character styles, u16 `Imported` (1 =
true), u8 1 if the name is a built-in key (`$ID/` in IDML), the name as
an in-object string, then, in newer files, a GUID string
(`StyleUniqueId`). The name's offset varies (23–26
bytes), so the converter locates it as a flag byte followed by a valid
in-object string.
All 486 style names and 291 `NextStyle` values in the pairs match.

**Imported and unique ID.** Over the 4,694 paragraph and character
styles of the trustworthy pairs whose IDML style has the same name, the
u16 after the kind equals `Imported` in 4,694 (180 imported). The GUID
is a 36-character in-object string after the name; where it is stored,
IDML has it as `StyleUniqueId` in 2,769 of 2,793 styles (DOM 11 on; the
other 24 have another GUID in IDML). Styles without a stored GUID
(1,108 from DOM 11 on) have a `StyleUniqueId` in IDML that the INDD does
not hold, which the converter leaves out. The byte after the GUID's
string tag is not always 0. In the trustworthy pairs of the corpus of
2026-10, accepting any second byte gives 724 more styles (479
paragraph, 245 character) the IDML `StyleUniqueId`; 4 more have another
GUID in IDML, as above.

**Kind field.** The kind and the `Imported` u16 after it can be read
as one u32 in most files, because the second u16 is 0. The InDesign 7.5
template fixture `scml-template/scml.indt` (no IDML) has 1 there in 600
of its styles: 151 have `00 00 01 00` before the flag and 449 have
`01 00 01 00`. The 151 are all listed in the tree of the root character
style group (chunk 0x28C2 = 0xCA0D, below) and the 449 in the tree of the
root paragraph style group, so the kind is the first u16 alone. Read as a
u32, the 151 character styles were taken for paragraph styles, which gave
`ParagraphStyle/…` references and a `NextStyle` that the schema does not
allow on a character style. In every other distinct little-endian corpus
file the u32 is 0 or 1 (2,125 styles), and the kind agrees with the root
group in all of them. In the two big-endian files the u16 kind is also
the first two bytes (`00 01` for their three paragraph styles, `00 00`
for their two character styles).

IDML writes a `BasedOn` of the root `[No paragraph style]` or
`[No character style]` as a string (`$ID/[No paragraph style]`), and any
other base as an object reference.

**Keyboard shortcuts.** Ten bytes before the name's flag byte is a u32
key; the two bytes 6 and 5 before the flag are modifier bytes *m0* and
*m1*. Evidence: the 6,957 paragraph and character styles of all pairs
whose IDML style has the same name.

| Key | `KeyboardShortcut` | `ExtendedKeyboardShortcut` |
|---|---|---|
| 0 (5,272 styles) | `0 0` | `0 0 0` |
| 0xC000*xx*, *xx* a digit `0`–`9`, *m1* = 0 | *m0* and the code below | `0 0 0` |
| the same, *m1* = 1, paragraph style | *m0* + 256 and 96 + the digit | `0 0 0` |
| 0x8000*xx* (4 styles, one file) | `0 0` | not written (`256 49 1` and similar in IDML) |

The 103 styles with a key other than 0 are in 38 files.

Codes with *m1* = 0: digits 0 to 7 give 82 to 89, 8 gives 91 and 9 gives
92 (every digit occurs). The converter writes the attribute only for
these rows; character styles with *m1* = 1 (11 styles: `257 83`, `257 84`,
`257 105`) are left out. Over all pairs, `KeyboardShortcut` is
reproduced for 5,273 of 5,273 paragraph styles and 2,191 of 2,202
character styles. IDML has `ExtendedKeyboardShortcut` from DOM 15 on;
the converter writes it from version 15.

**Export flags (chunk 0x28F0).** u32 count of export tag maps, the
maps, then u16 values: `SplitDocument`, `EmitCss` and, from DOM 13,
`IncludeClass` (1 `true`, 0 `false`); later versions add more u16
values. The tag maps (`StyleExportTagMap`) are not decoded, so the
flags are read only when the count is 0. IDML has `SplitDocument` and
`EmitCss` from DOM 10 and `IncludeClass` from DOM 13 on paragraph and
character styles. Without the chunk the values are `false`, `true`,
`true`. Evidence (trustworthy pairs, styles other than the root
styles): every converted value equals the IDML (paragraph styles
`SplitDocument` 3,751, `IncludeClass` 3,098, `EmitCss` 3,865; character
styles 1,962, 1,717, 1,962). The styles with tag maps keep the observed
values or none.

**Empty nested, line and GREP styles.** Paragraph styles have
`EmptyNestedStyles`, `EmptyLineStyles` and `EmptyGrepStyles` (DOM 8.1,
and 10 on). Each is `false` when the style's list of nested styles
(attribute 0x1B75), line styles (0x1BBB) or GREP styles (0x1BBA) has
items, where the list is that of the first style in the based-on chain
that has the attribute; an empty list is stored as a count of 0. This
gives the IDML value of 5,401 of 5,403 paragraph styles for each of the
three (all pairs; the other 2 are not written).

## Style groups and object styles

**Root groups** (class 0xCA8C for paragraph and character styles, 0x1B972
object styles, 0x20241 cell styles, 0x1044F table styles) are named in
IDML by UID (`RootParagraphStyleGroup Self="u7e"`). Chunk 0x28C2 says
which kind: 0xCA0C paragraph, 0xCA0D character, 0x1B924 object, 0xB669
cell, 0xB668 table (all 75 pairs). Children: chunk 0x28DC (text styles)
or 0x1B95A (object styles), each u32, u32, UID list. A built-in root style
such as `[No character style]` need not be listed.

**Groups** (class 0xCA8B): chunk 0x28D3 is parent, root, UID list of
children; chunk 0x28D2 a flag byte and the name. IDML names every group
`$ID/<name>` (20 of 21 groups), and a style inside groups is referenced as
`ParagraphStyle/<group>:<group>:<name>` with `:` escaped as `%3a`. All 412
paragraph styles and 21 groups in the pairs match.
Cell style groups work the same way: the root cell style group's
children (chunk 0x2024E) include groups of class 0xCA8B, and IDML writes
each as `<CellStyleGroup Self="CellStyleGroup/$ID/<group>"
Name="$ID/<group>">` holding its styles, in the order of the children (2
groups with 6 cell styles in 1 trustworthy DOM 18 pair). No table style
group occurs in the pairs; the converter writes them the same way.

**Style names.** Over the styles of the 489 trustworthy pairs, a style's
IDML `Name` is its group path and its own name joined by `:` (1,170
paragraph, 759 character, 8 object and 6 cell styles in groups, such as
`Name="Listes:Liste non numérotée"`), and its `Self` is the tag and that
name with `%` written `%25`, `:` written `%3a` and CR written `%0d`. Two
more rules:

- A `:` inside a style's own name is written `\:` in `Name`, so `\%3a`
  in `Self`: `Name="ss01\: People"`, `Self="CharacterStyle/ss01\%3a
  People"` (12 paragraph, 6 character and 3 object styles).
- A built-in style inside a group has `$ID/` before the whole path:
  `ParagraphStyle/$ID/INDEX%3aIndex Section Head` with
  `Name="$ID/INDEX:Index Section Head"` (one paragraph and one character
  style).

The same holds for object styles and cell styles in groups
(`ObjectStyle/Worksheet%3aworksheet`, `CellStyle/Standard
Table%3aHeader Row`). Elsewhere `%3a` stays plain: languages are
`Language/$ID/English%3a USA`.

**Object styles** (class 0x1B901): chunk 0x1B907 is u32 based-on style,
u8 1 for a built-in name, then the name. A page item's chunk 0x1B916 is
its applied object style (`AppliedObjectStyle`, 671/671 rectangles). A
style based on the root `[None]` has its `BasedOn` written as a string.

## Object style settings

Object styles (class 0x1B901) hold their settings in these chunks.
Evidence: the 337 object styles of the same-version pairs whose name
matches an IDML `ObjectStyle` (259 of them other than `[None]`); all
counts are matches of 337 unless stated.

| Chunk | Contents | IDML |
|---|---|---|
| 0x1B92B | Attribute list, u16 count, page item records (`attributes.md`) | `FillColor`, `StrokeColor`, `StrokeWeight`, `StrokeType`, `CornerOption`, `CornerRadius`, `GradientFillAngle` (0x551E), `GradientStrokeAngle` (0x5524) |
| 0x1B956 | Attribute list, frame fitting (see frame fitting) | `FrameFittingOption` |
| 0x1B924 | Text frame settings (below) | `TextFramePreference`, `TextFrameFootnoteOptionsObject` |
| 0x285B | u16 story orientation at 0, f64 12 at 2, u16 frame type at 14 | `StoryPreference` |
| 0x50F28 | u16 story direction | `StoryPreference/StoryDirection` |
| 0x3776, 0x3777 | Text wrap, as chunks 0x3703 and 0x373D of page items | `TextWrapPreference` |
| 0x1B92E | u32 count, IDs of the categories the style turns on | `Enable…` attributes (below) |
| 0x1B946 | u32 paragraph style, 0 = none | `AppliedParagraphStyle` (`n` for none) |

**Attribute list.** The IDs of page items carry over. `CornerOption` 0
is `None` and 0x5A16 `InverseRoundedCorner` (1 style). The two gradient
angles are told apart by one style with −90 and 0. Each corner radius
and corner option has its own ID, as in page items (`attributes.md`,
corners), and the converter writes each corner from its ID.

**Text frame settings (chunk 0x1B924).** 222 bytes; 162, 142 or 106 in
files from older versions (942, 48, 66 and 61 object styles in the
little-endian corpus). The converter reads it only with these sizes.
The InDesign 4.0 file has 104 bytes, with another layout; the converter
leaves it out with a warning (`big-endian.md`).

| Offset | Contents | Attribute |
|---|---|---|
| 0 | f64 | `TextColumnFixedWidth` |
| 8 | f64 | `TextColumnGutter` |
| 32 | u16, 1 = true | `VerticalBalanceColumns` |
| 34, 42, 50, 58 | four f64, inset spacing | `InsetSpacing` (below) |
| 66 | u32 | `TextColumnCount` |
| 70 | u16, 1 = true | `UseFixedColumnWidth` |
| 74 | u16, codes as for frames | `VerticalJustification` |
| 76 | u16, codes as for frames, and 4 `XHeight` | `FirstBaselineOffset` |
| 116 | u16, codes as for frames | `AutoSizingType` |
| 118 | u16, codes as for frames | `AutoSizingReferencePoint` |
| 120, 122 | u16 1 = true, f64 | `UseMinimumHeightForAutoSizing`, `MinimumHeightForAutoSizing` |
| 130, 132 | u16 1 = true, f64 | `UseMinimumWidthForAutoSizing`, `MinimumWidthForAutoSizing` |
| 144 | u16, 1 = true | `FootnotesSpanAcrossColumns`, `SpanFootnotesAcross` |
| 146 | f64 | `FootnotesMinimumSpacing`, `MinimumSpacingOption` |
| 154 | f64 | `FootnotesSpaceBetween`, `SpaceBetweenFootnotes` |
| 190 | f64 | `ColumnRuleStrokeWidth` |
| 198 | u32 swatch, 0 = `n` | `ColumnRuleStrokeColor` |
| 210 | f64 | `ColumnRuleStrokeTint` |

The fields from offset 32 to 132 were found by testing every offset
against the IDML style, over the 2,155 object styles of the trustworthy
pairs that match an IDML style by name (1,000 with 222 bytes, 607 with
142 and 520 with 162). Each is the only offset that matches all styles
of the 222- and 142-byte layouts:

| Offset | Values seen (222 / 142 bytes) |
|---|---|
| 32 | 1 true in each layout, all match |
| 70 | 142: 1 true; 222: all false |
| 74 | 222: 4 `CenterAlign`; 142: 3 `CenterAlign`, 2 `BottomAlign` |
| 76 | 222: 952 `AscentOffset`, 42 `EmboxHeight`, 6 `XHeight`; 142: 584, 1 and 22; 162: 516 `AscentOffset`, 4 `EmboxHeight` |
| 116 | 3 distinct values in each layout |
| 118 | 142: 5 distinct values |
| 120, 122 | 142: 6 true, 6 distinct heights |
| 130, 132 | 142: 2 true, 3 distinct widths |

In the 162-byte layout the justification and auto-sizing fields have one
value in every style, and in the 106-byte layout (28 styles) every field
but offset 76 does; the converter assumes the offsets of the larger
layouts for 162 bytes and reads only offset 76 from 106 bytes.
`IgnoreWrap`, `MinimumFirstBaselineOffset`, `VerticalThreshold`,
`UseFlexibleColumnWidth`, `TextColumnMaxWidth` and
`UseNoLineBreaksForAutoSizing` have one value in every style (false or
0), so no field can be shown for them; the converter writes the observed
value (`idml-values.md`). Code 4 of `FirstBaselineOffset` is shown only
in styles; the converter reads it in frames too.

Footnote and column rule values: 305 of 305 styles whose chunk has them.
The column rule colour maps one to one over 41 (file, UID) pairs. Only
one style (3 copies) has unequal insets, and it shows only the top inset
(offset 50), so the converter writes `InsetSpacing` only when all four
are equal (334 of 337).

**Story settings.** Frame type: 0 `Unknown`, 1 `TextFrameType`, 2
`FrameGridType`. Orientation: 0 `Unknown`, 1 `Horizontal`. Direction: 1
`LeftToRightDirection`, 0 or no chunk `UnknownDirection`. 337 of 337
each. The f64 at 2 is 12 in every style, like `OpticalMarginSize`, and is
not used.

**Text wrap.** 234 styles have chunk 0x3776 with mode 0, offsets 0 and
the u32 at 40 = 1, and chunk 0x3777 type 5: IDML `None` with
`ContourType="SameAsClipping"`. 103 styles have neither chunk and IDML
`None`.

**Categories (chunk 0x1B92E).** An ID in the list means the attribute is
`true`. Over the 1,684 object styles other than `[None]` of the
trustworthy pairs whose name matches an IDML style (fewer for attributes
that IDML has only from a later version), each attribute below equals
the presence of its ID in every style:

| ID | Attribute | From DOM |
|---|---|---|
| 0x1B933 | `EnableFill` | 7 |
| 0x1B934 | `EnableStroke` | 7 |
| 0x1B935, 0x1B936 | `EnableStrokeAndCornerOptions` | 7 |
| 0x1B93E | `EnableTextFrameGeneralOptions` | 7 |
| 0xADC8 | `EnableTextFrameBaselineOptions` | 7 |
| 0xADC9 | `EnableTextFrameAutoSizingOptions` | 8 |
| 0x1B940 | `EnableStoryOptions` | 7 |
| 0x1B960 | `EnableFrameFittingOptions` | 7 |
| 0x1B93F | `EnableParagraphStyle` | 7 |
| 0xADCB | `EnableTextFrameColumnRuleOptions` | 15.1 |
| 0xCA2F | `EnableAnchoredObjectOptions` | 7 |
| 0x1B942, 0x37C8, 0x37C9 | `EnableTextWrapAndOthers` | 7 |
| 0xADCA | `EnableTextFrameFootnoteOptions` | 12 |
| 0x6EA1 to 0x6EA7 | `EnableTransformAttributes` | 13 |
| 0x1B97A, 0x1B97C to 0x1B97E | `EnableExportTagging`, `EnableObjectExportAltTextOptions`, `EnableObjectExportEpubOptions`, `EnableObjectExportTaggedPdfOptions` | 9 |
| 0x1B937 | `EnableTransparency` of `ObjectStyleObjectEffectsCategorySettings` | 7 |
| 0x1B948, 0x1B950, 0x1B958 | `EnableTransparency` of the fill, stroke and content effects categories | 7 |

Where a row has several IDs, they are all present or all absent in every
style, so the corpus does not tell which one the attribute follows; the
converter writes the attribute only when they agree.

Versions are those of the INDD header, major and minor. Over the 489
trustworthy pairs: `EnableTextFrameColumnRuleOptions` is on no style in
the 42 documents of 15.0 (four builds), and on every style in the 7 of
15.1 and on all later ones. The footnote values of the text frame
settings (`FootnotesSpanAcrossColumns`, `FootnotesMinimumSpacing`,
`FootnotesSpaceBetween` of `TextFramePreference`) are on no style of
DOM 12 (127 styles) or 13.0 (12 styles), and on every style from 13.1.

**The root style `[None]`** has none of the `Enable…` attributes and none
of the four `ObjectStyle…EffectsCategorySettings` elements in 489 of 489
trustworthy references, though its INDD object has the category list.
The converter still reads the list: the text frames that use `[None]`
depend on it (text frame preferences). The four export
attributes are equal in every style (84 true, 1,358 false). With the 88
pairs of the earlier corpus, fill and stroke, and the general and
baseline frame options, were not told apart; the larger corpus has
styles that differ in them (10 without fill, 13 without stroke, 506
without general options and 531 without baseline options).

**Anchored object settings** are in chunk 0x2800, as for anchors (see
stories), including the two groups of fields that are written
together.
The other values of object styles come from IDML observation
(`idml-values.md`).

## Master spread names and sections

Master spread chunk 0x1402: a flag byte and the name prefix (`A`), a flag
byte and the base name (`Master`). IDML `Name` is `<prefix>-<base>`, and
the master's pages are named by the prefix (99/99 match).

**Sections (class 0x4C01).** The document's chunk 0x4C01 lists them.
Section chunk 0x4C02:

| Field | Contents |
|---|---|
| u8, string | `SectionPrefix` |
| u8, string | `Marker` |
| u32 | First page of the section; 0 for the section that starts at the document's first page |
| u32 | Page number start (`PageNumberStart`) |
| u32 | Page number style: 0x4C15 = `Arabic`, 0x4C17 = `LowerRoman`, 0x4C12 = `Kanji` |
| u32 | Continue numbering (1 = true, `ContinueNumbering`) |
| u32 | Not identified |
| u8 flag, string | `AlternateLayout`: the string, with `$ID/` before it if the flag is 1 |

In the 546 sections of the trustworthy pairs, 539 chunks have all these
fields; the prefix, marker and alternate layout equal the IDML values in
539 of 539 (417 named layouts such as `Letter V`, 82 empty, 40 `$ID/`).
The 7 shorter chunks are in DOM 7 and 8 files and end after the second
u32 above.

**Alternate layouts.** A section with a layout name (not empty and not
`$ID/`) starts an alternate layout, and so does the first section.
`AlternateLayoutLength` of a section is the number of pages from its
first page to the start of the next layout, or to the end of the
document (538 of 539). The `AppliedAlternateLayout` of a document page
is the section that starts its layout; master pages have `n`. Both are
in IDML from DOM 8 on. Over the trustworthy pairs the converted
`AppliedAlternateLayout` equals the IDML value for 9,347 of 9,347 pages.

**Page descriptor.** A document page has a `Descriptor` property, a list
of the section prefix, the page number style, `ContinueNumbering`,
`false`, the page number and the marker; from DOM 20 on the page number
is in the list twice. Over the trustworthy pairs: 7,800 of 7,924 pages;
64 others are in sections with a style the converter does not know (IDML
gives it as an empty string), and the converter leaves their descriptor
out. The fourth item is `IncludeSectionPrefix`, false in every sample.

Evidence:

- In every corpus pair the first section's IDML `PageStart` is the
  document's first page, and its first-page field is 0. All but one pair
  have one section; one has four. All 79 sections that the converter
  writes match on `PageStart`, `Length`, `ContinueNumbering` and, where
  IDML writes it, `PageNumberStart`. IDML writes `PageNumberStart` only for sections with
  `ContinueNumbering="false"` (14 of 14); a continuing section can store a
  start number that IDML leaves out.
- 84 distinct INDD files without an IDML have more than one section (254
  sections). The first section listed has first page 0 in all 84. Each of
  the other 170 names a page of the document (170 of 170). The list is not
  always in page order, so the converter sorts sections by their first
  page. A section ends where the next one starts; that gives IDML
  `Length`.
- The style is 0x4C15 in all 423 sections of the 250 distinct
  little-endian files, and those in the pairs are `Arabic`. The
  InDesign 3.0 file stores 0x4C06 (`big-endian.md`).
- Style 0x4C17 is lower-case Roman. No pair has it. The evidence is a
  sample and its print PDF. The PDF's page labels (`/PageLabels` in the
  document catalog) give the pages of the sample's 0x4C17 sections style
  `/r`, lower-case Roman, and the pages of its 0x4C15 section style
  `/D`, decimal.
- Style 0x4C12 has no pair either. The evidence is a sample typeset
  vertically and its print PDF. The folios on the pages of its 0x4C12
  section are Chinese numerals written digit by digit (〇 一 二 … 九,
  so 10 is 一〇 and 100 is 一〇〇), counting from the section's start
  number. The PDF's page labels belong to a larger document and do not
  show the style. `Kanji` is the only value of the schema's
  `PageNumberStyle` enumeration that uses such numerals.

The converter writes `PageNumberStyle` for these three codes and leaves
it out (with a warning) for other codes.

The converter names document pages by their number in their section,
counting on from the previous section when numbering continues. In a
section with style 0x4C17 it writes the number in lower-case Roman (i,
ii, iii), as in the PDF labels above, and in a section with style 0x4C12
in Chinese digits, as the folios show. No pair shows IDML page names in
such sections.

## Document preferences (class 0x2202)

One object of class 0x2202 holds document-wide preferences. Chunk 0x533
(`DocumentPreference`): f64 page width at 0, f64 page height at 8, byte 58
2 for facing pages and 1 otherwise, four f64 bleeds from offset 70 (inside,
top, outside, bottom; `preferences.md`), u32 intent at 142 (0 print, 1
web, 2 mobile). All values match the 75 pairs. Files from InDesign 3.0 to 7.5
have a shorter chunk with another layout; the converter leaves it out
with a warning (`big-endian.md`).

**Page binding (u16 at 64).** 0 is `PageBinding="LeftToRight"`, 1
`RightToLeft`. Evidence:

- The u16 is 0 in all 247 distinct little-endian public files that have
  the 146-byte chunk. All 88 distinct corpus IDML files have
  `PageBinding="LeftToRight"`, and in all 73 two-page spreads and master
  spreads of those files the first page lies left of the second.
- A sample typeset vertically, whose print PDF shows columns ordered
  right to left, stores 1. In each of its two-page spreads and master
  spreads the first page lies right of the second: the book is bound on
  the right. The schema's other value, `Default`, has no sample.

The converter writes `PageBinding` from this field and leaves it out,
with a warning, for other codes. Pages are written in the order the
spread lists them, for both bindings.

## Text frame preferences

From the frame's multi-column frame object (class 0x263):

| Chunk | Offset | `TextFramePreference` attribute |
|---|---|---|
| 0x2D1 | u32 0 | `TextColumnCount` (1,052/1,052) |
| 0x2D1 | f64 4 | `TextColumnGutter` (93/93) |
| 0x2D1 | f64 14 | `TextColumnFixedWidth` (1,045/1,045) |
| 0x2CE | u16 0 | `FirstBaselineOffset`: 0 LeadingOffset, 1 AscentOffset, 2 CapHeight, 3 EmboxHeight, 4 XHeight (below) |
| 0x2CE | u16 2 | `VerticalJustification`: 0 Top, 1 Center, 2 Bottom, 3 Justify (350/350) |
| 0x2CE | u16 20 | `VerticalBalanceColumns` (18/18) |
| 0x2CE | u16 22 | `AutoSizingType`: 0 Off, 1 HeightOnly, 2 WidthOnly, 3 HeightAndWidth (61/61) |
| 0x2CE | u16 24 | `AutoSizingReferencePoint`: 0–8, top-left to bottom-right by rows (58/58) |
| 0x2CE | u16 26, f64 28 | `UseMinimumHeightForAutoSizing`, `MinimumHeightForAutoSizing` |
| 0x2CE | u16 36, f64 38 | `UseMinimumWidthForAutoSizing`, `MinimumWidthForAutoSizing` |
| 0x2CE | u16 46 | `UseNoLineBreaksForAutoSizing` |
| 0x2D1 | u8 12 | `UseFixedColumnWidth` |
| 0x2D1 | f64 32 | `TextColumnMaxWidth` (in 40-byte chunks) |
| 0x3730 | u16 | `IgnoreWrap` |
| 0x22646 | f64 28, u32 36 | `ColumnRuleStrokeWidth`, `ColumnRuleStrokeColor` (a swatch; 0 = `n`) |
| 0x22608 | f64 4, f64 12 | `FootnotesMinimumSpacing`, `FootnotesSpaceBetween` |

The frame itself (class 0x6201) holds the inset spacing in chunk 0x3723
(44 bytes): f64, u32, then four f64 (left, top, right, bottom). IDML
writes `InsetSpacing` as a list of top, left, bottom and right.

Evidence over the text frames of the trustworthy pairs that the
converter writes (`TextFramePreference` values reproduced): the
auto-sizing minimums and their flags 4,069 to 4,071 of 4,069 to 4,071
each (5 frames with 108 pt minimums, 4 without line breaks),
`UseFixedColumnWidth` 747 of 747 (9 true), `TextColumnMaxWidth` 23,043
of 23,043 (one frame with 210.24; the 30-byte chunks of DOM 7 files have
no such field and their IDML no such attribute), `IgnoreWrap` 643 of the
817 frames that have it (74 true; 174 frames without the chunk have
`false` in IDML and are left without it), column rule width and colour
7,277 and 7,323 of 7,277 and 7,323, footnote spacings 4,351 of 4,351
each. Without chunk 0x22646 the IDML has width 1 and `Color/Black`
(2,816 frames); without chunk 0x22608, 12 and 6 (1,430 frames). The
column rule and footnote settings are written from the versions in
which IDML has them (`idml-values.md`).

`InsetSpacing`: 16,560 of 18,091 frames. IDML gives 870 frames a single
number instead of a list (the first f64 of chunk 0x3723 in most of
them); what decides this was not found, and the converter writes the
list for every frame from DOM 11 on. DOM 7 to 10 files give most frames
no `InsetSpacing` at all, and the converter writes none for them.

**Values written on frames.** IDML writes an attribute of a frame's
`TextFramePreference` when its category is off in the frame's object
style (the category ID is not in the style's list, chunk 0x1B92E, also
for `[None]`), or when the frame's value differs from the style's
value. The style's values are its own, or, if it has no text frame
settings, those of the style it is based on, then `[None]`'s.

| Category | ID | Attributes |
|---|---|---|
| General | 0x1B93E | `TextColumnCount`, `TextColumnGutter`, `TextColumnFixedWidth`, `UseFixedColumnWidth`, `UseFlexibleColumnWidth`, `TextColumnMaxWidth`, `VerticalJustification`, `VerticalThreshold`, `IgnoreWrap`, `VerticalBalanceColumns`, `InsetSpacing` |
| Baseline | 0xADC8 | `FirstBaselineOffset`, `MinimumFirstBaselineOffset` |
| Auto-sizing | 0xADC9 | the seven auto-sizing attributes |
| Footnotes | 0xADCA | the four `Footnotes…` attributes |
| Column rules | 0xADCB | the ten `ColumnRule…` attributes |

Evidence over the 489 trustworthy pairs. Frames with style `[None]`
write a whole category in a document if and only if `[None]`'s list
lacks the ID (counting documents with such frames):

| Category | ID missing | ID present |
|---|---|---|
| Auto-sizing (DOM 8 on) | written in 28 of 28 documents | written in 0 of 224 (one more has frames whose values differ) |
| Footnotes (13.1 on) | 21 of 21 | 0 of 153 |
| Column rules (DOM 15 on) | 35 of 35 | 0 of 64 |
| General, baseline | (ID present in all 254) | 0 of 254 |

Exceptions: `TextColumnMaxWidth` is on every frame from DOM 8 (22,775 of
22,775), `TextColumnCount` on every frame from DOM 10 (18,679 of 18,680;
before, it follows the rule: absent in 4,006 frames where it equals the
style), and `InsetSpacing` on every frame from DOM 11. The footnote
values are on no frame before 13.1: none of the 3 documents of 13.0 has
them, though one has frames whose style has the category off.

Applied to the converter's frame values and the reference styles'
values, the rule leaves out 379,953 values that the IDML does not have,
keeps the 200,434 it has, and leaves out one value that the IDML has (a
`TextColumnFixedWidth` equal to its style's).

`ColumnRuleOverride`: chunk 0x2265A is all zero in the frames of the
pairs except one, whose IDML has `true`; the converter writes `false`
for zero chunks and frames without it (7,247 of 7,248).

**Text orientation (chunk 0x2DE).** The multi-column frame holds a
matrix (six f64) in chunk 0x2DE. Its first four values are 1 0 0 1 for
horizontal text and 0 1 −1 0 (a quarter turn) for vertical text. IDML
has no frame attribute for this; it writes the orientation on the story
(`StoryPreference`, `StoryOrientation`). Evidence:

- All 18,207 multi-column frames in the 239 distinct little-endian
  public files that have text frames have 1 0 0 1. All 2,094 stories in
  the corpus IDML files have `StoryOrientation="Horizontal"`.
- A sample typeset vertically has 0 1 −1 0 in the frames of some
  stories and 1 0 0 1 in the others. In its print PDF, the text of every
  frame with the quarter turn is set in vertical columns ordered right to
  left (glyphs of a column share an x position and follow each other
  downwards), and the text of every frame with 1 0 0 1 is set in
  horizontal lines, such as the running heads. The frames of one story
  always agree.
- In the rotated frames, `TextColumnFixedWidth` (chunk 0x2D1) equals
  the frame's height, not its width: the columns run down the page.

The last two values of the matrix are not identified. The converter
writes `Vertical` for a story whose frames all have the quarter turn,
and otherwise `Horizontal`, the value every IDML has. A story whose
frames disagree or have another matrix gets a warning. Two frames in
the InDesign 3.0 file have no chunk 0x2DE; they do not count.

**First baseline offset.** IDML writes `FirstBaselineOffset` on a frame's
`TextFramePreference` only when it differs from the frame's object style.
Over the frames of the corpus pairs, compared with the value written on
the frame or, if absent, on its object style:

| Code | IDML value | Frames |
|---|---|---|
| 1 | `AscentOffset` | 1,121 of 1,121 (13 on the frame, 1,108 from the object style) |
| 2 | `CapHeight` | 10 of 10 (copies of one document) |

No pair has code 0. The evidence for it is a sample and its print PDF. In
the PDF, the first baseline in each frame with code 0 lies below the
frame's top edge by exactly the leading of that first line, for first
lines with several different leadings. Of the values in the IDML schema
(`AscentOffset`, `CapHeight`, `LeadingOffset`, `EmboxHeight`, `XHeight`,
`FixedHeight`), only `LeadingOffset` depends on the leading.

Code 3 is `EmboxHeight`. No public file has it: the 18,207 multi-column
frames of the 239 distinct little-endian public files have code 1 or 2
only. The evidence is a sample and its print PDF, and a sample typeset
vertically with the same print PDF:

- The frames measured have code 3, `TopAlign` and one line. No chunk
  of these frames or of their multi-column frame objects holds the
  measured distance as a number, so it is not a stored minimum offset.
- A first line set in a CJK font lies below the frame's top edge by
  0.880 of its point size. The font's ideographic em box (its `BASE`
  table) has its bottom at −0.120 em, so the em box top is at 0.880 em.
  The font's cap height (0.742 em), its x-height (0.503 em), the leading
  and a fixed offset of 0 give other distances. Its ascent is also
  0.880 em.
- A first line set in a Latin font lies below the frame's top edge by
  0.825 of its point size. The font's ascent is 0.710 em, its cap height
  0.650 em and its x-height 0.400 em; the leading and a fixed offset of
  0 do not match either. So code 3 is not `AscentOffset`, which is code
  1 in any case.
- Of the values in the IDML schema, only `EmboxHeight` remains. The
  Latin font has no em box data; the measured 0.825 em equals an em box
  centred on half the cap height (0.325 + 0.5).

The converter leaves out codes other than 0 to 3.

## Kinsoku and mojikumi tables

Kinsoku tables are objects of classes 0x4209 (hard), 0x420A (soft),
0x42B4 (Korean), 0x42B5 (Simplified Chinese), 0x42B6 (Traditional
Chinese) and 0x4204 (custom); mojikumi tables are classes 0x4206 and
0x4203 (custom, below). IDML writes one `KinsokuTable` or
`MojikumiTable` per object in `designmap.xml`, kinsoku tables first,
each in UID order (mojikumi tables of both classes together). Chunk 0x100B is
a flag byte (1 = built-in key, `$ID/`) and the name; `Self` is
`KinsokuTable/<Name>` or `MojikumiTable/<Name>`. Implemented in
`src/model/cjk.rs`.

Evidence: 8 pairs have such objects, and their IDML lists exactly them,
by name and in that order (6 with one table of each kind, 2 with 6
kinsoku and 16 mojikumi tables). The other 70 pairs have neither objects
nor elements.

**Custom kinsoku table (chunk 0x4214).** Five u16 counts, then that many
UTF-16 units for each of five lists: `CantBeginLineChars`,
`CantEndLineChars`, a list not identified, `HangingPunctuationChars` and
`CantBeSeparatedChars`. The evidence is one table in two copies of one
document: the four non-empty lists have different lengths (66, 22, 4
and 3), and each matches the IDML attribute of that length. The layout
parses in all 85 objects of class 0x4204 in the 250 distinct
little-endian files, and the third count is 0 in all of them.

Built-in tables have only `Self` and `Name` in IDML. Mojikumi chunk
0x421E (a u16 from 1 to 16) is not used.

**Custom mojikumi tables (class 0x4203).** Chunk 0x100B holds a flag
byte (0 in all of them) and the name, as for the other tables. Chunk
0x420A holds the settings:

| Field | Contents |
|---|---|
| u32 | 26 in all 835 objects of the 129 little-endian files that have the class |
| u32 | 0 in all |
| u32 *n* | entry count |
| *n* × 34 bytes | entries, below |
| u32, u32 | not identified |
| u16 | `BasedOnMojikumiSet`: 0 `Nothing`; *n* > 0 the built-in table `kMojikumiDefaultName<n>` (attributes.md): 1 `LineEndAllOneHalfEmEnum`, 16 `SimpChineseDefault` |

Each entry is one IDML `OverrideMojikumiAkiType`:

| Offset | Type | IDML |
|---|---|---|
| 0 | u16 | `TargetMojikumiClass` |
| 2 | u16 | `SideMojikumiClass` |
| 4, 12, 20 | f64 | `Minimum`, `Desired`, `Maximum` |
| 28 | u16 | `CompressionPriority` |
| 30 | u16 | `AkiDoesNotFloat` (0 `false`, 1 `true`) |
| 32 | u16 | `SideIsAfterTarget` (0 `false`, 1 `true`) |

IDML writes the table as `<MojikumiTable Self="MojikumiTable/<name>"
Name="<name>" BasedOnMojikumiSet="…">` with `Properties`,
`OverrideMojikumiAkiList` and one `OverrideMojikumiAkiType` per entry, in
file order, with the attributes in the order `TargetMojikumiClass`,
`SideMojikumiClass`, `SideIsAfterTarget`, `Minimum`, `Desired`,
`Maximum`, `CompressionPriority`, `AkiDoesNotFloat`.

Evidence, in the corpus of 2026-10: 185 tables in 66 pairs. The entry
count matches in 185 of 185 tables, all eight fields of the entries in
file order in 62,439 of 62,439 entries, and `BasedOnMojikumiSet` in 185
of 185 (code 1: 151 tables, 0: 33, 16: 1). `Self` is `MojikumiTable/`
and the name in all 187 custom tables of the corpus IDML files. The
layout parses in all 835 objects of the corpus; codes 3, 6, 9 and 11
also occur (232 objects, none with an IDML), and the converter leaves
`BasedOnMojikumiSet` out for them. A chunk whose length is not 22 + 34
*n*, or a flag other than 0 or 1, leaves the table out with a warning.
In the 66 pairs, the tables of classes 0x4206 and 0x4203 are listed
together in UID order, after all kinsoku tables.

## Colours (0x1F05)

| Chunk | Contents |
|---|---|
| 0x1F10 | u8 1 if the name is a built-in key; name; u32 flags: bit 0 removable, bit 1 visible, bit 2 editable |
| 0x1F01 | u32 space (5 RGB, 6 CMYK, 7 LAB, 14 HSB, below), u16 count, f64 components: fractions for RGB, CMYK and HSB; for LAB the L, a and b values as IDML writes them |
| 0x1F09 | u32 model: 0 Process, 1 Spot, 2 Registration (below) |
| 0x1F24 | f64 tint value (−1 for a colour, see tints below), then u32 `ColorOverride`: 0 Normal, 1 Specialpaper, 2 Specialblack, 3 Specialregistration, 4 Hiddenreserved |
| 0x1F0A | Alternate colour, as 0x1F01: space 3 none, 6 CMYK, 7 LAB |

After the flags, chunk 0x1F10 has two u32 and the `SwatchCreatorID`
(7937 for colours made in the document, other numbers for colours from a
swatch library, such as 31527). `AlternateSpace` is `NoAlternateColor`
with `AlternateColorValue=""` without chunk 0x1F0A or with space 3;
otherwise the space and the values, CMYK as percentages. Evidence over
the 12,197 colours of the trustworthy pairs whose IDML colour has the
same name: `SwatchCreatorID` 12,197 of 12,197; alternate space and
values 12,197 of 12,197 (7,527 without the chunk, 4,624 with space 3,
33 LAB and 13 CMYK alternates, all library colours).

Space 7 is `Space="LAB"`, with the three stored values as `ColorValue`
(for example `79.2156862745098 38 53`). The corpus IDML files have 92
LAB colours in 40 distinct files: 61 spot colours from swatch libraries
(41 with a LAB alternate) and 31 process colours made in the document.
Over all 803 pairs, `Space` is reproduced
for 18,935 of 18,936 colours and `ColorValue` for 18,926.

Space 14 holds hue, saturation and brightness as fractions. IDML writes
such a colour as `RGB` with `ConvertToHsb="true"`; the 9 such colours in
the pairs (all hue 0, saturation 1, brightness 1) are `255 0 0`. The
converter converts with the usual hexagon formula. Every other colour
has `ConvertToHsb="false"` (4,695 of 4,695 colours of the IDML files
from DOM 16 on, where the attribute exists).

**Colour groups.** From DOM 12 on, every swatch (colour, tint, gradient,
`None`) has `SwatchColorGroupReference`: the `ColorGroupSwatch` of the
colour group that lists it, or `n`. Over the trustworthy pairs from DOM
12 on: colours 9,135 of 9,135, gradients 450 of 450, `None` 353 of 353,
tints 24 of 24. DOM 11 files have the attribute on some swatches only
(639 of 1,584 colours), and the converter writes it from DOM 12 on.

Unnamed colours are referenced by UID (`Color/u93`). All 1,296 colours in
the pairs match on `Model`, `Space`, `ColorValue`, `ColorOverride`,
`Name` and the three flags.

**Which unnamed colours IDML writes.** A colour without a name is in the
IDML only if a value written in the package refers to it (an attribute
or element text `Color/u…`; a tint counts), if a stop of any gradient
refers to it, even of an unnamed gradient that IDML leaves out (below),
or if an entry of class 0x1F05 in one of the two tables of the
page item defaults (`preferences.md`) names it with either UID. Over the
489 trustworthy pairs, of the unnamed colours the converter wrote:

| | In the IDML | Not in the IDML |
|---|---:|---:|
| Referenced by the output | 2,309 | 0 |
| Not referenced, named in a page item defaults table | 564 | 0 |
| Neither | 10 | 8,684 |

All colours, written or not, are in the swatch list of the preferences
object (class 0x2202, chunk 0x1F02), and the colour objects of the two
groups have the same chunks and flags, so neither decides it. Named
colours are always written. Unnamed gradients follow the same rule with
class 0x5503: 457 unnamed gradients are referenced nowhere and named in
no table, and none is in the IDML; 526 of the 548 unnamed gradients
IDML writes meet the rule. Their stops go with them.

The other 22 are named by chunk 0x6E06 of the page item defaults object
(class 0x6E07). The chunk has 34 bytes in all 489 trustworthy documents
before 2026-10: u32 5, u32 0, u32 0, u32 14, then three entries of u16 0
and a u32 UID. Entry 0 names an object of class 0x5533, entry 1 a
gradient and entry 2 a colour (489 of 489 each). IDML writes the
gradient of entry 1 in every document: 480 are unnamed, and all 480 are
in the IDML; none of the 162 unnamed gradients IDML leaves out (in the
documents checked) is in the chunk. The colour of entry 2 is in the
IDML wherever the converter writes it (82 of 82). The converter treats
the swatches of chunk 0x6E06 as named by the page item defaults. With
it, every unnamed gradient of the trustworthy pairs of 2026-10 that IDML
writes is written (783 of 783 `Gradient` elements).

**Model codes.** The 250 distinct little-endian files hold 13,067
colours with chunk 0x1F09: 12,814 with code 0, 250 with code 2 (one
`Registration` colour per file) and 3 with code 1. Two of the three are
copies of one colour in two pairs, and their IDML has `Model="Spot"`;
the third is in a file without an IDML. No file has another code, so
the converter leaves `Model` out, with a warning, for any other code.

**Tints** are objects of the same class without chunks 0x1F10 and
0x1F01. Chunk 0x117 is the UID of the base colour, and chunk 0x1F24 holds
the tint value in percent (IDML `TintValue`) where a colour has −1,
followed by the colour override as for colours. IDML names a tint after
its base colour and value: `Name="Gold 80%"` and `Self="Tint/Gold 80%25"`
for 80 % of a colour named `Gold`. The black swatch (override
Specialblack) is written in brackets: `[Black] 40%`.

Evidence: all 2,107 colours with a name in the distinct little-endian
files have −1 at the start of chunk 0x1F24. The same files have 26 tints
in 24 files, all with this layout and a named base colour. The pairs have
3 IDML `Tint` elements (2 files, one of them on black), and all 3 match on
`TintValue`, `BaseColor`, `Name` and `ColorOverride`. Their 19 references
(`FillColor` of 15 page items, `StrokeColor` of 3, one gradient stop)
match too.

Tints have no flags chunk, so `ColorEditable`, `ColorRemovable` and
`Visible` (true in all 3 IDML tints) are not written. Brackets are known
only for black; other reserved colours as a tint base do not occur in the
pairs.

The `None` swatch is class 0x6E0B (name in chunk 0x1F30).

**Gradients (0x5503).** Chunk 0x5503: u16 stop count *n*, *n* u32 stop
colour UIDs, *n* f64 locations (0–1), *n* f64 midpoints, u32 type (1
linear). Chunk 0x5505: flag byte, name, u32 flags as for colours. All 99
gradients and 198 stops in the pairs match.

The midpoint stored with stop *i* is the position of the midpoint between
stops *i* and *i*+1, measured from the start of the gradient (0–1), as
for opacity gradient stops (`transparency.md`). IDML writes it on stop
*i*+1 as a percentage of the distance between the two stops:
`Midpoint` = (midpoint *i* − location *i*) / (location *i*+1 − location
*i*) × 100. Evidence:

- Most pairs have two stops at locations 0 and 1, where both readings
  give the same value. Two gradients in one pair have their first stop at
  0.0037; stored midpoints 0.50184 and 0.60147 give IDML `Midpoint` 50
  and 60 with this formula (2 of 2), not 50.18 and 60.15.
- Gradients with more than two stops, or with stops not at 0 and 1,
  occur in 26 files, 392 gradients. Only the two above are in a
  reference IDML. In all 426 stop gaps of non-zero width, the stored midpoint lies
  between the two stops' locations, as a position from the start of the
  gradient must (for example stops at 0, 0.5 and 1 with stored midpoints
  0.25 and 0.75). Written to IDML unconverted, 13 of them were outside
  the 13–87 that the IDML schema allows for `Midpoint`; converted, all
  426 are inside it.
- 8 gaps, all in one file, have width 0 (two stops at the same
  location), so the formula gives no value.

When the computed value is outside 13–87 or there is none, the converter
leaves the `Midpoint` out and warns.

## Inks (0x1F07)

Each ink object with chunk 0x1F0D is one IDML `Ink` in
`Resources/Graphic.xml`, in UID order (314 of 314 inks, 78 of 78 pairs;
the four process inks are UIDs 7 to 10). `Self` is `Ink/<Name>` (973 of
973 inks in the corpus IDML files). Chunk 0x1F0D is a flag byte (1 =
built-in key, `$ID/`) and the name, then 86 bytes. Offsets from the end
of the name:

| Offset | Contents | IDML | Evidence |
|---|---|---|---|
| 14 | f64 | `NeutralDensity` | 314 of 314 (2,786 of 2,786 over the 654 pairs of the later corpus) |
| 26 | u32, one less than the IDML value | `TrapOrder` | 314 of 314 |
| 32 | f64 | `Frequency` | 314 of 314 |
| 40 | f64 | `Angle` | 314 of 314 (0, 15, 27, 45, 63 and 75) |
| 56 | u8 | `ConvertToProcess` (1 `true`) | 2,065 of 2,065 inks of the trustworthy pairs (3 `true`) |

Byte 2 is 1 for the four process inks and 0 for spot inks; its meaning
is not known. The u8 at 56 is `ConvertToProcess` (1 `true`, 0 `false`):
it is 1 for the 3 inks of the trustworthy pairs whose IDML has `true`
(one spot ink, in three copies of one document) and 0 for the other
2,062; no other byte separates them. `InkType="Normal"` and
`PrintInk="true"` are in every IDML ink and are written from that
observation (`idml-values.md`). The schema puts inks after the
colours and before the tints.

In 41 files without an IDML (InDesign 6.0, 13.x and 14.x, from two
font projects), the neutral density of all four process inks, their
only inks, is −1. The IDML schema allows 0.001 to 10 and no corpus IDML
shows what InDesign writes for such an ink, so the converter leaves
`NeutralDensity` out (with a warning) when the value is outside that
range.

## Colour groups (0x1F39)

Chunk 0x13C is a flag byte and the group name, chunk 0x1F60 a UID list
of the group's swatches. The preferences object (class 0x2202) lists the
groups in chunk 0x1F61 (u32 count, UIDs). IDML writes a `ColorGroup` in
`designmap.xml` for each listed group, in that order, with
`Self="ColorGroup/<Name>"`; the first has `IsRootColorGroup="true"`, the
others `false` (77 of 77 pairs; the DOM 7 pair has no chunk 0x1F61 and
no groups). The root group is named `[Root Color Group]` with no `$ID/`,
though its flag byte is 0.

Before InDesign 11.3 (INDD header version), `Self` is the group's UID
(`u<hex>`) instead: in 58 of the 59 trustworthy pairs of versions 10.0
to 11.2 that have groups (one 11.0 file uses names), and names in 358 of
358 pairs from 11.3 on.

Each swatch of the list is a `ColorGroupSwatch` with
`Self="u<group UID>ColorGroupSwatch<index in hex>"` and `SwatchItemRef`
the swatch's reference (`Swatch/None`, `Color/…`, `Tint/…` or
`Gradient/…`). In 99 of 101 groups the list gives exactly the IDML
swatches, in order; the other two are in the pairs whose IDML was
exported from another save. The converter writes 907 of the 942 IDML
swatches, all with the IDML reference. A swatch without a reference is
left out, and the indices of the others are kept.

## Bullet characters

The preferences object (class 0x2202) lists the bullet characters
offered for lists in chunk 0x1A488: u16 1, u32 count, then for each
bullet u32 character type (0 `UnicodeOnly`, 1 `UnicodeWithFont`, 2
`GlyphWithFont`, as in `attributes.md`), u32 character value, u32 font
family UID (0 = none), a flag byte (1 = built-in key, `$ID/`) and the
font style as an in-object string, then a byte: 0, or 1 followed by four
more bytes. IDML writes one `ABullet` per entry in `designmap.xml`, with
`Self="dABullet<index>"` (the index in hexadecimal: `dABullet9`,
`dABulleta`, `dABulletb` in the 2 documents with more than ten bullets),
`CharacterType`, `CharacterValue` and the
`BulletsFont` (the family name, `$ID/` for none) and `BulletsFontStyle`
properties.

Evidence: all 78 pairs have the chunk; the count equals the number of
IDML `ABullet` elements in 78 of 78 and the records end the chunk. All
396 bullets match on type, value and font style, and 395 on the font;
the other is in the pair whose IDML names the family `Minion Pro (OTF)`
(`fonts.md`).

The byte after the font style is 0 in 7,443 of the 7,447 bullets in the
1,428 distinct little-endian files that have the chunk. In one InDesign
20.0 file (with IDML) it is 1 in four bullets, each with flag byte 2, and
four bytes follow: `00 00 90 01` after `Regular` and `00 00 BC 02` after
`Bold` (read as two u16: 0 and 400, 0 and 700). Their meaning is not
known, and IDML writes nothing for them. With this layout the records end
the chunk in all 1,428 files. In the same file, the same five bytes also
follow `Regular` in `FontStyle` attribute values (0x1B02,
`attributes.md`), whose length prefix includes them. Flag byte 2 occurs in
46 bullets; in that file's IDML its style names have no `$ID/` prefix, as
with flag 0.

## XML tags (0xBF19)

Chunk 0xBF2F is a u32 length and the tag name as text segments (in files
from InDesign 2.0 to 5.0, a flag byte and an in-object string;
`big-endian.md`); chunk
0x117 is the UID of the tag's colour, an object of class 0x1F11 whose
chunk 0x1F01 holds u32 colour space 5 (RGB), u16 3 and three f64
fractions, as for swatches. IDML writes each tag as an `XMLTag` in
`XML/Tags.xml`, with `Self="XMLTag/<Name>"` and the colour as the
`TagColor` property. In all 78 pairs the number of tag objects equals the
number of `XMLTag` elements and the names match (87 tags). IDML lists
them by name, ignoring case (the one pair with more than one tag, 10
tags).

| Red, green, blue | `TagColor` | Evidence |
|---|---|---|
| 0.31, 0.6, 1 | `LightBlue` | 78 tags (`Root`) |
| 1, 0, 0 | `Red` | 1 |
| 0.31, 1, 0.31 | `Green` | 1 |
| 0, 0, 1 | `Blue` | 1 |
| 1, 1, 0.31 | `Yellow` | 1 |
| 1, 0.31, 1 | `Magenta` | 1 |
| 0, 1, 1 | `Cyan` | 1 |
| 0.5, 0.5, 0.5 | `Gray` | 1 |
| 0, 0, 0 | `Black` | 1 |
| 0.6, 0, 0 | `BrickRed` | 1 |

Each colour other than light blue occurs once, all in one file. The
converter leaves out `TagColor` for other colours.

The XML structure (the backing story and the elements) is described in
`xml.md`.
