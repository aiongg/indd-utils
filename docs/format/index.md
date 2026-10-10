# Index

The index of a document: the index object, its sections and topics, and
the page references in the text. Little-endian files; no big-endian
sample has an index. A "u32 string" is a u32 length in UTF-16 units,
then text segments.

## Index object (class 0x13004)

A document with an index has one object of class 0x13004. IDML writes
an `Index` element in `designmap.xml` with `Self` `u` and the UID in
hexadecimal, after the `CrossReferenceFormat` elements and before
`idPkg:BackingStory`. Evidence: 17 of 17 pairs with an index (16
trustworthy); no other pair has either.

Chunk 0x13005: u16 0, u16 *n*, *n* UIDs of section objects (class
0x13005), u16 0. *n* is 27 in most pairs and 60 in a pair with a
Cyrillic group. Chunk 0x1300E holds the index's own copy of the header
groups; IDML takes the header list from the preferences
(`preferences.md`, index header setting).

## Sections (class 0x13005) and topics

Chunk 0x13006 of a section is a u32, the index UID. Chunk 0x13007 holds
the section's topics; sections without topics do not have it:

| Field | Contents |
|---|---|
| u32 | Number of level-1 topics |
| u16 | 0 |
| u32 | Number of topic records that follow (all levels) |
| records | Depth first: each topic is followed by its subtopics |

Topic record:

| Field | Contents | IDML `Topic` |
|---|---|---|
| u8, u8 | 0, 1 | |
| u16 | Level: 1 for a top topic, 2 for its subtopic, … | nesting |
| u32 | Number of direct subtopics | |
| u32 string | Topic text | `Name` |
| u32 string | Sort order (empty in every sample) | `SortOrder` |
| u32 *m*, *m* UIDs | The page references (class 0x13006) of this topic | their `ReferencedTopic` |
| u32 *k*, *k* UIDs | The topic's cross references (class 0x13007), below | `CrossReference` children |
| u32 | 0 in every sample | |

- IDML writes the topics as `Topic` children of `Index`, a subtopic as a
  child of its topic: sections in the order of chunk 0x13005, records in
  stored order.
- `Self` is the parent's `Self`, `Topicn` and the topic text, so a
  level-2 topic has `u<index>Topicn<parent text>Topicn<own text>`.
- Evidence: every chunk of the pairs is read to its end. The topics
  (`Self`, `Name`, `SortOrder`, nesting) equal the IDML in 2,185 of
  2,185 IDML topics of the trustworthy pairs (2,223 in all pairs). In
  one trustworthy pair the INDD has 3 more topics, and 3 more page
  references, than its IDML (below).

**Cross references.** In the corpus after 2026-10, 29 pairs (InDesign
13.0 and 13.1, from one set of tutorial files) have a topic with *k* = 1.
Read with a fixed 8 bytes after the page references, these sections
ended 4 bytes early and were left out. The UID is that of the topic's
IDML `CrossReference` (`Self="u…"`). The cross reference object has
chunk 0x1300B (u16 4 for `CrossReferenceType="SeeAlsoHerein"`, u32 UID
of a class 0x13008 object, u32 0) and chunk 0x13008 (u16 2, u32 the
section). The class 0x13008 object (chunk 0x1300C) holds the UID of
the section of the referenced topic, the topic's position in it (1) and
the cross reference's UID. Every topic of the pairs before 2026-10 has
*k* = 0. The converter reads the list so that the section's topics are
written; it does not write `CrossReference` elements, as one sample
shows one type code.

## Page references (class 0x13006)

A page reference is owned by a U+FEFF in the story text (owned items,
`objects.md`). IDML writes a `PageReference` in place of the character.

| Chunk | Contents |
|---|---|
| 0x13053 | u32: the owned-items strand that holds the marker |
| 0x13008 | u16 1, u32 the section that holds the topic |
| 0x13009 | 30 bytes (26 in InDesign 8.0): below |

Chunk 0x13009:

| Offset | Contents | IDML |
|---|---|---|
| 0–13 | u32 1, u16 0, u32 1, u32 0 in every sample | `PageReferenceType="CurrentPage"` (every IDML page reference) |
| 14, 18 | two equal u32 (0 to 8, or 0xFFFFFFFF) | not written |
| 22 | u32 (1 or 9) | not written |
| 26 | u32, only in a 30-byte chunk | `Id` |

- `Self` is `u` and the UID in hexadecimal.
- `ReferencedTopic` is the `Self` of the topic whose record lists the
  page reference.
- The 3 page references of the one InDesign 8.0 pair have the 26-byte
  chunk, and their IDML has no `Id`.

Evidence: all four attributes equal the IDML for 7,523 of 7,523 page
references of the trustworthy pairs (7,561 in all pairs; `Id` for 7,520
and 7,558, the others have the 26-byte chunk). The converter writes
`PageReferenceType` only when the first 14 bytes are those of the
samples.

## Topics missing from the IDML

In one trustworthy pair (DOM 14) the INDD has 3 topics and 3 page
references that its IDML lacks; the IDML text also lacks their 3
U+FEFF characters. Their `Id`s are the 3 highest of the document. The
IDML was most likely exported before these entries were made. The
converter writes them.
