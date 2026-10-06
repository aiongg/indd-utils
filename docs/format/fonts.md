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
| u8, u16 | Not identified | |
| string | Family name | `FontFamily/@Name`, `Font/@FontFamily` |
| u8 | Not identified | |
| string | Family name in the font's own script | not written |
| 6 bytes | Not identified | |
| u16 | Font count | |
| font records | See below | one `Font` each, in the same order |
| u32 | Writing script | `WritingScript` of every font in the family |

Strings are in-object strings (`objects.md`).

**Font record:**

| Field | Contents | IDML `Font` attribute |
|---|---|---|
| u8 | Not identified (0 or 1) | |
| string | Style name | `FontStyleName` |
| u16 *n*, *n* bytes | PostScript name, one byte per character | `PostScriptName` |
| u8, string | Full name | `FullName` |
| u8, string | Style name in the font's own script | `FontStyleNameNative` |
| u8, string | Full name in the font's own script | `FullNameNative` |
| u32 | Font type: 1 `TrueType`, 6 `OpenTypeCFF`, 7 `OpenTypeCID`, 8 `OpenTypeTT` | `FontType` |
| u32 *n*, segments | Version: *n* UTF-16 code units as text segments | `Version` |

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
- `FontType`: 4,464 of 4,464 (code 6: 3,043, 1: 905, 7: 341, 8: 175).
  Codes 0 (54 fonts) and 3 (1 font) occur in files without an IDML; the
  converter leaves out `FontType` for them.
- `WritingScript`: 4,464 of 4,464 with the u32 after the font records
  (values 0, 1 and 33).
- `Name` is the family name, a space and the style name, and `Self` is
  the family's `Self`, `Fontn` and `Name` (4,464 of 4,464, taking the
  family name as IDML writes it).
- `FontFamily/@Name`: 622 of 630. The other 8 have a technology suffix in
  IDML (`Minion Pro (OTF)`, `Montserrat (OTF)`, `Times (TT)`) that the
  INDD data does not determine; the converter writes the plain name, also
  in the fonts' `FontFamily`, `Name` and `Self`.
- IDML lists the families in UID order (135 of 135 pairs).

Text formatting refers to a family by UID (`AppliedFont`, `BulletsFont`;
see `attributes.md`) and IDML writes the family name there.

## Typekit IDs (chunk 0x3EEB)

u32 count *n*, then *n* entries: u8 1 if the string is a built-in key
(written with `$ID/`), then a string. Entry *i* is the `TypekitID` of font
*i*; fonts after the *n*-th have `TypekitID="$ID/"`.

- 945 of 945 fonts in families where *n* equals the font count.
- In the 38 families where *n* is smaller, the entries match the first
  *n* fonts and all other fonts have `$ID/` (38 of 38).
- Families with *n* = 0 or without the chunk: all 3,020 fonts in IDML
  from DOM 12 on have `$ID/`. The one DOM 7 IDML (32 fonts) has no
  `TypekitID`; the converter writes it for InDesign 12 and later.

## Not converted

- `Status` (`Installed`, `Substituted`, `NotAvailable`) depends on the
  fonts present where the IDML was exported. No INDD field agrees with
  it: the first byte of the font record is 1 for 2,042 `Installed` and
  1,047 `Substituted` fonts. The attribute is optional in the schema and
  left out.
- `NumDesignAxes`, `DesignAxesName` and `DesignAxesValues` (131 fonts of
  variable font families) are not in the records above, and were not
  found elsewhere.
- `CompositeFont` elements are not written.

`PlatformName` is `$ID/` in all 4,464 IDML fonts; the converter writes
that value (`idml-values.md`).
