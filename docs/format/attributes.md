# Attribute lists

Formatting is stored as attribute lists (on page items: chunk 0x6E03).
Decoded in `src/model/attrs.rs`. The IDML name and value kind of each ID
are in the tables of `src/idml/attrs.rs`; `src/idml/kind.rs` converts
each kind.

## Layout

u32 count, then for each attribute:

| Size | Field |
|---|---|
| 4 | Attribute ID |
| 2 | Payload size *n* |
| *n* | Payload: u16 value count, then values |

Each value: u32 type, u16 length, data. The first value is the attribute's
value. Every attribute in the samples has a second value of type 0x6E63
with 6 bytes, meaning unknown.

| Type | Length | Value |
|---|---|---|
| 0x6E68 | 8 | f64 |
| 0x6E67 | 4 | i32 |
| 0x6E65 | 2 | u16 (enumeration or boolean) |
| 0x6E69 | 16 | two f64 (a point) |
| 0x117 | 4 | UID reference |

All 2,730 page item chunks of this layout in the samples parse exactly.

## Page item attributes

The list holds the item's local values. IDML writes an attribute only when
it differs from the applied object style, so a value can be in the INDD
list and absent from the IDML.

| ID | IDML attribute | Evidence: IDML value equals INDD value |
|---|---|---|
| 0x6E68 | `FillColor` (swatch UID) | 1,190 of 1,194 |
| 0x6E69 | `FillTint` | 10 of 10 |
| 0x6E64 | `StrokeColor` (swatch UID) | 78 of 82 |
| 0x6E65 | `StrokeWeight` | 171 of 171 |
| 0x6E6D | `MiterLimit` | 778 of 778 |
| 0x6E6F | `CornerOption`: code 0 = `None`, 0x5A15 = `RoundedCorner`, 0x5A16 = `InverseRoundedCorner` (the codes of object styles, `objects.md`) | 20 of 23 for 0x5A15; with all three codes, 1,876 of 1,995 rectangles, 504 of 520 text frames and 489 of 489 page item defaults in the trustworthy pairs |
| 0x6E70 | `CornerRadius` | 213 of 216 |
| 0x6E70, 0x6E94, 0x6E92, 0x6E93 | `TopLeftCornerRadius`, `TopRightCornerRadius`, `BottomLeftCornerRadius`, `BottomRightCornerRadius` (below) | each radius ID against its own corner 4,357, 3,200, 3,276 and 3,325 matches, no mismatch |
| 0x6E6F, 0x6E91, 0x6E8F, 0x6E90 | `TopLeftCornerOption`, `TopRightCornerOption`, `BottomLeftCornerOption`, `BottomRightCornerOption`: codes as `CornerOption` (below) | 11,211 of 11,211 |
| 0x551F | `GradientFillLength` | all non-zero (30) |
| 0x5520 | `GradientFillStart` (point) | all non-zero (30) |
| 0x5525 | `GradientStrokeLength` (assumed; equal to fill in all samples) | 30 |
| 0x5526 | `GradientStrokeStart` (assumed; equal to fill in all samples) | 30 |
| 0x6E6E | `StrokeType` (built-in stroke style code, below) | 51 of 51 items, 576 of 576 object styles |
| 0x6E8C | `StrokeAlignment`: 0 `CenterAlignment`, 1 `InsideAlignment` (below) | 1 item, 576 object styles |

**Corners.** The top-left corner uses the same IDs as `CornerRadius`
and `CornerOption`. The corner option codes are 0 `None`, 0x5A15
`RoundedCorner`, 0x5A16 `InverseRoundedCorner`, 0x5A17 `InsetCorner`,
0x5A18 `BevelCorner` and 0x5A19 `FancyCorner`; all six occur. The
evidence is over the items of the trustworthy pairs whose corners
differ: each radius ID matches its own corner with no mismatch, and has
16 to 71 mismatches against each other corner. The same IDs hold the
corners of object styles (chunk 0x1B92B) and of the page item defaults
(class 0x6E07): all four radii and options of `PageItemDefault` match in
495 of 495 documents.

Transparency attributes (IDs 0x108xx and 0x1EBxx) are described in
`transparency.md`.

### Strokes

Object styles (class 0x1B901) hold a full page item attribute list in
chunk 0x1B92B, with the same record layout as chunk 0x6E03 but a u16
count. The converter reads them (`object style` lists in `indd audit`)
and writes them with the page item table. They also add evidence: the
corpus pairs have 576 object styles whose name matches an IDML
`ObjectStyle`.

**Stroke type (0x6E6E).** The record has three values: a reference (type
0x117), a code (type 0x6E64, 4 bytes) and the usual 6-byte value. The
reference is 0 in every sample and the code names a built-in stroke
style. IDML writes `StrokeType` as `StrokeStyle/$ID/<name>`.

| Code | IDML stroke style | Evidence |
|---|---|---|
| 0x5A29 | `Solid` | 574 object styles in 135 files |
| 0x5A38 | `Canned Dashed 3x2` | 2 items in 1 file |
| 0x5A39 | `Canned Dotted` | 46 items in 13 files |
| 0xB004 | `ThinThin` | 2 object styles in 1 file, and 2 items in the same file that have no IDML `StrokeType` and use one of those styles |
| 0xB01A | `Triple_Stroke` | 1 item |

No item or style has the same code with a different IDML value. The
converter writes `StrokeType` for these codes when the reference is 0.
Custom stroke styles (a non-zero reference) do not occur in the corpus;
every corpus IDML lists only the 18 built-in stroke styles.

**Stroke alignment (0x6E8C).** All 576 object styles have 0 and
`StrokeAlignment="CenterAlignment"`. One text frame in the pairs has 1
and `InsideAlignment`. Its INDD list has four 0x6E attributes, and its
IDML differs from its object style in four stroke attributes:
0x6E64 `StrokeColor`, 0x6E65 `StrokeWeight` and 0x6E6E `StrokeType` are
identified above, which leaves 0x6E8C for `StrokeAlignment`. The code for
`OutsideAlignment` is not known; the converter leaves out other codes.

**Not identified.** These attributes have one value in every object style
and in every item whose IDML writes them, so the field cannot be told
apart from others with the same value: `StrokeTint` (-1),
`GapColor` (`Swatch/None`), `GapTint` (-1), `EndCap` (`ButtEndCap`),
`EndJoin` (`MiterEndJoin`), `LeftArrowHeadScale` and
`RightArrowHeadScale` (100), `ArrowHeadAlignment` (`InsidePath`). Items
with another `StrokeTint` (20 in one pair) have no INDD object with the
same UID. `StrokeCornerAdjustment` and `StrokeDashAndGap` do not occur in
the corpus IDML files.

**Line ends.** One line in the pairs has
`LeftLineEnd="CurvedArrowHead"` and `RightLineEnd="BarArrowHead"`, and
its INDD list has 0x6E71 = 0x5A08 and 0x6E72 = 0x5A0D (both 0 in all 576
object styles, whose line ends are `None`). Either attribute could be
either ID, so line ends are not converted.

## Text attribute lists

Text formatting uses the same record layout with a **u16** count. It
appears in:

- **Styles:** chunk 0x23F of a style object (u16 count, records).
- **Story runs:** each paragraph-style or character-style run record
  (chunk 0x262, see `objects.md`) continues after the style UID with a u16
  count and records. These are the run's local overrides.

Value types are specific to each attribute (for example 0x1B05 for swatch
references, 0x1B28 for point size), so the converter decodes a text value
by the layout of its attribute. The attributes whose value is not a
single number have their layout listed in `text_layout`
(`src/model/attrs.rs`): string values (`FontStyle`) are a flag byte
followed by an in-object string; tab lists, nested styles, bullet
characters, points, ruby text and cell edge stroke types are described
below. Other values are numbers, decoded by length: 8 bytes f64, 4 bytes
u32, 2 bytes u16. A value that does not fit its attribute's layout is
kept undecoded and not written.

### Mapping and evidence

Found by aligning IDML `ParagraphStyleRange` and `CharacterStyleRange`
elements with INDD runs at the same text offset (1,562 ranges), and IDML
styles with INDD styles of the same name (486 styles), then checked with
`tools/compare.py`. Counts are matches/total in the current comparison
(ranges + paragraph styles + character styles where present).

| ID | IDML attribute | Encoding | Evidence |
|---|---|---|---|
| 0x1B01 | `FillColor` | swatch UID | 354/354 ranges, 226/226 styles |
| 0x1B02 | `FontStyle` | flag + string | 474/474 ranges, 202/202 styles |
| 0x1B03 | `PointSize` | f64 | 1,090/1,090 ranges |
| 0x1B06 | `HorizontalScale` | fraction ×100 | 2/2 ranges, 75/75 styles |
| 0x1B07 | `KerningMethod` | code: 15972 Metrics, 79875 Optical | 20/20 ranges, 81/81 styles |
| 0x1B08 | `Ligatures` | 1 = true | 201/201 ranges |
| 0x1B0A | `StrokeWeight` | f64 | 241/241 ranges |
| 0x1B0B | `Tracking` | thousandths of an em ×1000 | 49/49 ranges, 94/94 styles |
| 0x1B0C | `Composer` | code: 0x2001 HL Single, 0x2002 HL Composer, 0x2078 HL Composer Optyca | 4/4 ranges, 114/114 styles |
| 0x1B0D | `DropCapCharacters` | u16 | 79/79 styles; see below |
| 0x1B0E | `DropCapLines` | u16 | 79/79 styles; see below |
| 0x1B10 | `BaselineShift` | f64 | 79/79 styles; see below |
| 0x1B11 | `Capitalization` | 0 Normal, 1 SmallCaps, 2 AllCaps, 3 CapToSmallCap | 77/77 styles; codes 1 and 3 below |
| 0x1B12 | `StrokeColor` | swatch UID | 75/75 styles |
| 0x1B13 | `KerningValue` | ems ×1000; 1e8 = none | 8/8 ranges; scale from a sample and its print PDF, see below |
| 0x1B15 | `VerticalScale` | fraction ×100 | 79/79 styles; see below |
| 0x1B16 | `LeftIndent` | f64 | 22/22 ranges |
| 0x1B17 | `RightIndent` | f64 | 86/86 styles |
| 0x1B18 | `FirstLineIndent` | f64 | 12/12 ranges |
| 0x1B1A | `AutoLeading` | fraction ×100 | 75/75 styles |
| 0x1B1B | `Leading` (Properties) | f64; negative = Auto | 43/43 ranges, 137/137 styles |
| 0x1B1D | `AppliedLanguage` | language UID; see below | 88/88 styles |
| 0x1B1F | `Hyphenation` | 3 = true, 0 = false | 204/204 ranges |
| 0x1B24 | `NoBreak` | 1 = true | 78/78 styles |
| 0x1B25 | `HyphenationZone` | f64 | 155/155 ranges |
| 0x1B26 | `SpaceBefore` | f64 | 13/13 ranges |
| 0x1B27 | `SpaceAfter` | f64 | 42/42 ranges |
| 0x1B29 | `TabList` (Properties) | list; see below | 110/110 styles |
| 0x1B2A | `Underline` | 1 = true | 81/81 ranges |
| 0x1B2B | `AppliedFont` (Properties) | font family UID | 1,071/1,071 ranges, 236/247 styles |
| 0x1B2C | `OTFFigureStyle` | 1 ProportionalOldstyle, 2 ProportionalLining, 4 Default | 4/4 ranges, 138/138 styles |
| 0x1B2E | `MaximumWordSpacing` | fraction ×100 | 77/77 styles |
| 0x1B2F | `MinimumWordSpacing` | fraction ×100 | 77/77 styles |
| 0x1B31 | `MaximumLetterSpacing` | fraction ×100 | 78/78 styles; see below |
| 0x1B32 | `MinimumLetterSpacing` | fraction ×100 | 78/78 styles; see below |
| 0x1B37 | `StartParagraph` | 0 Anywhere, 2 NextPage | 78/78 styles; code 2 below |
| 0x1B3C | `Position` | 0 Normal, 5 OTNumerator | 20/20 ranges, 78/78 styles |
| 0x1B40 | `KeepLinesTogether` | 1 = true | 78/78 styles; see below |
| 0x1B42 | `FillTint` | f64 | 24/24 ranges |
| 0x1B46 | `GradientFillAngle` | f64 | 81/81 styles |
| 0x1B48 | `GradientFillLength` | f64 | 2/2 ranges, 81/81 styles |
| 0x1B4A | `GradientFillStart` | two f64 | 2/2 ranges, 81/81 styles |
| 0x1B4D | `RuleAboveLineWeight` | f64 | 155/155 ranges |
| 0x1B4F | `RuleAboveOffset` | f64 | 78/78 styles |
| 0x1B50 | `RuleAboveLeftIndent` | f64 | 91/91 styles |
| 0x1B51 | `RuleAboveRightIndent` | f64 | 91/91 styles |
| 0x1B52 | `RuleAboveWidth` | 1 ColumnWidth, 2 TextWidth | 91/91 styles |
| 0x1B53 | `RuleBelowColor` (Properties) | swatch UID, 0 = "Text Color" | 6/6 ranges, 114/114 styles |
| 0x1B54 | `RuleBelowLineWeight` | f64 | 155/155 ranges |
| 0x1B55 | `RuleBelowTint` | f64 | 78/78 styles |
| 0x1B56 | `RuleBelowOffset` | f64 | 78/78 styles |
| 0x1B5D | `RuleBelow` | 1 = true | 83/83 styles |
| 0x1B6A | `ParagraphBreakType` | 0 Anywhere, 1 NextColumn | 11/11 ranges, 78/78 styles |
| 0x1B6B | `SingleWordJustification` | 0 LeftAlign, 3 FullyJustified | 80/80 styles |
| 0x1B75 | `AllNestedStyles` (Properties) | list; see below | 14/14 styles |
| 0x1B7E | `Justification` | 0 LeftAlign, 1 CenterAlign, 2 RightAlign, 4 LeftJustified, 5 CenterJustified | 70/70 ranges |
| 0x1B80 | `DropcapDetail` | u32 | 5/5 ranges, 103/103 styles |
| 0x1B8C | `OTFContextualAlternate` | 1 = true | 220/220 ranges |
| 0x1B8D | `UnderlineColor` (Properties) | swatch UID, 0 = "Text Color" | 81/81 ranges |
| 0x1B91 | `UnderlineOffset` | f64 | 81/81 ranges |
| 0x1B94 | `UnderlineWeight` | f64 | 81/81 ranges |
| 0x1BB7 | `MiterLimit` | f64 | 161/161 ranges |
| 0x1BB9 | `EndJoin` | 0 MiterEndJoin, 1 RoundEndJoin | 86/86 styles |
| 0x1BBD | `SpanColumnType` | 0 SingleColumn, 1 SpanColumns | 41/41 ranges, 94/94 styles |
| 0x1BBE | `SpanSplitColumnCount` (Properties) | u16; 1 = All | 12/12 ranges, 78/78 styles |
| 0x1BBF | `SplitColumnInsideGutter` | f64 | 155/155 ranges |
| 0x1BC4 | `SpanColumnMinSpaceAfter` | f64 | 84/84 styles |
| 0x1BD2 | `ParagraphShadingColor` (Properties) | swatch UID | 25/25 ranges |
| 0x1BD3 | `ParagraphShadingTint` | f64 | 79/79 styles |
| 0x1BD6 | `ParagraphShadingOn` | 1 = true | 88/88 styles |
| 0x1BDB | `ParagraphShadingTopOffset` | f64 | 91/91 styles |
| 0x1BDC | `ParagraphShadingBottomOffset` | f64 | 90/90 styles |
| 0x1BF6 | `ParagraphBorderColor` (Properties) | swatch UID | 72/72 styles |
| 0x1BF9 | `ParagraphBorderOn` | 1 = true | 77/77 styles |
| 0x1DF03 | `ParagraphBorderTopOffset` | f64 | 83/83 styles |
| 0x1DF04 | `ParagraphBorderBottomOffset` | f64 | 83/83 styles |
| 0x1DF21 | `SameParaStyleSpacing` (Properties) | f64; −1 = SetIgnore | 15/15 ranges, 100/100 styles |
| 0x4265 | `GridAlignFirstLineOnly` | 1 = true | 81/81 styles |
| 0x4266 | `GridAlignment` | 0 None, 1 AlignBaseline | 86/86 styles |
| 0x425E | `Tatechuyoko` | 1 = true | root styles; 1 from a sample typeset vertically and its print PDF, see below |
| 0x4279 | `ShataiDegreeAngle` | degrees, written ×100 | root styles; see below |
| 0x427A | `ShataiAdjustTsume` | 1 = true | root styles; see below |
| 0x427B | `ShataiAdjustRotation` | 1 = true | root styles and a sample typeset vertically; see below |
| 0x422D | `RubyFlag` | number; written when not 0 | a sample and its print PDF; see below |
| 0x422E | `RubyString` | u32 length, text segments; written when not empty | a sample and its print PDF; see below |
| 0x1A401 | `BulletsAndNumberingListType` | 0 NoList, 1 BulletList | 10/12 ranges |
| 0x1A406 | `BulletChar` (Properties) | u32 type, u32 value; see below | 1/1 ranges, 84/84 styles |
| 0x1A413 | `BulletsFont` (Properties) | font family UID, 0 = `$ID/` | 1/1 ranges, 84/84 styles |
| 0x1A414 | `BulletsFontStyle` (Properties) | flag + string; empty = `Nothing` | 1/1 ranges, 84/84 styles |
| 0x1A419 | `NumberingContinue` | 1 = true | 2/2 ranges, 78/78 styles |
| 0x1A41F | `BulletsCharacterStyle` (Properties) | character style UID | 84/84 styles |
| 0x1A420 | `NumberingCharacterStyle` (Properties) | character style UID | 79/79 styles |
| 0x1A423 | `NumberingExpression` | flag + string | 79/79 styles |
| 0x4221 | `Mojikumi` (Properties) | table UID; see below | 936/1,019 styles |
| 0x4224 | `KinsokuSet` (Properties) | table UID; see below | 987/987 styles |
| 0x42C0 | `TreatIdeographicSpaceAsSpace` | 1 = true | 1,082/1,083 styles |
| 0x50F18 | `DiacriticPosition` | 4 OpentypePosition, 5 OpentypePositionFromBaseline | 1,007/1,010 styles |

The counts of these four are paragraph and character styles of all 654
pairs whose IDML style has the attribute and the same name (the root
paragraph style of every pair among them); they also reproduce the
`TextDefault` values of the preferences (`preferences.md`). The kinsoku
and mojikumi value is a UID: 0 is `Nothing`; a built-in table
(`kinsoku and mojikumi tables` in `objects.md`) is written as an
enumeration: `kHardKinsokuName` `HardKinsoku` (51 styles),
`kSoftKinsokuName` `SoftKinsoku` (11), `kKoreanKinsokuName`
`KoreanKinsoku` (12), `kSimpChineseKinsokuName`
`SimplifiedChineseKinsoku` (1), `kMojikumiDefaultName1`
`LineEndAllOneHalfEmEnum` (55) and `kMojikumiDefaultName16`
`SimpChineseDefault` (1); a custom kinsoku table (class 0x4204) as the
object `KinsokuTable/<name>` (38). Other built-in tables are not written.
81 styles name a custom mojikumi table of class 0x4203, which the
converter does not write, so their `Mojikumi` is left out.

Paragraph style counts include the 78 root styles `[No paragraph style]`
of the compared pairs. For many attributes above the root holds the only
value that is the same in every pair (for example `RightIndent="0"`), so
the evidence that tells the attribute apart is in the other styles:

| Attribute | Other styles or ranges with the attribute |
|---|---|
| `BaselineShift` | 1 style with 0; 1 range with −16 in a story that `compare.py` does not align |
| `RightIndent` | 8 styles in one pair, two different values |
| `AppliedLanguage` | 10 styles in 2 pairs |
| `NoBreak` | 1 style with 1 (true), in a pair whose IDML is from an older version |
| `TabList` | 33 styles in 5 pairs, left and right stops, with and without leaders |
| `OTFFigureStyle` | 60 styles in one pair (codes 1 and 2), 4 ranges in another (code 4) |
| `Position` | 20 ranges in one pair |
| `GradientFill*` | 3 styles in one pair (angle −90, distinct length and start), 2 ranges in another |
| `RuleAbove*Indent`, `RuleAboveWidth` | 13 styles in one pair (left −1.13, right −2.83) |
| `RuleBelowColor` | 36 styles in 2 pairs, four colours |
| `RuleBelow` | 5 styles in one pair (4 true, 1 false) |
| `ParagraphBreakType` | 19 ranges in one pair |
| `SingleWordJustification` | 2 styles in one pair with code 0 |
| `EndJoin` | 8 styles in one pair |
| `SpanColumnType` | 16 styles in 2 pairs, 46 ranges in one pair, both codes |
| `SpanSplitColumnCount` | 13 ranges in one pair with 2 |
| `SpanColumnMinSpaceAfter` | 6 styles in one pair |
| `ParagraphShading*`, `ParagraphBorder*` | 2 to 14 styles in one pair, several different offsets |
| `SameParaStyleSpacing` | 30 styles and 16 ranges, each in 2 pairs |
| `GridAlignment`, `GridAlignFirstLineOnly` | 8 and 3 styles in 3 pairs |
| `BulletChar`, `BulletsFont`, `BulletsFontStyle`, `BulletsCharacterStyle` | 6 styles in 2 pairs, 1 range in another |
| `NumberingExpression`, `NumberingCharacterStyle` | 1 style |
| `NumberingContinue` | 2 ranges in 2 pairs |
| `VerticalScale` | 1 style; see below |

For codes, the root style gives the default: for example every root style
has 0x1B6B = 3 and IDML `SingleWordJustification="FullyJustified"`, and
0x1BBE = 1 with `SpanSplitColumnCount` `All`.

**`VerticalScale` (0x1B15).** The value is 1 (100%) wherever it appears,
like `HorizontalScale` (0x1B06), so values do not tell them apart. Over
all pairs, including those with an IDML from another version, 0x1B15 is
present in exactly the 5 styles where IDML writes `VerticalScale`; 0x1B06
is identified separately by values other than 100%.

**Languages (0x1B1D).** The value is the UID of an object of class
0x2D07. Its chunk 0x2D0F is a flag byte, then the language name as an
in-object string (`English: USA`), followed by other strings. IDML writes
`$ID/` and that name. Over all pairs, all 62 styles (root styles
included) and 4 ranges with the attribute match, with 5 different
languages.

**Tab stops (0x1B29).** u16 count, then for each stop: f64 position, u16
alignment (0 LeftAlign, 2 RightAlign; no other code occurs), u16 leader
length and the leader in UTF-16 code units. IDML writes each stop as a
`ListItem` with `Alignment`, `AlignmentCharacter`, `Leader` and
`Position`. `AlignmentCharacter` is `.` in every IDML tab stop, and the
INDD record has no field for it, so the converter writes `.`. An empty
list is a count of 0. The converter leaves out a list with another
alignment code.

**Nested styles (0x1B75).** u32 count, then for each item: u32 character
style UID, u32 length, and a delimiter code of that many characters
stored as text segments (see `objects.md`). The code gives `Delimiter`,
`Inclusive` and `Repetition`:

| Code | IDML | Evidence |
|---|---|---|
| `[c]` | `Delimiter` string *c*, `Inclusive="false"` | 13 items in one pair (`.` and `:`) |
| `(c)` | `Delimiter` string *c*, `Inclusive="true"` | 1 item in the same pair |
| `^c` | `Dropcap`, `Inclusive="true"` | 1 item in the same pair |
| digits after `)` or `]` | `Repetition`; no digits = 1 | a sample and its print PDF |
| `(^w)` | `AnyWord` | a sample and its print PDF |
| `(^?)` | `AnyCharacter` | a sample and its print PDF |

The public pair has only single-character literals and repetition 1. The
last three rows rest on a sample and its print PDF:

- A sample has nested items with `(^?)` and with `(^w)` followed by a
  number, the latter applying a character style with Capitalization
  code 3 (`CapToSmallCap`, see below).
- In the PDF, capitals in the first words of paragraphs in those styles
  are set at small-cap height (ink height measured on the rendered
  page), and capitals later in the same lines at full height. Small
  capitals reach at most one word past the number, and capitals a few
  words further on are at full height, so the number is the repetition.
  Of the counting delimiters in the IDML schema, only `AnyWord` gives a stretch of several words: `AnyCharacter`,
  `Letters` and `Digits` would end within the first word, and `Sentence`
  would run past sentence ends, which the PDF does not show.
- A paragraph-level override has `(^?)` followed by a number *n*, then
  `(^w)`. In the PDF the first capital after the first *n* characters of
  that paragraph is a small cap. With a word delimiter, the first *n*
  words would keep full capitals, and that capital lies within them. So
  `^?` counts characters: `AnyCharacter`.

The converter leaves out a list that has any other code.

**Bullet character (0x1A406).** Two u32: the character type (0
`UnicodeOnly`, 1 `UnicodeWithFont`, 2 `GlyphWithFont`) and the character
value. IDML writes an empty `BulletChar` element with
`BulletCharacterType` and `BulletCharacterValue` attributes. In the
pairs, type 0 occurs in the root styles with value 8226 (•), type 1 in
one range and type 2 in 6 styles, each with its value and a
`BulletsFont`.

**`KeepLinesTogether` (0x1B40).** In every pair the root style has 0, and
IDML writes `KeepLinesTogether="false"`, `KeepAllLinesTogether="false"`,
`KeepFirstLines="2"` and `KeepLastLines="2"`. No public style sets
another value. The rest of the evidence is a sample and its print PDF:

- Some paragraph styles of the sample set 0x1B40 to 1. Apart from
  attributes identified above, they set nothing else.
- In the PDF, paragraphs in those styles that break across pages always
  leave at least two lines on each page. A paragraph of *n* lines broken
  at a random line leaves a single line on one side with probability
  2/(*n* − 1), so this pattern does not come from chance.
- Such paragraphs do break across pages, so the attribute is not
  `KeepAllLinesTogether`. A boolean that keeps at least two lines
  together at both ends, with the defaults above, is `KeepLinesTogether`.

**`Capitalization` codes 1 and 3.**

- Code 1 is `SmallCaps`. In one pair (InDesign 20.2, IDML from the same
  version), 8 of 8 paragraph styles with code 1 have
  `Capitalization="SmallCaps"`. The same pair confirms code 2 (4 of 4
  styles) and code 0.
- Code 3 is `CapToSmallCap`. No pair has it. The evidence is a sample
  and its print PDF. In the PDF, words set in a style with code 3 that
  start with a capital letter show that capital at the height of the
  letter after it, about two thirds of the height of word-initial
  capitals in ordinary text. Heights were measured on the rendered
  pages. So code 3 sets capital letters as small capitals too. Of the values in the IDML schema (`Normal`, `SmallCaps`,
  `AllCaps`, `CapToSmallCap`, `LowerCase`), only `CapToSmallCap` does
  that.

The converter leaves out other codes.

**Drop caps (0x1B0D, 0x1B0E).** In every pair, the root style
`[No paragraph style]` has 0 for both and IDML writes
`DropCapCharacters="0"` and `DropCapLines="0"`. One pair (InDesign 20.2,
IDML from the same version) also has a style with 0x1B0D = 1 and
0x1B0E = 3; its IDML has `DropCapCharacters="1"` and `DropCapLines="3"`.
The different values tell the two attributes apart.

**Letter spacing (0x1B31, 0x1B32).** In every pair both are 0 in the root
style, as are `MaximumLetterSpacing` and `MinimumLetterSpacing` in the
IDML. That does not tell them apart from other attributes that are 0
there. The rest of the evidence is a sample and its print PDF:

- A paragraph style of the sample sets 0x1B31 to a positive fraction
  and 0x1B32 to a negative one. The style sets no tracking, and its
  other unidentified attributes are not fractions.
- In the PDF, justified lines in that style have a character spacing
  (`Tc` operator) that changes from line to line, both positive and
  negative. So the style lets letter spacing vary in both directions,
  which needs a negative minimum and a positive maximum.
- The values are stored as fractions, like word spacing (0x1B2E,
  0x1B2F), so they are written ×100. The positive one is the maximum.

A layout of the converted sample with these two attributes reproduces
noticeably more of the PDF's line breaks than one without them.

**Ruby (0x422D, 0x422E).** IDML writes neither attribute on the root
style, and no corpus story has them. In every pair the root style stores
0 for 0x422D and a 4-byte 0 for 0x422E. The evidence is a sample typeset
vertically and its print PDF:

- Character runs of the sample set 0x422D to 1 together with 0x422E,
  whose value is a u32 length in characters followed by text segments
  (as story text, `objects.md`). The text is a short reading of the
  run's characters.
- In the PDF, each such text appears in small type in a narrow column
  right beside the run's characters and centred on them, as ruby is set
  beside vertical text.
- The converter decodes the value of 0x422E by this layout, whatever its
  length.

The converter writes `RubyString` and writes `RubyFlag` with the stored
number (1 in every such run; the schema type is an integer). The other
ruby settings (`RubyType`, `RubyAlignment`, `RubyPosition`, font and
size) are not identified.

**`Tatechuyoko` (0x425E).** Every public style that stores it has 0,
and IDML writes `Tatechuyoko="false"` on all 240 root paragraph styles
of the corpus IDML files; no public range has the attribute. The
evidence for 1 is a sample typeset vertically and its print PDF:

- Runs of a few Latin characters (letters or punctuation) set 0x425E
  to 1. Elsewhere in the same vertical stories, Latin text
  without it is drawn turned sideways: the PDF text matrix is a quarter
  turn, and the letters follow each other down the column.
- The glyphs of each run with 1 are drawn upright (identity text
  matrix), side by side on one baseline, and the group is centred on the
  column's centre line to 0.01 pt. That is horizontal-in-vertical
  setting.

**Shatai (0x4278 to 0x427B).** In every public file the root paragraph
style stores 0x4278 = 0, 0x4279 = 45, 0x427A = 1 and 0x427B = 0, and
IDML writes `ShataiMagnification="0"`, `ShataiDegreeAngle="4500"`,
`ShataiAdjustTsume="true"` and `ShataiAdjustRotation="false"` on all 240
root styles; no other public style or range sets them. Only 45 and 4500
are not 0 or a boolean, so 0x4279 is the angle, written ×100. The rest
of the evidence is a sample typeset vertically and its print PDF:

- Runs set 0x4278 = 20, 0x4279 = 60 and 0x427A = 0. Their glyphs have
  the text matrix (0.850, 0.0866, 0.0866, 0.950) × point size. That
  matrix scales by 1 along the direction at 60° and by 0.80 across it:
  a distortion of 20 % at 60°. So 0x4278 is the magnification in
  percent and 0x4279 the angle in degrees.
- A character style sets 0x4278 = 10, 0x4279 = 60 and 0x427B = 1. Its
  glyphs have the matrix (0.9222, 0.0843, 0, 0.9759) × point size. This
  is the 10 % distortion at 60°, turned so that the glyph's vertical
  axis stays vertical (the third value is 0). The runs above, with
  0x427B = 0, are not turned. So 0x427B is `ShataiAdjustRotation`.
- That leaves 0x427A for `ShataiAdjustTsume`, which also matches the root
  values (1 and `true`; 0x427B 0 and `false`). With 0 the glyphs keep
  an advance of one em; no sample has 1 outside root styles.

The converter writes the angle and the two booleans. It leaves out
`ShataiMagnification`: the sample stores the percentage, but the root
value 0 does not show the unit IDML uses, and the angle shows that it
need not be the stored number.

**`KerningValue` (0x1B13).** Every root paragraph style stores 1e8, and
IDML writes no `KerningValue` on root styles; the converter leaves 1e8
out. In one pair, 8 ranges store 0 and IDML writes `KerningValue="0"`
on the same 8 ranges; no other range in the pairs has the attribute. So
the field is located, but 0 does not show its scale. The scale rests on
a sample and its print PDF:

- Runs of one character set 0x1B13 to −0.1, −0.06, −0.12 and 0. In the
  PDF, the gap between that character and the next (start of the next
  glyph minus the end of the first glyph's advance width) is −0.1,
  −0.06, −0.12 and 0 times the point size, to 0.01 pt, once the letter
  spacing that every other gap in the same line shows is subtracted.
- The same letter pairs elsewhere, without the attribute, have other
  gaps, which come from the font's kerning. So the stored value replaces
  the font's kerning after that character.
- The value is in ems. The IDML schema types `KerningValue` as a plain
  number. `Tracking` (0x1B0B) is also stored in ems, and IDML writes it
  ×1000 (thousandths of an em, from the pairs); the converter writes
  `KerningValue` the same way. No sample shows this scale directly.
- The schema allows `KerningValue` on character styles and ranges, not
  on paragraph styles, so the converter leaves it out of paragraph
  styles (public samples without IDML have it there).

**`StartParagraph` (0x1B37).** In every pair the root style has 0 and
IDML writes `StartParagraph="Anywhere"`; no pair has another value. Code
2 rests on a sample and its print PDF:

- Every paragraph whose style has code 2 is the first text on its page
  in the PDF, and the page before it often ends with room for more
  lines. Those styles set no other attribute that is unidentified.
- Such paragraphs start on both odd and even pages, so code 2 is not
  `NextOddPage` or `NextEvenPage`.
- In the sample, `NextColumn`, `NextFrame` and `NextPage` would give the
  same layout, so the PDF does not tell them apart. The converter writes
  `NextPage`.

One public template without an IDML has a style with code 4; with no
reference, the converter leaves out codes other than 0 and 2.

### Paragraph borders and shading: corners and origins

| ID | IDML | Styles with both (other than the root) |
|---|---|---|
| 0x1DF12 | `ParagraphShadingTopLeftCornerRadius` | 452 (126) |
| 0x1DF14 | `ParagraphShadingBottomLeftCornerRadius` | 452 (126) |

On every style other than the root the ID is present exactly when the
IDML style has the attribute, and every value matches. The two are told
apart by 8 styles (4.5 against 0).

**Tied groups.** The IDs of each group below have the same value in
every style, so which ID belongs to which attribute is not known. The
converter writes a group only when all its IDs are present and equal:

| IDs | IDML | Styles |
|---|---|---|
| 0x1DF0A–0x1DF0D | the four `ParagraphBorder…CornerRadius` | 446 (120) |
| 0x1DF13, 0x1DF15 | `ParagraphShadingTopRightCornerRadius`, `ParagraphShadingBottomRightCornerRadius` | 444 (118) |
| 0x1DF0E, 0x1DF10 | `ParagraphShadingTopLeftCornerOption`, `ParagraphShadingBottomLeftCornerOption` | 339 (13) |
| 0x1DF0F, 0x1DF11 | `ParagraphShadingTopRightCornerOption`, `ParagraphShadingBottomRightCornerOption` | 331 (5) |

The shading corner option codes are 0 `None`, 0x5A15 `RoundedCorner`
and 0x5A18 `BevelCorner`.

**Origins.** The two IDs of each pair always change together; the
converter writes a pair only for these combinations of codes:

| IDs | IDML | Codes → values | Styles |
|---|---|---|---|
| 0x1BDD, 0x1BDE | `ParagraphShadingTopOrigin`, `ParagraphShadingBottomOrigin` | 0, 0 → `AscentTopOrigin`, `DescentBottomOrigin`; 3, 2 → `EmBoxTopOrigin`, `EmBoxBottomOrigin`; 1, 1 → `BaselineTopOrigin`, `BaselineBottomOrigin` | 378, 14, 4 |
| 0x1DF18, 0x1DF19 | `ParagraphBorderTopOrigin`, `ParagraphBorderBottomOrigin` | 0, 0 → `AscentTopOrigin`, `DescentBottomOrigin`; 3, 2 → `EmBoxTopOrigin`, `EmBoxBottomOrigin` | 325, 13 |

Attributes whose value never varies in the corpus cannot be located this
way. The converter writes the root styles' values for them from IDML
observation (`idml-values.md`); styles and ranges inherit them. These include
`KeepFirstLines`, `KeepLastLines`, `KeepAllLinesTogether`,
`KeepWithNext`, `KeepWithPrevious`, `Skew`, `StrikeThru`,
`LastLineIndent`, `RuleAbove`, `RuleAboveColor`, the `RuleBelow` indents
and width, and the hyphenation and word-spacing settings other than those
in the table. Some attributes vary but cannot be told apart from another
attribute, because both always have the same value or appear in the same
styles:

- left and right shading offsets, left and right border offsets, and the
  four border line weights (equal in every sample);
- `KeepRuleAboveInFrame` and `RuleAboveType` (always set together, with
  other attributes, in one pair);
- `TreatIdeographicSpaceAsSpace` and `DiacriticPosition`.

`OTFFigureStyle` code 3 and `ParagraphBreakType` code 2 occur only in
a sample without IDML; the converter leaves them out.

**Font family names.** `AppliedFont` and `BulletsFont` name a font family
(class 0x3E03), and IDML writes the family's name. See `fonts.md`.
