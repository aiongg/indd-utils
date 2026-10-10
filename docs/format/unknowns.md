# Open items

What the converter does not decode yet, and why. Each row is an item
that is open: no field was found, the value never varies, there is no
sample, or a rule does not explain every document. Items decoded since
are described in the page named in the "See" column; the rows here only
record what is left of them.

Counts are over the 1,251 trustworthy pairs of the corpus of 2026-10
(`docs/measurement.md`), "files" over the distinct corpus files outside
the privately held samples. "Code" is the language code of the last
session of the save history (`objects.md`, save history).

Status values:

| Status | Meaning |
|---|---|
| open: no field found | the IDML value varies, but no stored byte was found that follows it |
| open: never varies | the value is the same in every sample, so its field cannot be told apart |
| open: no pair | the stored value occurs, but no pair shows its IDML value |
| open: not explained | a rule holds for most documents; the exceptions follow no stored value |
| open: two collections only | the rule rests on documents of two related collections |
| open: not examined | the values are left; nobody has looked for their rule yet |
| open: not decoded | the structure is known to exist but is not read |
| open: not stored | the value depends on the layout or the export time |
| not needed | the value depends on the exporting computer or application, not on the document |

## Preferences

| Item | Where | Files | Status | Content and reason | See |
|---|---|---:|---|---|---|
| `TilingType` | `PrintPreference` | 1,251 | open: never varies | `Auto`, except 1 `AutoJustified`; no field in chunk 0xA4C | `preferences.md` |
| `PageRange` | `PrintPreference` | 1,251 | open: no field found | `AllPages` in 1,239, absent in 12; the stored page text does not separate the groups | `preferences.md` |
| `HighlightKeeps`, `HighlightSubstitutedFonts` | `TextPreference` | 1,251 | open: no field found | `false` and `true` with 4 and 3 exceptions; no chunk in any file | `preferences.md` |
| Highlight flags without chunks 0xCA62, 0xCAD3, 0xCAD4 | `TextPreference` | 5 | open: not explained | written `false`; 4 or 5 documents have every highlight on | `preferences.md`, flags |
| `GuideSnaptoZone` | `ViewPreference` | 1,251 | open: no field found | `4`; 5 documents have `2` | |
| `ShowNotes` | `ViewPreference` | 1,251 | open: no field found | 19 `false` | |
| `ViewAfterExport` | `XMLExportPreference` | 1,251 | open: never varies | 1 `true` | |
| `SpreadHiddenVisibility` | `DocumentPreference` (DOM 20 on) | 76 | open: never varies | 1 `true` | |
| `ColumnDirection` without chunk 0x555 | `DocumentPreference`, `MarginPreference` | 2 | open: not explained | `Vertical` with a horizontal story orientation | `preferences.md`, margins and columns |
| `WatermarkFontColor` without chunk 0x16344 | `WatermarkPreference` | 57 | open: no field found | `Red`, `LightBlue`, `Yellow`, `Cyan` | `preferences.md`, watermark |
| Booklet print settings without chunk 0xAF2 | `PrintBookletPrintPreference` | 24 | open: two collections only | 88 values are the same in all 24, 10 follow the code; not written | `preferences.md`, print settings |
| Grid, gutter, column guide and baseline frame grid colours, print marks, flattener resolutions of two Korean collections | preferences, spreads | about 97 | not needed | code 0x0101 with the values of a Roman edition: the IDML was exported by another installation than the one that saved the file | `preferences.md`, values of the exporting edition |
| `AppliedMathMLFontSize` | `Document` (DOM 20 on) | 99 | open: no field found | `10`, but `16` in 3 DOM 20.2 files; left out | `idml-values.md`, document attributes |
| Language quotes without chunk 0x2D26 | `Language` (Japanese, Korean, Hebrew) | 10 | open: no field found | `''` and `""` in most documents, `‘’` and `“”` in 2, 7 and 1; left out | `idml-values.md`, language quotes |
| `UntitledDocumentCount` | `Document` (DOM 12) | 72 pairs | open: no field found | `1` in most DOM 12 IDML files, other numbers in a few | |

## Resources and text

| Item | Where | Files | Status | Content and reason | See |
|---|---|---:|---|---|---|
| Language list | `Language` elements | 42 | open: no field found | IDML lists only the languages in use in 42 documents and all in the others; the language objects are the same in both kinds | |
| Font families IDML drops | class 0x3E03 | 101 | open: no field found | unused families left out of `Fonts.xml`, but many other unused ones kept; no record byte differs; files up to 17.x only | `fonts.md` |
| Fonts IDML adds | `Font` | 97 | not needed | IDML adds every style of a family installed on the exporting computer | `fonts.md` |
| `$ID/` PostScript names | font record | 4 pairs | open: no field found | every font of one DOM 7 set | `fonts.md` |
| u16 0 and u16 1 after `SeparatorStyle` | TOC entry, chunk 0x11605 | – | open: never varies | one may be `SortAlphabet` (`false` in all 255 entries) | `objects.md`, table of contents styles |
| u16 after `Level` | TOC entry, chunk 0x11605 | – | open: no pair | 0 in every paired entry, 1 in some entries of files without a pair | `objects.md`, table of contents styles |
| Value 2 of 0x1B09 with no IDML attribute | text attribute | 2 | open: no field found | `NextPageNumber` stored where IDML writes no attribute | `attributes.md`, page number type |
| Several alternate glyph features | text attribute 0x42AE | 0 | open: no sample | every run stores one feature; the IDML form of more than one is not shown, so such values are left out | `attributes.md`, alternate glyphs |
| `ResultText` of other text variables | `TextVariableInstance` | 200 | open: not stored | page numbers, dates and cross-reference page numbers depend on the layout or the export time; the last page number shows `1` in every sample | `text-variables.md` |
| Saving edition rule exceptions | tints, users, endnote and index titles | 9 | not needed | the IDML was exported by another edition than the one that saved the INDD (one trustworthy document, 8 stale pairs) | `objects.md`, saving edition |
| `ColorGroup` `Self` | `ColorGroup` | 2 | open: not examined | 6 extra values: groups whose `Self` differs from the one the DOM version gives | `objects.md`, colour groups |
| Order of stories outside the story list | `StoryList` | 2 pairs | open: not explained | IDML lists them in another order than text order | `objects.md`, stories |

## Page items

| Item | Where | Files | Status | Content and reason | See |
|---|---|---:|---|---|---|
| `GuideColor` | chunk 0x3308 of guides | 41 pairs | open: no field found | the u32 at 22 is 6 for guides that IDML colours `Cyan`, `Blue`, `Purple`, `LightBlue` or `BrickRed`; not the layer colour, not `RulerGuidesColor` | `objects.md`, guides |
| Single-number `InsetSpacing` | chunk 0x3723 of text frames | 161 pairs | open: no field found | one number (the f64 at 0) for some frames, a list for the others; not chosen by equal insets, the style or the version | `objects.md`, text frame preferences |
| `KnockedOut` exceptions | attribute 0x1EB6F | 3 | open: not explained | 380 items that store nothing, style `false`, and no IDML value | `transparency.md`, drop shadow knockout |
| Group corner radii | `Group` | 3 | open: not explained | groups of versions 10 to 12 with only `CornerRadius` and `TopLeftCornerRadius`, although every child has all four | `attributes.md`, groups |
| `OverprintFill` of groups | `Group` | 82 pairs | open: not explained | 75 groups have a child without the attribute; not written on groups | `attributes.md`, groups |
| Near-zero crops | `FrameFittingOption` | 84 pairs | open: not explained | IDML leaves out some crops below 1e-10 and keeps others | |
| `Inverse` | chunk 0x3703 | – | open: never varies | `false` on every item | `objects.md`, text wrap |
| First 8 bytes of 0x5A35 | page item dash attribute | – | open: never varies | 0 in every item | `attributes.md`, dashes |
| `MinimumFirstBaselineOffset` | text frame settings | – | open: never varies | 0 on every frame and style; written by the category rule | `objects.md`, text frame preferences |
| Flex padding order | chunk 0x1E244 | – | open: never varies | the four paddings are equal in every style, and so are the two gaps | `objects.md`, object style settings |
| `BeforeGroupingLayerReference` | page items (21.4 on) | 12 | open: not examined | 50 values | `objects.md`, page item settings |
| Unlinked images without chunk 0x8C23 | placed graphics | 25 pairs | open: no field found | IDML has `Contents` (362 values); the bytes were not located | `objects.md`, graphics pasted without a link |
| EPS `Contents` that differ | placed graphics | – | open: not examined | 18 of 515 | `objects.md`, graphics pasted without a link |
| Anchored settings of some nested items | `AnchoredObjectSetting` | 3 | open: not examined | 388 values | `objects.md`, items inside an anchored item |
| Graphic cells, `FlexObject`, story links, media posters | page items | 20–28 pairs | open: not decoded | structures not written | |
| Export attributes of 10.0 files exported by 10.2 | `ObjectExportOption` | 4 | not needed | the attribute set follows the exporting version, which the INDD does not record | `idml-values.md`, export options |
| `CustomImageSizeOption` | `ObjectExportOption` | 224 | not needed | IDML value not in the schema; left out on purpose | `idml-values.md`, export options |

## Stroke styles and tracked changes

| Item | Where | Files | Status | Content and reason | See |
|---|---|---:|---|---|---|
| Striped styles with several stripes | class 0xB016 | 98 objects | open: no pair | the paired definition has one stripe; the IDML form of several stripes and their number format are not shown | `attributes.md`, custom stroke styles |
| Corner codes 0 to 2 | dashed style chunk 0x5A4F, attribute 0x5A35 | 2 objects | open: no pair | only code 3 (`DashesAndGaps`) occurs in a pair; code 0 in 2 dashed styles | `attributes.md`, custom stroke styles |
| Name flag byte | chunks 0xB023, 0x5A48 | 78 | open: never varies in a pair | the byte before the style name, 0 or 2 | `attributes.md`, custom stroke styles |
| Dotted stroke styles | – | 0 | open: no sample | no corpus file has one | |
| Custom stroke styles outside items and object styles | text, table and cell stroke lists | – | open: no pair | only object styles and page items store code 0x5A42 | `attributes.md`, custom stroke styles |
| Order of several custom styles | `Graphic.xml` | – | open: no pair | every paired file has one; written in UID order | `attributes.md`, custom stroke styles |
| `Change` past the end of a paragraph-level source | story | 1 pair | not needed | an export defect of the reference IDML | `objects.md`, tracked changes |
