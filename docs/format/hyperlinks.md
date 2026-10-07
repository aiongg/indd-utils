# Hyperlinks and bookmarks

Hyperlinks, their text sources, page and URL destinations, and bookmarks.
Implemented in `src/model/hyperlink.rs`.

| Class | Object | IDML element |
|---|---|---|
| 0x13501 | Hyperlink | `Hyperlink` (designmap) |
| 0x13502 | Text source | `HyperlinkTextSource` (story text) |
| 0x13505 | Page destination | `HyperlinkPageDestination` (designmap) |
| 0x13506 | URL destination | `HyperlinkURLDestination` (designmap) |
| 0x1354C | Bookmark | `Bookmark` (designmap) |

## Evidence

Five corpus pairs have hyperlinks, all same-version. Two of them were
exported from another save than the INDD (their hyperlink objects have
other UIDs and keys than the IDML elements) and are used only where
noted. The other three have 43 hyperlinks, 43 text sources, 39 page
destinations, 4 URL destinations and 7 bookmarks, all with an IDML
element whose `Self` is the INDD UID (hyperlinks, sources, bookmarks) or
whose `Name` is the INDD name (destinations).

## Hyperlink (chunk 0x13502)

| Offset | Contents | IDML |
|---|---|---|
| 0 | u32 text source UID | `Source` |
| 4 | u16 0 | |
| 6 | u16 hidden (1 true) | `Hidden` |
| 8 | u32 key | `DestinationUniqueKey` |
| 12 | three u32, not identified (2001 or 2017; 2007; 0x13501) | |
| 24 | flag byte and string | `Name` |
| | flag byte and string (empty) | |

All four match in 43 of 43 (`Hidden` true for 7, false for 36).

The destination is the destination object with the same key (43 of 43).
IDML writes it as `Properties/Destination`, an object reference to the
destination's `Self`.

**Appearance.** `Visible="false"`, `Highlight="None"`, `Width="Thin"`,
`BorderStyle="Solid"` and `BorderColor` `Black` are the same for all 43,
so their fields cannot be located. Chunk 0x13553 (18 bytes) has two
values among them (byte 6 is 0x21 or 0x1B), and the u32 at offset 12 two
(2001, 2017). The converter writes these attributes only for hyperlinks
whose chunk 0x13553 and three u32 at offset 12 hold values seen in the
pairs.

## Text source (class 0x13502)

**Chunk 0x13504:** u8 hidden (1 true), u8, u32 hyperlink UID, flag byte
and string (`Name`), u32. `Name` and `Hidden` match in 43 of 43.

**Chunk 0x1352E:** u32 range strand (below), u32 node index in its tree,
u32 character style UID. A style UID of 0 is written as
`AppliedCharacterStyle="n"` (39 of 39); other values are the style
(4 of 4, `CharacterStyle/$ID/Hyperlink`).

**Chunk 0x135B7** (21 bytes) is present on 39 sources in the pairs, and on
no other source in the 251 little-endian files. It
has the same bytes in all 39, and IDML writes the same
`Properties/AlternativeDestination` for all of them (`Type="TocTextAnchor"`,
`IndexMarkerId="0"`, `TextAnchorName=""`, `TocEntryPageNumberString=""`,
`TocEntryLevel="0"`). The converter writes that element for sources with
exactly these bytes.

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
- `NameManually="true"` in all 41 has no located field and is left out,
  as is the `ViewBounds` property.

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

The pairs have no text destinations (`HyperlinkTextDestination`),
cross-reference sources, page item sources or QR code hyperlinks with an
INDD object of the classes above. One pair has a `HyperlinkQRCode`; it
is not converted.

A text source whose range holds an anchored object is left out with its
hyperlink. InDesign writes the anchored `Rectangle` inside the
`HyperlinkTextSource` (one source in each of two corpus pairs), but the
IDML schema allows no page item there.
