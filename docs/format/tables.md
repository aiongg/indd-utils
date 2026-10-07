# Tables

Implemented in `src/model/table.rs`. Checked against 37 tables in the
corpus pairs (InDesign 12–21); the InDesign 9.2 table layout differs and is
not read yet.

## Where a table sits

The story text contains U+0016 where the table is, usually followed by one
or more U+0017 (internal markers; their count does not follow from the
table size). The story's owned-items strand (run kind 0x209, see
`objects.md`) gives the U+0016 position an object of class 0xB651, whose
chunk 0xB67D starts with the table UID (class 0xB608). IDML writes the
`Table` element in place of U+0016 and drops the U+0017 characters.

**Cell text** is part of the same story text, after the paragraph that
holds the table, one stretch per cell in row-major order, each ending with
U+000D (the cell terminator, not written to IDML). A strand with run kind
0x2A4 assigns every stretch of text an owner: each record is u32 length,
u32 owner UID, u32 cell ID. Text owned by the story is the story's own; text
owned by a table belongs to that table's cell.

IDML names a table `<scope>i<table UID hex>`, where the scope is the story
(`udcf`) or, for a nested table, the enclosing cell; cells are named
`<table>i<cell ID hex>` and rows and columns `<table>Row<n>` and
`<table>Column<n>`, with the index *n* from 0 in lowercase hexadecimal
(`…Row9`, `…Rowa`, `…Row10`; `Name` keeps the decimal index). All 112
tables of the corpus pairs with more than 10 rows or columns, from DOM 8
to 20, use hexadecimal; none uses decimal.

## Table object (class 0xB608)

| Chunk | Contents |
|---|---|
| 0xB608 | u32 rows, u32 columns, u32 header rows, u32 footer rows (24/24 match) |
| 0xB617 | Column groups (below); attribute 0xB60D = `SingleColumnWidth` (24/24) |
| 0xB616 | Row groups; 0xB60C = `SingleRowHeight`, 0xB66E = `MinimumHeight` (24/24) |
| 0xB6FB | u32 count, then (UID, class) pairs of the table's parts |

The row count equals the number of grid rows in the cell data (below),
and the column count is at most the number of positions in its widest
grid row, in all 5,565 tables read from the corpus files (copies of a
file counted again). The converter treats larger counts as damage and leaves
the table out.

**Row and column groups:** u32 group count; per group: u32 number of
rows (columns), u16, u16 attribute count, text attribute records (see
`attributes.md`), 8 bytes.

**Row attributes.** IDML writes on a `Row` exactly the attributes its
row group has (presence matches for all three IDs below in the pairs):

| ID | IDML | Encoding | Evidence |
|---|---|---|---|
| 0xB69F | `AutoGrow` | 0 `false`, 1 `true` | 1,065 rows with 1, 63 with 0 |
| 0x10407 | `StartRow` | 0 `Anywhere`, 2 `NextColumn` | 10 rows (3 files) |
| 0xB6A1 | `KeepWithNextRow` | 0 `false` | 8 rows |

**Table direction.** Chunk 0x50F65 of the table object is a u16: 0 for
`TableDirection="LeftToRightDirection"`, 1 for `RightToLeftDirection`.
All 396 tables of the pairs (all versions) have it; 395 have 0 and are
left to right in IDML, the one table with 1 (DOM 21.5) is right to left.
IDML writes `TableDirection` on every table of every version.

## Cells

The part of class 0xB606 owns a strand (chunk 0x261) whose run data
(chunk 0x262, kind 0xB610) describes the grid. Each run record:

| Field | Contents |
|---|---|
| u32 | Number of attribute sets, *s* |
| *s* sets | u32 number of columns, u16 flag; if the flag is 1: u16 count, attributes; then u32 cell style priority, u32 cell style |
| u32 | Number of rows *r* in this record |
| *r* lists | u32 count, then that many position IDs (one per column) |
| u32 | Number of row records |
| per row | u16 count, then one attribute record per position |

A run record covers rows that share their cell formatting. Its
attribute sets cover the columns from left to right: the first set the
first *n* columns, and so on; the column counts add up to the number of
columns (185 of 185 records with sets; 12 records have none). A set
with flag 0 has no attributes. All 47
cell strands of the corpus pairs parse to the end with this layout. In
files whose sets all have flag 1 the earlier reader worked; files with
flag-0 sets (one pair, 8 tables) could not be read before.

Every grid position has an ID. A position whose attribute is 0xB666
starts a text cell (its ID is the cell ID); attribute 0x10469 starts a
graphic cell (22 cells, one stale pair; IDML
`CellType="GraphicTypeCell"`). Every other position is covered by a
merged cell: attribute 0xB614 (779 positions in the pairs, no values), or
no record at all, when a row record has fewer attribute records than the
row has positions (80 positions; the missing ones are the last).

**Layout record.** The second value of attribute 0xB666 is the cell's
layout:

| Field | Contents |
|---|---|
| u32 | Flags. If the low four bits are 0xF, one more u32 follows the count |
| u32 | Count of parcels (0: the cell was never laid out; the record is 8 bytes) |
| per parcel | u32 parcel flags; if bit 31 is set, one more u32; 16 bytes; f64 text area width; f64 text area height; more |

Only the first parcel's width is used. It is the cell's spanned column
widths minus its left and right text insets (11,558 of 11,700 laid-out
cells of the trustworthy pairs; the others differ by stroke-sized
amounts). Bit 31 of the parcel flags is set in 10,287 of 12,975 records;
without it the width is 4 bytes earlier.

### Spans

Two passes over the grid in row order:

1. Column span. For each starting position, *run* = 1 + the number of
   positions to its right before the next starting position. If the
   layout has a width *w* > 0, the span is the *n* in 1..*run* for which
   the sum of the widths of columns *c* .. *c*+*n*−1 is closest to *w* +
   left + right text inset (the values in effect, below, for the cell
   spanning *n* columns; the first *n* wins a tie). Otherwise the span is
   *run*. Positions *c*+1 .. *c*+span−1 of the row are taken.
2. Row span. For each starting position, count the rows below in which
   all positions *c* .. *c*+span−1 are covered and not taken; stop at the
   first row where one is not. Those positions are taken.

Evidence: all 11,844 cells of the trustworthy pairs and 1,297 of 1,297
cells of stale pairs get `Name` (column:row), `RowSpan` and `ColumnSpan`
right. Without the layout width (span = run everywhere) 78 cells are
wrong: a cell next to a multi-row merged cell would take the covered
position to its right. The smallest *n* whose widths reach *w* fails for
23 cells, 14 of them where the last spanned column is narrower than the
insets (an 18-pt column, insets 26.5). A single pass that resolves row
spans before the next row's column spans fails for 10 cells. Using the
layout height for row spans fails for 6 cells (rows that grew), so
heights are not used.

Cells that were never laid out (parcel count 0) are 1 × 1 cells with
the rule (run 1), as in IDML: 144 cells in 8 tables of one trustworthy
DOM 16 pair, the last three rows of each table, which are overset.

`CellType="TextTypeCell"` is on every text cell from DOM 11 (11,097 of
11,097 cells of the trustworthy pairs) and on none before (0 of 453 cells
of DOM 8 and 10).

## Values in effect

From DOM 11, IDML writes `TextTopInset`, `TextLeftInset`,
`TextBottomInset`, `TextRightInset` and `ClipContentToTextCell` on every
`Table`, `Row`, `Column` and `Cell`, with the value in effect. DOM 8 and
10 write none of them (7 tables, 59 rows, 453 cells); every table, row,
column and cell of DOM 11 to 21 has them (325 tables, 2,737 rows, 1,204
columns, 11,391 cells). The IDs are those of the cell attribute sets:
0xB62B left, 0xB62C top, 0xB62D right, 0xB62E bottom inset; 0xB6DE
`ClipContentToTextCell` (0 = false, 1 = true).

**Cell.** The first of:

1. the cell's own attribute set;
2. the applied cell style (the set's cell style UID), then its based-on
   chain;
3. the cell style of the cell's region in the table's style. Region:
   header rows (row index < header row count) → header; footer rows →
   footer; otherwise a cell in the first column → left column; a cell
   whose last spanned column is the last column → right column;
   otherwise body. If the region's "same as body" flag is 1, the body
   region is used. The region cell style UID and the flag are looked up
   along the table style chain (below); the value is then looked up in
   that cell style and its based-on chain. A region cell style UID of 0
   gives nothing;
4. the table's value.

| Region | Cell style | Same as body |
|---|---|---|
| Header | 0x10450 | 0x10457 |
| Footer | 0x10451 | 0x10458 |
| Body | 0x10452 | |
| Left column | 0x10453 | 0x10459 |
| Right column | 0x10454 | 0x1045A |

Evidence: all 56,955 values of the 11,391 cells (32 files take values
from the cell's set, 4 from a cell style, 4 from a region cell style, 24
from the table). Header rows take precedence over the left and right
column regions (the top-left cell of 2 tables in a DOM 20.2 file needs
it, 4 inset values each; the opposite order gets them wrong). No sample
shows whether the left or the right column region wins for a table of
one column.

**Table.** The table's own attribute list (chunk 0xB668), then the
applied table style (chunk 0xB6FC) and its based-on chain, ending at the
root style `[No table style]`, which has all five. The root style is
taken last even when the chain does not reach it. Evidence: 325 of 325
tables; values 4 (322), 1.417 (2, a root style with 1.417), 3.96 (1,
local values in chunk 0xB668).

**Row.** The values of the first cell that starts in that row (lowest
column). **Column.** The values of the first cell that starts in that
column (lowest row). Evidence: 2,796 of 2,796 rows, 1,267 of 1,267
columns. Taking the cell that covers column 0 (row 0) instead fails for
29 rows (159 columns), where a merged cell from an earlier row (column)
covers that position.

## Cell formatting

A cell's formatting is the attribute set that covers its position. IDML
writes these attributes on a `Cell` exactly when the set has them, so
they are the cell's local values. The exception is the text insets and
`ClipContentToTextCell`: from DOM 11 IDML writes them on every cell,
with the value in effect (above); where the set has the value, it is
that value.

Evidence: the same-version pairs have 1,125 IDML cells with an INDD
cell, 952 of them in a set (5 files). For every attribute below, the INDD set has the
attribute on exactly the cells whose IDML `Cell` has it (no cell has one
without the other), and the values match on all of them. The 144 cells
of pairs whose IDML is from an older version agree as well.

| ID | IDML | Encoding | Cells (files, distinct values) |
|---|---|---|---|
| 0xB62B | `LeftInset`, `TextLeftInset` | f64 | 601 (5, 5) |
| 0xB62C | `TopInset`, `TextTopInset` | f64 | 523 (5, 3) |
| 0xB62D | `RightInset`, `TextRightInset` | f64 | 631 (5, 3) |
| 0xB62E | `BottomInset`, `TextBottomInset` | f64 | 523 (5, 3) |
| 0xB63D | `FillColor` | swatch UID | 176 (4, 8) |
| 0xB63E | `FillTint` | f64 | 142 (2, 4) |
| 0xB677 | `VerticalJustification` | 1 `CenterAlign`, 2 `BottomAlign` | 457 (3) |
| 0xB6DE | `ClipContentToCell` | 0 = false | 664 (5, one value) |
| 0xB645, 0xB646, 0xB647, 0xB648 | `Left`, `Right`, `Top`, `BottomEdgeStrokeWeight` | f64 | 426, 358, 412, 443 (2) |
| 0xB649, 0xB64A, 0xB64B, 0xB64C | `Left`, `Top`, `Right`, `BottomEdgeStrokeColor` | swatch UID | 449, 449, 410, 449 (2) |
| 0xB64D, 0xB64E, 0xB64F, 0xB650 | `Left`, `Right`, `Top`, `BottomEdgeStrokeType` | stroke style code (below) | 426, 358, 412, 443 (2) |
| 0xB6A8, 0xB6A9, 0xB6AA, 0xB6AB | `Left`, `Right`, `Top`, `BottomEdgeStrokeTint` | f64 | 429, 364, 443, 443 (2) |
| 0xB6F9, 0xB6FA, 0xB6FB, 0xB6FC | `Left`, `Right`, `Top`, `BottomEdgeStrokePriority` | u32 | 593, 583, 593, 593 (3, 11–16 values) |

Note the colours: their order is left, top, right, bottom, while the
other edge attributes are left, right, top, bottom. Each assignment was
checked against the alternatives; any swap gives mismatches (for
example 0xB648 as the top weight differs on 10 cells and is present on
31 cells without a top weight). The top and bottom stroke types have one
value (`Solid`), but they are told apart by which cells have them.

**Edge stroke type.** Eight bytes: a stroke style code and 0. Code
0x5A29 is `StrokeStyle/$ID/Solid` (1,570 cell edges), the same code as
for page items (`attributes.md`); 0x1040C is `n`, no stroke type (69
cell edges).
The converter uses the page item stroke style codes for cells too.

**Right edge priority.** 10 cells have a right edge priority in IDML
but not in their set. The converter writes priorities only from the
set.

**Applied cell style.** The set's two u32 after the attributes are the
applied cell style priority and the cell style. All 952 cells in a set
match `AppliedCellStylePriority`, and all match `AppliedCellStyle` (a
cell style UID; 0 for 70 cells whose IDML style is `[None]`). The 173
cells of the pairs in records without sets all have
`AppliedCellStyle="CellStyle/$ID/[None]"` and priority 0, and the
converter writes those values for them.

**Not converted.** The gap colour, gap tint and overprint of the edges
(0x10420–0x10423, 0x1040F–0x10412, 0xB6BA–0xB6BD) have one value in
every cell and the same set of cells for three edges, so the edges
cannot be told apart.

## Cell and table styles

**Cell styles** are objects of class 0x2021A, **table styles** of class
0xB63F. Both have chunk 0x230 as text styles do (`objects.md`): u32 at
offset 4 is the based-on style, and the name is a flag byte (1 for a
built-in name) followed by a non-empty in-object string. Their
attributes are a u16 count and records: chunk 0x20253 for cell styles
(chunk 0x2020C holds the same list in all 142 cell styles), chunk 0xB667
for table styles. The root groups list their styles in chunk 0x2024E
(cell styles) and 0x104E5 (table styles): u32, u32, UID list.

Evidence: all 142 cell styles and 274 table styles of the corpus pairs
match an IDML style by name. Their based-on style matches IDML
`BasedOn` for all 146 styles that have one: a root style
(`[None]`, `[No table style]`) is written as a string, any other as an
object reference (one table style).

Cell styles use the cell attribute IDs above. All 7 named cell styles
of the pairs match their IDML on every attribute the converter writes.
Attribute 0x10463 is the paragraph style UID (`AppliedParagraphStyle`,
4 of 4 styles that have it).

## Table attributes

The table's chunk 0xB668 and a table style's chunk 0xB667 use the same
IDs. IDML writes on a `Table` the attributes its chunk 0xB668 has.

| ID | IDML | Encoding | Evidence |
|---|---|---|---|
| 0xB662 | `SpaceBefore` | f64 | 2 styles (5.67), 5 tables (14.17) |
| 0xB663 | `SpaceAfter` | f64 | 2 styles (11.34), 5 tables |
| 0xB67B | `StartRowFillColor` | swatch UID | 5 tables (None) and 2 styles |
| 0xB67C | `EndRowFillColor` | swatch UID | 5 tables (a colour) and 2 styles |
| 0xB6B0 | `StartRowFillTint` | f64 | 2 styles (10, 50), 3 tables (10) |
| 0xB6B1 | `EndRowFillTint` | f64 | 2 styles (50, 10) |
| 0xB683 | `ColumnFillsPriority` | 0 = false | 8 tables |
| 0xB684 | `StartRowStrokeColor` | swatch UID | 5 tables |
| 0xB688 | `StartRowStrokeType` | stroke style code, as for cell edges | 1 table (`Canned Dotted`) |
| 0xB690 | `StartRowStrokeWeight` | f64 | 5 tables (0.25), 3 styles (0) |
| 0x10450 | `HeaderRegionCellStyle` | cell style UID, 0 = `n` | 2 styles |
| 0x10452 | `BodyRegionCellStyle` | cell style UID | 3 styles, and the root style in 135 files (`[None]`) |
| 0x10453 | `LeftColumnRegionCellStyle` | cell style UID | 1 style |
| 0x10454 | `RightColumnRegionCellStyle` | cell style UID | 1 style |
| 0x10457 | `HeaderRegionSameAsBodyRegion` | 0 false, 1 true | 2 styles (0); root style 1 |

The style evidence is from one document with four named table styles;
the table evidence is from three documents. In these documents the two
spaces, the two fill tints and the two fill colours differ, which tells
each pair apart.

**Not converted** because two or more attributes always have the same
value: the border weights, colours and types (0xB653–0xB65E, three per
side, same on all four sides in every sample), the row and column
stroke and fill counts (0xB67D, 0xB67E, 0xB686, 0xB687, 0xB68C, 0xB68D:
always 1 or always 0 in pairs), the end row and column stroke weights
(0xB691–0xB693), and the left and right column region flags (0x10459,
0x1045A). The alternating-pattern skip counts, `BreakHeaders`,
`SkipFirstHeader` and `SkipLastFooter` of 12 tables in IDML are not in
chunk 0xB668 and were not found.
