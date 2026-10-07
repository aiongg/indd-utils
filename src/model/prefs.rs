//! Document preferences read from the preferences object (class 0x2202):
//! colour settings, view, grid, guide and document setup values, the
//! watermark and the text defaults. See `docs/format/preferences.md`.

use super::{Attrs, Reader, chunk, class};
use crate::Error;
use crate::audit::List;
use crate::object::{Cursor, big_endian};

/// A value for an element of `Resources/Preferences.xml` or a document
/// setting in `designmap.xml`.
#[derive(Debug, Clone, PartialEq)]
pub struct PrefValue {
    /// IDML element (`ViewPreference`, `Document`, ...).
    pub element: &'static str,
    /// Attribute name.
    pub name: &'static str,
    pub value: String,
}

/// Preference values read from the INDD.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Prefs {
    pub values: Vec<PrefValue>,
    /// Interface colours: element, `Properties` child, red, green, blue.
    pub colors: Vec<(&'static str, &'static str, [f64; 3])>,
    /// The text defaults (chunk 0x23F, a text attribute list).
    pub text_defaults: Option<Attrs>,
    /// Default anchored object settings (chunk 0x2800, as object styles
    /// have).
    pub anchor: Option<Vec<u8>>,
    /// Page item defaults (class 0x6E07), a page item attribute list.
    pub item_defaults: Option<Attrs>,
    /// `Properties` children: element, name, value.
    pub props: Vec<(&'static str, &'static str, PrefProp)>,
    /// Print records (element, bytes), written in base64 as `PrintRecord`.
    pub print_records: Vec<(&'static str, Vec<u8>)>,
}

/// The value of a `Properties` child of a preference element.
#[derive(Debug, Clone, PartialEq)]
pub enum PrefProp {
    /// Text of the given IDML type (`string`, `enumeration`).
    Text(&'static str, String),
    /// An empty element with these attributes.
    Attrs(Vec<(&'static str, String)>),
}

/// A flagged in-object string: a flag byte (1 for a built-in key) and the
/// string.
fn flagged(c: &mut Cursor) -> Result<(u8, String), Error> {
    let flag = c.u8()?;
    Ok((flag, c.string()?))
}

/// Print settings (chunk 0xA4C, and 0xAF2 for booklets): see
/// `docs/format/preferences.md`, print preferences.
fn print_prefs(
    d: &[u8],
    element: &'static str,
    values: &mut Vec<PrefValue>,
    props: &mut Vec<(&'static str, &'static str, PrefProp)>,
    records: &mut Vec<(&'static str, Vec<u8>)>,
) -> Result<(), Error> {
    let mut set = |name: &'static str, value: String| {
        values.push(PrefValue {
            element,
            name,
            value,
        })
    };
    let mut c = Cursor::new(d);
    let has_record = c.u8()? != 0;
    c.u8()?;
    let n = c.u32()? as usize;
    records.push((element, c.bytes(n)?.to_vec()));
    c.skip(if has_record { 6 } else { 2 })?;
    let preset = flagged(&mut c)?;
    let to = c.u32()?;
    let printer = flagged(&mut c)?;
    flagged(&mut c)?;
    let ppd = flagged(&mut c)?;
    let ppd_file = flagged(&mut c)?;
    let mut prop = |name: &'static str, ty: &'static str, text: String| {
        props.push((element, name, PrefProp::Text(ty, text)))
    };
    match (preset.0, preset.1.as_str()) {
        (1, "kPrSt_DefaultName") => prop("ActivePrinterPreset", "enumeration", "Default".into()),
        (1, "") => prop("ActivePrinterPreset", "enumeration", "Custom".into()),
        (1, _) => {}
        (_, name) => prop("ActivePrinterPreset", "string", name.into()),
    }
    match (printer.0, printer.1.as_str()) {
        (1, "kPrepress File") => prop("Printer", "enumeration", "PostscriptFile".into()),
        (1, _) => {}
        (_, name) => prop("Printer", "string", name.into()),
    }
    match (ppd.0, ppd.1.as_str()) {
        (1, "kDevice Independent") => prop("PPD", "enumeration", "DeviceIndependent".into()),
        (1, "") => prop("PPD", "string", "$ID/".into()),
        (1, _) => {}
        (_, name) => prop("PPD", "string", name.into()),
    }
    set("PrintTo", to.to_string());
    set("PrintToDisk", (to == 2).to_string());
    let file = if ppd_file.0 == 1 {
        format!("$ID/{}", ppd_file.1)
    } else {
        ppd_file.1
    };
    set("PPDFile", file);
    match c.u32()? {
        2 => set("PostScriptLevel", "Level2".into()),
        3 => set("PostScriptLevel", "Level3".into()),
        _ => {}
    }
    set("PrintResolution", num(c.f64()?));
    let mut rect = |name: &'static str, c: &mut Cursor| -> Result<(), Error> {
        let mut a = Vec::new();
        for side in ["Left", "Top", "Right", "Bottom"] {
            a.push((side, num_half_even(c.f64()?)));
        }
        props.push((element, name, PrefProp::Attrs(a)));
        Ok(())
    };
    rect("PaperSizeRect", &mut c)?;
    rect("ImageablePaperSizeRect", &mut c)?;
    let size = c.i32()?;
    let name = flagged(&mut c)?;
    let paper = match size {
        -3 => Some(("string", name.1)),
        -2 => Some(("enumeration", "DefinedByDriver".to_string())),
        -1 => Some(("enumeration", "Custom".to_string())),
        _ => None,
    };
    if let Some((ty, text)) = paper {
        props.push((element, "PaperSize", PrefProp::Text(ty, text)));
    }
    let q = c.pos();
    let f = |o: usize| Cursor::new(&d[q + o..]).f64();
    set("PaperWidthRange", format!("{} {}", num(f(8)?), num(f(16)?)));
    set(
        "PaperHeightRange",
        format!("{} {}", num(f(32)?), num(f(40)?)),
    );
    match Cursor::new(&d[q + 84..]).u16()? {
        0 => set("PrintPageOrientation", "Portrait".into()),
        1 => set("PrintPageOrientation", "Landscape".into()),
        _ => {}
    }
    set("Copies", Cursor::new(&d[q + 104..]).u32()?.to_string());
    match d.get(q + 118) {
        Some(0) => set("PrintBlankPages", "false".into()),
        Some(1) => set("PrintBlankPages", "true".into()),
        _ => {}
    }
    Ok(())
}

/// Chunks of the preferences object.
mod id {
    pub const VIEW: u32 = 0x1202;
    pub const BASELINE_GRID: u32 = 0x55F;
    pub const DOCUMENT_GRID: u32 = 0x545;
    pub const BASELINE_GRID_SHOWN: u32 = 0x56A;
    pub const GUIDES_LOCKED: u32 = 0x569;
    pub const COLUMN_GUIDES_LOCKED: u32 = 0x56B;
    pub const SHOW_FRAME_EDGES: u32 = 0x593;
    pub const SHOW_RULERS: u32 = 0x803;
    pub const SHOW_INVISIBLES: u32 = 0xCAC7;
    pub const COLOR_PROFILES: u32 = 0x7C04;
    pub const COLOR_INTENT: u32 = 0x7C08;
    pub const COLOR_POLICIES: u32 = 0x7C44;
    pub const WATERMARK: u32 = 0x16344;
    pub const TEXT: u32 = 0x280;
    pub const SMART_TEXT_REFLOW: u32 = 0x28BE;
    pub const MARGINS: u32 = 0x550;
    pub const COLUMNS: u32 = 0x555;
    pub const ANCHOR: u32 = 0x2800;
    pub const PRINT: u32 = 0xA4C;
    pub const PRINT_BOOKLET: u32 = 0xAF2;
    pub const PASTEBOARD: u32 = 0x5D2;
    pub const XML_TAGS: u32 = 0xBF4F;
    pub const GRIDS_IN_BACK: u32 = 0x567;
    pub const TEXT_WRAP: u32 = 0x3768;
    /// The chunk of the page item defaults object (class 0x6E07).
    pub const ITEM_DEFAULTS: u32 = 0x6E07;
    /// Name of a colour profile object (class 0x7D03).
    pub const PROFILE_NAME: u32 = 0x13C;
}

/// Measurement unit codes (`ViewPreference`).
fn unit(code: u32) -> Option<&'static str> {
    Some(match code {
        0x1201 => "Points",
        0x1202 => "Picas",
        0x1203 => "Inches",
        0x1205 => "Millimeters",
        0x1206 => "Centimeters",
        0x120B => "Pixels",
        0x5101 => "Ha",
        0x510F => "Q",
        _ => return None,
    })
}

/// Colour management policy codes: four characters, stored reversed.
fn policy(code: &[u8]) -> Option<&'static str> {
    Some(match code {
        b" ffo" => "ColorPolicyOff",
        b"serp" => "PreserveEmbeddedProfiles",
        b"lidp" => "CombinationOfPreserveAndSafeCmyk",
        b"vnoc" => "ConvertToWorkingSpace",
        _ => return None,
    })
}

fn num(v: f64) -> String {
    crate::idml::num(v)
}

/// A number as IDML writes the paper rectangles of the print settings:
/// the digits of the shortest form that reads back, the last one rounded
/// half to even from the exact value (583.2000122070312, where the
/// shortest form is …313).
fn num_half_even(v: f64) -> String {
    if v == 0.0 || !v.is_finite() {
        return num(v);
    }
    // As many digits as the shortest form that reads back, rounded half
    // to even from the exact value.
    let shortest = format!("{v:e}");
    let n = shortest
        .split('e')
        .next()
        .unwrap_or("")
        .chars()
        .filter(|c| c.is_ascii_digit())
        .count()
        .max(1);
    let e = format!("{v:.*e}", n - 1);
    let (mant, exp) = e.split_once('e').unwrap_or((&e, "0"));
    let exp: i32 = exp.parse().unwrap_or(0);
    let neg = mant.starts_with('-');
    let digits: String = mant.chars().filter(|c| c.is_ascii_digit()).collect();
    let point = exp + 1;
    let mut out = if point <= 0 {
        format!("0.{}{}", "0".repeat((-point) as usize), digits)
    } else if point as usize >= digits.len() {
        format!("{}{}", digits, "0".repeat(point as usize - digits.len()))
    } else {
        format!(
            "{}.{}",
            &digits[..point as usize],
            &digits[point as usize..]
        )
    };
    if out.contains('.') {
        out = out.trim_end_matches('0').trim_end_matches('.').to_string();
    }
    if neg {
        out.insert(0, '-');
    }
    out
}

impl Reader<'_> {
    pub(super) fn prefs(&self, major: u32) -> Result<Prefs, Error> {
        let mut values = Vec::new();
        let mut colors = Vec::new();
        let mut text_defaults = None;
        // The layouts below are those of InDesign CS5 (7.0) and later in
        // little-endian files: the versions the corpus pairs show.
        if major < 7 || big_endian() {
            return Ok(Prefs::default());
        }
        let Some(&(uid, _)) = self
            .db
            .classes()
            .iter()
            .find(|(_, c)| *c == class::PREFERENCES)
        else {
            return Ok(Prefs::default());
        };
        let get = |id: u32| self.chunk(uid, id);
        let mut set = |element: &'static str, name: &'static str, value: String| {
            values.push(PrefValue {
                element,
                name,
                value,
            })
        };
        // A u16 flag; `absent` is the value of documents without the chunk.
        let flag = |id: u32, absent: bool| -> Result<Option<String>, Error> {
            Ok(match get(id)? {
                Some(d) if d.len() == 2 => match Cursor::new(&d).u16()? {
                    0 => Some("false".into()),
                    1 => Some("true".into()),
                    _ => None,
                },
                Some(_) => None,
                None => Some(absent.to_string()),
            })
        };

        // Colour settings.
        if let Some(d) = get(id::COLOR_POLICIES)?.filter(|d| d.len() == 12) {
            if let Some(v) = policy(&d[4..8]) {
                set("Document", "RGBPolicy", v.into());
            }
            if let Some(v) = policy(&d[8..12]) {
                set("Document", "CMYKPolicy", v.into());
            }
        }
        if let Some(d) = get(id::COLOR_PROFILES)? {
            let mut c = Cursor::new(&d);
            c.u16()?;
            let n = c.u32()? as usize;
            c.skip(4 * n)?;
            if c.u32()? >= 2 {
                for name in ["RGBProfile", "CMYKProfile"] {
                    let profile = c.u32()?;
                    let value = if profile == 0 {
                        Some("$ID/".to_string())
                    } else if self.class(profile) == Some(class::COLOR_PROFILE) {
                        match self.chunk(profile, id::PROFILE_NAME)? {
                            Some(s) if s.len() > 1 => Cursor::new(&s[1..]).string().ok(),
                            _ => None,
                        }
                    } else {
                        None
                    };
                    if let Some(v) = value {
                        set("Document", name, v);
                    }
                }
            }
        }
        let intent = match get(id::COLOR_INTENT)? {
            None => Some("UseColorSettings"),
            Some(d) if d.len() >= 4 => match Cursor::new(&d[2..]).u16()? {
                0 => Some("Perceptual"),
                2 => Some("RelativeColorimetric"),
                11 => Some("UseColorSettings"),
                _ => None,
            },
            Some(_) => None,
        };
        if let Some(i) = intent {
            for name in [
                "SolidColorIntent",
                "AfterBlendingIntent",
                "DefaultImageIntent",
            ] {
                set("Document", name, i.into());
            }
        }

        // View settings.
        if let Some(d) = get(id::VIEW)?.filter(|d| d.len() == 56) {
            let mut c = Cursor::new(&d);
            let units: Vec<u32> = (0..6).map(|_| c.u32()).collect::<Result<_, _>>()?;
            for (i, names) in [
                &["HorizontalMeasurementUnits"][..],
                &["VerticalMeasurementUnits"],
                &["TypographicMeasurementUnits"],
                &["TextSizeMeasurementUnits"],
                &["PrintDialogMeasurementUnits"],
                &["LineMeasurementUnits", "StrokeMeasurementUnits"],
            ]
            .into_iter()
            .enumerate()
            {
                if let Some(u) = unit(units[i]) {
                    for name in names {
                        set("ViewPreference", name, u.into());
                    }
                }
            }
            for name in [
                "HorizontalCustomPoints",
                "VerticalCustomPoints",
                "CursorKeyIncrement",
                "PointsPerInch",
            ] {
                set("ViewPreference", name, num(c.f64()?));
            }
        }
        for (id, absent, element, name) in [
            (
                id::SHOW_FRAME_EDGES,
                true,
                "ViewPreference",
                "ShowFrameEdges",
            ),
            (id::SHOW_RULERS, true, "ViewPreference", "ShowRulers"),
            (
                id::BASELINE_GRID_SHOWN,
                false,
                "GridPreference",
                "BaselineGridShown",
            ),
            (id::GUIDES_LOCKED, false, "GuidePreference", "GuidesLocked"),
            (
                id::COLUMN_GUIDES_LOCKED,
                true,
                "DocumentPreference",
                "ColumnGuideLocked",
            ),
            (
                id::SHOW_INVISIBLES,
                false,
                "TextPreference",
                "ShowInvisibles",
            ),
            (id::GRIDS_IN_BACK, true, "GridPreference", "GridsInBack"),
        ] {
            if let Some(v) = flag(id, absent)? {
                set(element, name, v);
            }
        }

        // Document setup beyond `DocumentPreferences`.
        if let Some(d) = get(chunk::DOCUMENT_PREFERENCES)?.filter(|d| d.len() >= 146) {
            let f = |o: usize| Cursor::new(&d[o..]).f64();
            let b = |o: usize| (d[o] != 0).to_string();
            set("DocumentPreference", "DocumentBleedUniformSize", b(102));
            set("DocumentPreference", "SlugInsideOrLeftOffset", num(f(104)?));
            set("DocumentPreference", "SlugTopOffset", num(f(112)?));
            set(
                "DocumentPreference",
                "SlugRightOrOutsideOffset",
                num(f(120)?),
            );
            set("DocumentPreference", "SlugBottomOffset", num(f(128)?));
            set("DocumentPreference", "DocumentSlugUniformSize", b(136));
        }

        // Text preferences and the default text frame columns.
        if let Some(d) = get(id::TEXT)?.filter(|d| d.len() >= 174) {
            let f = |o: usize| Cursor::new(&d[o..]).f64();
            let b = |o: usize| (d[o] != 0).to_string();
            for (o, name) in [
                (0, "SmallCap"),
                (8, "SuperscriptSize"),
                (16, "SubscriptSize"),
                (24, "SuperscriptPosition"),
                (32, "SubscriptPosition"),
                (88, "LeadingKeyIncrement"),
                (96, "BaselineShiftKeyIncrement"),
            ] {
                set("TextPreference", name, num(f(o)?));
            }
            set(
                "TextPreference",
                "KerningKeyIncrement",
                num(f(104)? * 1000.0),
            );
            set("TextPreference", "TypographersQuotes", b(118));
            set("TextPreference", "LinkTextFilesWhenImporting", b(130));
            set("TextPreference", "UseParagraphLeading", b(162));
            if d.len() >= 212 {
                set("TextPreference", "QuoteCharactersRotatedInVertical", b(184));
            }
            if d.len() >= 214 {
                set("TextPreference", "ShapeIndicAndLatinWithHarbuzz", b(212));
            }
            set("TextFramePreference", "TextColumnGutter", num(f(40)?));
            set(
                "TextFramePreference",
                "TextColumnCount",
                Cursor::new(&d[112..]).u32()?.to_string(),
            );
            const BASELINE: [&str; 5] = [
                "LeadingOffset",
                "AscentOffset",
                "CapHeight",
                "EmboxHeight",
                "XHeight",
            ];
            if let Some(v) = BASELINE.get(d[142] as usize) {
                set("TextFramePreference", "FirstBaselineOffset", v.to_string());
            }
        }
        match get(id::SMART_TEXT_REFLOW)? {
            Some(d) if d.len() >= 14 => {
                let b = |o: usize| (d[o] != 0).to_string();
                set("TextPreference", "SmartTextReflow", b(4));
                set("TextPreference", "LimitToMasterTextFrames", b(6));
                set("TextPreference", "DeleteEmptyPages", b(8));
                set("TextPreference", "PreserveFacingPageSpreads", b(10));
                if d.len() >= 16 {
                    set("TextPreference", "SmartTextReflowSync", b(12));
                }
            }
            Some(_) => {}
            None => {
                set("TextPreference", "SmartTextReflow", "false".into());
                set("TextPreference", "LimitToMasterTextFrames", "true".into());
            }
        }

        // Default margins and columns of new pages.
        let margins = match get(id::MARGINS)? {
            Some(d) if d.len() >= 32 => {
                let f = |o: usize| Cursor::new(&d[o..]).f64();
                Some([f(0)?, f(8)?, f(16)?, f(24)?])
            }
            Some(_) => None,
            None => Some([36.0; 4]),
        };
        if let Some([left, top, right, bottom]) = margins {
            set("MarginPreference", "Left", num(left));
            set("MarginPreference", "Top", num(top));
            set("MarginPreference", "Right", num(right));
            set("MarginPreference", "Bottom", num(bottom));
        }
        let columns = match get(id::COLUMNS)? {
            Some(d) if d.len() >= 12 => Some((Cursor::new(&d).u32()?, Cursor::new(&d[4..]).f64()?)),
            Some(_) => None,
            None => Some((1, 12.0)),
        };
        if let Some((count, gutter)) = columns {
            set("MarginPreference", "ColumnCount", count.to_string());
            set("MarginPreference", "ColumnGutter", num(gutter));
        }
        let anchor = get(id::ANCHOR)?;
        if anchor.is_none() {
            set(
                "AnchoredObjectSetting",
                "VerticalAlignment",
                "TopAlign".into(),
            );
        }

        // Pasteboard: f64 horizontal and vertical margins.
        let pasteboard = match get(id::PASTEBOARD)? {
            Some(d) if d.len() >= 16 => {
                let f = |o: usize| Cursor::new(&d[o..]).f64();
                Some((f(0)?, f(8)?))
            }
            Some(_) => None,
            None => Some((-1.0, 72.0)),
        };
        if let Some((h, v)) = pasteboard {
            set(
                "PasteboardPreference",
                "PasteboardMargins",
                format!("{} {}", num(h), num(v)),
            );
            set("PasteboardPreference", "MinimumSpaceAboveAndBelow", num(v));
        }
        // Text wrap: u8 at 2.
        if let Some(d) = get(id::TEXT_WRAP)?.filter(|d| d.len() >= 3) {
            set("TextPreference", "ZOrderTextWrap", (d[2] != 0).to_string());
        }
        // Default XML tags: name (u32 length, text segments) and u32 UID of
        // an interface colour, for story, table, (untagged), cell, image.
        if let Some(d) = get(id::XML_TAGS)? {
            let mut c = Cursor::new(&d);
            for (name, color) in [
                (Some("DefaultStoryTagName"), "DefaultStoryTagColor"),
                (Some("DefaultTableTagName"), "DefaultTableTagColor"),
                (None, ""),
                (Some("DefaultCellTagName"), "DefaultCellTagColor"),
                (Some("DefaultImageTagName"), "DefaultImageTagColor"),
            ] {
                let n = c.u32()? as usize;
                let tag = c.segments(n)?;
                let ui = c.u32()?;
                if let Some(name) = name {
                    set("XMLPreference", name, tag);
                    if let Some(rgb) = self.ui_color(ui)? {
                        colors.push(("XMLPreference", color, rgb));
                    }
                }
            }
        }
        // Page item defaults: u32, u32, u32 n, n 12-byte entries, then a
        // page item attribute list.
        let item_defaults = match self
            .db
            .classes()
            .iter()
            .find(|(_, c)| *c == class::ITEM_DEFAULTS)
        {
            Some(&(u, _)) => match self.chunk(u, id::ITEM_DEFAULTS)? {
                Some(d) if d.len() >= 12 => {
                    let n = Cursor::new(&d[8..]).u32()? as usize;
                    d.get(12 + 12 * n..)
                        .and_then(|rest| Attrs::parse(rest, List::Item).ok())
                }
                _ => None,
            },
            None => None,
        };

        // Print settings.
        let mut props = Vec::new();
        let mut print_records = Vec::new();
        for (id, element) in [
            (id::PRINT, "PrintPreference"),
            (id::PRINT_BOOKLET, "PrintBookletPrintPreference"),
        ] {
            if let Some(d) = get(id)? {
                let (mut v, mut p, mut r) = (Vec::new(), Vec::new(), Vec::new());
                // A layout the corpus does not show leaves the element as
                // observed.
                if print_prefs(&d, element, &mut v, &mut p, &mut r).is_ok() {
                    for pv in v {
                        set(pv.element, pv.name, pv.value);
                    }
                    props.extend(p);
                    print_records.extend(r);
                }
            }
        }

        // Grids.
        if let Some(d) = get(id::BASELINE_GRID)?.filter(|d| d.len() >= 26) {
            let f = |o: usize| Cursor::new(&d[o..]).f64();
            set("GridPreference", "BaselineDivision", num(f(2)?));
            set("GridPreference", "BaselineStart", num(f(10)?));
            set(
                "GridPreference",
                "BaselineViewThreshold",
                num(f(18)? * 100.0),
            );
        }
        let (h, hs, v, vs) = match get(id::DOCUMENT_GRID)? {
            Some(d) if d.len() >= 32 => {
                let f = |o: usize| Cursor::new(&d[o..]).f64();
                let u = |o: usize| Cursor::new(&d[o..]).u32();
                (f(8)?, u(16)?, f(20)?, u(28)?)
            }
            Some(_) => (f64::NAN, 0, f64::NAN, 0),
            None => (72.0, 8, 72.0, 8),
        };
        if !h.is_nan() {
            set("GridPreference", "HorizontalGridlineDivision", num(h));
            set(
                "GridPreference",
                "HorizontalGridSubdivision",
                hs.to_string(),
            );
            set("GridPreference", "VerticalGridlineDivision", num(v));
            set("GridPreference", "VerticalGridSubdivision", vs.to_string());
        }

        // Watermark: u32 and u16 fields, then the font family and style
        // (u32 length, then text segments), the point size and the UID
        // of an interface colour.
        if let Some(d) = get(id::WATERMARK)?.filter(|d| d.len() > 10) {
            let mut c = Cursor::new(&d[10..]);
            let n = c.u32()? as usize;
            let family = c.segments(n)?;
            let n = c.u32()? as usize;
            let style = c.segments(n)?;
            let _size = c.u32()?;
            let color = c.u32()?;
            set("WatermarkPreference", "WatermarkFontFamily", family);
            set("WatermarkPreference", "WatermarkFontStyle", style);
            if let Some(rgb) = self.ui_color(color)? {
                colors.push(("WatermarkPreference", "WatermarkFontColor", rgb));
            }
        }

        // Text defaults: a text attribute list, as styles have.
        if let Some(d) = get(chunk::STYLE_ATTRS)?.filter(|d| d.len() >= 2) {
            let mut c = Cursor::new(&d);
            let n = c.u16()? as usize;
            text_defaults = Attrs::parse_text(&mut c, n, List::Style).ok();
        }
        Ok(Prefs {
            values,
            colors,
            text_defaults,
            anchor,
            item_defaults,
            props,
            print_records,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::num_half_even;

    #[test]
    fn rounds_shortest_digits_half_even() {
        assert_eq!(num_half_even(f64::from(583.2_f32)), "583.2000122070312");
        assert_eq!(num_half_even(f64::from(841.92_f32)), "841.9199829101562");
        assert_eq!(num_half_even(612.0), "612");
        assert_eq!(num_half_even(0.0), "0");
        assert_eq!(num_half_even(-12.5), "-12.5");
        assert_eq!(num_half_even(0.0625), "0.0625");
        assert_eq!(num_half_even(f64::from(16.6_f32)), "16.600000381469727");
    }
}
