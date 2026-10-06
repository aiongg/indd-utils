# Text variables

Text variable definitions are objects of class 0xCAB4. Instances placed in
text are objects of class 0xCA64, owned by a U+0018 character in the story
(`objects.md`, owned items). Implemented in `src/model/variable.rs`.

IDML writes each definition as a `TextVariable` element in
`designmap.xml`, between the preference elements and `idPkg:Tags`, as the
schema orders them. Its `Self` is `dTextVariablen` followed by the name;
it has no UID, so definitions were paired with INDD objects by name.

## Evidence

All 135 corpus pairs, including those whose IDML is from an older
version. They have 1,347 IDML `TextVariable` elements. Each has exactly
one class 0xCAB4 object with the same name, and no class 0xCAB4 object
lacks an IDML element. The layout below also holds for all 2,496 class
0xCAB4 objects in the 251 distinct little-endian files.

## Definition (chunk 0x28BD)

| Field | Contents |
|---|---|
| u32 *n*, segments | Name: *n* UTF-16 code units as text segments (`objects.md`) |
| u32 *n*, segments | Text: the date format of date variables (below) |
| u32 | Type code |
| 20 bytes | Settings, mostly not identified (below) |

**Type codes** (each code with its `VariableType` in every pair):

| Code | `VariableType` | Pairs | IDML settings element |
|---|---|---|---|
| 0xCAA1 | `OutputDateType` | 135 | `DateVariablePreference` |
| 0xCAA3 | `CustomTextType` | 2 | `CustomTextVariablePreference` |
| 0xCAA6 | `FileNameType` | 135 | `FileNameVariablePreference` |
| 0xCAA8 | `LastPageNumberType` | 135 | `PageNumberVariablePreference` |
| 0xCAA9 | `ChapterNumberType` | 135 | `ChapterNumberVariablePreference` |
| 0xCAAA | `MatchParagraphStyleType` | 135 | `MatchParagraphStylePreference` |
| 0xCAAB | `ModificationDateType` | 135 | `DateVariablePreference` |
| 0xCAAC | `CreationDateType` | 135 | `DateVariablePreference` |
| 0xCAB5 | `XrefPageNumberType` | 135 | none |
| 0xCAB6 | `XrefChapterNumberType` | 135 | none |
| 0xCAC0 | `LiveCaptionType` | 130 | `CaptionMetadataVariablePreference` |

**Name.** Equal to IDML `Name` in 1,347 of 1,347. The names of the two
cross-reference variables start with U+001B, which IDML writes as
`<?AID 001b?>` inside the attribute value (270 of 270).

**Order.** IDML lists the variables sorted by name in code point order
(135 of 135 pairs; U+001B sorts first). The UID order differs in 113.

**Date format.** For the three date types, the text field is IDML
`Format` (405 of 405, 9 different formats such as `dd.MM.yy` and
`d MMMM yyyy, H:mm`).

**Running header style.** For type 0xCAAA, the u32 at offset 4 of the
settings is a paragraph style UID. It differs between files, but in all
135 pairs it is the style that IDML writes as
`AppliedParagraphStyle="ParagraphStyle/$ID/NormalParagraphStyle"`.

## Settings with one value in the corpus

The other IDML settings have the same value in every pair:

| Element | Attributes |
|---|---|
| all settings elements | `TextBefore=""`, `TextAfter=""` |
| `PageNumberVariablePreference` | `Format="Current"`, `Scope="SectionScope"` |
| `ChapterNumberVariablePreference` | `Format="Current"` |
| `FileNameVariablePreference` | `IncludePath="false"`, `IncludeExtension="false"` |
| `MatchParagraphStylePreference` | `SearchStrategy="FirstOnPage"`, `ChangeCase="None"`, `DeleteEndPunctuation="false"` |
| `CaptionMetadataVariablePreference` | `MetadataProviderName="$ID/#LinkInfoNameStr"` |
| `CustomTextVariablePreference` | an empty `Contents` string |

Their INDD fields cannot be located, because nothing varies. The INDD
side has one pattern per type in all 2,496 objects of the little-endian
files (with or without IDML):

| Types | Text field | 20 settings bytes |
|---|---|---|
| Dates | the format | u32 0 or 0xCAB3, then zeros |
| 0xCAC0 | empty | u32 0x8C64, then zeros |
| 0xCAAA | empty | u32 0, u32 style, then zeros |
| Others | empty | zeros |

The exception is custom text variables (0xCAA3): in files without IDML,
5 of 7 have a non-empty text field (for example a person's name or a
date), probably the variable's contents, and one has 0xCAB3 in the first
u32. The two in the pairs have an empty text field and an empty
`Contents`.

The converter writes the settings above only for a definition whose
fields match its type's pattern. Then the INDD data equals that of the
samples, so the IDML settings are taken to be equal too. For any other
definition it writes the date format and the running header style (the
proven fields) and leaves out the rest. A custom text variable with a
non-empty text field gets no `CustomTextVariablePreference`: that the
text is its contents is not proven.

## Instances (class 0xCA64)

| Chunk | Contents |
|---|---|
| 0x100B | u8 (1 in every sample), then the name of the variable as an in-object string |
| 0x288B | u32 UID, not used |

IDML writes an instance as a `TextVariableInstance` element in place of
the U+0018, with `Self` the instance's UID, `Name` the variable's name and
`AssociatedTextVariable` the variable's `Self`. The pairs have 8
instances, in four copies of one document; all 8 match on all three.
In the 251 little-endian files, all 25 instance names name a definition
of the same file.

**Result text.** IDML `ResultText` is the text the variable displays.
The INDD does not store it: the four pairs' modification date text
(`October 31, 2023 4:06 PM`) does not occur in the INDD files, either as
single bytes or as UTF-16. The file name instances (4 of 4) show the
INDD file name without its extension, as their settings
(`IncludePath="false"`, `IncludeExtension="false"`) say. The converter
writes `ResultText` only for file name variables with those settings,
and leaves it out (it is optional in the schema) for all others.
