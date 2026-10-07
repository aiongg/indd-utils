//! Text forms of IDML values: numbers, matrices, base64 data, object
//! references, escaped names, UI colours and `<Properties>` children. These
//! follow the IDML specification; they hold no INDD facts.

use super::*;

/// The IDML name of an XML tag colour (red, green, blue fractions);
/// `None` for colours without evidence. See `docs/format/objects.md`.
pub(super) fn xml_tag_color(rgb: [f64; 3]) -> Option<&'static str> {
    ui_color_name(rgb)
}

/// The IDML name of an interface colour (class 0x1F11) by its red, green
/// and blue fractions. See `docs/format/objects.md`, interface colours.
pub(super) fn ui_color_name(rgb: [f64; 3]) -> Option<&'static str> {
    const NAMES: [([f64; 3], &str); 25] = [
        ([0.31, 0.6, 1.0], "LightBlue"),
        ([1.0, 0.0, 0.0], "Red"),
        ([0.31, 1.0, 0.31], "Green"),
        ([0.0, 0.0, 1.0], "Blue"),
        ([1.0, 1.0, 0.31], "Yellow"),
        ([1.0, 0.31, 1.0], "Magenta"),
        ([0.0, 1.0, 1.0], "Cyan"),
        ([0.5, 0.5, 0.5], "Gray"),
        ([0.0, 0.0, 0.0], "Black"),
        ([0.6, 0.0, 0.0], "BrickRed"),
        ([1.0, 0.6, 0.0], "Gold"),
        ([1.0, 0.4, 0.0], "Orange"),
        ([0.0, 0.33, 0.0], "DarkGreen"),
        ([0.6, 0.6, 1.0], "Lavender"),
        ([0.67, 0.64, 0.71], "Charcoal"),
        ([0.6, 0.2, 1.0], "Violet"),
        ([1.0, 1.0, 1.0], "White"),
        ([1.0, 0.6, 0.8], "Pink"),
        ([0.61, 0.87, 0.61], "GridGreen"),
        ([0.0, 0.0, 0.53], "DarkBlue"),
        ([0.6, 0.8, 0.0], "GrassGreen"),
        ([0.81, 0.51, 0.71], "Lipstick"),
        ([1.0, 0.71, 0.42], "GridOrange"),
        ([0.97, 0.35, 0.42], "Fiesta"),
        ([0.0, 0.6, 0.6], "Teal"),
    ];
    NAMES
        .iter()
        .find(|(c, _)| c.iter().zip(rgb).all(|(a, b)| (a - b).abs() < 1e-4))
        .map(|(_, n)| *n)
}

/// The `PageColor` property of a page colour.
pub(super) fn page_color(c: &UiColorRef) -> Option<Node> {
    let name = match c {
        UiColorRef::UseMaster => "UseMasterColor",
        UiColorRef::Nothing => "Nothing",
        UiColorRef::Rgb(rgb) => return ui_color_property("PageColor", *rgb),
        _ => return None,
    };
    Some(Node {
        tag: "PageColor".into(),
        attrs: vec![("type".into(), "enumeration".into())],
        text: Some(name.into()),
        children: Vec::new(),
    })
}

/// A `Properties` child as a values node.
pub(super) fn prop_node(p: &Property) -> Node {
    let (name, ty, value) = p;
    let mut n = Node {
        tag: name.to_string(),
        ..Node::default()
    };
    if !ty.is_empty() {
        n.attrs.push(("type".into(), ty.to_string()));
    }
    match value {
        PropValue::Text(t) => n.text = Some(t.clone()),
        PropValue::Attributes(a) => {
            n.attrs
                .extend(a.iter().map(|(k, v)| (k.to_string(), v.clone())));
        }
        PropValue::List(items) => {
            for item in items {
                n.children.push(Node {
                    tag: "ListItem".into(),
                    attrs: vec![("type".into(), "record".into())],
                    children: item
                        .iter()
                        .map(|(f, t, text)| Node {
                            tag: f.to_string(),
                            attrs: vec![("type".into(), t.to_string())],
                            text: Some(text.clone()),
                            children: Vec::new(),
                        })
                        .collect(),
                    text: None,
                });
            }
        }
    }
    n
}

/// A `Properties` child for an interface colour: its name as an
/// enumeration, or, for a colour of whole 255ths without a name, the three
/// numbers as a list. `None` for other colours.
pub(super) fn ui_color_property(tag: &str, rgb: [f64; 3]) -> Option<Node> {
    if let Some(name) = ui_color_name(rgb) {
        return Some(Node {
            tag: tag.into(),
            attrs: vec![("type".into(), "enumeration".into())],
            text: Some(name.into()),
            children: Vec::new(),
        });
    }
    let whole: Vec<f64> = rgb.iter().map(|v| (v * 255.0).round()).collect();
    if whole
        .iter()
        .zip(rgb)
        .any(|(w, v)| (w / 255.0 - v).abs() > 1e-6)
    {
        return None;
    }
    Some(Node {
        tag: tag.into(),
        attrs: vec![("type".into(), "list".into())],
        text: None,
        children: whole
            .iter()
            .map(|w| Node {
                tag: "ListItem".into(),
                attrs: vec![("type".into(), "double".into())],
                text: Some(num(*w)),
                children: Vec::new(),
            })
            .collect(),
    })
}

/// Format a number the way IDML does: shortest round-trip form. IDML
/// keeps negative zero (`1 -0 -0 1 0 0`).
/// A number as IDML text. IDML has no text for NaN or infinity; they are
/// written as 0, which keeps the package valid.
pub fn num(v: f64) -> String {
    if v.is_finite() {
        format!("{v}")
    } else {
        "0".to_string()
    }
}

/// Round away binary noise from scaled values (0.8 * 100 = 80.00000000000001).
pub(crate) fn round(v: f64) -> f64 {
    (v * 1e9).round() / 1e9
}

pub(super) fn nums(v: &[f64]) -> String {
    v.iter().map(|&x| num(x)).collect::<Vec<_>>().join(" ")
}

pub(super) fn matrix(m: &Matrix) -> String {
    nums(&m.0)
}

/// Characters per CDATA section of embedded file data, as in InDesign's
/// IDML export.
pub(super) const CDATA_SECTION: usize = 262_144;

/// Base64 (RFC 4648, with padding) in lines of 76 characters separated by
/// line feeds, as IDML writes embedded file data.
pub(super) fn base64_lines(data: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    const LINE: usize = 76;
    let encoded_len = data.len().div_ceil(3) * 4;
    let mut out = String::with_capacity(encoded_len + encoded_len / LINE);
    let mut column = 0;
    for group in data.chunks(3) {
        let b = [
            group[0],
            group.get(1).copied().unwrap_or(0),
            group.get(2).copied().unwrap_or(0),
        ];
        let n = (b[0] as u32) << 16 | (b[1] as u32) << 8 | b[2] as u32;
        if column == LINE {
            out.push('\n');
            column = 0;
        }
        for i in 0..4 {
            let c = if i <= group.len() {
                ALPHABET[(n >> (18 - 6 * i) & 0x3F) as usize] as char
            } else {
                '='
            };
            out.push(c);
        }
        column += 4;
    }
    out
}

pub(super) fn uref(uid: Option<u32>) -> String {
    uid.map_or("n".into(), |u| format!("u{u:x}"))
}

/// Name as used in a style's `Self` and in references: `$ID/` for built-ins.
pub(super) fn style_name(s: &Style) -> String {
    if s.builtin {
        builtin_key(&s.name)
    } else {
        s.name.clone()
    }
}

/// A text variable name as IDML writes it: control characters (U+001B in
/// the built-in cross-reference variables) become `<?AID 00xx?>`.
pub(super) fn variable_name(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    for c in name.chars() {
        if (c as u32) < 0x20 {
            out.push_str(&format!("<?AID {:04x}?>", c as u32));
        } else {
            out.push(c);
        }
    }
    out
}

/// Escape a name for use in a `Self` reference (`:` separates style
/// groups there).
pub(super) fn self_name(name: &str) -> String {
    name.replace('%', "%25").replace(':', "%3a")
}

impl Writer<'_> {
    pub(super) fn properties(x: &mut Xml, props: &[Property]) {
        Self::properties_with(x, props, &[]);
    }

    /// `<Properties>` with `props`, then the elements in `extra`.
    pub(super) fn properties_with(x: &mut Xml, props: &[Property], extra: &[Node]) {
        if props.is_empty() && extra.is_empty() {
            return;
        }
        x.start("Properties");
        for (name, ty, value) in props {
            x.start(name);
            if !ty.is_empty() {
                x.attr("type", *ty);
            }
            match value {
                PropValue::Attributes(attrs) => {
                    for (k, v) in attrs {
                        x.attr(k, v);
                    }
                }
                PropValue::Text(text) => {
                    x.text(text);
                }
                PropValue::List(items) => {
                    for item in items {
                        x.start("ListItem").attr("type", "record");
                        for (field, ty, text) in item {
                            x.start(field).attr("type", *ty).text(text).end();
                        }
                        x.end();
                    }
                }
            }
            x.end();
        }
        for n in extra {
            n.write(x);
        }
        x.end();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_without_text_are_written_as_zero() {
        assert_eq!(num(1.5), "1.5");
        assert_eq!(num(f64::NAN), "0");
        assert_eq!(num(f64::INFINITY), "0");
        assert_eq!(num(f64::NEG_INFINITY), "0");
    }

    #[test]
    fn formats_numbers_like_idml() {
        assert_eq!(num(205.2), "205.2");
        assert_eq!(num(-0.0), "-0");
        assert_eq!(num(1.0), "1");
        assert_eq!(num(-89.99999999999999), "-89.99999999999999");
    }

    #[test]
    fn encodes_base64_in_lines() {
        assert_eq!(base64_lines(b""), "");
        assert_eq!(base64_lines(b"f"), "Zg==");
        assert_eq!(base64_lines(b"fo"), "Zm8=");
        assert_eq!(base64_lines(b"foobar"), "Zm9vYmFy");
        let lines = base64_lines(&[0xFF; 58]);
        assert_eq!(lines.split('\n').map(str::len).collect::<Vec<_>>(), [76, 4]);
        assert!(base64_lines(&[0xFF; 57]).find('\n').is_none());
    }
}
