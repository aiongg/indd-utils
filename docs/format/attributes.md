# Attribute lists

Formatting is stored as attribute lists (on page items: chunk 0x6E03).
Implemented in `src/model/attrs.rs`.

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
| 0x6E6F | `CornerOption` (code 0x5A15 = RoundedCorner) | 20 of 23 |
| 0x6E70 | `CornerRadius` | 213 of 216 |
| 0x551F | `GradientFillLength` | all non-zero (30) |
| 0x5520 | `GradientFillStart` (point) | all non-zero (30) |
| 0x5525 | `GradientStrokeLength` (assumed; equal to fill in all samples) | 30 |
| 0x5526 | `GradientStrokeStart` (assumed; equal to fill in all samples) | 30 |

## Text attribute lists

Text formatting uses the same record layout with a **u16** count. It
appears in:

- **Styles:** chunk 0x23F of a style object (u16 count, records).
- **Story runs:** each paragraph-style or character-style run record
  (chunk 0x262, see `objects.md`) continues after the style UID with a u16
  count and records. These are the run's local overrides.

Value types are specific to each attribute (for example 0x1B05 for swatch
references, 0x1B28 for point size), so the converter decodes text values
by length: 8 bytes f64, 4 bytes u32, 2 bytes u16, other lengths raw.
String values (`FontStyle`) are a flag byte followed by an in-object
string.

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
| 0x1B11 | `Capitalization` | 0 Normal, 1 SmallCaps, 2 AllCaps, 3 CapToSmallCap | 77/77 styles; codes 1 and 3 below |
| 0x1B12 | `StrokeColor` | swatch UID | 75/75 styles |
| 0x1B16 | `LeftIndent` | f64 | 22/22 ranges |
| 0x1B18 | `FirstLineIndent` | f64 | 12/12 ranges |
| 0x1B1A | `AutoLeading` | fraction ×100 | 75/75 styles |
| 0x1B1B | `Leading` (Properties) | f64; negative = Auto | 43/43 ranges, 137/137 styles |
| 0x1B1F | `Hyphenation` | 3 = true, 0 = false | 204/204 ranges |
| 0x1B25 | `HyphenationZone` | f64 | 155/155 ranges |
| 0x1B26 | `SpaceBefore` | f64 | 13/13 ranges |
| 0x1B27 | `SpaceAfter` | f64 | 42/42 ranges |
| 0x1B2A | `Underline` | 1 = true | 81/81 ranges |
| 0x1B2B | `AppliedFont` (Properties) | font family UID | 1,071/1,071 ranges, 236/247 styles |
| 0x1B2E | `MaximumWordSpacing` | fraction ×100 | 77/77 styles |
| 0x1B2F | `MinimumWordSpacing` | fraction ×100 | 77/77 styles |
| 0x1B31 | `MaximumLetterSpacing` | fraction ×100 | 78/78 styles; see below |
| 0x1B32 | `MinimumLetterSpacing` | fraction ×100 | 78/78 styles; see below |
| 0x1B37 | `StartParagraph` | 0 Anywhere, 2 NextPage | 78/78 styles; code 2 below |
| 0x1B42 | `FillTint` | f64 | 24/24 ranges |
| 0x1B4D | `RuleAboveLineWeight` | f64 | 155/155 ranges |
| 0x1B4F | `RuleAboveOffset` | f64 | 78/78 styles |
| 0x1B54 | `RuleBelowLineWeight` | f64 | 155/155 ranges |
| 0x1B55 | `RuleBelowTint` | f64 | 78/78 styles |
| 0x1B56 | `RuleBelowOffset` | f64 | 78/78 styles |
| 0x1B7E | `Justification` | 0 LeftAlign, 1 CenterAlign, 2 RightAlign, 4 LeftJustified, 5 CenterJustified | 70/70 ranges |
| 0x1B80 | `DropcapDetail` | u32 | styles only |
| 0x1B8C | `OTFContextualAlternate` | 1 = true | 220/220 ranges |
| 0x1B8D | `UnderlineColor` (Properties) | swatch UID, 0 = "Text Color" | 81/81 ranges |
| 0x1B91 | `UnderlineOffset` | f64 | 81/81 ranges |
| 0x1B94 | `UnderlineWeight` | f64 | 81/81 ranges |
| 0x1BB7 | `MiterLimit` | f64 | 161/161 ranges |
| 0x1BBF | `SplitColumnInsideGutter` | f64 | 155/155 ranges |
| 0x1BD2 | `ParagraphShadingColor` (Properties) | swatch UID | 25/25 ranges |
| 0x1BD3 | `ParagraphShadingTint` | f64 | 79/79 styles |
| 0x1A401 | `BulletsAndNumberingListType` | 0 NoList, 1 BulletList | 10/12 ranges |

**`Capitalization` codes 1 and 3.**

- Code 1 is `SmallCaps`. In one pair (InDesign 20.2, IDML from the same
  version), 8 of 8 paragraph styles with code 1 have
  `Capitalization="SmallCaps"`. The same pair confirms code 2 (4 of 4
  styles) and code 0.
- Code 3 is `CapToSmallCap`. No pair has it. The evidence is a privately
  [detail of a non-public sample removed]
  [detail of a non-public sample removed]
  [detail of a non-public sample removed]
  [detail of a non-public sample removed]
  [detail of a non-public sample removed]
  rendered pages. So code 3 sets capital letters as small capitals
  too. Of the values in the IDML schema (`Normal`, `SmallCaps`,
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

Attributes whose value never varies in the corpus (for example
`BaselineShift`, `RightIndent`) cannot be located this way and are not
written; InDesign then uses its defaults.

**Font family names.** Font families are class 0x3E03; chunk 0x3E05 holds
a flag byte, u16, then the family name as an in-object string. IDML
`Fonts.xml` names a family `di<UID hex>`. In 7 of 362 families IDML adds a
technology suffix (`Montserrat (OTF)`, `Times (TT)`) that the INDD data
does not determine; the converter writes the plain name.
