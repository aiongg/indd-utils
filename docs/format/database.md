# INDD object database

The database pages (between the master pages and the contiguous objects;
see `container.md`) hold an object database: every object has a UID and a
byte stream, and the streams are found through a B+ tree. This layout is
our own analysis of the corpus. It is implemented in `src/database.rs`.

Everything here is little-endian, also in the two big-endian files
(InDesign 3.0 and 4.0), whose byte order flag applies to object data only
(`big-endian.md`).

**Evidence.** The reader in `src/database.rs` checks every rule below
while it reads. It reads every object of every corpus file and fixture, both
big-endian files included, without a single inconsistency
(`tests/corpus.rs`, `every_corpus_object_reads`; `tests/fixtures.rs`,
`fixture_objects_read`). The worked example is a blank
InDesign 19.0 document with 246 database pages.

## Page trailer

Every database page ends with 12 bytes:

| Offset | Size | Field |
|---|---|---|
| 0xFF4 | 4 | Page type |
| 0xFF8 | 4 | Type-specific number (below) |
| 0xFFC | 4 | Probably a checksum (not yet verified) |

The master pages have type 0 and an empty trailer apart from 0xFFC.

| Type | Role | 0xFF8 holds |
|---|---|---|
| 0 | Master page, or an old master page copy | 0 |
| 2 | Allocation directory | Physical page of its partner copy |
| 3 | Allocation bitmap | Physical page of its partner copy |
| 4 | Logical page directory | Physical page of its partner copy |
| 5 | Logical page table | Physical page of its partner copy |
| 6 | Object tree leaf | Its own logical page number |
| 7 | Object tree interior node | Its own logical page number |
| 8 | Data page (one object segment) | 0 |
| 9 | Slotted page (several small records) | Its own logical page number |

Types 2–5 come in pairs of physical pages that name each other (in the
blank document: 3↔4, 5↔6, 7↔8, 9↔10). Each master page points to one
member of each pair. The active master page points to the current copies;
an older master page copy (page 2 in the blank document) points to the
other members.

## Master page fields used

| Offset | Field |
|---|---|
| 0x160 | Physical page of the allocation directory (type 2) |
| 0x3A8 | Physical page of the logical page directory (type 4) |
| 0xB7C | Root (logical page) of the object tree |
| 0xB80 | Root of the class tree |
| 0xB84 | Root of the unclassed-UID tree |
| 0xB88 | u64: entries in the object tree |
| 0xB90 | u64: entries in the class tree |
| 0xB98 | u64: entries in the unclassed-UID tree |
| 0x130 | Number of object segments on data pages (185 in the blank document, equal to the count of such leaf entries) |
| 0x134 | Total bytes of those segments (≈ 3940 × the count above in 8 sampled files) |

Other fields at 0x100–0x3A8 and 0xB78–0xBAC are not yet identified. 0xBAC
holds the document's `xmp.did:` identifier as a NUL-terminated string.

## Logical pages

Tree and slotted pages are addressed by logical page number, so they can
be rewritten to a new physical page without changing their references
(copy on write). Superseded copies stay in the file until their pages are
reused.

- The logical page directory (type 4) holds, at 0x80 + 4·j, the physical
  page of table page *j* (0 = none).
- Each logical page table (type 5) holds, at 0x80 + 4·i, the physical page
  of logical page `j·989 + i` (0 = unmapped).
- 989 entries fit between 0x80 and the trailer. Types 2, 4 and 5 store a
  u32 at 0x00 followed by a bitmap up to 0x7F. In type 5 pages the u32 counts free
  entries (989 − used) and the bitmap has a set bit for each free entry,
  least significant bit first.

Checks: in all 351 files, the trailer number of every mapped page equals
its computed logical number. Two files use a second table page (j = 1).

## Allocation bitmap

Type 3 pages: a u32 count of free pages at 0x00, then one bit per physical
page from 0x04, least significant bit first, set = free. In the blank
document, the clear bits (211) plus the count (32429) equal the bitmap size
(32640 bits), and superseded tree pages are marked free. The reader does
not use the bitmap.

## Object tree

A B+ tree keyed by (UID, segment number).

**Leaf page (type 6).** A u32 entry count at 0x00, then 16-byte entries:

| Offset | Field |
|---|---|
| 0 | Segment number, 1-based |
| 4 | UID |
| 8 | Length field |
| 12 | Page field |

**Interior page (type 7).** A u32 child count *n* at 0x00, the first child
at 0x04, then *n*−1 groups of 12 bytes: segment and UID of a separator
key, then the next child. Children are logical page numbers.

Entries are sorted by (UID, segment). Each object's segments are numbered
1, 2, 3… without gaps. The object's bytes are its segments concatenated in
order.

**Where a segment's bytes are:**

- **Length field < 0x10000:** the segment is the first *length* bytes of
  data page (type 8) *page* (a physical page number). Length is at most
  0xF70 (3952). The rest of the page is zero up to the trailer.
- **Length field ≥ 0x10000:** the segment is a record in a slotted page.
  The high 16 bits are the slot number, the low 16 bits the byte count, and
  *page* is a logical page number.

## Class tree and unclassed-UID tree

Two more B+ trees use the same page types and leaf layout as the object
tree. In both, the segment and page fields are 0.

- **Class tree:** the length field holds the object's class ID. Sorted by
  UID, one entry per UID. In the blank document, UID 1 has class 0xE01.
- **Unclassed-UID tree:** lists UIDs with no class entry; the length field
  is 0. Empty in 288 of 351 files. These are most likely deleted objects
  whose data has not been reclaimed yet: across all files, none of the
  28,521 child references in spread, spread-layer and page-item hierarchy
  chunks points to one of these UIDs, and in the one case examined, an
  IDML exported earlier contains the object while the INDD no longer
  references it. The converter ignores them.

In all 351 files, every object with data is in exactly one of the two
trees. Some UIDs in either tree have no data: 215 class-tree entries in
total, and unclassed entries in 34 files.

## Slotted pages (type 9)

Records are stored from offset 0 upwards. A slot directory grows downwards
from a 24-byte footer at 0xFDC:

| Offset | Field |
|---|---|
| 0xFDC | Slot capacity (16 and 32 in the pages examined) |
| 0xFE0 | Slots in use |
| 0xFE4–0xFF3 | Not yet identified |
| 0xFDC − 4·s | Offset of slot *s*'s record (s ≥ 1) |

In the pages examined, unused directory entries hold a stack of free slot
numbers.

Each record starts with u16 record length (including this 4-byte header,
padded to a multiple of 4, sometimes longer than needed) and u16 slot ID.
Slot ID 0 marks free space.

If bit 0x8000 of the slot ID is set, the record continues elsewhere. Its
first 8 data bytes are a pointer in the same form as a leaf entry
(`slot << 16`, then a logical page number). The bytes after the pointer
are the first part of the data, and the rest is in the record the pointer
names. Example: the last segment of an object of 1983 bytes is split into
1856 bytes plus a 127-byte record in another slotted page.

## Open questions

- The checksum algorithm at 0xFFC.
- The remaining master page fields.
- What the high UIDs (from 0x80000000) are.
