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

All little-endian.

- **UID lists:** u32 count, then that many u32 UIDs.
- **Matrices:** six f64, in IDML `ItemTransform` order (a b c d tx ty).
- **Text segments:** a u16 header, flags in bits 14–15 and a count in bits
  0–13. 0x4000: *count* single-byte characters follow. 0x8000: *count*
  UTF-16LE code units follow. A text of *n* code units is a sequence of
  segments adding up to *n*. Example: "WOMEN’S\r" is
  `05 40 "WOMEN" 01 80 19 20 02 40 "S\r"`.
- **In-object strings:** u8 2, u8 (usually 0, meaning unknown), u16 length
  in code units, then segments. 4,135 occurrences in three sample files.

## Document (class 0xE01, always UID 1)

| Chunk | Contents |
|---|---|
| 0x501 | UID list: spreads, in order |
| 0x1401 | UID list: master spreads |
| 0x301 | UID list: layers (the first is an internal layer that holds pages) |
| 0x313 | u32: active layer (IDML `ActiveLayer`) |
| 0x222 | UID lists: stories (IDML `StoryList`) |
| 0x4C01 | UID list: sections |

## Spreads (0x501) and master spreads (0x1401)

| Chunk | Contents |
|---|---|
| 0x503 | u32 self, u32 0, UID list of spread layers |
| 0x56E | Matrix: `ItemTransform` |
| 0x1B8 | u32: `BindingLocation` |

**Spread layers (0x301)** hold the items of one document layer on one
spread. Chunk 0x302: u32 document layer UID, u16 1 for the layer's guide
part. Chunk 0x303: u32 spread, u32 spread, UID list of children. Pages are
children of the spread layer for the internal pages layer.

## Layers (0x302)

Chunk 0x304: u16 locked, u16 visible, then fields not yet identified, then
the name as an in-object string. The internal layer is named
`Internal_pages_layer_name` and has no IDML element.

## Pages (0x50F)

| Chunk | Contents |
|---|---|
| 0x140F | u32 applied master spread, u16 (unknown), matrix `MasterPageTransform` |
| 0x5CC | Matrix: `ItemTransform` |
| 0x5DD | Four f64: left, top, right, bottom. IDML `GeometricBounds` is top, left, bottom, right. |
| 0x51A | Four f64: margins (36 in the blank document) |

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
| 0x162B | Path geometry (below) |
| 0x15B | Hierarchy: u32 spread, u32 parent, UID list of children |
| 0x6E03 | Attribute list: local formatting (see `attributes.md`) |
| 0x1B916 | u32: applied object style |

**Path geometry:** u32 path count; per path: u32 point count, points, u16
1 if the path is open. Each point is a u32 type, then f64 values: type 2 =
anchor (x, y) only; types 0 and 1 = left direction, anchor, right
direction. All 3,191 path chunks in the samples parse exactly, and the
points of every page item path match the IDML `PathPointType` values.

**IDML element type** is not stored. A frame is a `TextFrame` if a child of
class 0x263 exists. Otherwise the converter uses the path: two points and
open → `GraphicLine`; four corner points on two distinct x and y values →
`Rectangle`; four smooth points → `Oval`; anything else → `Polygon`. In the
samples this gives 123/123 lines, 671/671 rectangles, 10/10 ovals and
396/402 polygons (6 polygons are ellipse-like).

**Text frames:** the frame's child of class 0x263 (multi-column frame) has
a child of class 0x227 (frame column). Chunk 0x220 of the column holds the
frame list (class 0x228). Its chunk 0x205: u32 story, then a UID list of
the columns of all threaded frames in order, which gives
`PreviousTextFrame` and `NextTextFrame`.

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
  and `Bottom="5.669291338582678"`. A privately held sample and its print
  PDF agree; see below.
- Offset 8 (first) is therefore `Left`. It is 0 in every sample that
  has a reference.
- The two contour frames have the same value at all four offsets, as in
  their IDML.

**Evidence from a privately held sample and its print PDF.** The sample
[detail of a non-public sample removed]
[detail of a non-public sample removed]
is set beside a frame, and text above the frame must end above the top
of the wrap area. Text line positions in the PDF were taken from
pdftotext word boxes and compared with the frame bounds:

- [detail of a non-public sample removed]
  ends at most 1.5 pt below the frame's top edge.
- [detail of a non-public sample removed]
  [detail of a non-public sample removed]
  frame's top edge, inside the frame; in each, the overlap is at most
  [detail of a non-public sample removed]
  next line position would have ended more than the offset's size plus
  1.5 pt below the edge, and is empty.
- [detail of a non-public sample removed]
  [detail of a non-public sample removed]
  [detail of a non-public sample removed]
  [detail of a non-public sample removed]
  [detail of a non-public sample removed]
  frame's top edge.

So the value at offset 16 moves the top edge of the wrap area, and a
negative value lets text into the frame from above.

**Other values.** One image in the same-version pairs has 0x40001 at
offset 40 and `TextWrapSide="SideAwayFromSpine"`; no other value is
shown, so the converter writes the side and inverse settings only when
the u32 at 40 is 1. Modes other than 0, 1, 3 and 6 have no IDML
evidence; the converter leaves out `TextWrapPreference` for them.

## Placed graphics

Images (0x1702), PDF (0x2501), EPS (0x6601) and SVG (0x6639) are children
of a frame (in its 0x15B list) and share these chunks:

| Chunk | Contents |
|---|---|
| 0x151 | Matrix: `ItemTransform` (437/437 match) |
| 0x1633 | Four f64: `GraphicBounds` left, top, right, bottom |
| 0x8CBC | u32, u32, u32 link UID |

**Links (0x8C42).** Chunk 0x8C9B: u32 0, u32, u32 link resource UID, u32,
u32 graphic UID, fields, then the import stamp as a u32 length and text
segments, then two 8-byte timestamps. **Link resources (0x8C41)**: chunk
0x8C92 is a flag byte, u32 length, then the URI as bytes
(`LinkResourceURI`; 335/340 match, the rest were relinked after export).

After the URI, chunk 0x8C92 continues:

| Offset after the URI | Field |
|---|---|
| 0 | u8 1, u8 1, u16 0 (all resources) |
| 4 | u32 0 or 1, not identified |
| 8 | u32 0, 2 or 3 |
| 12 | u32 UID of a raw data object (class 0x129), or 0 |

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
most likely screen previews. The converter does not use them.

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

**Owned items (run kind 0x209).** Each record: u32 run length, u16
count, then that many (u32 class, u32 UID) pairs, the objects owned by
the text position at the start of the run. An item anchored in text has
the character U+FFFC at that position and is owned through an object of
class 0x262, whose chunk 0x2C8 (u32, u32, UID list) lists the anchored page
item. IDML writes the page item element in place of the U+FFFC.

INDD stores a forced line break as U+000A; IDML writes it as U+2028. The
last paragraph return of a story is not written to IDML.

## Styles (0x205)

Paragraph and character styles share the class. Chunk 0x230: u32 next
style (0 = itself), u32 based-on style, fields not yet identified, u32 1
for paragraph styles or 0 for character styles, u8 1 if the name is a
built-in key (`$ID/` in IDML), the name as an in-object string, then a GUID
string in newer files. The name's offset varies (23–26 bytes), so the
converter locates it as a flag byte followed by a valid in-object string.
All 486 style names and 291 `NextStyle` values in the pairs match.

IDML writes a `BasedOn` of the root `[No paragraph style]` or
`[No character style]` as a string (`$ID/[No paragraph style]`), and any
other base as an object reference.

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

**Object styles** (class 0x1B901): chunk 0x1B907 is u32 based-on style,
u8 1 for a built-in name, then the name. A page item's chunk 0x1B916 is
its applied object style (`AppliedObjectStyle`, 671/671 rectangles). A
style based on the root `[None]` has its `BasedOn` written as a string.

## Master spread names and sections

Master spread chunk 0x1402: a flag byte and the name prefix (`A`), a flag
byte and the base name (`Master`). IDML `Name` is `<prefix>-<base>`, and
the master's pages are named by the prefix (99/99 match).

**Sections (class 0x4C01).** The document's chunk 0x4C01 lists them.
Section chunk 0x4C02:

| Field | Contents |
|---|---|
| u8, string | Not identified; empty in most samples |
| u8, string | Not identified; empty in most samples |
| u32 | First page of the section; 0 for the section that starts at the document's first page |
| u32 | Page number start (`PageNumberStart`) |
| u32 | Page number style: 0x4C15 = `Arabic`, 0x4C17 = `LowerRoman` |
| u32 | Continue numbering (1 = true, `ContinueNumbering`) |

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
  little-endian files, and those in the pairs are `Arabic`.
- Style 0x4C17 is lower-case Roman. No pair has it. The evidence is a
  [detail of a non-public sample removed]
  [detail of a non-public sample removed]
  PDF's page labels (`/PageLabels` in the document catalog) give every
  [detail of a non-public sample removed]
  [detail of a non-public sample removed]
  [detail of a non-public sample removed]
  [detail of a non-public sample removed]
  evidence. The
  converter writes `PageNumberStyle` for 0x4C15 and 0x4C17 and leaves it
  out (with a warning) for other codes.

The converter names document pages by their number in their section,
counting on from the previous section when numbering continues. In a
section with style 0x4C17 it writes the number in lower-case Roman (i,
ii, iii), as in the PDF labels above; no pair shows IDML page names in
such a section.

## Document preferences (class 0x2202)

One object of class 0x2202 holds document-wide preferences. Chunk 0x533
(`DocumentPreference`): f64 page width at 0, f64 page height at 8, byte 58
2 for facing pages and 1 otherwise, four f64 bleeds from offset 70, u32
intent at 142 (0 print, 1 web, 2 mobile). All values match the 75 pairs.
The four bleeds are equal in every sample, so their order (written as top,
bottom, inside, outside) is not verified.

## Text frame preferences

From the frame's multi-column frame object (class 0x263):

| Chunk | Offset | `TextFramePreference` attribute |
|---|---|---|
| 0x2D1 | u32 0 | `TextColumnCount` (1,052/1,052) |
| 0x2D1 | f64 4 | `TextColumnGutter` (93/93) |
| 0x2D1 | f64 14 | `TextColumnFixedWidth` (1,045/1,045) |
| 0x2CE | u16 0 | `FirstBaselineOffset`: 0 LeadingOffset, 1 AscentOffset, 2 CapHeight (below) |
| 0x2CE | u16 2 | `VerticalJustification`: 0 Top, 1 Center, 2 Bottom, 3 Justify (350/350) |
| 0x2CE | u16 20 | `VerticalBalanceColumns` (18/18) |
| 0x2CE | u16 22 | `AutoSizingType`: 0 Off, 1 HeightOnly, 2 WidthOnly, 3 HeightAndWidth (61/61) |
| 0x2CE | u32 24 | `AutoSizingReferencePoint`: 0–8, top-left to bottom-right by rows (58/58) |

Inset spacing is 0 in every sample and not located.

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
`FixedHeight`), only `LeadingOffset` depends on the leading. The sample
also has code 3, with no evidence; the converter leaves out codes other
than 0, 1 and 2.

## Colours (0x1F05)

| Chunk | Contents |
|---|---|
| 0x1F10 | u8 1 if the name is a built-in key; name; u32 flags: bit 0 removable, bit 1 visible, bit 2 editable |
| 0x1F01 | u32 space (5 RGB, 6 CMYK), u16 count, f64 components as fractions |
| 0x1F09 | u32 model: 0 Process, 2 Registration |
| 0x1F24 | f64 (−1), then u32 `ColorOverride`: 0 Normal, 1 Specialpaper, 2 Specialblack, 3 Specialregistration, 4 Hiddenreserved |

Unnamed colours are referenced by UID (`Color/u93`). All 1,296 colours in
the pairs match on `Model`, `Space`, `ColorValue`, `ColorOverride`,
`Name` and the three flags.

The `None` swatch is class 0x6E0B (name in chunk 0x1F30).

**Gradients (0x5503).** Chunk 0x5503: u16 stop count *n*, *n* u32 stop
colour UIDs, *n* f64 locations (0–1), *n* f64 midpoints (the midpoint
between stops i and i+1 is stored with stop i), u32 type (1 linear).
Chunk 0x5505: flag byte, name, u32 flags as for colours. All 99 gradients
and 198 stops in the pairs match.
