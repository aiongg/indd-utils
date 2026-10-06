# Tables

Implemented in `src/model/table.rs`. Checked against 24 tables in the
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
`<table>Column<n>`.

## Table object (class 0xB608)

| Chunk | Contents |
|---|---|
| 0xB608 | u32 rows, u32 columns, u32 header rows, u32 footer rows (24/24 match) |
| 0xB617 | Column groups (below); attribute 0xB60D = `SingleColumnWidth` (24/24) |
| 0xB616 | Row groups; 0xB60C = `SingleRowHeight`, 0xB66E = `MinimumHeight` (24/24) |
| 0xB6FB | u32 count, then (UID, class) pairs of the table's parts |

**Row and column groups:** u32 group count; per group: u32 number of
rows (columns), u16, u16 attribute count, text attribute records (see
`attributes.md`), 8 bytes.

## Cells

The part of class 0xB606 owns a strand (chunk 0x261) whose run data
(chunk 0x262, kind 0xB610) describes the grid. Each run record:

| Field | Contents |
|---|---|
| u32 | Number of attribute sets, *s* |
| *s* sets | u32, u16, u16 attribute count, attributes, u32, u32 |
| u32 | Number of rows *r* in this record |
| *r* lists | u32 count, then that many position IDs (one per column) |
| u32 | Number of row records |
| per row | u16 count, then one attribute record per position |

Every grid position has an ID. A position whose attribute is 0xB666 starts
a cell (its ID is the cell ID); 0xB614 marks a position covered by a merged
cell. The second value of attribute 0xB666 holds the cell's text area: f64
width at offset 32 and f64 height at offset 40, the cell's full size minus
its insets. The converter takes the smallest number of columns (rows)
whose widths (heights) reach the text area as the span. A cell that was
never laid out has width and height 0; then its column span is the run of
covered positions to its right, and its row span 1.

All 628 cells produced for the pairs match IDML `Name` (column:row),
`RowSpan` and `ColumnSpan`.

Not yet converted: cell, row and column formatting (insets, fills,
strokes), table and cell styles.
