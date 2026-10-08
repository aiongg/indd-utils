# Cross-reference formats

Cross-reference formats are objects of class 0x1355E. IDML writes each as
a `CrossReferenceFormat` element in `designmap.xml` with `Self` the UID,
holding one `BuildingBlock` element per block, whose `Self` is the
format's `Self`, `BuildingBlock` and the block's index (from 0). The
schema puts them after the sections and before `idPkg:BackingStory`.
Implemented in `src/model/xref.rs`.

## Evidence

All corpus pairs, including those whose IDML is from an older version:
1,215 IDML `CrossReferenceFormat` elements, of which 1,170 have an INDD
object with the same UID. The other 45 are in pairs whose IDML was
exported from another save. All 2,250 class 0x1355E objects in the 251
distinct little-endian files parse with the layout below.

## Format (chunk 0x13593)

| Field | Contents | IDML |
|---|---|---|
| u8, string | Name | `Name` (1,170 of 1,170) |
| u32 | 0 in every sample | `AppliedCharacterStyle="n"` in every sample |
| u16 | 0 or 1, not identified | |
| u32 | Block count | number of `BuildingBlock` elements (1,170 of 1,170) |
| blocks | below | `BuildingBlock`, in order |

**Building block:**

| Field | Contents | IDML |
|---|---|---|
| u32 | Block type (below) | `BlockType` |
| 10 bytes | 0 in every sample | `AppliedCharacterStyle="n"`, `AppliedDelimiter="$ID/"`, `IncludeDelimiter="false"` in every sample |
| u8, string | Custom text; the flag byte is 1 for a built-in key, written with `$ID/` | `CustomText` (3,510 of 3,510) |

| Code | `BlockType` | Blocks in the pairs |
|---|---|---|
| 0 | `CustomStringBuildingBlock` | 1,820 |
| 3 | `PageNumberBuildingBlock` | 650 |
| 4 | `FullParagraphBuildingBlock` | 260 |
| 5 | `ParagraphNumberBuildingBlock` | 260 |
| 6 | `ParagraphTextBuildingBlock` | 260 |
| 7 | `BookmarkNameBuildingBlock` | 260 |

Custom texts in the pairs include `"`, `" on page `, ` on page ` and
`page `; the other block types have `$ID/`.

Every corpus file has the same nine formats, so the fields that are 0
everywhere cannot be located. The converter writes the character style,
delimiter and include-delimiter values above only where those fields are
0, and leaves them out otherwise. It leaves out blocks with other type
codes (none occur in the corpus); the other blocks keep their index.

## Sources in text

Cross-reference sources in text are text sources with chunk 0x135A0;
`hyperlinks.md` describes them. Cross-reference sources inside
footnotes and tracked changes are not converted, because their text is
not.
