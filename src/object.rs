//! The contents of database objects. See `docs/format/objects.md`.
//!
//! Most objects are a sequence of chunks: u32 chunk ID, u32 length, data.
//! Some objects (embedded files, the XMP packet) are plain byte streams.

use crate::Error;

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
        let id = u32::from_le_bytes(head[..4].try_into().unwrap());
        let len = u32::from_le_bytes(head[4..].try_into().unwrap()) as usize;
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
        self.chunks()?
            .into_iter()
            .find(|c| c.id == id)
            .map(|c| c.data)
    }
}

/// Little-endian cursor over chunk data.
#[derive(Debug, Clone)]
pub struct Cursor<'a> {
    data: &'a [u8],
    pos: usize,
}

fn short(what: &str, pos: usize) -> Error {
    Error::Corrupt(format!("chunk data ends while reading {what} at {pos}"))
}

impl<'a> Cursor<'a> {
    pub fn new(data: &'a [u8]) -> Cursor<'a> {
        Cursor { data, pos: 0 }
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
        Ok(u16::from_le_bytes(self.bytes(2)?.try_into().unwrap()))
    }

    pub fn u32(&mut self) -> Result<u32, Error> {
        Ok(u32::from_le_bytes(self.bytes(4)?.try_into().unwrap()))
    }

    pub fn i32(&mut self) -> Result<i32, Error> {
        Ok(i32::from_le_bytes(self.bytes(4)?.try_into().unwrap()))
    }

    pub fn f64(&mut self) -> Result<f64, Error> {
        Ok(f64::from_le_bytes(self.bytes(8)?.try_into().unwrap()))
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
    /// the rest: 0x4000 = single-byte characters, 0x8000 = UTF-16LE code
    /// units.
    pub fn segments(&mut self, chars: usize) -> Result<String, Error> {
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
        Ok(String::from_utf16_lossy(&units))
    }

    /// A string stored inside object data: u16 2, u16 length in UTF-16
    /// code units, then segments.
    pub fn string(&mut self) -> Result<String, Error> {
        let start = self.pos;
        let tag = self.u16()?;
        if tag != 2 {
            return Err(Error::Corrupt(format!(
                "string tag {tag} at {start}, expected 2"
            )));
        }
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
}
