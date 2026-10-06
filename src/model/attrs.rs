//! Attribute lists (chunk 0x6E03 and others). See `docs/format/objects.md`.
//!
//! A list is a u32 count, then records: u32 attribute ID, u16 payload size,
//! payload. The payload is a u16 value count and that many values, each a
//! u32 type, u16 length and data. The first value is the attribute's value.

use crate::Error;
use crate::object::Cursor;

/// Value type codes.
pub mod ty {
    pub const DOUBLE: u32 = 0x6E68;
    pub const INT: u32 = 0x6E67;
    pub const ENUM: u32 = 0x6E65;
    pub const POINT: u32 = 0x6E69;
    pub const REF: u32 = 0x117;
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Double(f64),
    Int(i32),
    Enum(u16),
    Ref(u32),
    Point(f64, f64),
    Other(u32, Vec<u8>),
}

impl Value {
    pub fn as_f64(&self) -> Option<f64> {
        match *self {
            Value::Double(v) => Some(v),
            Value::Int(v) => Some(v as f64),
            Value::Enum(v) => Some(v as f64),
            _ => None,
        }
    }

    pub fn as_ref(&self) -> Option<u32> {
        match *self {
            Value::Ref(v) => Some(v),
            _ => None,
        }
    }

    /// Integer value of a reference, integer or enumeration.
    pub fn as_u32(&self) -> Option<u32> {
        match *self {
            Value::Ref(v) => Some(v),
            Value::Int(v) => Some(v as u32),
            Value::Enum(v) => Some(v as u32),
            _ => None,
        }
    }

    /// A string value: a flag byte, then an in-object string.
    pub fn as_string(&self) -> Option<String> {
        match self {
            Value::Other(_, b) if b.len() > 1 => Cursor::new(&b[1..]).string().ok(),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Attrs(pub Vec<(u32, Value)>);

impl Attrs {
    pub fn get(&self, id: u32) -> Option<&Value> {
        self.0.iter().find(|(a, _)| *a == id).map(|(_, v)| v)
    }

    /// A page item list (chunk 0x6E03): u32 count, then records.
    pub fn parse(data: &[u8]) -> Result<Attrs, Error> {
        let mut c = Cursor::new(data);
        let n = c.u32()? as usize;
        Attrs::records(&mut c, n, decode)
    }

    /// A text attribute list: `count` records at the cursor. Text value
    /// types differ per attribute, so values are decoded by length.
    pub fn parse_text(c: &mut Cursor, count: usize) -> Result<Attrs, Error> {
        Attrs::records(c, count, decode_text)
    }

    fn records(c: &mut Cursor, n: usize, decode: fn(u32, &[u8]) -> Value) -> Result<Attrs, Error> {
        let mut out = Vec::with_capacity(n.min(1024));
        for _ in 0..n {
            let id = c.u32()?;
            let size = c.u16()? as usize;
            let mut p = Cursor::new(c.bytes(size)?);
            let count = p.u16()?;
            let mut first = None;
            for _ in 0..count {
                let t = p.u32()?;
                let len = p.u16()? as usize;
                let data = p.bytes(len)?;
                if first.is_none() {
                    first = Some(decode(t, data));
                }
            }
            if let Some(v) = first {
                out.push((id, v));
            }
        }
        Ok(Attrs(out))
    }
}

fn decode_text(t: u32, data: &[u8]) -> Value {
    match data.len() {
        8 => Value::Double(f64::from_le_bytes(data.try_into().unwrap())),
        4 => Value::Ref(u32::from_le_bytes(data.try_into().unwrap())),
        2 => Value::Enum(u16::from_le_bytes(data.try_into().unwrap())),
        _ => Value::Other(t, data.to_vec()),
    }
}

fn decode(t: u32, data: &[u8]) -> Value {
    let f64_at = |o: usize| f64::from_le_bytes(data[o..o + 8].try_into().unwrap());
    match (t, data.len()) {
        (ty::DOUBLE, 8) => Value::Double(f64_at(0)),
        (ty::INT, 4) => Value::Int(i32::from_le_bytes(data.try_into().unwrap())),
        (ty::ENUM, 2) => Value::Enum(u16::from_le_bytes(data.try_into().unwrap())),
        (ty::REF, 4) => Value::Ref(u32::from_le_bytes(data.try_into().unwrap())),
        (ty::POINT, 16) => Value::Point(f64_at(0), f64_at(8)),
        // Other 4-byte types are references or codes (for example a
        // corner effect ID); keep the number.
        (_, 4) => Value::Ref(u32::from_le_bytes(data.try_into().unwrap())),
        _ => Value::Other(t, data.to_vec()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_stroke_weight() {
        // One attribute: 0x6E65 = double 0.25, plus a 6-byte trailer value.
        let mut d = 1u32.to_le_bytes().to_vec();
        d.extend_from_slice(&0x6E65u32.to_le_bytes());
        d.extend_from_slice(&28u16.to_le_bytes());
        d.extend_from_slice(&2u16.to_le_bytes());
        d.extend_from_slice(&ty::DOUBLE.to_le_bytes());
        d.extend_from_slice(&8u16.to_le_bytes());
        d.extend_from_slice(&0.25f64.to_le_bytes());
        d.extend_from_slice(&0x6E63u32.to_le_bytes());
        d.extend_from_slice(&6u16.to_le_bytes());
        d.extend_from_slice(&[0; 6]);
        let a = Attrs::parse(&d).unwrap();
        assert_eq!(a.get(0x6E65), Some(&Value::Double(0.25)));
    }
}
