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
- **In-object strings:** u8 2, u8 (usually 0, meaning unknown), u16 length
  in code units, then segments. 4,135 occurrences in three sample files.

## Document (class 0xE01, always UID 1)

| Chunk | Contents |
|---|---|
| 0x501 | UID list: spreads, in order |
| 0x1401 | UID list: master spreads |
| 0x301 | UID list: layers (the first is an internal layer that holds pages) |
| 0x313 | u32: active layer (IDML `ActiveLayer`) |
| 0x222 | Two UID lists: the stories, then the XML backing story; IDML `StoryList` is both (`xml.md`) |
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

**Not converted.** `Locked` (false for all 81 IDML guides),
`GuideZone` (1) and `OverriddenPageItemProps` (empty) have no identified
field. `PageIndex` is 0 for all 22 spread guides and the page's position
in the spread plus 1 for 46 of the 49 page guides; the other 3 do not
follow from the owner, so it is left out.

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

Files from InDesign 3.0 and 4.0 store the page transform and bounds in
chunks 0x151 and 0x154, and chunk 0x140F has no matrix
(`big-endian.md`).
| 0x51A | Margins: four f64 (left, top, right, bottom), u16 own-margins flag |
| 0x528 | Columns: u32 count *n*, *n* f64 column edges, f64 gutter, u16 own-columns flag, 4 bytes |
| 0xCD02 | Layout grid (`GridDataInformation`, below) |

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
the positions). The two bytes after the flag of 0x528 are 1 in 26 pages
and 0 elsewhere; they are not identified.

`ColumnDirection` has no identified field: all 832 `MarginPreference`
elements of the 240 corpus IDML files say `Horizontal`. The converter
writes that value (`idml-values.md`).

**Layout grid (chunk 0xCD02).** u32 font family UID, a flag byte, the
font style as an in-object string, five f64, four u32. IDML writes
`GridDataInformation`:

| Field | IDML | Evidence |
|---|---|---|
| Font family | `AppliedFont` (Properties, the family name) | 283 of 325 (below) |
| Font style | `FontStyle` | 325 of 325 (`Regular`, `Roman`) |
| f64 1 to 5 | `PointSize`, `CharacterAki`, `LineAki`, `HorizontalScale` (×100), `VerticalScale` (×100) | see below |
| u32 1 to 4 | `LineAlignment`, `GridAlignment`, `CharacterAlignment`; one not identified | see below |

The numbers are 12, 0, 9, 1, 1 and the codes 3, 0, 3, 1 in all 2,246
pages of the 250 distinct little-endian files, and all 832
`GridDataInformation` elements of the corpus IDML files have
`PointSize="12"`, `CharacterAki="0"`, `LineAki="9"`, scales of 100,
`LineAlignment="LeftOrTopLineJustify"` and `AlignEmCenter` for both
alignments. The numbers 12 and 9 occur once, so those fields are
located; the others are not proven. The converter writes each number only
when it is the observed value, and the three alignments only when the
codes are 3, 0, 3, 1. The other 42 pages are in one pair whose IDML
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

**ImageIOPreference.** Chunk 0x8C39 of an image, u16 at 0:
`ApplyPhotoshopClippingPath` (1 true, 0 false); 235 of 235 images (293
of 293 with the older-version pairs, 11 of them false).
`AllowAutoEmbedding="true"` and `AlphaChannelName="$ID/"` are in all 496
images of the corpus IDML files and are written from that observation.

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
A text variable instance (class 0xCA64) is owned by a U+0018 at its
position; see `text-variables.md`.

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
converter does the same. The other fields of the chunk are not
identified: `PinPosition` and `AnchorPoint` change together with three
other u16 fields in the object styles.

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
for paragraph styles or 0 for character styles, u16 not identified, u8 1
if the name is a built-in key (`$ID/` in IDML), the name as an in-object
string, then a GUID string in newer files. The name's offset varies (23–26
bytes), so the converter locates it as a flag byte followed by a valid
in-object string.
All 486 style names and 291 `NextStyle` values in the pairs match.

**Kind field.** The kind and the unidentified u16 after it can be read
as one u32 in most files, because the second u16 is 0. The InDesign 7.5
template in `tests/fixtures/scml-template/` (no IDML) has 1 there in 600
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
angles are told apart by one style with −90 and 0. Per-corner values
come in pairs of IDs whose order is not known: the top-left and
top-right radius are 0x6E70 and 0x6E94, the bottom ones 0x6E92 and
0x6E93; the options 0x6E6F and 0x6E91, and 0x6E8F and 0x6E90. The two
of each pair are equal in every style. The converter writes the four
corner radii, and the four corner options, only when all four values
are equal (336 of 337 styles).

**Text frame settings (chunk 0x1B924).** 222 bytes; 162, 142 or 106 in
files from older versions (942, 48, 66 and 61 object styles in the
little-endian corpus). The converter reads it only with these sizes.
The InDesign 4.0 file has 104 bytes, with another layout; the converter
leaves it out with a warning (`big-endian.md`).

| Offset | Contents | Attribute |
|---|---|---|
| 0 | f64 | `TextColumnFixedWidth` |
| 8 | f64 | `TextColumnGutter` |
| 34, 42, 50, 58 | four f64, inset spacing | `InsetSpacing` (below) |
| 66 | u32 | `TextColumnCount` |
| 144 | u16, 1 = true | `FootnotesSpanAcrossColumns`, `SpanFootnotesAcross` |
| 146 | f64 | `FootnotesMinimumSpacing`, `MinimumSpacingOption` |
| 154 | f64 | `FootnotesSpaceBetween`, `SpaceBetweenFootnotes` |
| 190 | f64 | `ColumnRuleStrokeWidth` |
| 198 | u32 swatch, 0 = `n` | `ColumnRuleStrokeColor` |
| 210 | f64 | `ColumnRuleStrokeTint` |

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
`true`: 0x1B940 `EnableStoryOptions`, 0x1B960
`EnableFrameFittingOptions`, 0xADCB `EnableTextFrameColumnRuleOptions`,
no disagreement. Three more go with a pair of IDs that are both present
or both absent in every style, so either ID gives the value: 0x1B933 and
0x1B934 for `EnableFill` and `EnableStroke` (equal in every style),
0xADC8 and 0x1B93E for `EnableTextFrameGeneralOptions` and
`EnableTextFrameBaselineOptions` (equal in every style), and 0xADC9 or
0xADCA for `EnableTextFrameAutoSizingOptions`. The converter writes them
when the two IDs agree. Other `Enable…` attributes vary together with
several IDs and are not written.

**Anchored object settings** are in chunk 0x2800, as for anchors (see
stories). **Not written:** `AnchoredObjectSetting` `PinPosition` and
`AnchorPoint` change together (312 against 25 styles) with
`VerticalAlignment` and three other u16 fields of chunk 0x2800, so they
cannot be told apart.
`EnableTransparency` of the effects categories varies and has no
identified field.

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
  little-endian files, and those in the pairs are `Arabic`. The
  InDesign 3.0 file stores 0x4C06 (`big-endian.md`).
- Style 0x4C17 is lower-case Roman. No pair has it. The evidence is a
  sample and its print PDF. The PDF's page labels (`/PageLabels` in the
  document catalog) give the pages of the sample's 0x4C17 sections style
  `/r`, lower-case Roman, and the pages of its 0x4C15 section style
  `/D`, decimal. The converter writes `PageNumberStyle` for 0x4C15 and 0x4C17 and leaves it
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
bottom, inside, outside) is not verified. Files from InDesign 3.0 to 7.5
have a shorter chunk with another layout; the converter leaves it out
with a warning (`big-endian.md`).

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
`FixedHeight`), only `LeadingOffset` depends on the leading. A sample
also has code 3, with no evidence; the converter leaves out codes other
than 0, 1 and 2.

## Kinsoku and mojikumi tables

Kinsoku tables are objects of classes 0x4209 (hard), 0x420A (soft),
0x42B4 (Korean), 0x42B5 (Simplified Chinese), 0x42B6 (Traditional
Chinese) and 0x4204 (custom); mojikumi tables are class 0x4206. IDML
writes one `KinsokuTable` or `MojikumiTable` per object in
`designmap.xml`, kinsoku tables first, each in UID order. Chunk 0x100B is
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

## Colours (0x1F05)

| Chunk | Contents |
|---|---|
| 0x1F10 | u8 1 if the name is a built-in key; name; u32 flags: bit 0 removable, bit 1 visible, bit 2 editable |
| 0x1F01 | u32 space (5 RGB, 6 CMYK), u16 count, f64 components as fractions |
| 0x1F09 | u32 model: 0 Process, 1 Spot, 2 Registration (below) |
| 0x1F24 | f64 tint value (−1 for a colour, see tints below), then u32 `ColorOverride`: 0 Normal, 1 Specialpaper, 2 Specialblack, 3 Specialregistration, 4 Hiddenreserved |

Unnamed colours are referenced by UID (`Color/u93`). All 1,296 colours in
the pairs match on `Model`, `Space`, `ColorValue`, `ColorOverride`,
`Name` and the three flags.

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
| 14 | f64 | `NeutralDensity` | 314 of 314 |
| 26 | u32, one less than the IDML value | `TrapOrder` | 314 of 314 |
| 32 | f64 | `Frequency` | 314 of 314 |
| 40 | f64 | `Angle` | 314 of 314 (0, 15, 27, 45, 63 and 75) |

Byte 2 is 1 for the four process inks and 0 for spot inks; its meaning
is not known. `InkType="Normal"`, `PrintInk="true"` and
`ConvertToProcess="false"` are in all 973 IDML inks and are written from
that observation (`idml-values.md`). The schema puts inks after the
colours and before the tints.

## Colour groups (0x1F39)

Chunk 0x13C is a flag byte and the group name, chunk 0x1F60 a UID list
of the group's swatches. The preferences object (class 0x2202) lists the
groups in chunk 0x1F61 (u32 count, UIDs). IDML writes a `ColorGroup` in
`designmap.xml` for each listed group, in that order, with
`Self="ColorGroup/<Name>"`; the first has `IsRootColorGroup="true"`, the
others `false` (77 of 77 pairs; the DOM 7 pair has no chunk 0x1F61 and
no groups). The root group is named `[Root Color Group]` with no `$ID/`,
though its flag byte is 0.

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
font style as an in-object string, and a byte that is 0 in every sample.
IDML writes one `ABullet` per entry in `designmap.xml`, with
`Self="dABullet<index>"`, `CharacterType`, `CharacterValue` and the
`BulletsFont` (the family name, `$ID/` for none) and `BulletsFontStyle`
properties.

Evidence: all 78 pairs have the chunk; the count equals the number of
IDML `ABullet` elements in 78 of 78 and the records end the chunk. All
396 bullets match on type, value and font style, and 395 on the font;
the other is in the pair whose IDML names the family `Minion Pro (OTF)`
(`fonts.md`).

## XML tags (0xBF19)

Chunk 0xBF2F is a u32 length and the tag name as text segments (in the
InDesign 3.0 and 4.0 files, an in-object string; `big-endian.md`); chunk
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
