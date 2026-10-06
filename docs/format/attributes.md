# Attribute lists

Formatting is stored as attribute lists (on page items: chunk 0x6E03).
Implemented in `src/model/attrs.rs`.

## Layout

u32 count, then for each attribute:

| Size | Field |
|---|---|
| 4 | Attribute ID |
| 2 | Payload size *n* |
| *n* | Payload: u16 value count, then values |

Each value: u32 type, u16 length, data. The first value is the attribute's
value. Every attribute in the samples has a second value of type 0x6E63
with 6 bytes, meaning unknown.

| Type | Length | Value |
|---|---|---|
| 0x6E68 | 8 | f64 |
| 0x6E67 | 4 | i32 |
| 0x6E65 | 2 | u16 (enumeration or boolean) |
| 0x6E69 | 16 | two f64 (a point) |
| 0x117 | 4 | UID reference |

All 2,730 page item chunks of this layout in the samples parse exactly.

## Page item attributes

The list holds the item's local values. IDML writes an attribute only when
it differs from the applied object style, so a value can be in the INDD
list and absent from the IDML.

| ID | IDML attribute | Evidence: IDML value equals INDD value |
|---|---|---|
| 0x6E68 | `FillColor` (swatch UID) | 1,190 of 1,194 |
| 0x6E69 | `FillTint` | 10 of 10 |
| 0x6E64 | `StrokeColor` (swatch UID) | 78 of 82 |
| 0x6E65 | `StrokeWeight` | 171 of 171 |
| 0x6E6D | `MiterLimit` | 778 of 778 |
