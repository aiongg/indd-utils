//! Composite fonts, kinsoku tables and mojikumi tables (CJK typography).
//! See `docs/format/fonts.md` and `docs/format/objects.md`.

use crate::Error;
use crate::object::{Cursor, Name};

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

#[derive(Debug, Clone, PartialEq)]
pub struct CompositeFont {
    pub uid: u32,
    pub name: Name,
    /// Entries, in IDML order.
    pub entries: Vec<CompositeFontEntry>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CompositeFontEntry {
    pub uid: u32,
    pub name: Name,
    pub font_family: u32,
    pub font_style: Name,
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
        let name = c.name()?;
        let font_family = c.u32()?;
        let font_style = c.name()?;
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
    /// the entry UIDs, which end the chunk. In files from InDesign 3.0
    /// four zero bytes come before the name (`big-endian.md`). Returns the
    /// name and the entry UIDs.
    pub fn read(obj: &crate::Object) -> Result<Option<(Name, Vec<u32>)>, Error> {
        let enc = obj.encoding;
        let Some(d) = obj.chunk(chunk::COMPOSITE_FONT) else {
            return Ok(None);
        };
        let mut c = enc.cursor(d);
        let name = match c.name() {
            Ok(name) => name,
            Err(e) if d.starts_with(&[0; 4]) => {
                c = enc.cursor(d);
                c.skip(4)?;
                c.name().map_err(|_| e)?
            }
            Err(e) => return Err(e),
        };
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
    pub name: Name,
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
        let name = enc.cursor(d).name()?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::synthetic::{flagged_string, object};
    use crate::object::Encoding;
    use crate::{ByteOrder, Version};

    fn big_endian() -> Encoding {
        Encoding::new(ByteOrder::Big, Version { major: 4, minor: 0 })
    }

    /// Chunk 0xCB03 of a composite font entry in `enc`: the Kanji entry,
    /// font family 7, style "W3", a BMP range and a range of surrogate
    /// pairs, scale option on.
    fn entry_chunk(enc: Encoding) -> Vec<u8> {
        let mut d = flagged_string(enc, 1, "Kanji");
        d.extend(enc.u32_bytes(7));
        d.extend(flagged_string(enc, 0, "W3"));
        for f in [100.0, 0.0, 100.0, 100.0] {
            d.extend(enc.f64_bytes(f));
        }
        d.extend(enc.u16_bytes(1));
        d.extend(enc.u16_bytes(2));
        // (first, last, first): U+4E00–U+4E01, then U+20000–U+20001.
        for u in [0x4E00u16, 0x4E01, 0x4E00] {
            d.extend(enc.u16_bytes(u));
        }
        for pair in [[0xD840u16, 0xDC00], [0xD840, 0xDC01], [0xD840, 0xDC00]] {
            for u in pair {
                d.extend(enc.u16_bytes(u));
            }
        }
        for _ in 0..4 {
            d.extend(enc.u16_bytes(1));
        }
        d
    }

    #[test]
    fn reads_composite_font_entries_in_either_byte_order() {
        for enc in [Encoding::default(), big_endian()] {
            let obj = object(
                0x60,
                class::COMPOSITE_FONT_ENTRY,
                &[(chunk::COMPOSITE_FONT_ENTRY, entry_chunk(enc))],
                enc,
            );
            let e = CompositeFontEntry::read(0x60, &obj).unwrap().unwrap();
            assert_eq!(e.name.idml(), "$ID/Kanji");
            assert_eq!((e.font_family, e.font_style.idml()), (7, "W3".into()));
            assert_eq!(e.numbers, [100.0, 0.0, 100.0, 100.0]);
            assert_eq!(e.ranges, [(0x4E00, 0x4E01), (0x20000, 0x20001)]);
            assert_eq!(e.scale, [1; 4]);
            assert_eq!(e.characters(), "\u{4E00}\u{4E01}\u{20000}\u{20001}");
        }
    }

    #[test]
    fn composite_fonts_of_version_3_have_four_bytes_before_the_name() {
        for enc in [Encoding::default(), big_endian()] {
            let mut d = vec![0; 4];
            d.extend(flagged_string(enc, 1, "[No composite font]"));
            d.extend([0; 6]);
            d.extend(enc.u16_bytes(1));
            d.extend(enc.u32_bytes(0x55));
            let obj = object(
                0x54,
                class::COMPOSITE_FONT,
                &[(chunk::COMPOSITE_FONT, d)],
                enc,
            );
            let (name, entries) = CompositeFont::read(&obj).unwrap().unwrap();
            assert_eq!(name.idml(), "$ID/[No composite font]");
            assert_eq!(entries, [0x55]);
        }
    }

    #[test]
    fn composite_fonts_end_with_their_entry_list() {
        let enc = Encoding::default();
        let mut d = flagged_string(enc, 1, "[No composite font]");
        // Fields not identified, then two entry UIDs.
        d.extend([9, 9, 9, 9, 9]);
        d.extend(enc.u16_bytes(2));
        d.extend(enc.u32_bytes(0x60));
        d.extend(enc.u32_bytes(0x61));
        let obj = object(
            0x5F,
            class::COMPOSITE_FONT,
            &[(chunk::COMPOSITE_FONT, d)],
            enc,
        );
        let (name, entries) = CompositeFont::read(&obj).unwrap().unwrap();
        assert!(name.builtin);
        assert_eq!(name.name, "[No composite font]");
        assert_eq!(entries, [0x60, 0x61]);
    }

    #[test]
    fn reads_custom_kinsoku_characters() {
        let enc = Encoding::default();
        let lists = ["、。", "（", "", "、", "—"];
        let mut chars = Vec::new();
        for l in lists {
            chars.extend(enc.u16_bytes(l.encode_utf16().count() as u16));
        }
        for l in lists {
            for u in l.encode_utf16() {
                chars.extend(enc.u16_bytes(u));
            }
        }
        let obj = object(
            0x70,
            class::CUSTOM_KINSOKU,
            &[
                (chunk::TABLE_NAME, flagged_string(enc, 0, "Mine")),
                (chunk::KINSOKU_CHARS, chars),
            ],
            enc,
        );
        let t = CjkTable::read(0x70, class::CUSTOM_KINSOKU, &obj)
            .unwrap()
            .unwrap();
        assert!(!t.mojikumi && !t.name.builtin);
        assert_eq!(t.name.name, "Mine");
        assert_eq!(t.chars, Some(lists.map(String::from)));
        // A mojikumi table has no character lists.
        let obj = object(
            0x71,
            class::MOJIKUMI,
            &[(
                chunk::TABLE_NAME,
                flagged_string(enc, 1, "kMojikumiDefaultName1"),
            )],
            enc,
        );
        let t = CjkTable::read(0x71, class::MOJIKUMI, &obj)
            .unwrap()
            .unwrap();
        assert!(t.mojikumi && t.name.builtin && t.chars.is_none());
    }
}
