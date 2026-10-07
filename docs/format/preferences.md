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
