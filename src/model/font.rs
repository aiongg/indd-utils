//! Font families (class 0x3E03). See `docs/format/fonts.md`.

use crate::Error;
use crate::object::{Cursor, Object};

pub mod chunk {
    /// Family name and font records.
    pub const FAMILY: u32 = 0x3E05;
    /// Typekit IDs of the first fonts of the family.
    pub const TYPEKIT_IDS: u32 = 0x3EEB;
}

/// A font family and the fonts of it that the document records.
#[derive(Debug, Clone, PartialEq)]
pub struct FontFamily {
    pub uid: u32,
    pub name: String,
    pub fonts: Vec<Font>,
    /// IDML `WritingScript` of every font in the family.
    pub writing_script: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Font {
    pub style: String,
    pub postscript_name: String,
    pub full_name: String,
    pub style_native: String,
    pub full_name_native: String,
    /// Font type code (see [`Font::type_name`]).
    pub font_type: u32,
    pub version: String,
    /// IDML `TypekitID`; `$ID/` when the font has none.
    pub typekit_id: String,
}

impl Font {
    /// IDML `FontType` for the type code, if known.
    pub fn type_name(&self) -> Option<&'static str> {
        match self.font_type {
            1 => Some("TrueType"),
            6 => Some("OpenTypeCFF"),
            7 => Some("OpenTypeCID"),
            8 => Some("OpenTypeTT"),
            _ => None,
        }
    }
}

impl FontFamily {
    pub fn read(uid: u32, obj: &Object) -> Result<Option<FontFamily>, Error> {
        let Some(data) = obj.chunk(chunk::FAMILY) else {
            return Ok(None);
        };
        // Files from InDesign 3.0 and 4.0 use older font records; one is
        // taken only if it reads the chunk to its end.
        let mut family = match parse(uid, data, Record::Current) {
            Ok(f) => f,
            Err(e) => parse(uid, data, Record::Version4)
                .or_else(|_| parse(uid, data, Record::Version3))
                .map_err(|_| e)?,
        };
        if let Some(ids) = obj.chunk(chunk::TYPEKIT_IDS) {
            for (font, id) in family.fonts.iter_mut().zip(typekit_ids(ids)?) {
                font.typekit_id = id;
            }
        }
        Ok(Some(family))
    }
}

/// Layouts of the font record.
#[derive(Clone, Copy, PartialEq)]
enum Record {
    Current,
    /// The PostScript name is a u8 and a string.
    Version4,
    /// As `Version4`, without the version.
    Version3,
}

/// Chunk 0x3E05: u8, u16, name, u8, native name, 6 bytes, u16 font count,
/// font records, u32 writing script.
fn parse(uid: u32, data: &[u8], record: Record) -> Result<FontFamily, Error> {
    let mut c = Cursor::new(data);
    c.skip(3)?;
    let name = c.string()?;
    c.skip(1)?;
    c.string()?; // native family name; IDML does not write it
    c.skip(6)?;
    let count = c.u16()?;
    let mut fonts = Vec::with_capacity(count as usize);
    for _ in 0..count {
        c.skip(1)?;
        let style = c.string()?;
        let postscript_name = if record != Record::Current {
            c.skip(1)?;
            c.string()?
        } else {
            let n = c.u16()? as usize;
            c.bytes(n)?.iter().map(|&b| b as char).collect()
        };
        c.skip(1)?;
        let full_name = c.string()?;
        c.skip(1)?;
        let style_native = c.string()?;
        c.skip(1)?;
        let full_name_native = c.string()?;
        let font_type = c.u32()?;
        let n = match record {
            Record::Version3 => 0,
            _ => c.u32()? as usize,
        };
        let version = if n == 0 {
            String::new()
        } else {
            c.segments(n)?
        };
        fonts.push(Font {
            style,
            postscript_name,
            full_name,
            style_native,
            full_name_native,
            font_type,
            version,
            typekit_id: "$ID/".into(),
        });
    }
    let writing_script = c.u32()?;
    if record != Record::Current && c.remaining() != 0 {
        return Err(Error::Corrupt(format!("font family {uid}: unknown record")));
    }
    Ok(FontFamily {
        uid,
        name,
        fonts,
        writing_script,
    })
}

/// Chunk 0x3EEB: u32 count, then per font a flag byte (1 = `$ID/` key)
/// and a string.
fn typekit_ids(data: &[u8]) -> Result<Vec<String>, Error> {
    let mut c = Cursor::new(data);
    let n = c.u32()?;
    let mut out = Vec::new();
    for _ in 0..n {
        let key = c.flag()? == 1;
        let s = c.string()?;
        out.push(if key { format!("$ID/{s}") } else { s });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn string(s: &str) -> Vec<u8> {
        let mut v = vec![2, 0];
        v.extend((s.len() as u16).to_le_bytes());
        if !s.is_empty() {
            v.extend((0x4000u16 | s.len() as u16).to_le_bytes());
            v.extend(s.as_bytes());
        }
        v
    }

    #[test]
    fn parses_a_family() {
        let mut d = vec![1, 0, 0];
        d.extend(string("Myriad Pro"));
        d.push(0);
        d.extend(string("Myriad Pro"));
        d.extend([0, 0, 0, 0, 0xFF, 0xFF, 1, 0]);
        d.push(1);
        d.extend(string("Bold"));
        d.extend(14u16.to_le_bytes());
        d.extend(b"MyriadPro-Bold");
        for s in ["Myriad Pro Bold", "Bold", "Myriad Pro Bold"] {
            d.push(0);
            d.extend(string(s));
        }
        d.extend(6u32.to_le_bytes());
        d.extend(11u32.to_le_bytes());
        d.extend(0x400Bu16.to_le_bytes());
        d.extend(b"Version 2.1");
        d.extend(1u32.to_le_bytes());
        let f = parse(0x9E, &d, Record::Current).unwrap();
        assert_eq!(f.name, "Myriad Pro");
        assert_eq!(f.writing_script, 1);
        let font = &f.fonts[0];
        assert_eq!(font.style, "Bold");
        assert_eq!(font.postscript_name, "MyriadPro-Bold");
        assert_eq!(font.full_name, "Myriad Pro Bold");
        assert_eq!(font.type_name(), Some("OpenTypeCFF"));
        assert_eq!(font.version, "Version 2.1");
    }

    #[test]
    fn reads_typekit_ids() {
        let mut d = 2u32.to_le_bytes().to_vec();
        d.push(1);
        d.extend(string(""));
        d.push(0);
        d.extend(string("TkD-1-ab"));
        assert_eq!(typekit_ids(&d).unwrap(), ["$ID/", "TkD-1-ab"]);
    }
}
