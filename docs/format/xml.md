# XML structure

How the document's XML structure (IDML `XmlStory` and `XMLElement`) is
stored. XML tags are in `objects.md`. Implemented in `src/model/xml.rs`
and the story writer in `src/idml/mod.rs`.

**Evidence.** All 251 distinct little-endian corpus files have an XML
structure; in 250 it is only the root element. The corpus pairs have 78
IDML `XmlStory` elements and 86 `XMLElement` elements: 78 root elements
and 8 more elements in one document. Unless stated otherwise, counts are
over the 251 files.

## Nodes

The structure is a tree of nodes. The document node is the root
element's parent and has no IDML element. Each node belongs to a story,
the one whose text holds its markers (below), and is named by that
story's UID and an ID that is unique within the story: a *node key*.

**Node store.** A story's chunk 0x16126 starts with the UID of the first
page of its node store. Store pages are objects of class 0x1610A with the
same chunk 0x16127 layout as hyperlink range tree pages
(`hyperlinks.md`): u32 next page, u32 previous page, u32 story, u32, u32
*n*, *n* node IDs, u32 node count, then the nodes, each u32 type, u32
size and the node data. Types: 0xBF2F the document node (251, one per
file), 0xBF34 an element (259).

**Node data:** a u16 count, then that many parts, each u32 ID, u32
length and data. Two parts are used:

| Part | Contents |
|---|---|
| 0xBF0D | u32 tag UID (0 for the document node), u32 content UID (0 = none), 22 bytes, parent node key (u32 story, u32 ID; 0, 0 for the document node), the node's own key, u32 *n*, *n* child node keys |
| 0x1032 | u32 start marker ID, u32 end marker ID; 0xFFFFFFFF = none |

All 510 nodes have part 0xBF0D of exactly this size, and the 22 bytes are
the same in all of them. The own key names the story whose store holds
the node (510 of 510). In the InDesign 3.0 and 4.0 files the 22 bytes
are 4 and 20 bytes (`big-endian.md`).

**References to nodes.** Chunk 0xBF14 is a node key:

- In the document (class 0xE01): the backing story and the document node
  (251 of 251).
- In a story: the element whose content is that story (4 stories, all in
  one document; each element's content UID is the story).
- In an object of class 0x2C65: the element whose content it is (1
  object). This object is a child of a page item: its chunk 0x15B names
  the page item as parent.

## Markers

Each node has a start marker and, unless it is an element that stands
for content held elsewhere, an end marker. A marker is a U+FEFF
character in the text of the node's story. Its position is in a tree,
chunk 0xCA57 of the story's strand of class 0x2B8:

- u16: 0 for an empty tree.
- The tree's nodes in preorder, each u16 (0 or 1, not identified), u32
  value, u32 marker ID, u32 length (1 in every sample), u16 flags: 1 =
  has a left child, 2 = has a right child.
- The rest is not used (u32, u32 *n*, then *n* pairs of u32).

The root's value is its position, a left child is *value* before its
parent and a right child *value* after it, as in hyperlink range trees.
Positions count characters (`objects.md`).

Checks: the tree parses in all 17,911 stories (252 trees are not empty).
Of their 1,015 markers, all are on a U+FEFF character, all are named by
a node's part 0x1032, and every marker that a node names is in the tree.
The texts hold another 2,690 U+FEFF characters that are not markers.

## The backing story

The document's chunk 0xBF14 names the backing story, which IDML writes
as `XmlStory` with `Self="u<UID>"` in `XML/BackingStory.xml` (76 of 78
pairs; the other 2 IDML files were exported from another save). Its text
is markers and a paragraph return. The document's chunk 0x222 holds two
UID lists: the stories and then the backing story. IDML `StoryList` is
the two lists in that order (74 of 78; the other 4 are the pairs from
another save, or whose IDML has stories that the INDD no longer has).

## IDML elements

**`Self`** is `d`, then `i` and the hexadecimal node ID of each element
on the path from the root element down to the element: the root element
with ID 2 is `di2`, its child with ID 0xB `di2ib`. The path follows the
parent keys, so it can pass through nodes of several stories (an element
with ID 4 in another story's store, whose parent is `di2ibifi1f`, is
`di2ibifi1fi4`). Matches 84 of 86 IDML elements; the other 2 are the
root elements of the 2 pairs from another save. Root element IDs are 2
(63 pairs) and 3 (15).

**`MarkupTag`** is `XMLTag/` and the name of the tag that the node names
(84 of 84).

**`XMLContent`** (5 elements in the pairs): a story's `Self` when the
content UID is a story (4 of 4); for an object of class 0x2C65, the
`Self` of its parent page item (1 of 1).

## Where IDML writes the elements

IDML writes elements in story text at their markers, and drops the
marker characters:

- An element with a start and an end marker holds the text between them.
- An element with only a start marker is written empty, at the marker.
- The start marker of the document node is not written. Its end marker
  and the U+FEFF characters that are not markers are written as text.
- An element whose content is a story holds all of that story's text, in
  the story's own file: the story's chunk 0xBF14 names it, and IDML
  writes it around the story's paragraph ranges (4 stories).

Nesting with character ranges, from the 78 backing stories and the 4
tagged stories of the pairs:

- Within a character range, an element is written inside the
  `CharacterStyleRange`.
- An element that holds, at any depth, an element with a story as
  content is written between character ranges (a *block* element): the
  open character range ends before it, and a new one opens after its
  start marker and before its end marker. In the one sample with such
  elements, 3 of 4 elements with an end marker are blocks, and IDML
  writes an empty `CharacterStyleRange` before each of them.
- After an empty element with a story as content, the character range
  ends; the following text opens a new one.

With these rules the converter writes all 76 backing stories of the
pairs from the same save exactly as IDML does (elements, character
ranges and U+FEFF text), and the elements of the 4 tagged stories at the
same places.

## Not converted

- No node type other than the document node and elements occurs, so
  XML attributes, comments, processing instructions and DTDs are not
  known.
- No sample has an element whose markers are in different paragraphs.
  The converter leaves such an element out, with a warning, and does not
  write its markers.
- Content of other classes: `XMLContent` is left out, with a warning.
