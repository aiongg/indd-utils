//! Colours and swatches. See `docs/format/objects.md`.

use crate::Error;
use crate::object::Cursor;

pub mod class {
    pub const COLOR: u32 = 0x1F05;
    pub const SWATCH_NONE: u32 = 0x6E0B;
}

pub mod chunk {
    pub const COLOR_VALUE: u32 = 0x1F01;
    pub const COLOR_MODEL: u32 = 0x1F09;
    pub const COLOR_NAME: u32 = 0x1F10;
    pub const COLOR_OVERRIDE: u32 = 0x1F24;
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
        match self.color_override {
            1 => "Specialpaper",
            2 => "Specialblack",
            3 => "Specialregistration",
            4 => "Hiddenreserved",
            _ => "Normal",
        }
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
