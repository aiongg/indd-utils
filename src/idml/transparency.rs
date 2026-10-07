//! Transparency settings of page items. See `docs/format/transparency.md`.

use super::applied::{StyleValues, same_value};
use super::kind::Kind;
use super::xml::Xml;
use super::{Writer, num};
use crate::model::{Attrs, Value};

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
        Kind::Range(0.0, 100.0),
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
        Kind::Range(0.0, 1000.0),
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
        Kind::Range(0.0, 1000.0),
    ),
    (
        0x10855,
        "TransparencySetting",
        "InnerShadowSetting",
        "Size",
        Kind::Range(0.0, 1000.0),
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
        Kind::Range(-180.0, 180.0),
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
        Kind::Range(-180.0, 180.0),
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
        Kind::Range(-180.0, 180.0),
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

/// Opacity stops as IDML writes them, from the stops of the attribute
/// list (location, absolute midpoint position, opacity). `None` for no
/// stops, or two stops at the same location (the midpoint is undefined).
fn stops(raw: &[[f64; 3]]) -> Option<Vec<Stop>> {
    if raw.is_empty() || (1..raw.len()).any(|i| raw[i][0] == raw[i - 1][0]) {
        return None;
    }
    Some(
        raw.iter()
            .enumerate()
            .map(|(i, &[location, _, opacity])| {
                let midpoint = (i > 0).then(|| {
                    let [prev, mid, _] = raw[i - 1];
                    (mid - prev) / (location - prev) * 100.0
                });
                Stop {
                    opacity,
                    location: super::round(location * 100.0),
                    midpoint,
                }
            })
            .collect(),
    )
}

/// Write the transparency settings found in a page item's attribute list.
/// IDML writes only the values that differ from the object style `style`
/// (all values without a style), and an effect element only when one of
/// its values or its opacity stops differ. `item` is the item's `Self`,
/// which names the opacity stops. Returns the values left out because
/// they are outside the schema's range, as (effect, attribute, value).
pub(super) fn write(
    w: &Writer,
    x: &mut Xml,
    attrs: &Attrs,
    style: Option<&StyleValues>,
    item: &str,
) -> Vec<(&'static str, &'static str, f64)> {
    let mut left_out = Vec::new();
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
                let Some(v) = attrs.get(id) else { continue };
                if let Kind::Range(lo, hi) = kind
                    && let Some(f) = v.as_f64()
                    && !(lo..=hi).contains(&f)
                {
                    left_out.push((effect, name, f));
                    continue;
                }
                match w.value_text(kind, v) {
                    Some(t) => {
                        let same = style.is_some_and(|s| {
                            s.transparency(id)
                                .and_then(|sv| w.value_text(kind, sv))
                                .is_some_and(|st| same_value(&st, &t))
                        });
                        if !same {
                            values.push((name, t));
                        }
                    }
                    None if kind.is_code() => attrs.unknown_code(id, v),
                    None => {}
                }
            }
            let stop_list = if effect == "GradientFeatherSetting" {
                STOPS
                    .iter()
                    .find(|(_, s)| *s == setting)
                    .and_then(|(id, _)| {
                        match attrs.get(*id) {
                        Some(Value::Stops(raw))
                            if !style.is_some_and(|s| {
                                matches!(s.transparency(*id), Some(Value::Stops(st)) if st == raw)
                            }) =>
                        {
                            stops(raw)
                        }
                        _ => None,
                    }
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
    left_out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_opacity_stops() {
        // Two stops: location 0.15, midpoint at 0.5, opacity 100; location
        // 0.85, midpoint 1, opacity 0.
        let s = stops(&[[0.15, 0.5, 100.0], [0.85, 1.0, 0.0]]).unwrap();
        assert_eq!(s.len(), 2);
        assert_eq!(
            (s[0].opacity, s[0].location, s[0].midpoint),
            (100.0, 15.0, None)
        );
        assert_eq!(s[1].location, 85.0);
        assert!((s[1].midpoint.unwrap() - 50.0).abs() < 1e-9);
        assert_eq!(stops(&[]), None);
    }

    #[test]
    fn stops_at_one_location_are_not_read() {
        assert_eq!(stops(&[[0.5, 0.5, 100.0], [0.5, 1.0, 0.0]]), None);
    }
}
