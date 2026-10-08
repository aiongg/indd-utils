# Hyperlinks and bookmarks

Hyperlinks, their text sources, page and URL destinations, and bookmarks.
Implemented in `src/model/hyperlink.rs`.

| Class | Object | IDML element |
|---|---|---|
| 0x13501 | Hyperlink | `Hyperlink` (designmap) |
| 0x13502 | Text source | `HyperlinkTextSource` (story text) |
| 0x13503 | Page item source | `HyperlinkPageItemSource` (designmap) |
| 0x13505 | Page destination | `HyperlinkPageDestination` (designmap) |
| 0x13506 | URL destination | `HyperlinkURLDestination` (designmap) |
| 0x1354C | Bookmark | `Bookmark` (designmap) |

## Evidence

The first facts below came from five corpus pairs. They were checked
again on the later corpus: 108 of its 654 same-version pairs have
hyperlinks (75 trustworthy), with 11,166 `Hyperlink`, 10,491
`HyperlinkTextSource`, 191 `CrossReferenceSource`, 430
`HyperlinkPageItemSource`, 1,159 `HyperlinkPageDestination`, 27,724
`HyperlinkURLDestination`, 643 `HyperlinkExternalPageDestination`, 1,021
`HyperlinkTextDestination`, 169 `ParagraphDestination` and 1,074
`Bookmark` elements. Hyperlinks, sources, external page destinations and
bookmarks have `Self` equal to the INDD UID; the other destinations are
found by `DestinationUniqueKey`. Counts below without a qualifier are over
all 108 pairs.

## Hyperlink (chunk 0x13502)

| Offset | Contents | IDML |
|---|---|---|
| 0 | u32 text source UID (0 for none) | `Source` |
| 4 | u16 0 | |
| 6 | u16 hidden (1 true) | `Hidden` |
| 8 | u32 key | `DestinationUniqueKey` |
| 12 | u32 destination kind (below) | |
| 16 | u32: 2007 in 11,121 of 11,164; 2006 (37), 2005 (4) or 2019 (2) in the others | |
| 20 | u32 0x13501 | |
| 24 | flag byte and string | `Name` |
| | flag byte and string (empty) | |

`Source`, `Hidden`, the key and `Name` match in all. 48 hyperlinks in one
pair have source UID 0; IDML writes them without `Source`, which the
schema requires, so the converter leaves them out.

| Kind at 12 | IDML `Destination` |
|---|---|
| 2000 | `n` (no destination), or a list (another document) |
| 2001 | `HyperlinkPageDestination/…` (1,151) |
| 2002, 2017, 2018 | `HyperlinkURLDestination/…` (9,030) |
| 2003 | `ParagraphDestination/…` (191, cross-references) |
| 2004 | an external page destination's UID (576) |

**Destination.** IDML writes it as `Properties/Destination`. A hyperlink
whose object has chunk 0x1359F (u32 0x13501, u32 1, u32 link UID) points
into another document: IDML writes a list destination (file name, volume
name and three numbers) for 155 of 156 such hyperlinks. The volume and
the numbers are not located, so the converter leaves `Destination` out
for them (the schema allows that). For all other hyperlinks the
destination is the destination object with the same key (10,946 of
10,947; each key names one object), or, with kind 2000 and no such
object, the value `n` (61). In the trustworthy pairs, after this rule,
the converter's `Destination` is right for 2,850 of 2,878 hyperlinks it
writes and left out for the other 28 (list destinations).

**Appearance (chunk 0x13553, 18 bytes).** Over all 11,164 hyperlinks the
chunk is `00 00 01 00 00 00 b6 00 00 00 b10 00 00 00 00 00 00 00`. Byte 6
takes the values 0x1A, 0x1B, 0x1D, 0x20, 0x21 and does not follow any
IDML attribute. Byte 10 is `Highlight`: 0 `None` (11,156), 1 `Invert`
(8). `Visible="false"`, `Width="Thin"` and `BorderStyle="Solid"` are the
same in all 11,164, so their fields cannot be located; the converter
writes them for hyperlinks whose chunk has this pattern. `BorderColor` is
`Black` for all 10,588 hyperlinks of kinds other than 2004; the 576 of
kind 2004 (two pairs) have `Black` (255), `Violet` (280) or `Green` (41),
and no chunk of the hyperlink object differs between the colours, so the
converter writes `Black` only for the other kinds. The attribute order is
`Self Name Source Visible Highlight Width BorderStyle Hidden
DestinationUniqueKey`, then `Properties` with `BorderColor` and
`Destination`.

## Text source (class 0x13502)

**Chunk 0x13504:** u8 hidden (1 true), u8, u32 hyperlink UID, flag byte
and string (`Name`), u32. `Name` and `Hidden` match in 43 of 43.

**Chunk 0x1352E:** u32 range strand (below), u32 node index in its tree,
u32 character style UID. A style UID of 0 is written as
`AppliedCharacterStyle="n"` (39 of 39); other values are the style
(4 of 4, `CharacterStyle/$ID/Hyperlink`).

**Chunk 0x135B7: alternative destination.** u32 type (2 =
`TocTextAnchor`, the only type seen), flag byte and string
(`TextAnchorName`), u32 (`IndexMarkerId`), u32 length *n* and *n* UTF-16
code units of text segments (`TocEntryPageNumberString`), u32
(`TocEntryLevel`). IDML writes it as `Properties/AlternativeDestination`
with the attributes `Type IndexMarkerId TextAnchorName
TocEntryPageNumberString TocEntryLevel`. 1,163 of 1,163 sources with the
chunk match on all five attributes, and the chunk ends after the last
field. In the page number string, a character below U+0020 is written as
the text `<?AID 00xx?>` with the code in lowercase hexadecimal (U+0008,
before the page number, in all samples). In the trustworthy pairs the
converter writes 1,048 of 1,048 of them right.

### Position in the text

IDML writes the source as a `HyperlinkTextSource` element around its text
inside a `CharacterStyleRange`. The INDD stores the text ranges of a
story's sources in a tree:

- One of the story's strands (chunk 0x223, `objects.md`) is of class
  0xCA1C. Its chunk 0x16126 starts with the UID of the tree's first page.
- A page is an object of class 0x1610A. Chunk 0x16127: u32 next page,
  u32 previous page, u32 strand, u32, u32 *n*, *n* u32 node indexes, u32
  node count, then the nodes. Each node is u32 class (0xCA1E), u32 size
  (60), and the node data.
- Node data: u16, u32, u32, u16, then four references, each u32 object
  (the strand, or 0 for none) and u32 node index: parent, the node itself,
  left child, right child. Then u32 value, u32 length, u32 text source
  UID, u32 0.
- The root's value is its start in the story text (UTF-16 offset). A left
  child starts *value* before its parent, a right child *value* after it.

Checked by taking the INDD story text at the computed start and length
and comparing it with the text in the IDML source element: 43 of 43 (in
4 the text contains U+0008, which IDML writes as a processing
instruction). In the 251
little-endian files there are 59,532 nodes in 90 files; all are class
0xCA1E and name a class 0x13502 object.

In the pairs every source lies within one character style range. In 16
files without IDML, all versions of one template, 2,608 sources cross the
boundary of a style range; how IDML writes those is not shown. The
converter leaves them out with a warning, and a hyperlink whose source is
left out is left out too.

## Page item source (class 0x13503)

**Chunk 0x13505:** u8 hidden (1 true), u8, u32 hyperlink UID, flag byte
and string (`Name`), u32. **Chunk 0x13525:** u32 page item UID
(`SourcePageItem`). The page item's chunk 0x1351D names the source back.
IDML writes `HyperlinkPageItemSource` in `designmap.xml` with `Self`,
`Name`, `SourcePageItem`, `Hidden`, in that order, after the destinations
and before the hyperlinks. 430 of 430 match on `Hidden` and
`SourcePageItem` (rectangles and groups, classes 0x6201 and 0x401), and
the converter writes 136 of 136 in the trustworthy pairs with all values
right. A source whose page item is not written is left out, with its
hyperlink.

## Page destination (class 0x13505)

**Chunk 0x13507:** u8 hidden, u8, flag byte and string (`Name`), u32 key
(`DestinationUniqueKey`). **Chunk 0x13527:** u32 page UID
(`DestinationPage`), f64 zoom as a fraction, u32 view setting, then four
f64.

Over 41 destinations (39, plus 2 in a pair from another save):

- `Name`, `DestinationUniqueKey` and `Hidden` (all true): 41 of 41.
- `DestinationPage`: 39 of 39, and 0 of 2 in the other save.
- Zoom 1 and `ViewPercentage="100"`: 39 of 39; the other two have 91 in
  IDML, also with zoom 1. The converter writes the zoom × 100. Over the
  654 pairs of the later corpus: 1,157 of 1,159 destinations (33 files),
  with values from 75.35 to 800.
- View setting 1 and `ViewSetting="FitWindow"`: 41 of 41, and 592 of 592
  in 26 files of the later corpus. View setting 0 and
  `ViewSetting="Fixed"`: 567 of 567 in 11 files. The converter writes
  these two and leaves the attribute out for other codes.
- One file without an IDML has three destinations with view setting 0
  and zoom 0. The schema allows `ViewPercentage` from 5 to 4000, so the
  converter leaves it out (with a warning) for a zoom outside 0.05–40.
- After the view setting, chunk 0x13527 has four f64: `ViewBounds`
  `Left`, `Top`, `Right` and `Bottom`, in that order (1,157 of 1,159 in
  the later corpus). 590 have 1e+256 in all four, which IDML writes as
  `1e+256`. IDML writes them as `Properties/ViewBounds` with the
  attributes `Top Left Bottom Right`.
- `NameManually="true"` is in all 1,159 page destinations of the later
  corpus. The flag byte before the name is 0 (4) or 2 (1,155), so it
  does not hold this value; no field was located. The converter writes
  `true`, the corpus value, after `Name`.

`Self` is `HyperlinkPageDestination/` and the name.

## URL destination (class 0x13506)

**Chunk 0x13509:** u8 hidden, u8, flag byte and string (`Name`), u32 key.
**Chunk 0x100B:** flag byte and string, the URL (`DestinationURL`).

The 4 URL destinations in the pairs match on `Name`, `DestinationURL`,
`Hidden` (false) and `DestinationUniqueKey`, but their name and URL are
the same text, so they do not show which chunk is which. In a pair from
another save, an INDD destination has an e-mail address followed by
` 1` in chunk 0x13509 and a `mailto:` URL for that address in chunk
0x100B, and the IDML has a destination named with the address alone and
with that `mailto:` URL as `DestinationURL`. Chunk 0x100B is therefore
taken as the URL. `Self` is
`HyperlinkURLDestination/` and the name with `:` written as `%3a`.

## Bookmarks (class 0x1354C)

**Chunk 0x13547:** flag byte and string (`Name`), u32 (1 for a top-level
bookmark, 2 for one inside another), u32 parent (the document, UID 1, or
a bookmark), UID list of child bookmarks, u32 destination UID.

All 7 bookmarks match on `Name` and `Destination` (the destination's
`Self`), and IDML nests each bookmark's children inside it, in the order
of the child list (2 top-level bookmarks with 2 and 3 children).

The document's chunk 0x13501 is u32, u16, then UID lists of the text
sources, the hyperlinks and the bookmarks, then a fourth list. The
converter writes the top-level bookmarks in the order of the bookmark
list.

Files from InDesign 3.0 and 4.0 lay out chunk 0x13501 differently: it
starts with the UID lists, and there are four before anything else: text
sources, page destinations, hyperlinks, bookmarks. Evidence: of the 58
distinct 3.0 and 4.0 files, three have the chunk, and the classes of the
listed UIDs show the order. In a 3.0 file (little-endian) the first list
holds one text source (class 0x13502), the third one hyperlink (0x13501),
and the four lists end the chunk. In two 4.0 files (big-endian) the
second list holds 8 page destinations (0x13505) and the fourth 8
bookmarks (0x1354C); then come three empty lists, a count of 8 and eight
strings, not identified. The 6.0 layout above reads in all 4,075
distinct 6.0–21.x files that have the chunk.

InDesign 5.0 files use the 3.0 and 4.0 layout. 18 of the 108 distinct
5.0 files have the chunk, and in all 18 the four lists hold, in order,
text sources (classes 0x13502 and 0x13503), destinations (0x13504,
0x13505 and 0x13506), hyperlinks (0x13501) and bookmarks (0x1354C, in
3 files; empty in the others). Read with the 6.0 layout, 17 of them give
a list longer than the chunk. No 5.0 file has an IDML. Their page and
URL destination objects (109, in 17 of these files) end before the fields
described above, so the converter leaves them out with a warning.

## Order in designmap

The schema puts page destinations, URL destinations and hyperlinks after
the `idPkg:Story` elements, and bookmarks after them. The converter sorts
destinations by key, as the pair with the most destinations does, and
hyperlinks by UID.

## Not converted

The pairs have no QR code hyperlinks with an INDD object of the classes
above. One pair has a `HyperlinkQRCode`; it
is not converted.

A text source whose range holds an anchored object is left out with its
hyperlink. InDesign writes the anchored `Rectangle` inside the
`HyperlinkTextSource` (one source in each of two corpus pairs), but the
IDML schema allows no page item there.
