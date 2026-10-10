# Footnotes and endnotes

Footnotes, endnotes and their document options. Implemented in
`src/model/story.rs` (text), `src/model/prefs.rs` (options) and
`src/idml/story.rs`, `designmap.rs`, `resources.rs` (writing).

| Class | Object | IDML element |
|---|---|---|
| 0x24F | Footnote | `Footnote` (story text) |
| 0x2801 | Endnote story | `Story` with `IsEndnoteStory="true"` |
| 0x2805 | Endnote | `Endnote` (story text) |
| 0x2804 | Endnote range | `EndnoteRange` (endnote story text) |

Counts are over the little-endian corpus without the privately held
samples, unless they name the trustworthy pairs (`measurement.md`).

## Footnotes

**Reference.** A footnote reference is a U+0004 character in story or
table cell text. The owned-items strand (run kind 0x209, `objects.md`)
lists an object of class 0x24F, the footnote, at that position. 14,026
footnotes in 391 files, InDesign 5 to 21, are all at a U+0004: 14,002 in
story text and 24 in table cell text. In the pairs, the 819 `Footnote`
elements of the IDML are in place of exactly these characters; the count
per story matches in 47 of 47 stories.

**Text.** The footnote's text is in the same story text, after the
story's own text, as table cell text is (`tables.md`). The text-owner
strand (run kind 0x2A4) gives it the footnote UID as owner and 1 as the
cell field. In all 14,026 footnotes the text is one stretch, its cell
field is 1, and it ends with U+000D. 14,001 footnote texts contain one
U+0004, the footnote number, which IDML writes as `<?ACE 4?>`; the other
25 have none. 261 footnote texts contain an anchored object (U+FFFC) and
1 contains a table.

**IDML.** IDML writes a `Footnote` element, without `Self` or attributes,
in place of the reference character, inside the `CharacterStyleRange` of
that position. The `Footnote` holds `ParagraphStyleRange` elements built
from the footnote text, like a table cell; the final U+000D is not
written. Evidence: the footnote texts of a story in the INDD equal the
IDML footnote texts of that story in 819 of 819 footnotes (47 stories, 46
pairs), compared as multisets with U+0004, U+0018, U+FEFF and U+FFFC left
out; the sequence of paragraph styles matches in 498 of 499 footnotes of
the trustworthy pairs. Inside footnotes IDML has hyperlink sources (182
in a character range, 315 in a paragraph range), page references (33)
and cross-reference sources (25); the converter places sources in
footnotes by the rules of `hyperlinks.md`.

A U+0004 without a footnote or endnote object stays `<?ACE 4?>`, as in
IDML: footnote numbers inside footnote and endnote text, and one pasted
number in a 17.4 pair.

With this, 8 more stories of the trustworthy pairs have identical text,
and their text ranges are compared.

## Footnote options (preferences chunk 0x2820)

Chunk 0x2820 of the preferences object (class 0x2202) holds the
document's `FootnoteOption`. It is 231 bytes in InDesign 7 to 11 and 233
bytes from 12 on, longer when the strings are longer, and missing in 90
of the 733 pairs collected. 643 files have the chunk and a
`FootnoteOption` in their IDML, 442 of them trustworthy pairs; every
attribute decoded below matches in 442 of 442 trustworthy pairs. Over
all 643 files there are 4 mismatches, all in stale pairs
(`EnableStraddling` in 3 files from 12.0, and a `RuleColor`).

| Offset | Type | IDML |
|---|---|---|
| 0 | u32 character style UID | `FootnoteMarkerStyle` |
| 4 | u32 paragraph style UID | `FootnoteTextStyle` |
| 8 | u32 code | `FootnoteNumberingStyle`: 0xCA07 `Arabic` (641), 0xCA0A `Symbols` (1), 0xCA58 `Asterisks` (1) |
| 12 | u32 | `StartAt` |
| 16 | u16 | `RestartNumbering`: 0 `DontRestart` (641), 1 `PageRestart` (2) |
| 18, 22 | u16, u16 | `MarkerPositioning`: (1, 0) `SuperscriptMarker` (635), (0, 0) `NormalMarker` (4), (0, 1) `RubyMarker` (4) |
| 20 | u16 | 0 in all |
| 24 | f64 | `SpaceBetween` |
| 32 | f64 | `Spacer` |
| 40 | u16 | `EosPlacement` (1 `true`): 21 files `true`, 1,364 `false`, all matching (corpus of 2026-10) |
| 42 | u16 | `ShowPrefixSuffix`: 0 `NoPrefixSuffix` (640), 1 `PrefixSuffixReference` (1), 3 `PrefixSuffixBoth` (2) |
| 44 | string | `Prefix` |
| | string | `Suffix` |
| | string | `SeparatorText` (tab, U+3000, U+2002, U+2003 and `.` seen) |

A string here is a u32 length in UTF-16 units followed by text segments
(`objects.md`), not the in-object string form.

The tail follows the separator: 172 bytes in files from InDesign 7 to 11,
174 bytes from 12 on. Offsets are from the start of the tail.

| Offset | Type | IDML |
|---|---|---|
| 0 | u16 | `NoSplitting` |
| 4 | f64 | `FootnoteMinimumFirstBaselineOffset`: 19.84 in one file, 0 in the 1,384 others with a known tail, all matching (corpus of 2026-10) |
| 12 | u16, 174-byte tail only | `EnableStraddling` (317 of 317 trustworthy pairs) |
| R | rule block, 78 bytes | `Rule…` |
| R + 78 | the same block | `ContinuingRule…` |

R is 18 for a 174-byte tail and 16 for a 172-byte tail. Each block:

| Block offset | Type | IDML (`Rule` or `ContinuingRule` prefix) |
|---|---|---|
| 0 | u16 | `…On` |
| 2 | u32 code, then u32 0 | `…Type`, as the stroke types of page items and cells: 0x5A29 `StrokeStyle/$ID/Solid`, 0x5A39 `…/Canned Dotted`, 0xB007 `…/ThickThick` |
| 10 | u32 swatch UID | `…Color` |
| 14 | f64 | `…LineWeight` |
| 22 | f64 | `…Tint` |
| 30 | u16 | 0 in all |
| 32 | u32 swatch UID | `…GapColor` |
| 36 | f64 | `…GapTint` |
| 44 | 2 bytes | 0 in all; the two overprint flags are `false` in all, so they cannot be told apart |
| 46 | f64 | `…LeftIndent` |
| 54 | f64 | `…Width` |
| 62 | f64 | `…Offset` |
| 70 | 8 bytes | 0 in all |

A tail of another length is not read: the converter keeps the observed
values for the rules and warns. Codes not listed are left out.
`FootnoteFirstBaselineOffset` and the overprint flags have one value in
every sample and come from the generated value file (`idml-values.md`).

**Without the chunk** (90 files) the IDML has the defaults of the value
file, `FootnoteTextStyle="ParagraphStyle/$ID/NormalParagraphStyle"`,
`SeparatorText` tab, and `EnableStraddling="true"` from DOM 12 (67 of 67).
In the corpus of 2026-10 the 75 files without the chunk (51 trustworthy)
also have, each in 75 of 75: `StartAt="1"`, `Prefix=""`, `Suffix=""`,
`FootnoteMarkerStyle="CharacterStyle/$ID/[No character style]"`,
`SpaceBetween="0"`, `Spacer="0"`,
`FootnoteMinimumFirstBaselineOffset="0"`, `EosPlacement="false"`,
`NoSplitting="false"`; for the rule and the continuing rule `…On`
`true`, `…LineWeight` 1, `…Tint` and `…GapTint` 100, `…LeftIndent` 0,
`…Width` 72 (rule) and 288 (continuing rule), `…Offset` 0, `…Type`
`StrokeStyle/$ID/Solid`, `…Color` `Color/Black`, `…GapColor`
`Swatch/None`; and the properties `FootnoteNumberingStyle` `Arabic`,
`RestartNumbering` `DontRestart`, `ShowPrefixSuffix` `NoPrefixSuffix`
and `MarkerPositioning` `SuperscriptMarker`. The value files lost most of
these when the corpus grew, because files with the chunk vary; the
converter writes them for files without the chunk.

`SeparatorText` in the value file is the reference `&#9;`; the value file
reader now decodes numeric character references, so the tab is written
as a tab.

In the trustworthy pairs every `FootnoteOption` value is now reproduced
(489 of 489 pairs for each attribute).

## Text frame footnote options

`TextFrameFootnoteOptionsObject` is a child of `TextFrame` in IDML from
DOM 12, with `EnableOverrides`, `SpanFootnotesAcross`,
`MinimumSpacingOption` and `SpaceBetweenFootnotes`: the frame's
`FootnotesEnableOverrides`, `FootnotesSpanAcrossColumns`,
`FootnotesMinimumSpacing` and `FootnotesSpaceBetween`.

IDML writes it on a frame of DOM 12 or later when one of these holds:

- the frame's applied object style lacks category 0xADCA (footnotes) in
  its category list (chunk 0x1B92E; `objects.md`, text frame
  preferences);
- the frame's footnote values (span, minimum spacing, space between)
  differ from the style's;
- the frame has no object style object.

This is the rule `TextFramePreference` follows for its `Footnotes…`
attributes, applied from DOM 12 although `TextFramePreference` has no
footnote values before 13.1. The frame's values are in chunk 0x22608 of
its multi-column frame (`objects.md`, text frame preferences): u16 at 2
the span (1 = true), f64 at 4 and 12 the spacings. Without the chunk they
are `false`, 12 and 6. The style's values are those of its text frame
settings (chunk 0x1B924: u16 at 144, f64 at 146 and 154), or of the
style it is based on, or `false`, 12 and 6.

Evidence: measured on 16,900 text frames of DOM 12 or later in 606
trustworthy pairs before 2026-10 (every frame with the category off or
without a style has the element, 6,011 of 6,011; every frame with the
category on and equal values has none, 10,888 of 10,888; the one frame
with the category on and other values has it). Over the 1,251
trustworthy pairs after 2026-10, the converter's elements by this rule
are 20,096 of the 20,096 reference elements on the frames it writes, with
all four values, and no element that the reference lacks; the other 255
are on frames it does not write. Three
elements of the output have an `EnableOverrides` that the reference
element lacks (not explained).

`EnableOverrides` is `false` in every element. Chunk 0x22608 has a u16 0
at offset 0 in every sample; it is probably that flag, which no sample
shows set. The converter writes `false`. The element follows
`TextFramePreference` and the frame's `BaselineFrameGridOption`.

## Endnotes

**Endnote story.** Document chunk 0x22618 is a u32, the UID of the
endnote story, class 0x2801 (12 of 12 files with endnotes). The story is
listed in document chunk 0x222 with the other stories and has the same
strands as a class 0x201 story. IDML lists it in `StoryList` in that
order and writes it as a `Story` with `IsEndnoteStory="true"`. One per
file; 729 endnotes in 12 files (InDesign 16.2, 17.2, 20.0 and 21.2).

**Reference.** An endnote reference is a U+0004 in story text. The
owned-items strand lists an object of class 0x2805 at that position (729
of 729). IDML writes, in place of the character,
`<Endnote Self="u<0x2805 UID>" EndnoteTextRange="u<0x2804 UID>"/>`.

**Endnote objects.** Class 0x2805, chunk 0x22616: u32 UID of its range
(class 0x2804). Class 0x2804, chunk 0x2261A: u32 UID of its endnote. The
two point at each other in 729 of 729.

**Range position.** The endnote story's strand of class 0xCA1C holds a
range tree (`hyperlinks.md`, position in the text) whose node sources
are the 0x2804 objects: 729 nodes for 729 endnotes. The same tree also
holds 15 hyperlink text sources (class 0x13502). The range text starts
with U+FEFF U+0004 (710 of 729), ends with U+FEFF (729 of 729) and holds a
paragraph return in 13 of 729. IDML writes
`<EndnoteRange Self="u<0x2804 UID>" SourceEndnote="u<0x2805 UID>">`
around the range, both U+FEFF included in `Content`: inside one
`CharacterStyleRange` when the range lies in one (2 ranges in a 21.2
pair), or as a child of the `ParagraphStyleRange` around several (1
range in a 17.2 file with a DOM 14 IDML). How IDML writes a range across
paragraphs is not shown. The converter writes ranges within one
character range and leaves out the wrapper of others, with a warning.

**Endnote text frame.** A text frame whose story is the endnote story is
written as `EndnoteTextFrame`, with the attributes and children of a
`TextFrame` (1 frame, compared attribute by attribute).

**Schema.** The 21.5 schema rejects InDesign's own endnote markup:
`Endnote` is allowed only directly in `Story`, `Cell` and similar
places, not in `CharacterStyleRange`, and `EndnoteRange` is defined
without content. Jing reports both on the reference IDML. The converter
writes the markup as InDesign does; `measurement.md` describes the
validation exception.

## Endnote options (preferences chunk 0x2261E)

Fields in order:

1. string `EndnoteTitle`
2. u32 `EndnoteTitleStyle` (paragraph style UID)
3. u32 numbering code (0xCA07 `Arabic`, the only value)
4. u32 `StartEndnoteNumberAt`
5. u32 restart (0 `Continuous`, the only value)
6. u32 `EndnoteMarkerPositioning`: 1 `SuperscriptMarker`, 3 `RubyMarker`
7. u32 `EndnoteMarkerStyle` (character style UID)
8. u32 `EndnoteTextStyle` (paragraph style UID)
9. string `EndnoteSeparatorText`
10. u32 (1 in all)
11. u32 (1 in all)
12. string (empty in all)
13. string (empty in all)
14. u32 (0 in all)

Strings are as in chunk 0x2820. 359 files have the chunk and an
`EndnoteOption` in their IDML (240 trustworthy): all values match in all
359, and the parse ends at the end of the chunk. Fields 10 to 14 never
vary (`ScopeValue` `EndnoteDocumentScope`, `FrameCreateOption`
`NewPage`, `EndnotePrefix`, `EndnoteSuffix`, `ShowEndnotePrefixSuffix`
`NoPrefixSuffix`), so which is which is not proven; their values come
from the value file.

**Without the chunk** (149 files with DOM 13 or later) the marker and
text styles are the defaults, which the converter writes. The title is
the exporting InDesign's localized default (`Endnotes` 108, `Notes de
fin` 18, `Eindnoten` 12, 4 others in the 654 pairs before 2026-10), and
the separator and marker position follow its language. The converter
writes the separator U+3000 when the last session of the save history
has language code 0x0101, and tab otherwise (140 of 141 pairs before
2026-10; `objects.md`, save history). The title and the marker position
follow the saving edition that the XMP black name shows (`objects.md`,
saving edition): `RubyMarker` for a Japanese edition only, so Korean
editions, which also have code 0x0101, get `SuperscriptMarker`. Over the
trustworthy pairs of the corpus after 2026-10 this gives 917 of 918
titles and 916 of 918 marker positions (692 and 876 before); without a
black name the marker follows the code.

## IsEndnoteStory

`IsEndnoteStory` is on every `Story` and `XmlStory` from DOM 13: 0 of
2,467 stories and 0 of 39 XML stories in DOM 12 IDML, 3,094 of 3,094 and
173 of 173 in DOM 13.0–14.0.
