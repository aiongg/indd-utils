//! Attribute lists (chunk 0x6E03 and others). See `docs/format/objects.md`.
//!
//! A list is a u32 count, then records: u32 attribute ID, u16 payload size,
//! payload. The payload is a u16 value count and that many values, each a
//! u32 type, u16 length and data. The first value is the attribute's value.

use crate::Error;
use crate::audit::{List, Recorder};
use crate::object::{Cursor, Encoding};

/// Value type codes.
pub mod ty {
    pub const DOUBLE: u32 = 0x6E68;
    pub const INT: u32 = 0x6E67;
    pub const ENUM: u32 = 0x6E65;
    pub const POINT: u32 = 0x6E69;
    pub const REF: u32 = 0x117;
    /// A built-in item code, the second value after a reference of 0.
    pub const CODE: u32 = 0x6E64;
    /// A flag byte and an in-object string.
    pub const STRING: u32 = 0x1B02;
    pub const FORM_STRING: u32 = 0x1451F;
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Double(f64),
    Int(i32),
    Enum(u16),
    Ref(u32),
    Point(f64, f64),
    /// A reference followed by a built-in code (value type 0x6E64), as in
    /// a stroke type: the reference is 0 for a built-in style.
    RefOrCode(u32, u32),
    /// Two u32, as in a cell edge stroke type: a code and 0.
    Words(u32, u32),
    /// A flag byte, then an in-object string.
    String(String),
    /// u32 length in characters, then text segments.
    Text(String),
    /// A tab list.
    TabList(Vec<TabStop>),
    /// A list of nested, line or GREP styles: u32 count, then records.
    /// The records are decoded for nested styles only; `nested` is `None`
    /// if they cannot be.
    StyleList {
        count: u32,
        nested: Option<Vec<NestedStyle>>,
    },
    /// u32 bullet character type and u32 character value.
    BulletChar {
        kind: u32,
        value: u32,
    },
    /// Opacity gradient stops: location (0–1), absolute position of the
    /// midpoint to the next stop (0–1), opacity in percent.
    Stops(Vec<[f64; 3]>),
    /// A value whose layout is not known: its type code and bytes.
    Other(u32, Vec<u8>),
}

/// A tab stop. See `docs/format/attributes.md`, tab lists.
#[derive(Debug, Clone, PartialEq)]
pub struct TabStop {
    pub position: f64,
    /// 0 left, 2 right; other codes have no evidence.
    pub alignment: u16,
    pub leader: String,
}

/// A nested style: the character style it applies (0 for none) and up to
/// where. See `docs/format/attributes.md`, nested styles.
#[derive(Debug, Clone, PartialEq)]
pub struct NestedStyle {
    pub style: u32,
    pub delimiter: Delimiter,
    pub repetition: u32,
    pub inclusive: bool,
}

/// Where a nested style ends.
#[derive(Debug, Clone, PartialEq)]
pub enum Delimiter {
    /// `^c`: the drop cap.
    Dropcap,
    /// `^w`
    AnyWord,
    /// `^?`
    AnyCharacter,
    /// A single character.
    Character(String),
}

/// Delimiter, repetition and inclusiveness of a nested style's delimiter
/// code: `^c`, or `(d)` / `[d]` followed by an optional count. `None` for
/// codes without evidence. See `docs/format/attributes.md`.
pub fn nested_delimiter(code: &str) -> Option<(Delimiter, u32, bool)> {
    if code == "^c" {
        return Some((Delimiter::Dropcap, 1, true));
    }
    let (inclusive, close) = match code.chars().next()? {
        '(' => (true, ')'),
        '[' => (false, ']'),
        _ => return None,
    };
    let end = code.rfind(close)?;
    let inner = &code[1..end];
    let count = &code[end + 1..];
    let repetition = if count.is_empty() {
        1
    } else {
        count.parse().ok()?
    };
    let delimiter = match inner {
        "^w" => Delimiter::AnyWord,
        "^?" => Delimiter::AnyCharacter,
        _ if inner.chars().count() == 1 && inner != "^" => Delimiter::Character(inner.to_string()),
        _ => return None,
    };
    Some((delimiter, repetition, inclusive))
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

    /// A string value.
    pub fn as_string(&self) -> Option<String> {
        match self {
            Value::String(s) => Some(s.clone()),
            _ => None,
        }
    }

    /// The code `indd audit` reports for a value without an IDML value:
    /// its integer, the code of a reference-or-code or the first of two
    /// words.
    pub fn code(&self) -> Option<u32> {
        match *self {
            Value::RefOrCode(_, code) | Value::Words(code, _) => Some(code),
            _ => self.as_u32(),
        }
    }
}

/// Attribute IDs and values, the kind of list they come from and the
/// recorder of the database they come from (for `indd audit`).
#[derive(Debug, Clone, Default)]
pub struct Attrs {
    pub values: Vec<(u32, Value)>,
    pub list: List,
    recorder: Option<Recorder>,
}

/// Lists are equal when their values and kinds are.
impl PartialEq for Attrs {
    fn eq(&self, other: &Attrs) -> bool {
        self.values == other.values && self.list == other.list
    }
}

impl Attrs {
    pub fn get(&self, id: u32) -> Option<&Value> {
        let v = self.values.iter().find(|(a, _)| *a == id).map(|(_, v)| v);
        if v.is_some()
            && let Some(r) = &self.recorder
        {
            r.attr_read(self.list, id);
        }
        v
    }

    /// Report a value of attribute `id` that has no IDML value to
    /// `indd audit`: its code (see [`Value::code`]).
    pub fn unknown_code(&self, id: u32, v: &Value) {
        if let Some(r) = &self.recorder {
            r.unknown_code(self.list, id, v.code().unwrap_or(u32::MAX));
        }
    }

    /// A page item list (chunk 0x6E03): u32 count, then records.
    pub fn parse(
        enc: Encoding,
        data: &[u8],
        list: List,
        recorder: Option<&Recorder>,
    ) -> Result<Attrs, Error> {
        let mut c = enc.cursor(data);
        let n = c.u32()? as usize;
        Attrs::records(&mut c, n, decode, list, recorder)
    }

    /// A page item attribute list (u32 count, records) at the cursor,
    /// which is left after the list.
    pub fn parse_at(
        c: &mut Cursor,
        list: List,
        recorder: Option<&Recorder>,
    ) -> Result<Attrs, Error> {
        let n = c.u32()? as usize;
        Attrs::records(c, n, decode, list, recorder)
    }

    /// A page item attribute list with a u16 count, as object styles hold.
    pub fn parse_short(
        enc: Encoding,
        data: &[u8],
        list: List,
        recorder: Option<&Recorder>,
    ) -> Result<Attrs, Error> {
        let mut c = enc.cursor(data);
        let n = c.u16()? as usize;
        Attrs::records(&mut c, n, decode, list, recorder)
    }

    /// A text attribute list: `count` records at the cursor. Values are
    /// decoded by their attribute's layout (see `text_layout`).
    pub fn parse_text(
        c: &mut Cursor,
        count: usize,
        list: List,
        recorder: Option<&Recorder>,
    ) -> Result<Attrs, Error> {
        Attrs::records(c, count, decode_text, list, recorder)
    }

    fn records(
        c: &mut Cursor,
        n: usize,
        decode: fn(Encoding, u32, u32, &[u8]) -> Value,
        list: List,
        recorder: Option<&Recorder>,
    ) -> Result<Attrs, Error> {
        let enc = c.encoding();
        let mut out = Vec::with_capacity(n.min(1024));
        for _ in 0..n {
            let id = c.u32()?;
            let size = c.u16()? as usize;
            let mut p = enc.cursor(c.bytes(size)?);
            // A record without values (for example a merged-cell marker).
            if size < 2 {
                continue;
            }
            let count = p.u16()?;
            let mut first = None;
            for i in 0..count {
                let t = p.u32()?;
                let len = p.u16()? as usize;
                let data = p.bytes(len)?;
                match (i, &first) {
                    (0, _) => first = Some(decode(enc, id, t, data)),
                    (1, Some(Value::Ref(r))) if t == ty::CODE && len == 4 => {
                        if let Some(code) = enc.u32_at(data, 0) {
                            first = Some(Value::RefOrCode(*r, code));
                        }
                    }
                    _ => {}
                }
            }
            if let Some(v) = first {
                out.push((id, v));
            }
        }
        if let Some(r) = recorder {
            r.attrs_parsed(list, out.iter().map(|(id, _)| *id));
        }
        Ok(Attrs {
            values: out,
            list,
            recorder: recorder.cloned(),
        })
    }
}

/// How the value of a text, table or cell attribute is laid out, for the
/// attributes whose value is not a single number. See
/// `docs/format/attributes.md` and `tables.md`.
#[derive(Clone, Copy)]
enum Layout {
    /// Two f64.
    Point,
    /// A flag byte and an in-object string.
    String,
    /// u32 length, then text segments.
    Text,
    TabList,
    NestedStyles,
    /// Line or GREP styles: u32 count, then records not decoded.
    StyleList,
    BulletChar,
    /// Two u32.
    Words,
    /// u32 reference (0 for a built-in) and u32 code.
    RefOrCode,
}

fn text_layout(id: u32) -> Option<Layout> {
    Some(match id {
        0x1B4A => Layout::Point,
        // Ruby and kenten font styles are strings too (attributes.md,
        // ruby, kenten and warichu).
        0x1B02 | 0x1A414 | 0x1A423 | 0x1A424 | 0x4234 | 0x424B => Layout::String,
        // Stroke types of rules, underline and strikethrough.
        0x1B71 | 0x1B72 | 0x1B95 | 0x1B9D => Layout::RefOrCode,
        0x422E => Layout::Text,
        0x1B29 => Layout::TabList,
        0x1B75 => Layout::NestedStyles,
        0x1BBA | 0x1BBB => Layout::StyleList,
        0x1A406 => Layout::BulletChar,
        // Stroke types of cell edges, table borders, rows and columns
        // (tables.md).
        0xB64D | 0xB64E | 0xB64F | 0xB650 | 0xB655 | 0xB658 | 0xB65B | 0xB65E | 0xB688 | 0xB689
        | 0xB68E | 0xB68F => Layout::Words,
        _ => return None,
    })
}

/// A text, table or cell attribute value. Values of the attributes in
/// [`text_layout`] are decoded by their layout (`Other` if they do not
/// fit it); the others are numbers of 8, 4 or 2 bytes.
fn decode_text(enc: Encoding, id: u32, t: u32, data: &[u8]) -> Value {
    let Some(layout) = text_layout(id) else {
        let number = match data.len() {
            8 => enc.f64_at(data, 0).map(Value::Double),
            4 => enc.u32_at(data, 0).map(Value::Ref),
            2 => enc.u16_at(data, 0).map(Value::Enum),
            _ => None,
        };
        return number.unwrap_or_else(|| Value::Other(t, data.to_vec()));
    };
    let mut c = enc.cursor(data);
    let value = match layout {
        Layout::Point if data.len() == 16 => {
            (|| Ok::<_, Error>(Value::Point(c.f64()?, c.f64()?)))().ok()
        }
        Layout::Point => None,
        Layout::String => (|| {
            c.flag()?;
            c.string()
        })()
        .ok()
        .map(Value::String),
        Layout::Text => (|| {
            let n = c.u32()? as usize;
            c.segments(n)
        })()
        .ok()
        .map(Value::Text),
        Layout::TabList => tab_list(&mut c).map(Value::TabList),
        Layout::NestedStyles | Layout::StyleList => c.u32().ok().map(|count| Value::StyleList {
            count,
            nested: match layout {
                Layout::NestedStyles => nested_styles(&mut enc.cursor(data)),
                _ => None,
            },
        }),
        Layout::BulletChar if data.len() == 8 => (|| {
            Ok::<_, Error>(Value::BulletChar {
                kind: c.u32()?,
                value: c.u32()?,
            })
        })()
        .ok(),
        Layout::BulletChar => None,
        Layout::Words => (|| Ok::<_, Error>(Value::Words(c.u32()?, c.u32()?)))().ok(),
        Layout::RefOrCode if data.len() == 8 => {
            (|| Ok::<_, Error>(Value::RefOrCode(c.u32()?, c.u32()?)))().ok()
        }
        Layout::RefOrCode => None,
    };
    value.unwrap_or_else(|| Value::Other(t, data.to_vec()))
}

/// A tab list: u16 count, then per stop f64 position, u16 alignment, u16
/// leader length and the leader in UTF-16 code units. `None` unless the
/// stops fill the value exactly.
fn tab_list(c: &mut Cursor) -> Option<Vec<TabStop>> {
    let n = c.u16().ok()?;
    let mut out = Vec::new();
    for _ in 0..n {
        let position = c.f64().ok()?;
        let alignment = c.u16().ok()?;
        let len = c.u16().ok()? as usize;
        let units = (0..len)
            .map(|_| c.u16())
            .collect::<Result<Vec<_>, _>>()
            .ok()?;
        out.push(TabStop {
            position,
            alignment,
            leader: String::from_utf16_lossy(&units),
        });
    }
    (c.remaining() == 0).then_some(out)
}

/// Nested styles: u32 count, then per style u32 character style, u32
/// length and the delimiter code as text segments. `None` unless every
/// delimiter is known and the styles fill the value exactly.
fn nested_styles(c: &mut Cursor) -> Option<Vec<NestedStyle>> {
    let n = c.u32().ok()?;
    let mut out = Vec::new();
    for _ in 0..n {
        let style = c.u32().ok()?;
        let len = c.u32().ok()? as usize;
        let code = if len == 0 {
            String::new()
        } else {
            c.segments(len).ok()?
        };
        let (delimiter, repetition, inclusive) = nested_delimiter(&code)?;
        out.push(NestedStyle {
            style,
            delimiter,
            repetition,
            inclusive,
        });
    }
    (c.remaining() == 0).then_some(out)
}

/// Page item attributes whose value is a list of opacity gradient stops:
/// u32 count, then three f64 per stop. See `docs/format/transparency.md`.
pub const OPACITY_STOPS: [u32; 3] = [0x1EB8C, 0x1EB95, 0x1EB9E];

fn decode(enc: Encoding, id: u32, t: u32, data: &[u8]) -> Value {
    let f = |o: usize| enc.f64_at(data, o);
    if OPACITY_STOPS.contains(&id)
        && let Some(n) = enc.u32_at(data, 0).map(|n| n as usize)
        && n > 0
        && data.len() == 4 + n * 24
        && let Some(stops) = (0..n)
            .map(|i| {
                let at = |k: usize| f(4 + (3 * i + k) * 8);
                Some([at(0)?, at(1)?, at(2)?])
            })
            .collect::<Option<Vec<_>>>()
    {
        return Value::Stops(stops);
    }
    let value = match (t, data.len()) {
        (ty::DOUBLE, 8) => f(0).map(Value::Double),
        (ty::INT, 4) => enc.i32_at(data, 0).map(Value::Int),
        (ty::ENUM, 2) => enc.u16_at(data, 0).map(Value::Enum),
        (ty::REF, 4) => enc.u32_at(data, 0).map(Value::Ref),
        (ty::POINT, 16) => f(0).zip(f(8)).map(|(x, y)| Value::Point(x, y)),
        // A flag byte and an in-object string, as form fields store their
        // names (objects.md, form fields).
        (ty::STRING | ty::FORM_STRING, _) => (|| {
            let mut c = enc.cursor(data);
            c.flag()?;
            Ok::<_, Error>(Value::String(c.string()?))
        })()
        .ok(),
        // Other 4-byte types are references or codes (for example a
        // corner effect ID); keep the number.
        (_, 4) => enc.u32_at(data, 0).map(Value::Ref),
        _ => None,
    };
    value.unwrap_or_else(|| Value::Other(t, data.to_vec()))
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
        let a = Attrs::parse(Encoding::default(), &d, List::Item, None).unwrap();
        assert_eq!(a.get(0x6E65), Some(&Value::Double(0.25)));
    }

    #[test]
    fn parses_a_stroke_type_code() {
        // 0x6E6E: reference 0, code 0x5A39, then the 6-byte trailer value.
        let mut d = 1u32.to_le_bytes().to_vec();
        d.extend_from_slice(&0x6E6Eu32.to_le_bytes());
        d.extend_from_slice(&34u16.to_le_bytes());
        d.extend_from_slice(&3u16.to_le_bytes());
        for (t, v) in [(ty::REF, 0u32), (ty::CODE, 0x5A39)] {
            d.extend_from_slice(&t.to_le_bytes());
            d.extend_from_slice(&4u16.to_le_bytes());
            d.extend_from_slice(&v.to_le_bytes());
        }
        d.extend_from_slice(&0x6E63u32.to_le_bytes());
        d.extend_from_slice(&6u16.to_le_bytes());
        d.extend_from_slice(&[0; 6]);
        let a = Attrs::parse(Encoding::default(), &d, List::Item, None).unwrap();
        assert_eq!(a.get(0x6E6E), Some(&Value::RefOrCode(0, 0x5A39)));
    }

    /// One text attribute record: ID, then a single value of type 0.
    fn text_record(id: u32, value: &[u8]) -> Vec<u8> {
        let mut d = id.to_le_bytes().to_vec();
        d.extend_from_slice(&(8 + value.len() as u16).to_le_bytes());
        d.extend_from_slice(&1u16.to_le_bytes());
        d.extend_from_slice(&0u32.to_le_bytes());
        d.extend_from_slice(&(value.len() as u16).to_le_bytes());
        d.extend_from_slice(value);
        d
    }

    fn text_value(id: u32, value: &[u8]) -> Value {
        let d = text_record(id, value);
        let a =
            Attrs::parse_text(&mut Encoding::default().cursor(&d), 1, List::Text, None).unwrap();
        a.get(id).unwrap().clone()
    }

    #[test]
    fn decodes_tab_stops() {
        // Two stops: 12 pt left aligned, 237.5 pt right aligned with "." leader.
        let mut b = 2u16.to_le_bytes().to_vec();
        b.extend(12f64.to_le_bytes());
        b.extend([0, 0, 0, 0]);
        b.extend(237.5f64.to_le_bytes());
        b.extend([2, 0, 1, 0, b'.', 0]);
        let Value::TabList(stops) = text_value(0x1B29, &b) else {
            panic!("not a tab list");
        };
        assert_eq!(stops.len(), 2);
        assert_eq!((stops[0].position, stops[0].alignment), (12.0, 0));
        assert_eq!(
            (
                stops[1].position,
                stops[1].alignment,
                stops[1].leader.as_str()
            ),
            (237.5, 2, ".")
        );
        assert_eq!(text_value(0x1B29, &[0, 0]), Value::TabList(Vec::new()));
        // A stop cut short.
        assert!(matches!(text_value(0x1B29, &b[..20]), Value::Other(..)));
    }

    #[test]
    fn decodes_nested_style_delimiters() {
        let d = |c| nested_delimiter(c);
        assert_eq!(d("^c"), Some((Delimiter::Dropcap, 1, true)));
        assert_eq!(d("[.]"), Some((Delimiter::Character(".".into()), 1, false)));
        assert_eq!(d("(:)"), Some((Delimiter::Character(":".into()), 1, true)));
        assert_eq!(d("(^w)5"), Some((Delimiter::AnyWord, 5, true)));
        assert_eq!(d("(^?)"), Some((Delimiter::AnyCharacter, 1, true)));
        assert_eq!(d("(^x)"), None);
        assert_eq!(d("^t"), None);
    }

    #[test]
    fn decodes_nested_styles() {
        // One style: character style 7 up to and including 2 words.
        let mut b = 1u32.to_le_bytes().to_vec();
        b.extend(7u32.to_le_bytes());
        b.extend(5u32.to_le_bytes());
        b.extend([5, 0x40, b'(', b'^', b'w', b')', b'2']);
        let nested = vec![NestedStyle {
            style: 7,
            delimiter: Delimiter::AnyWord,
            repetition: 2,
            inclusive: true,
        }];
        assert_eq!(
            text_value(0x1B75, &b),
            Value::StyleList {
                count: 1,
                nested: Some(nested)
            }
        );
        // GREP styles: only the count is read.
        assert_eq!(
            text_value(0x1BBA, &b),
            Value::StyleList {
                count: 1,
                nested: None
            }
        );
    }

    #[test]
    fn decodes_bullet_chars_and_strings() {
        let b = [0, 0, 0, 0, 0x22, 0x20, 0, 0];
        assert_eq!(
            text_value(0x1A406, &b),
            Value::BulletChar {
                kind: 0,
                value: 0x2022
            }
        );
        // Font style: flag byte, then an in-object string.
        let b = [0, 2, 0, 4, 0, 4, 0x40, b'B', b'o', b'l', b'd'];
        assert_eq!(text_value(0x1B02, &b), Value::String("Bold".into()));
        // Ruby font style: an empty string whose second byte is 1.
        let b = [0, 2, 1, 0, 0];
        assert_eq!(text_value(0x4234, &b), Value::String(String::new()));
        // Ruby text: u32 length, then segments.
        let b = [2, 0, 0, 0, 2, 0x40, b'a', b'b'];
        assert_eq!(text_value(0x422E, &b), Value::Text("ab".into()));
        // Other attributes are numbers by length.
        assert_eq!(
            text_value(0x1B03, &12f64.to_le_bytes()),
            Value::Double(12.0)
        );
    }

    #[test]
    fn decodes_opacity_stops() {
        let mut v = 2u32.to_le_bytes().to_vec();
        for f in [0.15, 0.5, 100.0, 0.85, 1.0, 0.0f64] {
            v.extend(f.to_le_bytes());
        }
        let mut d = 1u32.to_le_bytes().to_vec();
        d.extend(0x1EB8Cu32.to_le_bytes());
        d.extend((8 + v.len() as u16).to_le_bytes());
        d.extend(1u16.to_le_bytes());
        d.extend(0u32.to_le_bytes());
        d.extend((v.len() as u16).to_le_bytes());
        d.extend(&v);
        let a = Attrs::parse(Encoding::default(), &d, List::Item, None).unwrap();
        assert_eq!(
            a.get(0x1EB8C),
            Some(&Value::Stops(vec![[0.15, 0.5, 100.0], [0.85, 1.0, 0.0]]))
        );
    }
}
