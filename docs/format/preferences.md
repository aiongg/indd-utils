# Preferences

Document preferences are chunks of the preferences object (class 0x2202,
one per document). The converter reads them in `src/model/prefs.rs` and
writes them to `Resources/Preferences.xml` and `designmap.xml`. Values it
does not read stay as observed (`idml-values.md`).

**Evidence.** Each field was compared with the IDML of every corpus pair
(654 pairs, 495 of them trustworthy; `docs/measurement.md`). The counts
below are pairs whose IDML has the value the field gives. Where a pair
disagrees, it is a stale pair unless stated. The layouts are those of
InDesign 7 (CS5) to 21, the versions the pairs show; the converter reads
them for little-endian files of version 7 and later only.

Booleans stored as a u16 are 0 (`false`) or 1 (`true`). Several of them
are chunks that only some documents have; a document without the chunk
has the value given as "absent".

## Colour settings (`Document` in designmap.xml)

| Chunk | Layout | IDML |
|---|---|---|
| 0x7C44 (12 bytes) | Three four-character codes, each stored reversed. The first is `off ` in all 654 files. The second is the RGB policy, the third the CMYK policy | `RGBPolicy`, `CMYKPolicy` |
| 0x7C04 | u16 0, u32 *n*, *n* UIDs of profile objects (class 0x7D03), u32 6, then 6 UIDs: the RGB working profile, the CMYK working profile, and four more | `RGBProfile`, `CMYKProfile` |
| 0x7C08 (6 bytes) | u16 at 2: rendering intent | `SolidColorIntent`, `AfterBlendingIntent`, `DefaultImageIntent` |

Policy codes (as read, reversed): `off ` `ColorPolicyOff`, `pres`
`PreserveEmbeddedProfiles`, `pdil` `CombinationOfPreserveAndSafeCmyk`,
`conv` `ConvertToWorkingSpace`. Both policies match in 495 of 495
trustworthy pairs and 654 of 654 pairs.

A profile is the in-object string at offset 1 of chunk 0x13C of its
0x7D03 object (`U.S. Web Coated (SWOP) v2`, `sRGB IEC61966-2.1`, ...).
UID 0 is written `$ID/`. Matches: RGB 495 of 495 trustworthy and 653 of
654 pairs, CMYK 495 of 495 and 654 of 654. The profile names are stored
in the INDD, so they describe the document, not the computer that
exported it.

Intent codes: 11 `UseColorSettings` (136 files), 0 `Perceptual` (1
file), 2 `RelativeColorimetric` (1 file). The 516 files without the
chunk all have `UseColorSettings`. In every file the three intents are
equal, so the one code is written for all three.

`AccurateLABSpots` is `false` in all 654 IDML files; no field for it was
looked for. The converter writes `false`.

## View (`ViewPreference`)

Chunk 0x1202 (56 bytes in all 654 files):

| Offset | Type | Attribute |
|---|---|---|
| 0 | u32 unit | `HorizontalMeasurementUnits` |
| 4 | u32 unit | `VerticalMeasurementUnits` |
| 8 | u32 unit | `TypographicMeasurementUnits` |
| 12 | u32 unit | `TextSizeMeasurementUnits` |
| 16 | u32 unit | `PrintDialogMeasurementUnits` |
| 20 | u32 unit | `LineMeasurementUnits` and `StrokeMeasurementUnits` |
| 24 | f64 | `HorizontalCustomPoints` |
| 32 | f64 | `VerticalCustomPoints` |
| 40 | f64 | `CursorKeyIncrement` |
| 48 | f64 | `PointsPerInch` |

Unit codes: 0x1201 `Points`, 0x1202 `Picas`, 0x1203 `Inches`, 0x1205
`Millimeters`, 0x1206 `Centimeters`, 0x120B `Pixels`, 0x5101 `Ha`,
0x510F `Q`. Every field matches 495 of 495 trustworthy pairs; the
horizontal, vertical, line and stroke units match 653 of 654 pairs (one
stale pair differs).

## Flags in their own chunks

| Chunk | Attribute | Absent | Matches (trustworthy, all) |
|---|---|---|---|
| 0x593 | `ViewPreference` `ShowFrameEdges` | `true` | 490 of 495, 649 of 654 |
| 0x803 | `ViewPreference` `ShowRulers` | `true` | 493 of 495, 642 of 654 |
| 0x56A | `GridPreference` `BaselineGridShown` | `false` | 495 of 495, 654 of 654 |
| 0x569 | `GuidePreference` `GuidesLocked` | `false` | 495 of 495, 654 of 654 |
| 0x56B | `DocumentPreference` `ColumnGuideLocked` | `true` | 495 of 495, 654 of 654 |
| 0xCAC7 | `TextPreference` `ShowInvisibles` | `false` | 495 of 495, 654 of 654 |

Every file with the chunk matches. The mismatches are files without it
whose IDML has the other value (5 and 12 files), so these two settings
are stored somewhere else as well in some documents; that place is not
known.

## Document setup (`DocumentPreference`)

Chunk 0x533 (146 bytes) also holds the page size, facing pages, intent
and page binding (`objects.md`, document preferences). Its bleed and
slug offsets are rectangles in the order inside (left), top, outside
(right), bottom:

| Offset | Type | Attribute |
|---|---|---|
| 70, 78, 86, 94 | f64 | `DocumentBleedInsideOrLeftOffset`, `…TopOffset`, `…OutsideOrRightOffset`, `…BottomOffset` |
| 102 | u8 | `DocumentBleedUniformSize` |
| 104, 112, 120, 128 | f64 | `SlugInsideOrLeftOffset`, `SlugTopOffset`, `SlugRightOrOutsideOffset`, `SlugBottomOffset` |
| 136 | u8 | `DocumentSlugUniformSize` |

In the 645 files with the 146-byte chunk, 42 have bleeds that are not
all equal. The inside and outside bleeds match only at 70 and 86 (645 of
645 files but one stale pair), and the slugs: inside only at 104, bottom
only at 128. No sample has top and bottom bleeds that differ, or top and
outside slugs that differ; the order of those pairs follows the order
the other fields show. Uniform flags: 645 of 645 files (`DocumentBleedUniformSize`
false in 42, `DocumentSlugUniformSize` true in 3).

**The 126-byte chunk.** Files saved by InDesign 7.0 and 7.5 have a
126-byte chunk 0x533: 16 bytes before the page size, and the fields
from the facing pages flag on 20 bytes earlier than in the 146-byte
chunk:

| Offset | Field | 146-byte offset |
|---|---|---|
| 16, 24 | f64 page width, height | 0, 8 |
| 32 | u8 intent (equal to the u32 at 122) | |
| 38 | u8 2 facing pages, 1 not | 58 |
| 44 | u16 page binding | 64 |
| 50, 58, 66, 74 | f64 bleeds | 70–94 |
| 82 | u8 `DocumentBleedUniformSize` | 102 |
| 84, 92, 100, 108 | f64 slugs | 104–128 |
| 116 | u8 `DocumentSlugUniformSize` | 136 |
| 122 | u32 intent | 142 |

Evidence: the 11 pairs of the corpus of 2026-10 with this chunk (9
trustworthy): page size 11 of 11 (5 sizes), intent 11 of 11 (3 web),
facing pages 11 of 11 (3 facing), binding 0 and `LeftToRight` in 11,
bleeds 11 of 11 (all four 8.504 in 3, 0 in the others), uniform bleed
`true` and uniform slug `false` in 11, slugs all 0. As the four bleeds
are equal in every sample, their order follows the 146-byte chunk.

## Grids (`GridPreference`)

Chunk 0x55F (36 bytes in all 654 files): f64 at 2 `BaselineDivision`,
f64 at 10 `BaselineStart`, f64 at 18 the view threshold as a fraction
(`BaselineViewThreshold` is it times 100). 654 of 654 pairs each.

Chunk 0x545 (16 files): f64 at 8 `HorizontalGridlineDivision`, u32 at
16 `HorizontalGridSubdivision`, f64 at 20 `VerticalGridlineDivision`,
u32 at 28 `VerticalGridSubdivision`; 16 of 16. Of the 638 files without
it, 627 have 72 and 8 for both directions, which the converter writes;
11 have other values stored elsewhere.

## Watermark (`WatermarkPreference` in designmap.xml)

Chunk 0x16344 (621 files): from offset 10, the font family and the font
style, each a u32 length in code units followed by text segments, then
u32 point size and u32 UID of an interface colour (`objects.md`,
interface colours), written as `WatermarkFontColor`. Family, style and
size match 621 of 621. The 33 files without the chunk are left with the
observed values only.

## Text defaults (`TextDefault`)

Chunk 0x23F of the preferences object is a text attribute list in the
layout of a style's list (u16 count, then records; `attributes.md`). The
converter writes it as `TextDefault` with the same attribute table as
styles. Over all pairs this reproduces 182,742 of 196,543 `TextDefault`
values, against 159,982 from the observed values alone; no attribute
reproduced before is lost. As for paragraph styles, `KerningValue` is
left out: the schema does not allow it there.

## Text (`TextPreference`, `TextFramePreference`)

Chunk 0x280 (174 bytes in InDesign 7, 212 in 8 to 14, 214 from 15):

| Offset | Type | Attribute |
|---|---|---|
| 0 | f64 | `SmallCap` |
| 8, 16 | f64 | `SuperscriptSize`, `SubscriptSize` |
| 24, 32 | f64 | `SuperscriptPosition`, `SubscriptPosition` |
| 40 | f64 | `TextFramePreference` `TextColumnGutter` |
| 88 | f64 | `LeadingKeyIncrement` |
| 96 | f64 | `BaselineShiftKeyIncrement` |
| 104 | f64 | `KerningKeyIncrement` divided by 1000 |
| 112 | u32 | `TextFramePreference` `TextColumnCount` |
| 118 | u8 | `TypographersQuotes` |
| 130 | u8 | `LinkTextFilesWhenImporting` |
| 142 | u8 | `TextFramePreference` `FirstBaselineOffset`: 0 `LeadingOffset`, 1 `AscentOffset`, 2 `CapHeight`, 3 `EmboxHeight`, 4 `XHeight` |
| 162 | u8 | `UseParagraphLeading` |
| 184 | u8 | `QuoteCharactersRotatedInVertical` (212 bytes and more) |
| 212 | u8 | `ShapeIndicAndLatinWithHarbuzz` (214 bytes) |

Each matches all 654 pairs that have the attribute (288 for the last,
645 for the one before), and the two positions match each other's
attribute too, except where the files show different values: positions
differ in 2 files, which tells 24 from 32; the sizes are equal in every
file, so 8 and 16 follow the order of the positions. Offsets 168 and 170
match both `UseCidMojikumi` and `UseNewVerticalScaling`, which are equal
in every file. In the corpus of 2026-10 the two bytes are equal to each
other and to both IDML values in 803 of 803 pairs (161 `true`, 642
`false`). Which byte is which is not known, so the converter writes
both attributes only when the bytes are equal (0 `false`, 1 `true`).

**Baseline frame grid colour.** The `BaselineFrameGridColor` of the
`BaselineFrameGridOption` preference follows an observed rule over
stored data, not a decoded field. Over the 803 pairs of the corpus of
2026-10:

| Stored data | Pairs | IDML colour |
|---|---:|---|
| Chunk 0x2834 (24 bytes), u32 at 20 not 0: the UID of an interface colour object (as for layers, `objects.md`) | 11 | that colour (`Charcoal` in all 11) |
| u32 at 20 is 0, offset 168 of chunk 0x280 is 1 | 154 | `Charcoal` |
| u32 at 20 is 0, offset 168 is 0 | 513 | `LightBlue` |
| No chunk 0x2834 (offset 168 is 0 in all) | 125 | `LightBlue` |

No stored field was found that holds the colour when the u32 is 0. Both
the colour and offset 168 appear to be defaults of the edition that
created the document. Every object style's `BaselineFrameGridColor`
equals the preference's in 803 of 803 pairs (3,515 object styles), so
the converter writes this colour in the preference and in every object
style. A UID that is not an interface colour, or another byte at 168,
leaves the observed value (`LightBlue`).

Chunk 0x28BE (14 bytes, 16 from InDesign 18), u8 flags: 4
`SmartTextReflow`, 6 `LimitToMasterTextFrames`, 8 `DeleteEmptyPages`, 10
`PreserveFacingPageSpreads`, 12 `SmartTextReflowSync` (16 bytes). 620 of
620 files with the chunk (283 of 283 for the last). The 34 files without
it have `SmartTextReflow="false"` and `LimitToMasterTextFrames="true"`.

Chunk 0x3768 (4 or 6 bytes): u8 at 0 `AbutTextToTextWrap`, u8 at 2
`ZOrderTextWrap`, and in 6-byte chunks u8 at 4
`HonourTextIndentsWithTextWrap` (IDML has it only for those). Evidence:
`ZOrderTextWrap` 654 of 654; the other two 489 of 489 and 139 of 139
trustworthy pairs.

## Margins and columns of new pages (`MarginPreference`)

Chunk 0x550: f64 `Left`, `Top`, `Right`, `Bottom` at 0, 8, 16, 24 (528
of 528 files). The 126 files without it have 36 for all four.

Chunk 0x555: u32 `ColumnCount` at 0, f64 `ColumnGutter` at 4 (119 of
119). Without it, `ColumnCount` is 1 (535 of 535) and `ColumnGutter` 12
(526 of 535; 9 files have another gutter stored elsewhere).

## Anchored objects (`AnchoredObjectSetting`)

Chunk 0x2800, as anchors and object styles have (`objects.md`): 527
files. `VerticalAlignment` and the two groups of fields described there
match 527 of 527. The 127 files without the chunk have
`VerticalAlignment="TopAlign"`.

## Pasteboard (`PasteboardPreference`)

Chunk 0x5D2: f64 horizontal and vertical margin at 0 and 8, written as
`PasteboardMargins`; the vertical one is also
`MinimumSpaceAboveAndBelow`. 130 of 130 files with the chunk; the 524
without it have `-1 72` and 72, except one stale pair. Four u32 UIDs of
interface colours follow; the IDML colours (`LightGray`, `GridBlue`) are
not in the table of `objects.md`, so they are not written.

## Default XML tags (`XMLPreference`)

Chunk 0xBF4F: five entries, each a name (u32 length, text segments) and
the u32 UID of an interface colour: the story, table, untagged
(`[None]`, not in IDML), cell and image tags. `DefaultStoryTagName`,
`DefaultTableTagName`, `DefaultCellTagName` and `DefaultImageTagName`
match 654 of 654; the names are in the language of the InDesign that
made the document (`Story`, `Article`, `Textabschnitt`, ...). The
colours are written as `Default…TagColor`.

## Grids in back

Chunk 0x567 (u16, 53 files): `GridPreference` `GridsInBack`; 53 of 53.
The 601 files without it have `true`.

## Page item defaults (`PageItemDefault`, `FrameFittingOption`)

The object of class 0x6E07 has a chunk 0x6E07: u32, u32, u32 *n*, *n*
12-byte entries, then a page item attribute list (u32 count, records;
`attributes.md`), then u32, u32, u32 *m* and *m* entries of the same
layout. An entry is u32 class, u32 UID, u32 UID. In every sample the
first table has entries for classes 0x1F05 (colour), 0x5503 (gradient),
0x5533, 0x6E0B (`None`) and 0x6E11. The entries name the unnamed colours
and gradients that IDML writes though nothing refers to them
(`objects.md`, colours). The list has the attribute IDs of page
items. Written with the page item attribute table, it gives
`PageItemDefault` `StrokeWeight` in 628 of 628 files in which an
analysis script found the list; over all pairs the converter reproduces
`StrokeWeight`, `CornerRadius` and `MiterLimit` of every file.

The frame fitting attributes of the list (`objects.md`, frame fitting)
are the preference `FrameFittingOption`. IDML writes all seven in every
file; the list often lacks some (`AutoFit` in 1,126 of the 1,251
trustworthy pairs, the crops in 1,125, `FittingAlignment` in 93), and an
attribute the list lacks has code or number 0 in IDML: `AutoFit="false"`
(1,126 of 1,126), crops `0`, `FittingOnEmptyFrame="None"` (1,123 of
1,123) and `FittingAlignment="TopLeftAnchor"` (93 of 93). Where the list
has the attribute, IDML has its value. The converter reproduces each of
the seven attributes in 1,251 of 1,251 trustworthy pairs (all 1,460
pairs: `AutoFit` absent and `false` in 1,324, `FittingOnEmptyFrame`
`None` in 1,321, `FittingAlignment` `TopLeftAnchor` in 107).

The page item attributes of the list also give the stroke attributes of
`PageItemDefault` (`attributes.md`, strokes): `StrokeTint`, `GapColor`,
`GapTint`, `EndCap`, `EndJoin`, the line ends and `ArrowHeadAlignment`,
each reproduced in every trustworthy pair whose IDML has it, except 4
`RightLineEnd` values (a line end code not identified).

## Print settings (`PrintPreference`, `PrintBookletPrintPreference`)

Chunk 0xA4C holds the print settings and chunk 0xAF2 those of booklet
printing. Both have one layout: a head of fields and strings, seven
fixed blocks (A to G) with strings between them, and a tail. All 1,308
chunks of the 654 pairs (DOM 7 to 21) parse with it, with the same block
lengths. A "string" is a flag byte (1 for a built-in key, 0 for plain
text; 2 and 3 also occur) and an in-object string. Booleans are u8: 1
`true`, 0 `false`. A code not listed leaves the attribute out.

### Head

| Field | IDML |
|---|---|
| u8 1 if a print record follows, u8, u32 length *n*, *n* bytes | `PrintRecord`: `$ID/` and the bytes in base64, 76 characters per line |
| With a record: u16 0, u32. Without: 2 bytes | `DeviceType`: the u32 in decimal; 0 without a record |
| String | `ActivePrinterPreset`: built-in `kPrSt_DefaultName` is `Default`, built-in empty `Custom` (enumerations), otherwise the name (string) |
| u32 | `PrintTo`; `PrintToDisk` is `true` when it is 2 |
| String | `Printer`: built-in `kPrepress File` is the enumeration `PostscriptFile`, built-in empty the string `$ID/`, otherwise the name |
| String | not identified (built-in empty in every file) |
| String | `PPD`: built-in `kDevice Independent` is the enumeration `DeviceIndependent` when the printer is `kPrepress File`, otherwise the string `$ID/kDevice Independent`; built-in empty the string `$ID/`; otherwise the name |
| String | `PPDFile` (`$ID/` and the text for a built-in string) |
| u32 | `PostScriptLevel`: 2 `Level2`, 3 `Level3` |
| f64 | `PrintResolution` |
| 4 f64 | `PaperSizeRect`: left, top, right, bottom |
| 4 f64 | `ImageablePaperSizeRect`: left, top, right, bottom |
| i32, then a string | `PaperSize`: −3 the string, −2 `DefinedByDriver`, −1 `Custom` |

IDML ends the base64 text with a line feed when its last line is full
(76 characters). This holds for `PrintRecord` and `PaperSizeSelector`.

### Blocks

"A+8" is offset 8 in block A.

| Part | Field | IDML |
|---|---|---|
| A (126 bytes) | f64 at A+8 and A+16 | `PaperWidthRange` ("min max") |
| | f64 at A+32 and A+40 | `PaperHeightRange` |
| | f64 at A+56 and A+64 | `PaperOffsetRange` |
| | u16 at A+84 | `PrintPageOrientation`: 0 `Portrait`, 1 `Landscape` |
| | u32 at A+104 | `Copies` |
| | u8 at A+118 | `PrintBlankPages` |
| String | page range text (empty, or text such as `1-2`) | not mapped: IDML `PageRange` is `AllPages` in every file |
| B (30 bytes) | u8 at B+4 | `PrintSpreads` |
| | u8 at B+6 | `ColorOutput`: 0 `CompositeGray`, 1 `CompositeRGB`, 2 `CompositeCMYK`, 5 `CompositeLeaveUnchanged` |
| | u8 at B+10 | `TextAsBlack` |
| | f64 at B+14, B+22 | `CompositeAngle`, `CompositeFrequency` |
| String | | `CompositeScreening`: `$ID/` and the key for a built-in string, else the text |
| String | | `SeparationScreening`, same rule |
| C (25 bytes) | u8 at C+0 | `ScaleMode`: 0 `ScaleToFit`, 1 `ScaleWidthHeight` |
| | u8 at C+4 | `ScaleProportional` |
| | f64 at C+6, C+14 | `ScaleWidth`, `ScaleHeight` |
| | u8 at C+22 | `PagePosition`: 0 `UpperLeft`, 3 `Centered` |
| String | | not identified (plain and empty in every file) |
| D (48 bytes) | u8 at D+0 | 0: `Tile` and `Thumbnails` `false`; 1: `Tile` `true`; 2: `Thumbnails` `true` |
| | f64 at D+8 | `TilingOverlap` |
| | u8 at D+16 | `ThumbnailsPerPage`: 2 `K1x2`, 4 `K2x2`, 9 `K3x3` |
| | u8 at D+20 | `SendImageData`: 0 `AllImageData`, 1 `OptimizedSubsampling` |
| | u8 at D+24 | `DataFormat`: 0 `Binary`, 1 `ASCII` |
| | u16 at D+30 | `BitmapResolution` |
| | u8 at D+42 | `FontDownloading`: 0 `None`, 1 `Complete`, 2 `Subset` |
| | u8 at D+46 | `DownloadPPDFonts` |
| String | | `MarkType` (enumeration): built-in empty `Default`, built-in `kJMarksWithCircle` `JMarkWithCircle` |
| E (72 bytes) | u8 at E+0 | `MarkLineWeight`: 1 `P25pt`, 2 `P50pt`, 4 `P07mm`, 5 `P10mm` |
| | f64 at E+4 | `MarkOffset` |
| | u8 at E+12, 14, 16, 18, 20 | `CropMarks`, `PageInformationMarks`, `ColorBars`, `RegistrationMarks`, `BleedMarks` |
| | u8 at E+22 | `UseDocumentBleedToPrint` |
| | f64 at E+24 | `BleedTop` |
| | u8 at E+32 | `BleedChain` |
| | f64 at E+34, E+42, E+50 | `BleedInside`, `BleedBottom`, `BleedOutside` |
| | u8 at E+58 | `IncludeSlugToPrint` |
| | u8 at E+68 | `Profile` (enumeration): 1 `UseDocument`, 0 `PostScriptCMS` |
| String, F (4 bytes), string | | not identified (built-in empty strings in every file) |
| G (98 bytes) | f64 at G+12, G+20 | `CyanFrequency`, `CyanAngle` |
| | f64 at G+30, G+38 | `MagentaFrequency`, `MagentaAngle` |
| | f64 at G+48, G+56 | `YellowFrequency`, `YellowAngle` |
| | f64 at G+66, G+74 | `BlackFrequency`, `BlackAngle` |
| | f64 at G+82, G+90 | `SpotFrequency`, `SpotAngle` |
| String | | `FlattenerPresetName`: `$ID/` and the key for a built-in string; flag 3 with `[High Resolution]` (or its Czech name, `[Vysoké rozlišení]`) is `$ID/kFlSt_HighDefaultName`; other names are left out |
| Tail | 6 zero bytes, u32 *n*, *n* bytes, then u16 (DOM 7 to 9) or u32 (DOM 10 on), 0 or 1 | `PaperSizeSelector`: `$ID/` and the *n* bytes in base64, as `PrintRecord` |

Also in every block are f64 −1 at A+0 and A+24 (`PaperWidth` and
`PaperHeight` are `Auto` in every file). The other bytes of the blocks
are not mapped: the IDML attributes left (`Collating`, `Sequence`,
`Trapping`, `PrintCyan`, `Intent`, …) have one value in every file, so
they cannot be told apart.

Three attributes are derived from fields:

- `AllPrinterMarks` is `true` exactly when the five marks of block E are
  all `true` (`PrintPreference`: 2 of 2 `true`, 487 of 487 `false`;
  booklets: 5 of 5 and 484 of 484).
- `PreserveColorNumbers` is `false` when `ColorOutput` is
  `CompositeRGB` and `true` otherwise. Two of the 37 documents with
  `CompositeGray` have `false`.
- `PrintToDisk` (above).

`BitmapPrinting` is not stored: no byte of any chunk of the preferences
object (0x2202) or of the document object, nor of the print record,
follows it. The converter does not write it.

`PrintBookletPrintPreference` has no `PrintSpreads`, `Thumbnails`,
`Tile`, `TilingOverlap`, `ThumbnailsPerPage`, `IncludeSlugToPrint` or
`PageRange` (schema); the converter reads the fields and does not write
these.

Evidence: each attribute above was compared over the 489 trustworthy
pairs for both elements; every one matches in all 489, except
`PreserveColorNumbers` (487, the two `CompositeGray` documents) and
`MarkType` (488: one DOM 16.1 document stores `kJMarksWithCircle` and
its IDML has `Default`). Over all 654 pairs, the only further misses are
`PPDFile` of two stale booklet pairs (a built-in path written without
`$ID/`). The paper rectangles are written as IDML writes them: with the
digits of the shortest form that reads back, the last rounded half to
even from the exact value (583.2000122070312, not …313). Several
ranges and rectangles are f32 values widened to f64
(`16.600000381469727`); they match only when written in full.

Which of two equal fields is which is assumed, because they are equal
in every sample: `CyanFrequency` and `MagentaFrequency` (the stride of
the ink entries supports the order), `ScaleWidth` and `ScaleHeight`,
and `BleedTop`, `BleedInside` and `BleedOutside`.

### Booklet options (`PrintBookletOption`)

Chunk 0xAF0 of the preferences object: u32, a string (a page range or
paper name, not mapped), 4 bytes, then f64 `TopMargin`, `BottomMargin`,
`LeftMargin`, `RightMargin`, then 34 bytes (`AutoAdjustMargins` and the
other attributes have one value in every file). The chunk is in 11
trustworthy pairs (12 of all pairs); the margins match in all of them.
Without the chunk, IDML has the margins 36 (478 of 478). Left and right
are equal in every sample, so their order is assumed.

## Index header setting (`IndexHeaderSetting`)

Chunk 0x1300E of the preferences object holds the attributes of
`IndexHeaderSetting` and its `Properties/ListOfIndexHeaderGroup`. Every
trustworthy pair has the chunk. A "u32 string" is a u32 length in UTF-16
units, then text segments.

| Field | IDML |
|---|---|
| String | `HeaderSetName` (`$ID/` and the text for a built-in string) |
| u16 | `HeaderSetLanguage` |
| u32 | `IndexHeaderSetHandler` |
| u32 | `IndexHeaderSetGroupValue` |
| u32 | `IndexHeaderSetGroupOptionValue` |
| u16 | not identified (1) |
| u32 | number of groups |

Each group is an `IndexHeaderGroupType`:

| Field | IDML |
|---|---|
| String | `InternalName` (`$ID/` and the text for a built-in string) |
| String | `UIString` (same rule) |
| u32 string | `DocumentString`, written `$ID/` and the text |
| u16 | `Visibility`: 1 `true`, 0 `false` |
| u32 | number of sections |

Each section is a `SectionHeaderType` in the group's
`SectionHeaderArray`:

| Field | IDML |
|---|---|
| u32 string | `SortingHeaderString`, written `$ID/` and the text |
| u32 string | `DocumentHeaderString`, written `$ID/` and the text |
| String | `UIHeaderString` (`$ID/` and the text for a built-in string) |
| u16 | `Language` |

The chunk ends after the last section. Evidence: the five attributes and
the whole list equal the IDML in 489 of 489 trustworthy pairs; the parse
reads every chunk of the 654 pairs to its end. The default list has two
groups (`kIndexGroup_Symbol` with one section, `$ID/IDX_Basic` with 26);
documents with CJK index groups have more.

## Index options (`IndexOptions`)

Chunk 0x13010 of the preferences object, in 10 of the 489 trustworthy
pairs and in no stale pair:

| Field | IDML |
|---|---|
| u32 string | `Title` |
| String | `TitleStyle` (a paragraph style name, below) |
| u8 | `ReplaceExistingIndex` (1 `true`) |
| u8 | `IncludeBookDocuments` |
| String | not identified (empty in every file) |
| 14 bytes | byte 10: `IncludeSectionHeadings`; the rest 0 |
| 6 u32 strings | `FollowingTopicSeparator`, `BetweenPageNumbersSeparator`, `BetweenEntriesSeparator`, `BeforeCrossReferenceSeparator`, `PageRangeSeparator`, `EntryEndSeparator` |
| 8 strings | `Level1Style` to `Level4Style`, `SectionHeadingStyle` (paragraph styles), `PageNumberStyle`, `CrossReferenceStyle`, `CrossReferenceTopicStyle` (character styles) |
| 4 bytes | 0 |

- The en dash (U+2013) stored as `PageRangeSeparator` is written `^=`
  (10 of 10 pairs match).
- A style is stored by name. IDML names the style of that name that is
  in no style group, whatever the built-in flag of the stored name. If
  there is none, IDML writes `ParagraphStyle/$ID/[No paragraph style]`
  (`CharacterStyle/$ID/[No character style]`). Examples: a stored
  built-in `Index Level 1` in a document without that style gives the
  root style; a stored built-in `Index Section Head` gives
  `ParagraphStyle/Index Section Head` where a user style of that name
  exists; a stored name equal to a style inside a style group gives the
  root style.

Evidence: every attribute in 10 of 10 trustworthy pairs (titles in
several languages, `ReplaceExistingIndex` `false` once,
`IncludeSectionHeadings` `false` once, user style references).

Without the chunk, IDML has `Title="Index"` in most documents and a
translation in the others (`索引`, `Indice`, `Указатель`, …), and
`BetweenEntriesSeparator` `; ` or `、`. Both follow the language of the
exporting application. The language code of the document's last
session (`objects.md`, save history) shows a Japanese or Chinese
edition: the converter writes `Title="索引"` and `、` for code 0x0101,
and `; ` (no title) for other codes, whose titles vary.

## Chapter numbering (`ChapterNumberPreference`)

Chunk 0x1A4C4 of the preferences object (14 bytes), in 37 of the 489
trustworthy pairs:

| Offset | Field | IDML |
|---|---|---|
| 0 | u32 | `ChapterNumberFormat` (string): 0x1A477 `1, 2, 3, 4...`, 0x1A47A `A, B, C, D...`, 0x1A479 `i, ii, iii, iv...`, 0x1A473 `001,002,003...` |
| 4 | u32 | `ChapterNumberSource`: 1 `UserDefined`, 3 `ContinueFromPreviousDocument` |
| 8 | u32 | `ChapterNumber` |
| 12 | u16 | not identified |

Evidence: 37 of 37. Without the chunk, IDML has the observed values
(`ChapterNumber="1"`, `ContinueFromPreviousDocument`, `1, 2, 3, 4...`)
in 452 of 452.

## Dictionary (`DictionaryPreference`)

Chunk 0x2806 of the preferences object (6 bytes, the same in every
file). Without it, `Composition` is `Both` (468 of 468 trustworthy
pairs). With it, `Composition` is `UseDocument` in 16 of 21; the other 5
(DOM 14 and 15) have `Both` with the same bytes. The converter writes
`UseDocument` when the chunk is there.

## EPUB export (`EPubExportPreference`)

From DOM 8. Chunk 0x21A1A of the preferences object, in 13 of the 482
trustworthy pairs with DOM 8 or later:

- u32 at 0: `Version`, 0 `Epub2`, 1 `Epub3` (13 of 13).
- `Id` is the in-object string in the chunk that starts with
  `urn:uuid:` (13 of 13). The rest of the chunk is not decoded; its
  length varies.

Without the chunk (469 of 469):

- `Id` is `urn:uuid:29d919dd-24f5-4384-be78-b447c9dc299b` and
  `TocStyleName` is `$ID/`.
- `Version` is `Epub2` for INDD files up to version 18.0 and `Epub3`
  from 18.1 (the 5 DOM 18.0 pairs whose INDD is 18.0 have `Epub2`, the
  13 whose INDD is 18.1 to 18.4 have `Epub3`).

## Default styles, grids and other settings

Chunks of the preferences object, trustworthy pairs. A colour is the UID
of an interface colour (`objects.md`, interface colours).

**Default styles.** Each chunk is a list of u32 UIDs; the third is the
default:

| Chunk | u32 at 8 | u32 at 12 | u32 at 16 |
|---|---|---|---|
| 0x28D4 (12 bytes) | `TextDefault` `AppliedParagraphStyle` | | |
| 0x28D5 (12 bytes) | `TextDefault` `AppliedCharacterStyle` | | |
| 0x1B959 (20 bytes) | `PageItemDefault` `AppliedGraphicObjectStyle` | `AppliedTextObjectStyle` | `AppliedGridObjectStyle` |

The first two UIDs are the root style and the root style group. The
reference is written as other style references; UID 0 is the root
style. Evidence: every pair has the three chunks. Paragraph 487 of 489
(the other two name another style in IDML; not explained), character
489 of 489, graphic and text object styles 489 of 489.

**Layout and story grids.** Chunks 0xCD2F (`LayoutGridDataInformation`)
and 0xCD2E (`StoryGridDataInformation`) start as page chunk 0xCD02
(`objects.md`): u32 font family UID (`AppliedFont`), a flag byte and the
font style (`FontStyle`), five f64 (`PointSize`, `CharacterAki`,
`LineAki`, and `HorizontalScale` and `VerticalScale` as fractions of
100 %), four u32 codes. Chunk 0xCD2E continues with u32 0 and f64
`CharacterCountSize`. Evidence: every pair has both chunks; all values
match in 489 of 489 but `AppliedFont` (488: one family that IDML names
in another form, `fonts.md`).

**Single settings.**

| Chunk | Layout | IDML | Without the chunk | Evidence |
|---|---|---|---|---|
| 0x59C | u16 1 | `DocumentPreference` `MasterTextFrame`, `CreatePrimaryTextFrame` (DOM 8 on) `true` | `false` | 14 with, 475 without; 489 of 489 |
| 0x1081F | u32: 2 `RGB`, 3 `CMYK` | `TransparencyPreference` `BlendingSpace` | `CMYK` | 62 of 62; 427 of 427 |
| 0x5A6 | u16: 1 `true`, 0 `false` | `DocumentPreference` `AllowPageShuffle` | (in every file) | 489 of 489 |
| 0x568 | u16 | `GuidePreference` `GuidesShown` | `true` | 114 of 114; 375 of 375 |
| 0x55A | u8 at 0, u8 at 2 | `GuidePreference` `GuidesSnapto`, `GridPreference` `DocumentGridSnapto` | `true`, `false` | 26 of 26; 463 of 463 |
| 0x53F (22 bytes) | u8 at 0; colour at 18 | `GuidePreference` `GuidesInBack`, `RulerGuidesColor` | (in every file) | 489 of 489 |
| 0x55F | colour at 30; u8 at 34: 0 `TopOfPageOfBaselineGridRelativeOption`, 1 `TopOfMarginOfBaselineGridRelativeOption` | `GridPreference` `BaselineColor`, `BaselineGridRelativeOption` | (in every file) | 489 of 489 |
| 0x545 | colour at 32 | `GridPreference` `GridColor` | `LightGray` | 14 of 14; 475 of 475 |
| 0x550 | colour at 36 | `DocumentPreference` `MarginGuideColor` | `Magenta` | 388 of 388; 101 of 101 |
| 0x555 | colour at 18 | `DocumentPreference` `ColumnGuideColor` | (observed value) | 72 of 72 |
| 0x5D2 | colours at 20, 24, 28 | `PasteboardPreference` `BleedGuideColor`, `SlugGuideColor`, `PreviewBackgroundColor` | `Fiesta`, `GridBlue`, `LightGray` | 80 of 80; 409 of 409 |
| 0x54A (18 bytes) | f64 at 0, f64 at 8 | `Document` `ZeroPoint` | `0 0` | 46 of 46; 443 of 443 |
| 0x7006 (20 bytes) | u8 at 0; f64 at 12 | `LayoutAdjustmentPreference` `EnableLayoutAdjustment`, `SnapZone` | `false` (observed), `SnapZone` not written | 24 of 24 |
| 0xCA0B | u16 1 | `ViewPreference` `ShowTextThreads` `true`, INDD version 21.1 on | `false` | IDML has the attribute only from version 21.1: 26 of 26 (28 of 28 in all pairs) |

Without chunk 0x555, `ColumnGuideColor` keeps its observed value, which
4 of the 417 pairs without the chunk contradict. Without chunk 0x7006,
`SnapZone` varies and is not written.
