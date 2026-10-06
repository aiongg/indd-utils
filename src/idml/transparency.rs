//! Transparency settings of page items. See `docs/format/transparency.md`.

use std::collections::BTreeMap;

use super::num;
use super::xml::Xml;
use crate::model::{Attrs, Value};

#[derive(Clone, Copy)]
enum Kind {
    Number,
    Bool,
    Point,
    Enum(&'static [(u32, &'static str)]),
    /// A swatch UID, written only when the effect is applied.
    Swatch,
}

/// Blend mode codes seen in the corpus.
const BLEND_MODES: &[(u32, &str)] = &[
    (0, "Normal"),
    (1, "Multiply"),
    (3, "Overlay"),
    (9, "Lighten"),
];

/// Transparency elements on a page item, in the order the schema lists them.
const SETTINGS: &[&str] = &[
    "TransparencySetting",
    "StrokeTransparencySetting",
    "FillTransparencySetting",
];

/// Effect elements of a setting, in schema order.
const EFFECTS: &[&str] = &[
    "BlendingSetting",
    "DropShadowSetting",
    "InnerShadowSetting",
    "GradientFeatherSetting",
];

/// Attribute-list ID, setting, effect, IDML attribute, value kind; in the
/// schema's attribute order within each effect.
const ATTRS: &[(u32, &str, &str, &str, Kind)] = &[
    (
        0x10817,
        "TransparencySetting",
        "BlendingSetting",
        "BlendMode",
        Kind::Enum(BLEND_MODES),
    ),
    (
        0x10816,
        "TransparencySetting",
        "BlendingSetting",
        "Opacity",
        Kind::Number,
    ),
    (
        0x1081A,
        "TransparencySetting",
        "DropShadowSetting",
        "Mode",
        Kind::Enum(&[(0, "None"), (1, "Drop")]),
    ),
    (
        0x10820,
        "TransparencySetting",
        "DropShadowSetting",
        "Size",
        Kind::Number,
    ),
    (
        0x1084D,
        "TransparencySetting",
        "InnerShadowSetting",
        "Applied",
        Kind::Bool,
    ),
    (
        0x1084E,
        "TransparencySetting",
        "InnerShadowSetting",
        "EffectColor",
        Kind::Swatch,
    ),
    (
        0x10852,
        "TransparencySetting",
        "InnerShadowSetting",
        "Distance",
        Kind::Number,
    ),
    (
        0x10855,
        "TransparencySetting",
        "InnerShadowSetting",
        "Size",
        Kind::Number,
    ),
    (
        0x1EB8A,
        "TransparencySetting",
        "GradientFeatherSetting",
        "Applied",
        Kind::Bool,
    ),
    (
        0x1EB8D,
        "TransparencySetting",
        "GradientFeatherSetting",
        "Angle",
        Kind::Number,
    ),
    (
        0x1EB8E,
        "TransparencySetting",
        "GradientFeatherSetting",
        "Length",
        Kind::Number,
    ),
    (
        0x1EB8F,
        "TransparencySetting",
        "GradientFeatherSetting",
        "GradientStart",
        Kind::Point,
    ),
    (
        0x1EB91,
        "TransparencySetting",
        "GradientFeatherSetting",
        "HiliteAngle",
        Kind::Number,
    ),
    (
        0x1EB92,
        "TransparencySetting",
        "GradientFeatherSetting",
        "HiliteLength",
        Kind::Number,
    ),
    (
        0x1EB93,
        "StrokeTransparencySetting",
        "GradientFeatherSetting",
        "Applied",
        Kind::Bool,
    ),
    (
        0x1EB96,
        "StrokeTransparencySetting",
        "GradientFeatherSetting",
        "Angle",
        Kind::Number,
    ),
    (
        0x1EB97,
        "StrokeTransparencySetting",
        "GradientFeatherSetting",
        "Length",
        Kind::Number,
    ),
    (
        0x1EB98,
        "StrokeTransparencySetting",
        "GradientFeatherSetting",
        "GradientStart",
        Kind::Point,
    ),
    (
        0x1EB9C,
        "FillTransparencySetting",
        "GradientFeatherSetting",
        "Applied",
        Kind::Bool,
    ),
    (
        0x1EB9F,
        "FillTransparencySetting",
        "GradientFeatherSetting",
        "Angle",
        Kind::Number,
    ),
    (
        0x1EBA0,
        "FillTransparencySetting",
        "GradientFeatherSetting",
        "Length",
        Kind::Number,
    ),
    (
        0x1EBA1,
        "FillTransparencySetting",
        "GradientFeatherSetting",
        "GradientStart",
        Kind::Point,
    ),
];

/// Opacity gradient stops of a gradient feather, per setting.
const STOPS: &[(u32, &str)] = &[
    (0x1EB8C, "TransparencySetting"),
    (0x1EB95, "StrokeTransparencySetting"),
    (0x1EB9E, "FillTransparencySetting"),
];

/// Applied flag of an effect whose colour is written only when applied.
const INNER_SHADOW_APPLIED: u32 = 0x1084D;

/// An effect element: name, attributes and opacity stops.
type Effect = (&'static str, Vec<(&'static str, String)>, Vec<Stop>);

/// One opacity gradient stop, as IDML writes it.
#[derive(Debug, PartialEq)]
struct Stop {
    opacity: f64,
    location: f64,
    /// Relative position of the midpoint between the previous stop and
    /// this one, in percent; `None` for the first stop.
    midpoint: Option<f64>,
}

/// Opacity stops: u32 count, then per stop f64 location (0–1), f64
/// position of the midpoint to the next stop (0–1, absolute), f64 opacity
/// in percent.
fn stops(data: &[u8]) -> Option<Vec<Stop>> {
    let n = u32::from_le_bytes(data.get(..4)?.try_into().ok()?) as usize;
    if n == 0 || data.len() != 4 + n * 24 {
        return None;
    }
    let f = |i: usize| f64::from_le_bytes(data[4 + i * 8..12 + i * 8].try_into().unwrap());
    Some(
        (0..n)
            .map(|i| {
                let location = f(3 * i);
                let midpoint = (i > 0).then(|| {
                    let (prev, mid) = (f(3 * (i - 1)), f(3 * (i - 1) + 1));
                    (mid - prev) / (location - prev) * 100.0
                });
                Stop {
                    opacity: f(3 * i + 2),
                    location: super::round(location * 100.0),
                    midpoint,
                }
            })
            .collect(),
    )
}

fn value(v: &Value, kind: Kind, swatches: &BTreeMap<u32, String>) -> Option<String> {
    match kind {
        Kind::Number => v.as_f64().map(num),
        Kind::Bool => match v.as_u32()? {
            0 => Some("false".into()),
            1 => Some("true".into()),
            _ => None,
        },
        Kind::Point => match v {
            Value::Point(a, b) => Some(format!("{} {}", num(*a), num(*b))),
            _ => None,
        },
        Kind::Enum(map) => {
            let code = v.as_u32()?;
            map.iter()
                .find(|(k, _)| *k == code)
                .map(|(_, n)| n.to_string())
        }
        Kind::Swatch => v.as_ref().and_then(|u| swatches.get(&u).cloned()),
    }
}

/// Write the transparency settings found in a page item's attribute list.
/// `item` is the item's `Self`, which names the opacity stops.
pub(super) fn write(x: &mut Xml, attrs: &Attrs, item: &str, swatches: &BTreeMap<u32, String>) {
    let inner_shadow_applied = attrs.get(INNER_SHADOW_APPLIED).and_then(Value::as_u32) == Some(1);
    for &setting in SETTINGS {
        let mut effects: Vec<Effect> = Vec::new();
        for &effect in EFFECTS {
            let mut values = Vec::new();
            for &(id, s, e, name, kind) in ATTRS {
                if s != setting || e != effect {
                    continue;
                }
                if matches!(kind, Kind::Swatch) && !inner_shadow_applied {
                    continue;
                }
                if let Some(t) = attrs.get(id).and_then(|v| value(v, kind, swatches)) {
                    values.push((name, t));
                }
            }
            let stop_list = if effect == "GradientFeatherSetting" {
                STOPS
                    .iter()
                    .find(|(_, s)| *s == setting)
                    .and_then(|(id, _)| match attrs.get(*id) {
                        Some(Value::Other(_, data)) => stops(data),
                        _ => None,
                    })
                    .unwrap_or_default()
            } else {
                Vec::new()
            };
            if !values.is_empty() || !stop_list.is_empty() {
                effects.push((effect, values, stop_list));
            }
        }
        if effects.is_empty() {
            continue;
        }
        x.start(setting);
        for (effect, values, stop_list) in effects {
            x.start(effect);
            for (name, v) in values {
                x.attr(name, v);
            }
            for (i, s) in stop_list.iter().enumerate() {
                x.start("OpacityGradientStop")
                    .attr(
                        "Self",
                        format!("{item}{setting}1{effect}1OpacityGradientStop{i}"),
                    )
                    .attr("Opacity", num(s.opacity))
                    .attr("Location", num(s.location));
                if let Some(m) = s.midpoint {
                    x.attr("Midpoint", num(m));
                }
                x.end();
            }
            x.end();
        }
        x.end();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_opacity_stops() {
        // Two stops: location 0.15, midpoint at 0.5, opacity 100; location
        // 0.85, midpoint 1, opacity 0.
        let mut d = 2u32.to_le_bytes().to_vec();
        for v in [0.15, 0.5, 100.0, 0.85, 1.0, 0.0f64] {
            d.extend_from_slice(&v.to_le_bytes());
        }
        let s = stops(&d).unwrap();
        assert_eq!(s.len(), 2);
        assert_eq!(
            (s[0].opacity, s[0].location, s[0].midpoint),
            (100.0, 15.0, None)
        );
        assert_eq!(s[1].location, 85.0);
        assert!((s[1].midpoint.unwrap() - 50.0).abs() < 1e-9);
        assert_eq!(stops(&0u32.to_le_bytes()), None);
    }
}
