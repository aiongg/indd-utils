//! Colours and swatches. See `docs/format/objects.md`.

use crate::Error;
use crate::object::Cursor;

pub mod class {
    pub const COLOR: u32 = 0x1F05;
    pub const SWATCH_NONE: u32 = 0x6E0B;
    pub const GRADIENT: u32 = 0x5503;
    pub const INK: u32 = 0x1F07;
    pub const COLOR_GROUP: u32 = 0x1F39;
}

pub mod chunk {
    pub const COLOR_VALUE: u32 = 0x1F01;
    pub const COLOR_MODEL: u32 = 0x1F09;
    pub const COLOR_NAME: u32 = 0x1F10;
    /// f64 tint value (−1 for colours), then u32 colour override.
    pub const COLOR_OVERRIDE: u32 = 0x1F24;
    /// Base colour of a tint.
    pub const TINT_BASE: u32 = 0x117;
    pub const GRADIENT_STOPS: u32 = 0x5503;
    pub const GRADIENT_NAME: u32 = 0x5505;
    pub const INK: u32 = 0x1F0D;
    pub const COLOR_GROUP_NAME: u32 = 0x13C;
    pub const COLOR_GROUP_SWATCHES: u32 = 0x1F60;
    /// In the preferences object: u32 list of the colour groups, the root
    /// group first.
    pub const COLOR_GROUPS: u32 = 0x1F61;
}

#[derive(Debug, Clone, PartialEq)]
pub struct GradientStop {
    pub color: u32,
    /// 0–1.
    pub location: f64,
    /// 0–1, between this stop and the next one.
    pub midpoint: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Gradient {
    pub uid: u32,
    pub name: String,
    pub builtin_name: bool,
    /// 1 linear, 2 radial.
    pub kind: u32,
    pub stops: Vec<GradientStop>,
    pub editable: bool,
    pub removable: bool,
    pub visible: bool,
}

impl Gradient {
    pub fn reference(&self) -> String {
        if self.name.is_empty() {
            format!("Gradient/u{:x}", self.uid)
        } else {
            format!(
                "Gradient/{}",
                self.name.replace('%', "%25").replace(':', "%3a")
            )
        }
    }

    pub fn idml_name(&self) -> String {
        if self.builtin_name {
            format!("$ID/{}", self.name)
        } else {
            self.name.clone()
        }
    }

    pub fn read(uid: u32, obj: &crate::Object) -> Result<Option<Gradient>, Error> {
        let (Some(stops), Some(name)) = (
            obj.chunk(chunk::GRADIENT_STOPS),
            obj.chunk(chunk::GRADIENT_NAME),
        ) else {
            return Ok(None);
        };
        let mut c = Cursor::new(stops);
        let n = c.u16()? as usize;
        let colors = (0..n).map(|_| c.u32()).collect::<Result<Vec<_>, _>>()?;
        let locations = (0..n).map(|_| c.f64()).collect::<Result<Vec<_>, _>>()?;
        let midpoints = (0..n).map(|_| c.f64()).collect::<Result<Vec<_>, _>>()?;
        let kind = c.u32()?;
        let mut nc = Cursor::new(name);
        let builtin_name = nc.u8()? == 1;
        let name = nc.string()?;
        let flags = nc.u32()?;
        Ok(Some(Gradient {
            uid,
            name,
            builtin_name,
            kind,
            stops: (0..n)
                .map(|i| GradientStop {
                    color: colors[i],
                    location: locations[i],
                    midpoint: midpoints[i],
                })
                .collect(),
            removable: flags & 1 != 0,
            visible: flags & 2 != 0,
            editable: flags & 4 != 0,
        }))
    }
}

/// A tint swatch: a colour object with a base colour and a tint value
/// instead of a name and colour values.
#[derive(Debug, Clone, PartialEq)]
pub struct Tint {
    pub uid: u32,
    pub base: u32,
    /// Percent.
    pub value: f64,
    pub color_override: u32,
}

impl Tint {
    pub fn read(uid: u32, obj: &crate::Object) -> Result<Option<Tint>, Error> {
        if obj.chunk(chunk::COLOR_NAME).is_some() {
            return Ok(None);
        }
        let (Some(base), Some(tint)) = (
            obj.chunk(chunk::TINT_BASE),
            obj.chunk(chunk::COLOR_OVERRIDE),
        ) else {
            return Ok(None);
        };
        let base = Cursor::new(base).u32()?;
        let mut c = Cursor::new(tint);
        let value = c.f64()?;
        let color_override = c.u32()?;
        if base == 0 || value < 0.0 {
            return Ok(None);
        }
        Ok(Some(Tint {
            uid,
            base,
            value,
            color_override,
        }))
    }

    /// IDML `Name`: the base colour's name and the tint value, with the
    /// black swatch written as `[Black]`.
    pub fn idml_name(&self, base: &Color) -> String {
        let name = if base.color_override == 2 {
            format!("[{}]", base.name)
        } else {
            base.name.clone()
        };
        format!("{name} {}%", crate::idml::num(self.value))
    }

    pub fn reference(&self, base: &Color) -> String {
        format!(
            "Tint/{}",
            self.idml_name(base).replace('%', "%25").replace(':', "%3a")
        )
    }

    pub fn override_name(&self) -> &'static str {
        override_name(self.color_override)
    }
}

fn override_name(code: u32) -> &'static str {
    match code {
        1 => "Specialpaper",
        2 => "Specialblack",
        3 => "Specialregistration",
        4 => "Hiddenreserved",
        _ => "Normal",
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Space {
    Rgb,
    Cmyk,
    Lab,
    Other(u32),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Color {
    pub uid: u32,
    /// Empty for unnamed colours, which IDML names by UID.
    pub name: String,
    /// The name is an InDesign key, written with `$ID/` in IDML.
    pub builtin_name: bool,
    pub model: u32,
    pub space: Space,
    /// Component values as fractions (0–1) for RGB and CMYK.
    pub values: Vec<f64>,
    pub editable: bool,
    pub removable: bool,
    pub visible: bool,
    pub color_override: u32,
}

impl Color {
    /// The IDML reference for this colour (`Color/<name>` or `Color/u<hex>`).
    pub fn reference(&self) -> String {
        if self.name.is_empty() {
            format!("Color/u{:x}", self.uid)
        } else {
            format!(
                "Color/{}",
                self.name.replace('%', "%25").replace(':', "%3a")
            )
        }
    }

    /// The `Name` attribute.
    pub fn idml_name(&self) -> String {
        if self.builtin_name {
            format!("$ID/{}", self.name)
        } else {
            self.name.clone()
        }
    }

    pub fn model_name(&self) -> &'static str {
        match self.model {
            1 => "Spot",
            2 => "Registration",
            3 => "MixedInkModel",
            _ => "Process",
        }
    }

    pub fn space_name(&self) -> &'static str {
        match self.space {
            Space::Rgb => "RGB",
            Space::Lab => "LAB",
            _ => "CMYK",
        }
    }

    /// Component values as IDML writes them: percentages for CMYK, 0–255
    /// for RGB, raw values otherwise.
    pub fn idml_values(&self) -> Vec<f64> {
        match self.space {
            Space::Cmyk => self.values.iter().map(|v| v * 100.0).collect(),
            Space::Rgb => self.values.iter().map(|v| v * 255.0).collect(),
            _ => self.values.clone(),
        }
    }

    pub fn override_name(&self) -> &'static str {
        override_name(self.color_override)
    }

    pub fn read(uid: u32, obj: &crate::Object) -> Result<Option<Color>, Error> {
        let (Some(name), Some(value)) =
            (obj.chunk(chunk::COLOR_NAME), obj.chunk(chunk::COLOR_VALUE))
        else {
            return Ok(None);
        };
        let mut c = Cursor::new(name);
        let builtin_name = c.u8()? == 1;
        let name_str = c.string()?;
        let flags = c.u32()?;
        let mut v = Cursor::new(value);
        let space = match v.u32()? {
            5 => Space::Rgb,
            6 => Space::Cmyk,
            7 => Space::Lab,
            other => Space::Other(other),
        };
        let n = v.u16()?;
        let values = (0..n).map(|_| v.f64()).collect::<Result<Vec<_>, _>>()?;
        let model = match obj.chunk(chunk::COLOR_MODEL) {
            Some(d) => Cursor::new(d).u32()?,
            None => 0,
        };
        let color_override = match obj.chunk(chunk::COLOR_OVERRIDE) {
            Some(d) if d.len() >= 12 => Cursor::new(&d[8..]).u32()?,
            _ => 0,
        };
        Ok(Some(Color {
            uid,
            name: name_str,
            builtin_name,
            model,
            space,
            values,
            removable: flags & 1 != 0,
            visible: flags & 2 != 0,
            editable: flags & 4 != 0,
            color_override,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn color(name: &str, color_override: u32) -> Color {
        Color {
            uid: 0xb,
            name: name.into(),
            builtin_name: false,
            model: 0,
            space: Space::Cmyk,
            values: vec![0.0, 0.0, 0.0, 1.0],
            editable: false,
            removable: false,
            visible: true,
            color_override,
        }
    }

    #[test]
    fn names_tints_after_their_base() {
        let t = Tint {
            uid: 0x3fa,
            base: 0xb,
            value: 40.0,
            color_override: 0,
        };
        assert_eq!(t.idml_name(&color("Black", 2)), "[Black] 40%");
        assert_eq!(t.reference(&color("Gold", 0)), "Tint/Gold 40%25");
    }
}

/// An ink (class 0x1F07). Chunk 0x1F0D: flag byte and name, then fields
/// at offsets from the end of the name: f64 neutral density at 14, u32
/// trap order − 1 at 26, f64 frequency at 32, f64 angle at 40.
#[derive(Debug, Clone, PartialEq)]
pub struct Ink {
    pub uid: u32,
    /// The IDML name, with `$ID/` for a built-in name.
    pub name: String,
    pub neutral_density: f64,
    pub trap_order: u32,
    pub frequency: f64,
    pub angle: f64,
}

impl Ink {
    pub fn read(uid: u32, obj: &crate::Object) -> Result<Option<Ink>, Error> {
        let Some(d) = obj.chunk(chunk::INK) else {
            return Ok(None);
        };
        let mut c = Cursor::new(d);
        let builtin = c.u8()? == 1;
        let name = c.string()?;
        let end = c.pos();
        if d.len() < end + 48 {
            return Ok(None);
        }
        let f = |o: usize| Cursor::new(&d[end + o..]).f64();
        Ok(Some(Ink {
            uid,
            name: if builtin { format!("$ID/{name}") } else { name },
            neutral_density: f(14)?,
            trap_order: Cursor::new(&d[end + 26..]).u32()? + 1,
            frequency: f(32)?,
            angle: f(40)?,
        }))
    }
}

/// A colour group (class 0x1F39): chunk 0x13C is a flag byte and the name,
/// chunk 0x1F60 a UID list of its swatches.
#[derive(Debug, Clone, PartialEq)]
pub struct ColorGroup {
    pub uid: u32,
    pub name: String,
    pub swatches: Vec<u32>,
}

impl ColorGroup {
    pub fn read(uid: u32, obj: &crate::Object) -> Result<Option<ColorGroup>, Error> {
        let Some(d) = obj.chunk(chunk::COLOR_GROUP_NAME) else {
            return Ok(None);
        };
        let name = Cursor::new(&d[1.min(d.len())..]).string()?;
        let swatches = match obj.chunk(chunk::COLOR_GROUP_SWATCHES) {
            Some(d) => Cursor::new(d).u32_list()?,
            None => Vec::new(),
        };
        Ok(Some(ColorGroup {
            uid,
            name,
            swatches,
        }))
    }
}
