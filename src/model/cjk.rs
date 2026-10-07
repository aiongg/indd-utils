//! Composite fonts, kinsoku tables and mojikumi tables (CJK typography).
//! See `docs/format/fonts.md` and `docs/format/objects.md`.

use crate::Error;
use crate::object::Cursor;

pub mod class {
    pub const COMPOSITE_FONT: u32 = 0xCB02;
    pub const COMPOSITE_FONT_ENTRY: u32 = 0xCB03;
    /// Kinsoku tables: hard, soft, Korean, Simplified Chinese, Traditional
    /// Chinese and custom.
    pub const KINSOKU: [u32; 6] = [0x4209, 0x420A, 0x42B4, 0x42B5, 0x42B6, CUSTOM_KINSOKU];
    pub const CUSTOM_KINSOKU: u32 = 0x4204;
    pub const MOJIKUMI: u32 = 0x4206;
}

pub mod chunk {
    pub const COMPOSITE_FONT: u32 = 0xCB02;
    pub const COMPOSITE_FONT_ENTRY: u32 = 0xCB03;
    /// Name of a kinsoku or mojikumi table: flag byte and string.
    pub const TABLE_NAME: u32 = 0x100B;
    /// Character lists of a custom kinsoku table.
    pub const KINSOKU_CHARS: u32 = 0x4214;
}

/// A name stored as a flag byte (1 = built-in key, `$ID/` in IDML) and an
/// in-object string.
fn flagged_name(c: &mut Cursor) -> Result<String, Error> {
    let builtin = c.flag()? == 1;
    let name = c.string()?;
    Ok(if builtin { format!("$ID/{name}") } else { name })
}

#[derive(Debug, Clone, PartialEq)]
pub struct CompositeFont {
    pub uid: u32,
    /// The IDML name, with `$ID/` for a built-in name.
    pub name: String,
    /// Entries, in IDML order.
    pub entries: Vec<CompositeFontEntry>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CompositeFontEntry {
    pub uid: u32,
    pub name: String,
    pub font_family: u32,
    pub font_style: String,
    /// Four f64; (100, 0, 100, 100) in every sample, the 0 being the
    /// baseline shift.
    pub numbers: [f64; 4],
    /// Character ranges as (first, last) code points.
    pub ranges: Vec<(u32, u32)>,
    /// The four u16 after the ranges: all 1 for `ScaleOption="true"`, all 0
    /// for false.
    pub scale: [u16; 4],
}

/// A code point stored as one UTF-16 unit or a surrogate pair.
fn code_point(c: &mut Cursor) -> Result<u32, Error> {
    let hi = c.u16()? as u32;
    if (0xD800..0xDC00).contains(&hi) {
        let lo = c.u16()? as u32;
        Ok(0x10000 + ((hi - 0xD800) << 10) + (lo.wrapping_sub(0xDC00) & 0x3FF))
    } else {
        Ok(hi)
    }
}

impl CompositeFontEntry {
    /// Chunk 0xCB03: name, u32 font family, font style, four f64, u16 1,
    /// u16 range count, ranges (first, last, first), four u16.
    pub fn read(uid: u32, obj: &crate::Object) -> Result<Option<CompositeFontEntry>, Error> {
        let enc = obj.encoding;
        let Some(d) = obj.chunk(chunk::COMPOSITE_FONT_ENTRY) else {
            return Ok(None);
        };
        let mut c = enc.cursor(d);
        let name = flagged_name(&mut c)?;
        let font_family = c.u32()?;
        let font_style = flagged_name(&mut c)?;
        let numbers = [c.f64()?, c.f64()?, c.f64()?, c.f64()?];
        c.u16()?;
        let n = c.u16()?;
        let mut ranges = Vec::new();
        for _ in 0..n {
            let first = code_point(&mut c)?;
            let last = code_point(&mut c)?;
            code_point(&mut c)?;
            ranges.push((first, last));
        }
        let scale = [c.u16()?, c.u16()?, c.u16()?, c.u16()?];
        Ok(Some(CompositeFontEntry {
            uid,
            name,
            font_family,
            font_style,
            numbers,
            ranges,
            scale,
        }))
    }

    /// The characters of the ranges, in order (IDML `CustomCharacters`).
    pub fn characters(&self) -> String {
        self.ranges
            .iter()
            .flat_map(|&(a, b)| (a..=b).filter_map(char::from_u32))
            .collect()
    }
}

impl CompositeFont {
    /// Chunk 0xCB02: the name, fields not identified, then a u16 count and
    /// the entry UIDs, which end the chunk. Returns the name and the
    /// entry UIDs.
    pub fn read(obj: &crate::Object) -> Result<Option<(String, Vec<u32>)>, Error> {
        let enc = obj.encoding;
        let Some(d) = obj.chunk(chunk::COMPOSITE_FONT) else {
            return Ok(None);
        };
        let mut c = enc.cursor(d);
        let name = flagged_name(&mut c)?;
        let start = c.pos();
        let Some(p) = (start..d.len().saturating_sub(1)).find(|&p| {
            let n = enc.u16_from([d[p], d[p + 1]]) as usize;
            n > 0 && p + 2 + 4 * n == d.len()
        }) else {
            return Ok(Some((name, Vec::new())));
        };
        let mut c = enc.cursor(&d[p..]);
        let n = c.u16()?;
        let entries = (0..n).map(|_| c.u32()).collect::<Result<Vec<_>, _>>()?;
        Ok(Some((name, entries)))
    }
}

/// A kinsoku or mojikumi table.
#[derive(Debug, Clone, PartialEq)]
pub struct CjkTable {
    pub uid: u32,
    pub mojikumi: bool,
    pub name: String,
    /// The character lists of a custom kinsoku table: cannot begin a line,
    /// cannot end a line, (not identified), hanging punctuation, cannot be
    /// separated.
    pub chars: Option<[String; 5]>,
}

impl CjkTable {
    pub fn read(uid: u32, cls: u32, obj: &crate::Object) -> Result<Option<CjkTable>, Error> {
        let enc = obj.encoding;
        let Some(d) = obj.chunk(chunk::TABLE_NAME) else {
            return Ok(None);
        };
        let name = flagged_name(&mut enc.cursor(d))?;
        let chars = match obj.chunk(chunk::KINSOKU_CHARS) {
            Some(d) if cls == class::CUSTOM_KINSOKU => {
                let mut c = enc.cursor(d);
                let counts = [c.u16()?, c.u16()?, c.u16()?, c.u16()?, c.u16()?];
                let mut lists: [String; 5] = Default::default();
                for (list, n) in lists.iter_mut().zip(counts) {
                    let units = (0..n).map(|_| c.u16()).collect::<Result<Vec<_>, _>>()?;
                    *list = String::from_utf16_lossy(&units);
                }
                Some(lists)
            }
            _ => None,
        };
        Ok(Some(CjkTable {
            uid,
            mojikumi: cls == class::MOJIKUMI,
            name,
            chars,
        }))
    }
}
