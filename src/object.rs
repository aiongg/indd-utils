//! The contents of database objects. See `docs/format/objects.md`.
//!
//! Most objects are a sequence of chunks: u32 chunk ID, u32 length, data.
//! Some objects (embedded files, the XMP packet) are plain byte streams.

use std::cell::Cell;

use crate::{ByteOrder, Error, Version};

thread_local! {
    static BIG_ENDIAN: Cell<bool> = const { Cell::new(false) };
    static STRING_TAG: Cell<u8> = const { Cell::new(2) };
}

/// Object data is in the file's byte order (`docs/format/objects.md`).
/// Everything that decodes object data on this thread uses the byte order
/// set here, until the returned guard is dropped.
pub fn use_byte_order(order: ByteOrder) -> ByteOrderGuard {
    ByteOrderGuard(BIG_ENDIAN.replace(order == ByteOrder::Big))
}

/// Restores the previous byte order when dropped.
pub struct ByteOrderGuard(bool);

impl Drop for ByteOrderGuard {
    fn drop(&mut self) {
        BIG_ENDIAN.set(self.0);
    }
}

/// Whether object data on this thread is big-endian.
pub fn big_endian() -> bool {
    BIG_ENDIAN.get()
}

/// The first byte of an in-object string: 1 in files from InDesign 2.0, 2
/// in later files (`docs/format/objects.md`).
pub fn string_tag_for(version: Version) -> u8 {
    if version.major <= 2 { 1 } else { 2 }
}

/// Everything that decodes object data on this thread expects in-object
/// strings to start with `tag`, until the returned guard is dropped.
pub fn use_string_tag(tag: u8) -> StringTagGuard {
    StringTagGuard(STRING_TAG.replace(tag))
}

/// Restores the previous string tag when dropped.
pub struct StringTagGuard(u8);

impl Drop for StringTagGuard {
    fn drop(&mut self) {
        STRING_TAG.set(self.0);
    }
}

macro_rules! decoders {
    ($($name:ident, $enc:ident: $t:ty;)*) => {$(
        /// Decode object data in the current byte order.
        pub fn $name(b: [u8; size_of::<$t>()]) -> $t {
            if big_endian() { <$t>::from_be_bytes(b) } else { <$t>::from_le_bytes(b) }
        }
        /// Encode a value as object data in the current byte order.
        pub fn $enc(v: $t) -> [u8; size_of::<$t>()] {
            if big_endian() { v.to_be_bytes() } else { v.to_le_bytes() }
        }
    )*};
}

decoders! {
    u16_from, u16_bytes: u16;
    i16_from, i16_bytes: i16;
    u32_from, u32_bytes: u32;
    i32_from, i32_bytes: i32;
    f64_from, f64_bytes: f64;
}

/// Decode a u32 at `offset`, if there are four bytes there.
pub fn u32_at(b: &[u8], offset: usize) -> Option<u32> {
    Some(u32_from(b.get(offset..offset + 4)?.try_into().ok()?))
}

/// Decode an f64 at `offset`, if there are eight bytes there.
pub fn f64_at(b: &[u8], offset: usize) -> Option<f64> {
    Some(f64_from(b.get(offset..offset + 8)?.try_into().ok()?))
}

/// Decode a u16 at `offset`, if there are two bytes there.
pub fn u16_at(b: &[u8], offset: usize) -> Option<u16> {
    Some(u16_from(b.get(offset..offset + 2)?.try_into().ok()?))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Chunk<'a> {
    pub id: u32,
    pub data: &'a [u8],
}

/// Split an object's bytes into chunks. `None` if the bytes are not a
/// chunk sequence.
pub fn chunks(bytes: &[u8]) -> Option<Vec<Chunk<'_>>> {
    let mut out = Vec::new();
    let mut pos = 0;
    while pos < bytes.len() {
        let head = bytes.get(pos..pos + 8)?;
        let id = u32_from(head[..4].try_into().unwrap());
        let len = u32_from(head[4..].try_into().unwrap()) as usize;
        let data = bytes.get(pos + 8..pos + 8 + len)?;
        out.push(Chunk { id, data });
        pos += 8 + len;
    }
    Some(out)
}

/// A decoded object: its UID, class and chunks.
#[derive(Debug, Clone)]
pub struct Object {
    pub uid: u32,
    pub class: Option<u32>,
    pub bytes: Vec<u8>,
}

impl Object {
    pub fn chunks(&self) -> Option<Vec<Chunk<'_>>> {
        chunks(&self.bytes)
    }

    /// Data of the first chunk with this ID.
    pub fn chunk(&self, id: u32) -> Option<&[u8]> {
        crate::audit::chunk_read(self.uid, id);
        self.chunks()?
            .into_iter()
            .find(|c| c.id == id)
            .map(|c| c.data)
    }
}

/// Cursor over object data, in the byte order set by [`use_byte_order`].
#[derive(Debug, Clone)]
pub struct Cursor<'a> {
    data: &'a [u8],
    pos: usize,
    /// Big-endian data: [`Cursor::flag`] has read the tag of the next
    /// string.
    tag_read: bool,
}

fn short(what: &str, pos: usize) -> Error {
    Error::Corrupt(format!("chunk data ends while reading {what} at {pos}"))
}

impl<'a> Cursor<'a> {
    pub fn new(data: &'a [u8]) -> Cursor<'a> {
        Cursor {
            data,
            pos: 0,
            tag_read: false,
        }
    }

    pub fn pos(&self) -> usize {
        self.pos
    }

    pub fn remaining(&self) -> usize {
        self.data.len() - self.pos
    }

    pub fn bytes(&mut self, n: usize) -> Result<&'a [u8], Error> {
        let out = self
            .data
            .get(self.pos..self.pos + n)
            .ok_or_else(|| short("bytes", self.pos))?;
        self.pos += n;
        Ok(out)
    }

    pub fn skip(&mut self, n: usize) -> Result<(), Error> {
        self.bytes(n).map(|_| ())
    }

    pub fn u8(&mut self) -> Result<u8, Error> {
        Ok(self.bytes(1)?[0])
    }

    pub fn u16(&mut self) -> Result<u16, Error> {
        Ok(u16_from(self.bytes(2)?.try_into().unwrap()))
    }

    pub fn u32(&mut self) -> Result<u32, Error> {
        Ok(u32_from(self.bytes(4)?.try_into().unwrap()))
    }

    pub fn i32(&mut self) -> Result<i32, Error> {
        Ok(i32_from(self.bytes(4)?.try_into().unwrap()))
    }

    pub fn f64(&mut self) -> Result<f64, Error> {
        Ok(f64_from(self.bytes(8)?.try_into().unwrap()))
    }

    /// A u32 count followed by that many u32 values.
    pub fn u32_list(&mut self) -> Result<Vec<u32>, Error> {
        let n = self.u32()? as usize;
        if n > self.remaining() / 4 {
            return Err(Error::Corrupt(format!(
                "list of {n} at {} is too long",
                self.pos
            )));
        }
        (0..n).map(|_| self.u32()).collect()
    }

    /// Text made of `chars` UTF-16 code units, stored as segments. Each
    /// segment is a u16 header, flags in the top two bits and a count in
    /// the rest: 0x4000 = single-byte characters, 0x8000 = UTF-16 code
    /// units.
    pub fn segments(&mut self, chars: usize) -> Result<String, Error> {
        Ok(String::from_utf16_lossy(&self.segment_units(chars)?))
    }

    /// Like [`Cursor::segments`], but returns the UTF-16 code units, so a
    /// surrogate pair split between two runs can be joined by the caller.
    pub fn segment_units(&mut self, chars: usize) -> Result<Vec<u16>, Error> {
        let mut units: Vec<u16> = Vec::with_capacity(chars);
        while units.len() < chars {
            let header = self.u16()?;
            let n = (header & 0x3FFF) as usize;
            match header & 0xC000 {
                0x4000 => units.extend(self.bytes(n)?.iter().map(|&b| b as u16)),
                0x8000 => {
                    for _ in 0..n {
                        units.push(self.u16()?);
                    }
                }
                _ => {
                    return Err(Error::Corrupt(format!(
                        "unknown text segment header {header:#06x} at {}",
                        self.pos - 2
                    )));
                }
            }
            if n == 0 {
                return Err(Error::Corrupt(format!(
                    "empty text segment at {}",
                    self.pos - 2
                )));
            }
        }
        if units.len() != chars {
            return Err(Error::Corrupt(format!(
                "text segments hold {} units, expected {chars}",
                units.len()
            )));
        }
        Ok(units)
    }

    /// The flag byte before a string (see [`Cursor::string`]).
    pub fn flag(&mut self) -> Result<u8, Error> {
        let tag = STRING_TAG.get();
        if big_endian() && self.data.get(self.pos) == Some(&tag) && self.remaining() >= 2 {
            self.tag_read = true;
            self.pos += 2;
            return Ok(self.data[self.pos - 1]);
        }
        self.u8()
    }

    /// A string stored inside object data: u8 2 (1 in files from InDesign
    /// 2.0, see [`use_string_tag`]), a u8 whose meaning is unknown (usually
    /// 0), u16 length in UTF-16 code units, then segments.
    ///
    /// The byte before the 2 is often a flag (1 = the string is a built-in
    /// key), and the two bytes form a u16 with the 2 in the high byte. In
    /// big-endian data the 2 therefore comes first: read such a flag with
    /// [`Cursor::flag`]. If the caller skipped the byte before the string
    /// instead, the 2 is that byte (`docs/format/big-endian.md`).
    pub fn string(&mut self) -> Result<String, Error> {
        let start = self.pos;
        let expected = STRING_TAG.get();
        if big_endian() {
            if !std::mem::take(&mut self.tag_read) {
                let before = start.checked_sub(1).map(|i| self.data[i]);
                if before != Some(expected) && self.u8()? != expected {
                    return Err(Error::Corrupt(format!("no string tag at {start}")));
                }
                // The flag.
                self.u8()?;
            }
        } else {
            let tag = self.u8()?;
            if tag != expected {
                return Err(Error::Corrupt(format!(
                    "string tag {tag} at {start}, expected {expected}"
                )));
            }
        }
        self.u8()?;
        let n = self.u16()? as usize;
        if n == 0 {
            return Ok(String::new());
        }
        self.segments(n)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_chunks() {
        let mut b = Vec::new();
        b.extend_from_slice(&0x501u32.to_le_bytes());
        b.extend_from_slice(&2u32.to_le_bytes());
        b.extend_from_slice(&[7, 8]);
        b.extend_from_slice(&0x15bu32.to_le_bytes());
        b.extend_from_slice(&0u32.to_le_bytes());
        let c = chunks(&b).unwrap();
        assert_eq!(c.len(), 2);
        assert_eq!(
            c[0],
            Chunk {
                id: 0x501,
                data: &[7, 8]
            }
        );
        assert!(c[1].data.is_empty());
        assert!(chunks(&b[..b.len() - 1]).is_none());
    }

    #[test]
    fn decodes_mixed_segments() {
        // "WOMEN’S\r": 5 single-byte, 1 UTF-16, 2 single-byte.
        let data = [
            0x05, 0x40, b'W', b'O', b'M', b'E', b'N', 0x01, 0x80, 0x19, 0x20, 0x02, 0x40, b'S',
            b'\r',
        ];
        assert_eq!(Cursor::new(&data).segments(8).unwrap(), "WOMEN\u{2019}S\r");
    }

    #[test]
    fn decodes_object_string() {
        let data = [
            2, 0, 7, 0, 7, 0x40, b'R', b'e', b'g', b'u', b'l', b'a', b'r',
        ];
        assert_eq!(Cursor::new(&data).string().unwrap(), "Regular");
    }

    #[test]
    fn decodes_version_2_string() {
        let _tag = use_string_tag(string_tag_for(Version { major: 2, minor: 0 }));
        let data = [1, 0, 5, 0, 5, 0x40, b'R', b'o', b'm', b'a', b'n'];
        assert_eq!(Cursor::new(&data).string().unwrap(), "Roman");
        let data = [2, 0, 5, 0, 5, 0x40, b'R', b'o', b'm', b'a', b'n'];
        assert!(Cursor::new(&data).string().is_err());
    }

    #[test]
    fn decodes_big_endian_data() {
        let _order = use_byte_order(ByteOrder::Big);
        // A colour name from the InDesign 4.0 fixture: 2, flag 0, 0, length
        // 5, then a single-byte segment, and a u32 after it.
        let data = [
            2, 0, 0, 0, 5, 0x40, 5, b'B', b'l', b'a', b'c', b'k', 0, 0, 0, 0x0A,
        ];
        let mut c = Cursor::new(&data);
        assert_eq!(c.flag().unwrap(), 0);
        assert_eq!(c.string().unwrap(), "Black");
        assert_eq!(c.u32().unwrap(), 10);
        // Code that skips the flag byte skips the 2 instead.
        let mut c = Cursor::new(&data);
        c.skip(1).unwrap();
        assert_eq!(c.string().unwrap(), "Black");
        // UTF-16 code units are big-endian too.
        let data = [0x80, 0x01, 0x5C, 0x0F];
        assert_eq!(Cursor::new(&data).segments(1).unwrap(), "\u{5C0F}");
    }
}
