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
writes the attributes below on a `Cell` exactly when the set has them,
so they are the cell's local values. The exceptions are the text insets
and `ClipContentToTextCell`, written on every cell from DOM 11 with the
value in effect (above), and the edges (below).

Evidence: the INDD set has each attribute below on exactly the cells
whose IDML `Cell` has it, and the values match on all of them. The
counts of the insets, fills and 0xB6DE are from a first sample of 1,125
IDML cells (952 in a set, 5 files); the others, and codes 0 and 3 of
0xB677, are counted over the trustworthy pairs (11,844 cells in 53
files), where every attribute of the table now matches on every cell.

| ID | IDML | Encoding | Cells (files) |
|---|---|---|---|
| 0xB62B | `LeftInset`, `TextLeftInset` | f64 | 601 (5) |
| 0xB62C | `TopInset`, `TextTopInset` | f64 | 523 (5) |
| 0xB62D | `RightInset`, `TextRightInset` | f64 | 631 (5) |
| 0xB62E | `BottomInset`, `TextBottomInset` | f64 | 523 (5) |
| 0x10470, 0x10471, 0x10472, 0x10473 | `GraphicLeftInset`, `GraphicTopInset`, `GraphicRightInset`, `GraphicBottomInset` | f64 | 96 (2), all 1; order below |
| 0xB63D | `FillColor` | swatch UID | 176 (4) |
| 0xB63E | `FillTint` | f64 | 142 (2) |
| 0xB639 | `OverprintFill` | u16, 0 = false | 153 (1); 6 cell styles |
| 0xB677 | `VerticalJustification` | 0 `TopAlign`, 1 `CenterAlign`, 2 `BottomAlign`, 3 `JustifyAlign` | 5,361 (21); 383 cells with 0 (1 file); 3 in 1 cell style |
| 0xB676 | `FirstBaselineOffset` | 0 `LeadingOffset`, 1 `AscentOffset` | 4,445 (3); 2 cell styles |
| 0xB6E1 | `WritingDirection` | 1 = true | 4,445 (3) |
| 0xB675 | `RotationAngle` | f64 | 1 (270) |
| 0xB6DE | `ClipContentToCell` | 0 = false | 664 (5, one value) |
| 0xB6DC | `DiagonalLineStrokeOverprint` | u16, 0 = false | 16 (1) |

`TextLeftInset` and the other three exist from DOM 11. Before, IDML
writes only `LeftInset` and the others: the 453 cells of DOM 8 and 10
have no `TextTopInset` (28 of them, in one DOM 8.1 pair, have
`TopInset`), and 5 named cell styles of DOM 8 and 10 with `TopInset`
have no `TextTopInset`, while all 25 of DOM 11 to 21 with `TopInset`
have both.

0xB6E1 and 0xB676 always occur together on cells, both with value 1.
Cell styles can have 0xB6E1 too, but the IDML schema has no
`WritingDirection` on `CellStyle`, so it is not written there.
0xB676 is the baseline offset because cell styles have it alone (2
styles). The graphic insets are all equal on cells; in cell styles
{0x10470, 0x10472} are left and right and {0x10471, 0x10473} top and
bottom (values 3 and 0); the order inside each pair follows the text
insets.

**Applied cell style.** The set's two u32 after the attributes are the
applied cell style priority and the cell style. All 952 cells in a set
of the first sample match `AppliedCellStylePriority`, and all match
`AppliedCellStyle` (a cell style UID; 0 for 70 cells whose IDML style is
`[None]`). The 173 cells of the pairs in records without sets all have
`AppliedCellStyle="CellStyle/$ID/[None]"` and priority 0, and the
converter writes those values for them.

## Cell edges

| Attribute | Left | Right | Top | Bottom | Encoding |
|---|---|---|---|---|---|
| `…EdgeStrokeWeight` | 0xB645 | 0xB646 | 0xB647 | 0xB648 | f64 |
| `…EdgeStrokeColor` | 0xB649 | 0xB64B | 0xB64A | 0xB64C | swatch UID |
| `…EdgeStrokeType` | 0xB64D | 0xB64E | 0xB64F | 0xB650 | stroke style code |
| `…EdgeStrokeTint` | 0xB6A8 | 0xB6A9 | 0xB6AA | 0xB6AB | f64 |
| `…EdgeStrokePriority` | 0xB6F9 | 0xB6FA | 0xB6FB | 0xB6FC | u32 |
| `…EdgeStrokeGapTint` | 0x1040F | 0x10410 | 0x10411 | 0x10412 | f64 |
| `…EdgeStrokeGapColor` | 0x10420 | 0x10421 | 0x10422 | 0x10423 | swatch UID |
| `…EdgeStrokeOverprint` | 0xB6BA | 0xB6BB | 0xB6BC | 0xB6BD | u16, 0 = false |
| `…EdgeStrokeGapOverprint` | 0x10431 | 0x10432 | 0x10433 | 0x10434 | u16, 0 = false |

Note the colours: their order is left, top, right, bottom, while the
other edge attributes are left, right, top, bottom. Each assignment is
the only one with no presence mismatch once the rule below is applied
(for example 0xB648 as the top weight differs on 10 cells and is present
on 31 cells without a top weight; 0x10410 as the left gap tint would be
present on 58 cells without one). Evidence for the last four groups
(trustworthy cells with the attribute): gap tint 7,767 / 7,713 / 8,190 /
8,150 (9 files), gap colour 7,698 / 7,646 / 7,922 / 7,880 (9 files),
overprint 7,540 / 7,514 / 7,962 / 7,947 (9 files), gap overprint 174 /
79 / 865 / 834 (2 files).

**Which set holds an edge.** An edge is held by the grid positions along
it: the left edge by the positions of the cell's first column in all its
rows, the right edge by its last column, the top edge by its first row,
the bottom edge by its last row. IDML writes an edge attribute when
every position along the edge has it with the same value, and writes that
value. For a 1 × 1 cell this is its own set. Evidence: 47,376 edge
values per attribute kind (46,844 one-position edges, 532 longer edges):
presence and value match for weight, colour, type, tint, gap tint, gap
colour, overprint and gap overprint, with one exception (a bottom weight
of a 5-column cell in a row whose records are short; IDML writes 0 for
values 3, 1, 3, 0, 1). Taking the cell's own set for all edges misses
144 right colours, 45 bottom colours and their weights, types and tints
in 6 files.

**Priority** is not taken along the edge. It is the value in the cell's
own set for all four edges. If the own set has no right (bottom)
priority, the cell spans more than one column (row), and a position
along the right (bottom) edge has one, IDML writes `1`. Evidence: 11,578
one-position cells (all four edges), 266 merged cells; the `1` case is
157 right and 45 bottom edges, with no counter-example; 4 + 12 merged
cells whose edge positions have no priority have none in IDML.

**Values.**

- Tint and gap tint −1 are written as `100` (2,712 stroke tints in 4
  files, 1,200 gap tints in 1 file); other values as they are.
  `FillTint` −1 stays `-1`.
- Weight: when the edge's colour, as written, is the `None` swatch, IDML
  writes weight `0` whatever is stored (428 edges in 2 files with stored
  weights 0.35 to 3; every other edge with a `None` colour stores 0).
- Neither rule holds for cell styles: one style stores tint −1 on all
  four edges and IDML writes `-1`; two styles store weight 1 with the
  `None` colour and IDML writes `1`.
- Colour or gap colour UID 0 is not a swatch and is not written (209
  edges in 1 file have it, none in IDML).
- Why −1 becomes 100 and why a `None` colour zeroes the weight is not
  known; both are seen without exception.

**Stroke type.** Eight bytes: a stroke style code and 0. Code 0x5A29 is
`StrokeStyle/$ID/Solid`, the same code as for page items
(`attributes.md`); 0x1040C is `n`, no stroke type. Code 0x5A3F is
`StrokeStyle/$ID/Japanese Dots` (13,409 edges in cell sets and cell
styles, 3 files) and 0xB007 `ThickThick` (one table border, all four
sides). These two are used for cells and tables only: page item stroke
types are not checked against them.

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

Cell styles use the cell attribute IDs above, except the edges.
Attribute 0x10463 is the paragraph style UID (`AppliedParagraphStyle`,
4 of 4 styles that have it). A style without 0x10463 has
`AppliedParagraphStyle="ParagraphStyle/$ID/[No paragraph style]"`, even
when its based-on style has a paragraph style: 26 named cell styles
without 0x10463 (7 of them based on another named style) and 495 root
styles.

**Edge IDs in cell styles are rotated.** A cell edge ID gives a
different edge in a cell style: cell left is style top, cell right is
style bottom, cell top is style right, cell bottom is style left.
Evidence: 8 named styles in 3 trustworthy files whose edges differ (a
header row style with top and bottom 0.25 Black and left and right 0
None; a body style with right and bottom 1 and no left; others). Weights
show the rotation for all four edges, colours and tints for top and
bottom. The other IDs follow the same rotation by analogy: every sample
has equal left and right colours and tints, equal stroke types on all
four edges, and equal gap tints, gap colours and overprints (3 styles).
Text insets are not rotated (a style with left and right 10, top and
bottom 5 has 0xB62B = 10, 0xB62C = 5).

## Table attributes

The table's chunk 0xB668 and a table style's chunk 0xB667 use the same
IDs. IDML writes on a `Table` exactly the attributes its chunk 0xB668
has (332 tables of the trustworthy pairs, no presence mismatch for any
ID below), and on a named `TableStyle` those of its chunk 0xB667.

| ID | IDML | Encoding | Evidence |
|---|---|---|---|
| 0xB662 / 0xB663 | `SpaceBefore` / `SpaceAfter` | f64 | 2 styles (5.67, 11.34), 5 tables; they differ |
| 0xB686 / 0xB687 | `StartRowStrokeCount` / `EndRowStrokeCount` | u32 | 69 tables (6 files); 8 tables and styles with 1 / 2 tell them apart |
| 0xB68C / 0xB68D | `StartColumnStrokeCount` / `EndColumnStrokeCount` | u32 | 64 tables; 5 samples with 1 / 2 |
| 0xB67D / 0xB67E | `StartRowFillCount` / `EndRowFillCount` | u32 | 8 tables, 13 styles; 1 style with 1 / 2 |
| 0xB684 / 0xB685 | `StartRowStrokeColor` / `EndRowStrokeColor` | swatch UID | 68 tables, 6 styles; told apart by presence |
| 0xB68A / 0xB68B | `StartColumnStrokeColor` / `EndColumnStrokeColor` | swatch UID | 63 tables, 4 styles; presence |
| 0xB690, 0xB691, 0xB692, 0xB693 | `StartRowStrokeWeight`, `EndRowStrokeWeight`, `StartColumnStrokeWeight`, `EndColumnStrokeWeight` | f64 | 61 / 55 / 56 / 55 tables; a style with 4 / 0 and 3 styles by presence tell them apart |
| 0xB688, 0xB689 | `StartRowStrokeType`, `EndRowStrokeType` | stroke style code | 9 tables (1 `Canned Dotted`); one style by presence |
| 0xB6B4, 0xB6B5, 0xB6B6, 0xB6B7 | `StartRowStrokeTint`, `EndRowStrokeTint`, `StartColumnStrokeTint`, `EndColumnStrokeTint` | f64 | 3 styles (50 / 70); root style 100 / 50 / 100 / 50 in 10 documents |
| 0xB683 | `ColumnFillsPriority` | 0 = false | 8 tables |
| 0xB67B, 0xB67C, 0xB67F, 0xB680 | `StartRowFillColor`, `EndRowFillColor`, `StartColumnFillColor`, `EndColumnFillColor` | swatch UID | 5 tables, 2 styles; root style Black / None / Black / None |
| 0xB6B0, 0xB6B1, 0xB6B2, 0xB6B3 | `StartRowFillTint`, `EndRowFillTint`, `StartColumnFillTint`, `EndColumnFillTint` | f64 | 2 styles (10, 50), 3 tables; root style 20 / 100 / 20 / 100 |
| 0xB695 | `SkipFirstAlternatingFillRows` | u32 | 5 tables (value 1), 7 styles |
| 0xB6E2 | `SkipFirstAlternatingStrokeRows` | u32 | 5 tables (value 3) |
| 0x10470, 0x10471, 0x10472, 0x10473 | `GraphicLeftInset`, `GraphicTopInset`, `GraphicRightInset`, `GraphicBottomInset` | f64 | 7 tables, root style |
| 0x10478 | `ClipContentToGraphicCell` | u16, 0 = false | 7 tables, root style |

Tables only (the IDML schema has none of them on `TableStyle`):

| ID | IDML | Encoding | Evidence |
|---|---|---|---|
| 0xB62B, 0xB62C, 0xB62D, 0xB62E | `LeftInset`, `TopInset`, `RightInset`, `BottomInset` | f64 | 1 table |
| 0x10408 | `BreakHeaders` | u32, 2 = `OncePerPage` | 6 tables (2 files) |

Table styles only (region cell styles, see "Values in effect"):

| ID | IDML | Encoding | Evidence |
|---|---|---|---|
| 0x10450, 0x10451, 0x10452, 0x10453, 0x10454 | `HeaderRegionCellStyle`, `FooterRegionCellStyle`, `BodyRegionCellStyle`, `LeftColumnRegionCellStyle`, `RightColumnRegionCellStyle` | cell style UID, 0 = `n` | named styles; the root style has 0x10452 (`[None]`) in every file |
| 0x10457, 0x10458, 0x10459, 0x1045A | `HeaderRegionSameAsBodyRegion`, `FooterRegionSameAsBodyRegion`, `LeftColumnRegionSameAsBodyRegion`, `RightColumnRegionSameAsBodyRegion` | u16, 0 false, 1 true | 5 to 12 styles each; the left and right flags are present exactly with the matching region cell style |

**Groups that are always equal.** In every sample the IDs of each group
below have the same value, so which ID is which attribute is not known.
IDML writes the attributes of a group together. The converter writes all
of them, with that value, only when every ID of the group is present
with the same value: this needs no assignment and covers every sample.

| IDs | IDML | Encoding | Evidence |
|---|---|---|---|
| 0xB653, 0xB656, 0xB659, 0xB65C | `Top`, `Left`, `Bottom`, `RightBorderStrokeWeight` | f64 | 57 tables, 18 styles |
| 0xB654, 0xB657, 0xB65A, 0xB65D | the four `…BorderStrokeColor` | swatch UID | 12 tables, 17 styles |
| 0xB655, 0xB658, 0xB65B, 0xB65E | the four `…BorderStrokeType` | stroke style code | 11 tables (`n`, `ThickThick`, `Canned Dotted`) |
| 0xB6AC, 0xB6AD, 0xB6AE, 0xB6AF | the four `…BorderStrokeTint` | f64 | 3 styles |
| 0x10429, 0x1042A, 0x1042B, 0x1042C | the four `…BorderStrokeGapColor` | swatch UID | 1 table |
| 0xB68E, 0xB68F | `StartColumnStrokeType`, `EndColumnLineStyle` | stroke style code | 8 tables (1 file, `n`) |
| 0x1040A, 0x1040B | `SkipFirstHeader`, `SkipLastFooter` | u16, 1 = true | 5 tables (tables only) |

**Stroke type values.** All stroke type attributes of tables (0xB655,
0xB658, 0xB65B, 0xB65E, 0xB688, 0xB689, 0xB68E, 0xB68F) are eight bytes,
a stroke style code and 0, as for cell edges.

**Root table style.** `[No table style]` (class 0xB63F) has a full
attribute list in chunk 0xB667. IDML writes on it:

- from DOM 11: `TextTopInset` and the other text insets (0xB62C, 0xB62B,
  0xB62E, 0xB62D), `ClipContentToTextCell` (0xB6DE), the graphic insets
  (0x10470–0x10473) and `ClipContentToGraphicCell` (0x10478). DOM 7 to
  10 have none (135 pairs); DOM 11 to 21 all (380 trustworthy pairs).
  Values: insets 4 (373) or 1.417 (7, a template family); the others 0
  and false.
- in every version, the attributes every root table style has
  (`idml-values.md`). Some of them differ between documents: 10
  trustworthy documents have `SpaceBefore`/`SpaceAfter` 2.83, border,
  row and column stroke weights 0.709 and end stroke tints 50 where the
  others have 4 / −4, 1, 0.25 and 100. All 495 match the INDD value. The
  converter writes the INDD value of every attribute above that the
  observed root values have, and leaves out the others.

The alternating-pattern skip counts not listed above and the table-only
attributes of one table (0xB634/0xB637, `DefaultRowStrokeWeight` and
`DefaultColumnStrokeWeight`, equal so their order is not shown; 0xB66F
`MaximumHeight`; 0xB6D8 `DiagonalLineStrokeWeight`), `NumHeaderColumns`
and `HeaderColumnsPosition` (DOM 21.5, 1 table, no INDD attribute
found), `ColumnType` on cells and columns, cell `P.Label` and
`ECTablePopulationData` are not converted.
