//! Tables. See `docs/format/tables.md`.

use super::{Attrs, Reader, TextRun, Value};
use crate::Error;
use crate::object::Cursor;

pub mod class {
    pub const CELL_STYLE: u32 = 0x2021A;
    pub const TABLE_STYLE: u32 = 0xB63F;
    /// Owned by the U+0016 character where a table sits in its story.
    pub const TABLE_ANCHOR: u32 = 0xB651;
    pub const TABLE: u32 = 0xB608;
    /// Owns the cell strand (row groups with cell records).
    pub const CELL_STRAND_OWNER: u32 = 0xB606;
}

pub mod chunk {
    pub const ANCHOR_TABLE: u32 = 0xB67D;
    pub const TABLE_SIZE: u32 = 0xB608;
    pub const TABLE_ROWS: u32 = 0xB616;
    pub const TABLE_COLUMNS: u32 = 0xB617;
    pub const TABLE_PARTS: u32 = 0xB6FB;
    /// u32 applied table style, u32 (not identified).
    pub const TABLE_STYLE: u32 = 0xB6FC;
    /// Local table attributes: u16 count, attributes.
    pub const TABLE_ATTRS: u32 = 0xB668;
    /// Attributes of a cell style: u16 count, attributes.
    pub const CELL_STYLE_ATTRS: u32 = 0x20253;
    /// Attributes of a table style: u16 count, attributes.
    pub const TABLE_STYLE_ATTRS: u32 = 0xB667;
    /// Children of the root cell style group: u32, u32, UID list.
    pub const CELL_STYLE_ROOT_CHILDREN: u32 = 0x2024E;
    /// Children of the root table style group: u32, u32, UID list.
    pub const TABLE_STYLE_ROOT_CHILDREN: u32 = 0x104E5;
}

/// Attribute IDs in table data.
pub mod attr {
    pub const COLUMN_WIDTH: u32 = 0xB60D;
    pub const ROW_HEIGHT: u32 = 0xB60C;
    pub const ROW_MIN_HEIGHT: u32 = 0xB66E;
    /// 0 = `AutoGrow="false"`.
    pub const ROW_AUTO_GROW: u32 = 0xB69F;
    /// A grid position that starts a cell; its value holds cell geometry.
    pub const CELL: u32 = 0xB666;
    /// A grid position covered by a merged cell.
    pub const COVERED: u32 = 0xB614;
}

#[derive(Debug, Clone, PartialEq)]
pub struct Row {
    pub height: Option<f64>,
    pub min_height: Option<f64>,
    /// Stored auto-grow value (0 in every sample, `AutoGrow="false"`).
    pub auto_grow: Option<u32>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Cell {
    /// Cell ID, local to the table (IDML `Self` suffix `i<hex>`).
    pub id: u32,
    pub row: usize,
    pub column: usize,
    pub row_span: usize,
    pub column_span: usize,
    pub runs: Vec<TextRun>,
    /// Formatting of the cell: its attribute set, if it has one.
    pub format: Option<CellFormat>,
}

/// An attribute set shared by a run of cells in a row group.
#[derive(Debug, Clone, PartialEq)]
pub struct CellFormat {
    /// Local cell attributes (insets, fill, edge strokes, ...).
    pub attrs: Attrs,
    /// Applied cell style priority (IDML `AppliedCellStylePriority`).
    pub style_priority: u32,
    /// Applied cell style UID, 0 for none.
    pub style: u32,
}

/// A cell or table style (class 0x2021A or 0xB63F).
#[derive(Debug, Clone, PartialEq)]
pub struct TableStyle {
    pub uid: u32,
    pub name: String,
    /// The name is a built-in key, written with `$ID/` in IDML.
    pub builtin: bool,
    pub based_on: Option<u32>,
    pub attrs: Attrs,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Table {
    pub uid: u32,
    /// Applied table style.
    pub style: Option<u32>,
    /// Local table attributes.
    pub attrs: Attrs,
    pub header_rows: u32,
    pub footer_rows: u32,
    pub rows: Vec<Row>,
    pub columns: Vec<f64>,
    pub cells: Vec<Cell>,
}

/// Groups of rows or columns sharing attributes: u32 group count; per
/// group u32 count, u16, u16 attribute count, attributes, 8 bytes.
fn groups(data: &[u8]) -> Result<Vec<(usize, Attrs)>, Error> {
    let mut c = Cursor::new(data);
    let n = c.u32()?;
    let mut out = Vec::new();
    for _ in 0..n {
        let count = c.u32()? as usize;
        c.u16()?;
        let na = c.u16()? as usize;
        let attrs = Attrs::parse_text(&mut c, na)?;
        c.skip(8)?;
        out.push((count, attrs));
    }
    Ok(out)
}

fn f64_attr(a: &Attrs, id: u32) -> Option<f64> {
    a.get(id).and_then(Value::as_f64)
}

/// One grid position from the cell strand.
struct Position {
    id: u32,
    /// Text area width and height, for positions that start a cell.
    size: Option<(f64, f64)>,
    /// Covered by a merged cell.
    covered: bool,
    format: Option<CellFormat>,
}

impl Reader<'_> {
    /// An attribute list stored as a u16 count and records.
    fn counted_attrs(&self, uid: u32, id: u32) -> Result<Attrs, Error> {
        Ok(match self.chunk(uid, id)? {
            Some(d) if d.len() >= 2 => {
                let mut c = Cursor::new(&d);
                let n = c.u16()? as usize;
                Attrs::parse_text(&mut c, n)?
            }
            _ => Attrs::default(),
        })
    }

    /// A cell or table style: name and base from chunk 0x230 (as for text
    /// styles), attributes from `attrs_chunk`.
    pub(super) fn table_style(
        &self,
        uid: u32,
        attrs_chunk: u32,
    ) -> Result<Option<TableStyle>, Error> {
        let Some(data) = self.chunk(uid, super::chunk::STYLE_INFO)? else {
            return Ok(None);
        };
        let based_on = Cursor::new(&data[4.min(data.len())..]).u32()?;
        // A flag byte (1 = built-in name), then a non-empty in-object string.
        let Some((at, name)) = (12..data.len().saturating_sub(6)).find_map(|i| {
            if data[i] > 2 || data[i + 1] != 2 {
                return None;
            }
            match Cursor::new(&data[i + 1..]).string() {
                Ok(n) if !n.is_empty() => Some((i, n)),
                _ => None,
            }
        }) else {
            return Ok(None);
        };
        Ok(Some(TableStyle {
            uid,
            name,
            builtin: data[at] == 1,
            based_on: super::uid_or_none(based_on),
            attrs: self.counted_attrs(uid, attrs_chunk)?,
        }))
    }

    /// The table owned by a table anchor object, without cell text.
    pub(super) fn table_of_anchor(&self, anchor: u32) -> Result<Option<Table>, Error> {
        let Some(d) = self.chunk(anchor, chunk::ANCHOR_TABLE)? else {
            return Ok(None);
        };
        let uid = Cursor::new(&d).u32()?;
        if self.class(uid) != Some(class::TABLE) {
            return Ok(None);
        }
        self.table(uid).map(Some)
    }

    fn table(&self, uid: u32) -> Result<Table, Error> {
        let size = self.required(uid, chunk::TABLE_SIZE)?;
        let mut c = Cursor::new(&size);
        let nrows = c.u32()? as usize;
        let ncols = c.u32()? as usize;
        let header_rows = c.u32()?;
        let footer_rows = c.u32()?;

        let mut columns = Vec::with_capacity(ncols);
        if let Some(d) = self.chunk(uid, chunk::TABLE_COLUMNS)? {
            for (count, a) in groups(&d)? {
                let w = f64_attr(&a, attr::COLUMN_WIDTH).unwrap_or(0.0);
                columns.extend(std::iter::repeat_n(w, count));
            }
        }
        columns.resize(ncols, columns.last().copied().unwrap_or(0.0));
        let mut rows = Vec::with_capacity(nrows);
        if let Some(d) = self.chunk(uid, chunk::TABLE_ROWS)? {
            for (count, a) in groups(&d)? {
                let row = Row {
                    height: f64_attr(&a, attr::ROW_HEIGHT),
                    min_height: f64_attr(&a, attr::ROW_MIN_HEIGHT),
                    auto_grow: a.get(attr::ROW_AUTO_GROW).and_then(Value::as_u32),
                };
                rows.extend(std::iter::repeat_n(row, count));
            }
        }
        rows.resize(
            nrows,
            Row {
                height: None,
                min_height: None,
                auto_grow: None,
            },
        );

        let grid = self.cell_grid(uid)?;
        let cells = cells_from_grid(&grid, &rows, &columns);
        let style = match self.chunk(uid, chunk::TABLE_STYLE)? {
            Some(d) => super::uid_or_none(Cursor::new(&d).u32()?),
            None => None,
        };
        let attrs = self.counted_attrs(uid, chunk::TABLE_ATTRS)?;
        Ok(Table {
            uid,
            style,
            attrs,
            header_rows,
            footer_rows,
            rows,
            columns,
            cells,
        })
    }

    /// Grid positions by row, from the cell strand's row groups.
    fn cell_grid(&self, table: u32) -> Result<Vec<Vec<Position>>, Error> {
        let parts = self.required(table, chunk::TABLE_PARTS)?;
        let mut c = Cursor::new(&parts);
        let n = c.u32()?;
        let mut owner = None;
        for _ in 0..n {
            let u = c.u32()?;
            if c.u32()? == class::CELL_STRAND_OWNER {
                owner = Some(u);
            }
        }
        let Some(owner) = owner else {
            return Ok(Vec::new());
        };
        let mut grid = Vec::new();
        let Some(list) = self.chunk(owner, super::chunk::STRAND_DATA)? else {
            return Ok(grid);
        };
        let mut lc = Cursor::new(&list);
        let count = lc.u16()?;
        for _ in 0..count {
            lc.u32()?;
            let data = self.required(lc.u32()?, super::chunk::STRAND_RUNS)?;
            let mut r = Cursor::new(&data);
            r.skip(8)?;
            let groups = r.u16()?;
            for _ in 0..groups {
                let size = r.u32()? as usize;
                let mut g = Cursor::new(r.bytes(size)?);
                // Attribute sets, each shared by a run of columns: u32
                // number of columns, u16 1 if attributes follow (u16 count,
                // attributes), u32 cell style priority, u32 cell style.
                let sets = g.u32()?;
                let mut formats = Vec::new();
                for _ in 0..sets {
                    let columns = g.u32()? as usize;
                    let attrs = if g.u16()? != 0 {
                        let na = g.u16()? as usize;
                        Attrs::parse_text(&mut g, na)?
                    } else {
                        Attrs::default()
                    };
                    let style_priority = g.u32()?;
                    let style = g.u32()?;
                    let f = CellFormat {
                        attrs,
                        style_priority,
                        style,
                    };
                    formats.extend(std::iter::repeat_n(f, columns));
                }
                let nr = g.u32()?;
                let mut ids = Vec::new();
                for _ in 0..nr {
                    let k = g.u32()?;
                    ids.push((0..k).map(|_| g.u32()).collect::<Result<Vec<_>, _>>()?);
                }
                let nrec = g.u32()?;
                for (i, row_ids) in ids.into_iter().enumerate() {
                    if i as u32 >= nrec {
                        break;
                    }
                    let n = g.u16()? as usize;
                    let raw = raw_attrs(&mut g, n)?;
                    let row = row_ids
                        .into_iter()
                        .zip(raw)
                        .enumerate()
                        .map(|(k, (id, (aid, value)))| Position {
                            id,
                            size: (aid == attr::CELL).then(|| cell_size(&value)).flatten(),
                            covered: aid == attr::COVERED,
                            format: formats.get(k).cloned(),
                        })
                        .collect();
                    grid.push(row);
                }
            }
        }
        Ok(grid)
    }
}

/// Attribute records keeping the raw payload of their second value.
fn raw_attrs(c: &mut Cursor, n: usize) -> Result<Vec<(u32, Vec<u8>)>, Error> {
    let mut out = Vec::with_capacity(n);
    for _ in 0..n {
        let id = c.u32()?;
        let size = c.u16()? as usize;
        let payload = c.bytes(size)?;
        let mut p = Cursor::new(payload);
        let mut second = Vec::new();
        if size >= 2 {
            let count = p.u16()?;
            for i in 0..count {
                p.u32()?;
                let len = p.u16()? as usize;
                let data = p.bytes(len)?;
                if i == 1 {
                    second = data.to_vec();
                }
            }
        }
        out.push((id, second));
    }
    Ok(out)
}

/// Text area width and height of a cell record.
fn cell_size(v: &[u8]) -> Option<(f64, f64)> {
    let f = |o: usize| {
        v.get(o..o + 8)
            .map(|b| f64::from_le_bytes(b.try_into().unwrap()))
    };
    Some((f(32)?, f(40)?))
}

/// Cells from the grid: a position with geometry starts a cell. Its text
/// area is its spanned width and height minus the insets, so its span is
/// the smallest number of columns (rows) whose sizes reach the text area.
fn cells_from_grid(grid: &[Vec<Position>], rows: &[Row], columns: &[f64]) -> Vec<Cell> {
    let mut cells = Vec::new();
    for (r, row) in grid.iter().enumerate() {
        for (c, pos) in row.iter().enumerate() {
            let Some((w, h)) = pos.size else { continue };
            let span = |sizes: &[f64], target: f64| -> usize {
                let mut sum = 0.0;
                for (k, s) in sizes.iter().enumerate() {
                    sum += s;
                    if sum >= target - 1e-3 {
                        return k + 1;
                    }
                }
                sizes.len().max(1)
            };
            let heights: Vec<f64> = rows[r.min(rows.len())..]
                .iter()
                .map(|x| x.height.unwrap_or(0.0))
                .collect();
            // A cell that was never laid out has no geometry; then take the
            // covered positions to its right as its span.
            let column_span = if w > 0.0 {
                span(&columns[c.min(columns.len())..], w)
            } else {
                1 + row[c + 1..].iter().take_while(|p| p.covered).count()
            };
            let row_span = if h > 0.0 { span(&heights, h) } else { 1 };
            cells.push(Cell {
                id: pos.id,
                row: r,
                column: c,
                row_span,
                column_span,
                runs: Vec::new(),
                format: pos.format.clone(),
            });
        }
    }
    cells
}
