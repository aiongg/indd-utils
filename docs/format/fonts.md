# Fonts

Font families are objects of class 0x3E03. IDML writes them in
`Resources/Fonts.xml` as `FontFamily` elements with `Self="di<UID hex>"`,
each holding a `Font` element per font. Implemented in `src/model/font.rs`.

## Evidence

All corpus pairs, including those whose IDML is from an older version:
133 distinct INDD files with 630 font families that have an IDML
`FontFamily` with the same UID, holding 4,464 IDML `Font` elements. The
record layout below parses without error in all 1,354 font families of
the 251 distinct little-endian files.

## Family record (chunk 0x3E05)

| Field | Contents | IDML |
|---|---|---|
| u8 | Record kind: 1 or 4 for the layout here, 3 for missing fonts (below), 2 for a family that refers to another (below) | |
| u8 | 0 | |
| u8 | Key flag of the family name: 1 for a built-in key | `$ID/` before the name (below) |
| string | Family name | `FontFamily/@Name`, `Font/@FontFamily` |
| u8 | Key flag of the native name | |
| string | Family name in the font's own script ("native name") | used in place of the name in some documents (below) |
| 6 bytes | Not identified | |
| u16 | Font count | |
| font records | See below | one `Font` each, in the same order |
| u32 | Writing script | `WritingScript` of every font in the family |

Strings are in-object strings (`objects.md`).

**A family that refers to another (record kind 2).** The record is u8
2, u8 0, u32 the UID of another font family (class 0x3E03, whose
record has another kind) and a u16 count of entries that follow. 21
families in 19 distinct corpus files have such a record. In 17 the count
is 0 and the record has 8 bytes; in the other 4 (two files, no IDML)
font entries follow, each three flagged strings (such as `Bold`,
`Arial Black`, `Bold`) and 10 bytes, as in a missing-font record.

IDML writes a family with a count of 0 as a copy of the family it refers
to: the same `Name`, and the same fonts with the same attributes,
except `FontType="Unknown"`, `Version=""` and `TypekitID="$ID/"`. This
holds in all 15 pairs that have such a family (one family in each, with
1 or 2 fonts; 14 of the pairs are versions of one document set from
InDesign 18.4, one is from 17.4). The referring object's own chunk
0x3EEB, when present, is not used: IDML writes `$ID/`. The converter
writes the family this way. For a family with entries it writes the
name and no fonts, with a warning, since no pair shows how IDML writes
them.

**Font record:**

| Field | Contents | IDML `Font` attribute |
|---|---|---|
| u8 | Not identified (0 or 1) | |
| string | Style name | `FontStyleName` |
| u16 *n*, *n* bytes | PostScript name, one byte per character | `PostScriptName` |
| u8, string | Full name | `FullName` |
| u8 key flag, string | Style name in the font's own script | `FontStyleNameNative`, with `$ID/` first when the flag is 1 |
| u8, string | Full name in the font's own script | `FullNameNative` |
| u32 | Font type: 0 `Type1`, 1 `TrueType`, 3 `ATC`, 6 `OpenTypeCFF`, 7 `OpenTypeCID`, 8 `OpenTypeTT`, 0xFFFFFFFF `Unknown` | `FontType` |
| u32 *n*, segments | Version: *n* UTF-16 code units as text segments | `Version` |

In files from InDesign 3.0 and 4.0 the PostScript name is a byte and an
in-object string, and InDesign 3.0 has no version (`big-endian.md`).

**Matches.**

- Font count: 629 of 630 families. The other family has 9 fonts in the
  INDD and 4 in the IDML, which differ in their names, so the IDML was
  probably exported from another save.
- Font order: the same in 627 of the 629 families. In the other two
  (copies of one document), one font is at another position.
- In those 627 families, all 4,434 fonts match on `FontStyleName`,
  `PostScriptName`, `FullName`, `FontStyleNameNative`, `FullNameNative`
  and `Version`. In the other two, matched by name, 6 fonts of each have
  another `Version` in IDML. That document's IDML gives its other font
  families other UIDs, so it was probably exported from another save.
- `FontType`: over the 31,969 fonts of all pairs whose INDD family has
  as many fonts as the IDML family: code 0 `Type1` 799, 1 `TrueType`
  3,338, 3 `ATC` 21, 6 `OpenTypeCFF` 22,325, 7 `OpenTypeCID` 2,559, 8
  `OpenTypeTT` 2,893, 0xFFFFFFFF `Unknown` 34. The converted `FontType`
  equals the IDML for all 32,002 matched fonts of all pairs.
- Key flag of the native style: 1 gives `$ID/` and the string in 1,364
  of 1,364 fonts, 0 the plain string in 30,591 of 30,605 (the other 14
  are in stale pairs whose fonts differ).
- A font with an empty style name: in all 21 such IDML fonts over all
  pairs (12 in the trustworthy pairs), `Name` is the family name and
  ` Regular`, and `Self` is the family's `Self`, `Fontn` and the family
  name, with no trailing space. All have `FontType="ATC"`.
- `WritingScript`: 4,464 of 4,464 with the u32 after the font records
  (values 0, 1 and 33). Over all 654 pairs later in the corpus, 30,406
  of 30,410 fonts; the other four, one family in one file, have 0 in the
  INDD and 7 in the IDML.
  One family in a file without an IDML stores `FF FF FF FF`. The schema
  types `WritingScript` as `xsd:int`, and no corpus IDML has a negative
  value, so the converter leaves the attribute out (with a warning) when
  the u32 does not fit in an `int`.
- `Name` is the family name, a space and the style name, and `Self` is
  the family's `Self`, `Fontn` and `Name` (4,464 of 4,464, taking the
  family name as IDML writes it).
- `FontFamily/@Name`: 622 of 630. The other 8 have a technology suffix in
  IDML that the INDD name lacks (`Montserrat (OTF)` 6 times, `Minion Pro
  (OTF)` and `Trustpilot Display (OTF)` once each). A family whose INDD
  name already ends in ` (TT)` keeps it. The converter writes the INDD
  name, also in the fonts' `FontFamily`, `Name` and `Self`.
- IDML lists the families in UID order (135 of 135 pairs).

**Why the suffix is not predicted.** No field of the family record
separates the 8 from the other 622 families:

- The font type does not. The 8 hold OpenType CFF fonts (2) and
  OpenType TrueType fonts (6), and both types occur in families without
  a suffix: 130 other families with the name of the suffixed CFF family
  hold the same CFF fonts, and 4 other `Montserrat` families the same
  TrueType font.
- The first byte of the family record is 4 in 7 of the 8 and 1 in the
  other, but 68 families without a suffix also have 4. One `Montserrat`
  family with 4 has no suffix.
- A second family of the same name does not explain it. Only one of the
  8 documents has one: two `Trustpilot Display` families, where the CFF
  one has the suffix and the TrueType one has none. Other documents have
  4 and 2 families of one name and type without a suffix.
- The other bytes of the record do not. The byte after the name is 0 in
  the 8 and in 613 others. The first byte of each font record is 1 in
  the 8 and in most others. The 6 bytes before the font count, read as
  three u16, are (0, 0, 0xFFFF) or (0, 0xFFFF, 0xFFFF) in 7 of the 8,
  as in 387 families without a suffix; the eighth has (0, 0, 2), and the
  last u16 takes 12 different values in families without a suffix (2 in
  4 of them), so it does not look like a flag.

The suffix most likely depends on the fonts installed where the IDML
was exported, as `Status` does (below).

Text formatting refers to a family by UID (`AppliedFont`, `BulletsFont`;
see `attributes.md`) and IDML writes the family name there.

**Built-in family names.** The family-name key flag is 1 in 9 families
of 8 pairs. IDML then writes `$ID/` and the name in `FontFamily/@Name`,
in `Font/@Name` (`$ID/`, name, space, style) and in every `AppliedFont`
that names the family; `Font/@FontFamily` and `Font/@Self` have the
plain name. No family with flag 0 gets `$ID/`.

**Native family names.** IDML writes the native name in place of the
family name when the family's writing script equals the script byte of
the document users (`objects.md`, document users), the byte is not 0,
and the native name is not empty and differs from the name. It is then
used wherever the name would be: `FontFamily/@Name`, `Font/@FontFamily`,
`Font/@Name`, `Font/@Self` and every `AppliedFont`.

| Users' script byte | Family writing script | Native name in IDML | Families (documents), all pairs |
|---|---|---|---|
| 1 | 1 | yes | 43 of 46 (9 of 10) |
| 25 | 25 | yes | 2 of 2 (1 of 1) |
| 1 | 2, 3 or 25 | no | 5 of 5 |
| 0, 7, 29 | any | no | 604 of 604 |

The 3 misses are in one trustworthy DOM 16 pair whose IDML has a third
document user that the INDD lacks, so it was exported on another
computer.

**Missing fonts (record kind 3).** After the native name come either 6
zero bytes and the end of the chunk (no fonts; IDML writes a
`FontFamily` without `Font`), or 4 bytes, a u16 font count and short
font records:

| Field | IDML |
|---|---|
| u8 key flag, string | `FontStyleName` (`$ID/Regular` for flag 1); the plain style in `Name` and `Self` |
| u8 key flag, string (the family name) | `FontStyleNameNative` and `PlatformName` (`$ID/` and the name for flag 1) |
| u8 key flag, string | the style again; not mapped |
| u32 style index, u32 0, u16 2 | not mapped |

The other attributes are `PostScriptName=""`, `FontType="Unknown"`,
`WritingScript="0"`, `FullName=""`, `FullNameNative=""`, `Version=""`,
`TypekitID="$ID/"`. Evidence: 3 families with fonts in 2 trustworthy
pairs (5 fonts), every attribute equal; 3 families without fonts in 3
trustworthy pairs. Over the corpus, 14 records without fonts and 49
with fonts parse to their end. Kind 2 records (7 in 5 files) do not
parse; their family keeps its name only.

## Typekit IDs (chunk 0x3EEB)

u32 count *n*, then *n* entries: u8 1 if the string is a built-in key
(written with `$ID/`), then a string. Entry *i* is the `TypekitID` of font
*i*; fonts after the *n*-th have `TypekitID="$ID/"`.

- 945 of 945 fonts in families where *n* equals the font count.
- In the 38 families where *n* is smaller, the entries match the first
  *n* fonts and all other fonts have `$ID/` (38 of 38).
- Families with *n* = 0 or without the chunk: every font has `$ID/`.
- IDML from INDD files of version 9.0 (231 fonts) and 9.1 (29) and
  earlier has no `TypekitID`; from 9.2 (169 fonts) on, every font has
  it. The converter writes it from version 9.2.

## Not converted

- `Status` (`Installed`, `Substituted`, `NotAvailable`) depends on the
  fonts present where the IDML was exported. No INDD field agrees with
  it: the first byte of the font record is 1 for 2,042 `Installed` and
  1,047 `Substituted` fonts. The attribute is optional in the schema and
  left out.
- `NumDesignAxes`, `DesignAxesName` and `DesignAxesValues` (131 fonts of
  variable font families) are not in the records above, and were not
  found elsewhere.

## Composite fonts

Every INDD file has an object of class 0xCB02 for `[No composite font]`,
and every IDML that `CompositeFont` (240 of 240 IDML files of the early
corpus). User composite fonts are more objects of the class: 47 of the
1,251 trustworthy pairs of the corpus after 2026-10 have them, 1,434
fonts in all. The entries are objects of class 0xCB03; IDML writes them
as `CompositeFontEntry` with `Self="u<UID>"`. IDML lists the composite
fonts in UID order (47 of 47 documents) and each font's entries in the
order of its list (1,434 of 1,434 fonts). Implemented in
`src/model/cjk.rs`.

**Composite font (chunk 0xCB02).**

| Field | Contents |
|---|---|
| flagged string | name (`Name`; flag 1 → `$ID/`) |
| flagged string | empty |
| flagged string | a PostScript-style name (`ATC-…`) |
| flagged string | the name again, or empty |
| u32, u16 | not identified |
| u16 *n*, *n* × u32 | the entry UIDs, which end the chunk |

The four strings were found by parsing every 0xCB02 chunk of the pairs,
which then end exactly after the UIDs. A chunk of another layout (files
from InDesign 3.0 have four zero bytes before the name,
`big-endian.md`) is read as the name followed by the first u16 count
whose UIDs end the chunk. That search alone took a wrong position in one
12.1 document, whose name bytes fit the test.

**Entry (chunk 0xCB03).**

| Field | IDML | Evidence (trustworthy pairs, 8,589 entries) |
|---|---|---|
| Flag byte, string | `Name` | 8,589 |
| u32 font family UID | `AppliedFont` (Properties): the family's name | 8,553; the other 36 are families that IDML names in another form (below) |
| Flag byte, string | `FontStyle` | 8,589 |
| f64 | `RelativeSize` | 8,589 (55 to 116) |
| f64 | `BaselineShift` | 8,589 (−10 to 38) |
| f64, f64 | `HorizontalScale`, `VerticalScale` | 100 in every entry, so their order is not shown |
| u16 | `Locked`: 1 `true`, 0 `false` | 8,589 (204 `false`) |
| u16 count *n*, *n* ranges | `CustomCharacters` | 7,155 |
| Four u16 | `ScaleOption`: all 1 `true`, all 0 `false` | 8,589 |

Each range is three code points (first, last, first again), each one
UTF-16 unit or a surrogate pair. `CustomCharacters` is every character
from first to last of each range, in order. IDML writes no
`CustomCharacters` for the first entry of each font, the base font
(1,434 of 1,434; every other entry has them, 7,155 of 7,155). Its ranges
cover most of Unicode: for example U+0000–U+D7FF and U+E000–U+10FFFF, or
U+0009–U+000A, U+000D, U+0020–U+D7FF and U+E000–U+FFFD. XML 1.0 does not
allow U+FFFE and U+FFFF, so the converter leaves those two characters
out of every attribute value and text. Which of the four u16 is
`ScaleOption` is not known; the converter writes it only when all four
agree.

The 36 `AppliedFont` values not reproduced are in one document whose
IDML names its families in English where the converter writes the
native name (family names, above).

`PlatformName` is `$ID/` in all 4,464 IDML fonts; the converter writes
that value (`idml-values.md`).

### Composite fonts as applied fonts

A text attribute `AppliedFont` (0x1B2B, `attributes.md`) that names a
composite font holds the UID of a font family (class 0x3E03) whose name
is the composite font's name; no `AppliedFont` value is the UID of the
0xCB02 object. The family name can contain escapes `<hhhh>`, four
hexadecimal digits of a UTF-16 code unit, for characters outside ASCII
(such as `<5927><6A19>`). Decoded, the name equals the composite font
name: in the conversions of the 1,251 trustworthy pairs, families with
escapes are used in 27 documents, and in 24 their decoded name is one of
the document's composite fonts. The other escaped names (in 3
documents) match no composite font and are written as family names, as
before.

IDML then writes `AppliedFont` as a reference to the composite font,
`<AppliedFont type="object">CompositeFont/<name></AppliedFont>`, in
every element but two. Over the trustworthy reference IDMLs, counting
every `AppliedFont` whose value is a composite font of the document:

| Element | Written as | Values (documents) |
|---|---|---:|
| `CharacterStyleRange` | object | 742 (30) |
| `ParagraphStyle` | object | 139 (27) |
| `TextDefault` | object | 18 (18) |
| `CharacterStyle` | string: the name | 48 (11) |
| `GridDataInformation` | string: the name | 12 (6) |

The converter writes the reference in paragraph styles, text defaults
and text ranges, and the name in character styles (grid settings do
not use these attribute lists). Reproduced over the trustworthy pairs:
`ParagraphStyle` `AppliedFont` 4,803 of 4,804 (4,664 by family name
alone), `TextDefault` 1,250 of 1,251 (1,232), text ranges 85,587 of
85,587 (84,909), `CharacterStyle` 1,102 of 1,102 (unchanged). The value
left in each of the first two is a family name that IDML writes in
English and the converter in the native script (family names, above).
