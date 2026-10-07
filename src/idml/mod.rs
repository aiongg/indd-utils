//! IDML package writer.

mod transparency;
mod values;
mod xml;
pub mod zip;

use std::collections::BTreeMap;

use crate::model::{
    Attrs, Document, Graphic, GraphicKind, Guide, ItemKind, Matrix, Orientation, Page, PageItem,
    Path, Section, Shape, Spread, Story, Style, StyleGroup, Table, TextFramePreferences, TextRun,
    TextVariable, TextWrap, UiColorRef, Value, XmlElement, XmlMarker, hyperlink::DestinationKind,
    numbering, root_kind, variable::Instance, wrap_mode, xml::Key as XmlKey,
};

#[derive(Clone, Copy)]
enum AttrKind {
    Number,
    Swatch,
    Point,
    Enum(&'static [(u32, &'static str)]),
    /// A built-in style named by its code (reference 0), written as
    /// `<prefix><name>`.
    Builtin(&'static str, &'static [(u32, &'static str)]),
}

/// Codes of built-in stroke styles. See `docs/format/attributes.md`.
/// Frame fitting attributes of page items and object styles, in the
/// order IDML writes them (`docs/format/objects.md`).
const FITTING_ATTRS: [(u32, &str); 7] = [
    (0x6E83, "AutoFit"),
    (0x6E7E, "LeftCrop"),
    (0x6E7F, "TopCrop"),
    (0x6E80, "RightCrop"),
    (0x6E81, "BottomCrop"),
    (0x6E7C, "FittingOnEmptyFrame"),
    (0x6E7D, "FittingAlignment"),
];

/// The IDML value of frame fitting attribute `id`; `None` for codes
/// without evidence.
fn fitting_value(id: u32, v: &Value) -> Option<String> {
    match id {
        0x6E83 => (v.as_u32()? == 0).then(|| "false".to_string()),
        0x6E7C => match v.as_u32()? {
            0 => Some("None".into()),
            1 => Some("ContentToFrame".into()),
            2 => Some("Proportionally".into()),
            3 => Some("FillProportionally".into()),
            _ => None,
        },
        0x6E7D => match v.as_u32()? {
            0 => Some("TopLeftAnchor".into()),
            4 => Some("CenterAnchor".into()),
            _ => None,
        },
        _ => v.as_f64().map(num),
    }
}

/// Anchored object settings from chunk 0x2800 (of an anchor or an object
/// style): f64 `AnchorYoffset` at 0, u16 `VerticalAlignment` at 52. See
/// `docs/format/objects.md`.
fn anchored_settings(d: &[u8]) -> Vec<(&'static str, String)> {
    let mut out = Vec::new();
    if d.len() < 54 {
        return out;
    }
    let y = f64_from(d[0..8].try_into().unwrap());
    let align = match u16_from([d[52], d[53]]) {
        0 => Some("TopAlign"),
        1 => Some("CenterAlign"),
        2 => Some("BottomAlign"),
        _ => None,
    };
    if let Some(a) = align {
        out.push(("VerticalAlignment", a.to_string()));
    }
    out.push(("AnchorYoffset", num(y)));
    out
}

/// The IDML name of an XML tag colour (red, green, blue fractions);
/// `None` for colours without evidence. See `docs/format/objects.md`.
fn xml_tag_color(rgb: [f64; 3]) -> Option<&'static str> {
    ui_color_name(rgb)
}

/// The IDML name of an interface colour (class 0x1F11) by its red, green
/// and blue fractions. See `docs/format/objects.md`, interface colours.
fn ui_color_name(rgb: [f64; 3]) -> Option<&'static str> {
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

/// A `Properties` child for an interface colour: its name as an
/// enumeration, or, for a colour of whole 255ths without a name, the three
/// numbers as a list. `None` for other colours.
fn ui_color_property(tag: &str, rgb: [f64; 3]) -> Option<Node> {
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

/// Frame fitting attributes of `attrs` with IDs in `ids`, in IDML order.
fn fitting_attrs(attrs: &Attrs, ids: &[u32]) -> Vec<(&'static str, String)> {
    FITTING_ATTRS
        .iter()
        .filter(|(id, _)| ids.contains(id))
        .filter_map(|&(id, name)| Some((name, fitting_value(id, attrs.get(id)?)?)))
        .collect()
}

const STROKE_TYPES: &[(u32, &str)] = &[
    (0x5A29, "Solid"),
    (0x5A38, "Canned Dashed 3x2"),
    (0x5A39, "Canned Dotted"),
    (0xB004, "ThinThin"),
    (0xB01A, "Triple_Stroke"),
];

/// The stroke styles every corpus IDML lists in `Graphic.xml`, in order.
/// See `docs/format/idml-values.md`.
const BUILTIN_STROKE_STYLES: &[&str] = &[
    "Triple_Stroke",
    "ThickThinThick",
    "ThinThickThin",
    "ThickThick",
    "ThickThin",
    "ThinThick",
    "ThinThin",
    "Japanese Dots",
    "White Diamond",
    "Left Slant Hash",
    "Right Slant Hash",
    "Straight Hash",
    "Wavy",
    "Canned Dotted",
    "Canned Dashed 3x2",
    "Canned Dashed 4x4",
    "Dashed",
    "Solid",
];

#[derive(Clone, Copy)]
enum TextKind {
    Number,
    /// Stored as a fraction, written as a percentage.
    Percent,
    /// Multiplied by the factor when written.
    Scale(f64),
    /// True when the value equals the given code.
    Bool(u32),
    Enum(&'static [(u32, &'static str)]),
    Swatch,
    /// A swatch, or 0 for "Text Color" (written in Properties).
    SwatchOrText,
    Font,
    FontStyle,
    Leading,
    /// A language object, written as `$ID/<name>`.
    Language,
    /// The given value is written as the enumeration value, others as
    /// numbers of the given IDML type.
    NumberOr(f64, &'static str, &'static str),
    /// Two f64 (a point).
    Point,
    /// A list of tab stops (`TabList`).
    TabList,
    /// A list of nested styles (`AllNestedStyles`).
    NestedStyles,
    /// A character style reference.
    CharacterStyle,
    /// A font family, or 0 for none (`$ID/`).
    FontOrNone,
    /// A string, or the empty string for `Nothing`.
    StringOrNothing,
    /// u32 bullet character type and u32 character value (`BulletChar`).
    BulletChar,
    /// u32 length in characters, then text segments; left out when empty.
    Text,
    /// A number, left out when 0.
    NonZero,
    /// Manual kerning in ems, written in thousandths of an em; 1e8 (the
    /// root style's value) is left out.
    Kerning,
}

/// Value of the kerning attribute (0x1B13) in every root paragraph style;
/// IDML writes no `KerningValue` for it.
const KERNING_NONE: f64 = 1e8;

/// Text attributes: ID, IDML name, kind, written in `<Properties>`.
/// See `docs/format/attributes.md` for the evidence behind each entry.
const TEXT_ATTRS: &[(u32, &str, TextKind, bool)] = &[
    (0x1B01, "FillColor", TextKind::Swatch, false),
    (0x1B02, "FontStyle", TextKind::FontStyle, false),
    (0x1B03, "PointSize", TextKind::Number, false),
    (0x1B06, "HorizontalScale", TextKind::Percent, false),
    (0x1B08, "Ligatures", TextKind::Bool(1), false),
    (
        0x1B07,
        "KerningMethod",
        TextKind::Enum(&[(15972, "$ID/Metrics"), (79875, "$ID/Optical")]),
        false,
    ),
    (0x1B0A, "StrokeWeight", TextKind::Number, false),
    (0x1B0B, "Tracking", TextKind::Scale(1000.0), false),
    (0x1B13, "KerningValue", TextKind::Kerning, false),
    (
        0x1B0C,
        "Composer",
        TextKind::Enum(&[
            (0x2001, "HL Single"),
            (0x2002, "HL Composer"),
            (0x2078, "HL Composer Optyca"),
        ]),
        false,
    ),
    (0x1B0D, "DropCapCharacters", TextKind::Number, false),
    (0x1B0E, "DropCapLines", TextKind::Number, false),
    (0x1B10, "BaselineShift", TextKind::Number, false),
    (
        0x1B11,
        "Capitalization",
        TextKind::Enum(&[
            (0, "Normal"),
            (1, "SmallCaps"),
            (2, "AllCaps"),
            (3, "CapToSmallCap"),
        ]),
        false,
    ),
    (0x1B12, "StrokeColor", TextKind::Swatch, false),
    (0x1B15, "VerticalScale", TextKind::Percent, false),
    (0x1B16, "LeftIndent", TextKind::Number, false),
    (0x1B17, "RightIndent", TextKind::Number, false),
    (0x1B18, "FirstLineIndent", TextKind::Number, false),
    (0x1B1A, "AutoLeading", TextKind::Percent, false),
    (0x1B1B, "Leading", TextKind::Leading, true),
    (0x1B1D, "AppliedLanguage", TextKind::Language, false),
    (0x1B1F, "Hyphenation", TextKind::Bool(3), false),
    (0x1B24, "NoBreak", TextKind::Bool(1), false),
    (0x1B25, "HyphenationZone", TextKind::Number, false),
    (0x1B26, "SpaceBefore", TextKind::Number, false),
    (0x1B27, "SpaceAfter", TextKind::Number, false),
    (0x1B29, "TabList", TextKind::TabList, true),
    (0x1B2A, "Underline", TextKind::Bool(1), false),
    (0x1B2B, "AppliedFont", TextKind::Font, true),
    (
        0x1B2C,
        "OTFFigureStyle",
        TextKind::Enum(&[
            (1, "ProportionalOldstyle"),
            (2, "ProportionalLining"),
            (4, "Default"),
        ]),
        false,
    ),
    (0x1B2E, "MaximumWordSpacing", TextKind::Percent, false),
    (0x1B2F, "MinimumWordSpacing", TextKind::Percent, false),
    (0x1B31, "MaximumLetterSpacing", TextKind::Percent, false),
    (0x1B32, "MinimumLetterSpacing", TextKind::Percent, false),
    (
        0x1B37,
        "StartParagraph",
        TextKind::Enum(&[(0, "Anywhere"), (2, "NextPage")]),
        false,
    ),
    (
        0x1B3C,
        "Position",
        TextKind::Enum(&[(0, "Normal"), (5, "OTNumerator")]),
        false,
    ),
    (0x1B40, "KeepLinesTogether", TextKind::Bool(1), false),
    (0x1B42, "FillTint", TextKind::Number, false),
    (0x1B46, "GradientFillAngle", TextKind::Number, false),
    (0x1B48, "GradientFillLength", TextKind::Number, false),
    (0x1B4A, "GradientFillStart", TextKind::Point, false),
    (0x1B4D, "RuleAboveLineWeight", TextKind::Number, false),
    (0x1B4F, "RuleAboveOffset", TextKind::Number, false),
    (0x1B50, "RuleAboveLeftIndent", TextKind::Number, false),
    (0x1B51, "RuleAboveRightIndent", TextKind::Number, false),
    (
        0x1B52,
        "RuleAboveWidth",
        TextKind::Enum(&[(1, "ColumnWidth"), (2, "TextWidth")]),
        false,
    ),
    (0x1B53, "RuleBelowColor", TextKind::SwatchOrText, true),
    (0x1B54, "RuleBelowLineWeight", TextKind::Number, false),
    (0x1B55, "RuleBelowTint", TextKind::Number, false),
    (0x1B56, "RuleBelowOffset", TextKind::Number, false),
    (0x1B5D, "RuleBelow", TextKind::Bool(1), false),
    (
        0x1B6A,
        "ParagraphBreakType",
        TextKind::Enum(&[(0, "Anywhere"), (1, "NextColumn")]),
        false,
    ),
    (
        0x1B6B,
        "SingleWordJustification",
        TextKind::Enum(&[(0, "LeftAlign"), (3, "FullyJustified")]),
        false,
    ),
    (0x1B75, "AllNestedStyles", TextKind::NestedStyles, true),
    (
        0x1B7E,
        "Justification",
        TextKind::Enum(&[
            (0, "LeftAlign"),
            (1, "CenterAlign"),
            (2, "RightAlign"),
            (4, "LeftJustified"),
            (5, "CenterJustified"),
        ]),
        false,
    ),
    (0x1B80, "DropcapDetail", TextKind::Number, false),
    (0x1B8C, "OTFContextualAlternate", TextKind::Bool(1), false),
    (0x1B8D, "UnderlineColor", TextKind::SwatchOrText, true),
    (0x1B91, "UnderlineOffset", TextKind::Number, false),
    (0x1B94, "UnderlineWeight", TextKind::Number, false),
    (0x1BB7, "MiterLimit", TextKind::Number, false),
    (
        0x1BB9,
        "EndJoin",
        TextKind::Enum(&[(0, "MiterEndJoin"), (1, "RoundEndJoin")]),
        false,
    ),
    (
        0x1BBD,
        "SpanColumnType",
        TextKind::Enum(&[(0, "SingleColumn"), (1, "SpanColumns")]),
        false,
    ),
    (
        0x1BBE,
        "SpanSplitColumnCount",
        TextKind::NumberOr(1.0, "All", "short"),
        true,
    ),
    (0x1BBF, "SplitColumnInsideGutter", TextKind::Number, false),
    (0x1BC4, "SpanColumnMinSpaceAfter", TextKind::Number, false),
    (0x1BD2, "ParagraphShadingColor", TextKind::Swatch, true),
    (0x1BD3, "ParagraphShadingTint", TextKind::Number, false),
    (0x1BD6, "ParagraphShadingOn", TextKind::Bool(1), false),
    (0x1BDB, "ParagraphShadingTopOffset", TextKind::Number, false),
    (
        0x1BDC,
        "ParagraphShadingBottomOffset",
        TextKind::Number,
        false,
    ),
    (0x1BF6, "ParagraphBorderColor", TextKind::Swatch, true),
    (0x1BF9, "ParagraphBorderOn", TextKind::Bool(1), false),
    (0x1DF03, "ParagraphBorderTopOffset", TextKind::Number, false),
    (
        0x1DF04,
        "ParagraphBorderBottomOffset",
        TextKind::Number,
        false,
    ),
    (
        0x1DF21,
        "SameParaStyleSpacing",
        TextKind::NumberOr(-1.0, "SetIgnore", "unit"),
        true,
    ),
    (0x4265, "GridAlignFirstLineOnly", TextKind::Bool(1), false),
    (0x425E, "Tatechuyoko", TextKind::Bool(1), false),
    (0x4279, "ShataiDegreeAngle", TextKind::Scale(100.0), false),
    (0x427A, "ShataiAdjustTsume", TextKind::Bool(1), false),
    (0x427B, "ShataiAdjustRotation", TextKind::Bool(1), false),
    (0x422D, "RubyFlag", TextKind::NonZero, false),
    (0x422E, "RubyString", TextKind::Text, false),
    (
        0x4266,
        "GridAlignment",
        TextKind::Enum(&[(0, "None"), (1, "AlignBaseline")]),
        false,
    ),
    (
        0x1A401,
        "BulletsAndNumberingListType",
        TextKind::Enum(&[(0, "NoList"), (1, "BulletList")]),
        false,
    ),
    (0x1A406, "BulletChar", TextKind::BulletChar, true),
    (0x1A413, "BulletsFont", TextKind::FontOrNone, true),
    (0x1A414, "BulletsFontStyle", TextKind::StringOrNothing, true),
    (0x1A419, "NumberingContinue", TextKind::Bool(1), false),
    (
        0x1A41F,
        "BulletsCharacterStyle",
        TextKind::CharacterStyle,
        true,
    ),
    (
        0x1A420,
        "NumberingCharacterStyle",
        TextKind::CharacterStyle,
        true,
    ),
    (0x1A423, "NumberingExpression", TextKind::FontStyle, false),
];

/// An attribute written as a `<Properties>` child: name, type, value.
type Property = (&'static str, &'static str, PropValue);

/// One field of a record in a list property: name, type, text.
type Field = (&'static str, &'static str, String);

/// The value of a `<Properties>` child.
enum PropValue {
    Text(String),
    /// Records, each written as a `ListItem` (type `list`).
    List(Vec<Vec<Field>>),
    /// An empty element with these attributes and no `type`.
    Attributes(Vec<(&'static str, String)>),
}

impl From<String> for PropValue {
    fn from(s: String) -> PropValue {
        PropValue::Text(s)
    }
}

/// Page item attributes: attribute-list ID, IDML name, value kind.
/// See `docs/format/attributes.md` for the evidence behind each entry.
const ITEM_ATTRS: &[(u32, &str, AttrKind)] = &[
    (0x6E68, "FillColor", AttrKind::Swatch),
    (0x6E69, "FillTint", AttrKind::Number),
    (0x6E64, "StrokeColor", AttrKind::Swatch),
    (0x6E65, "StrokeWeight", AttrKind::Number),
    (0x6E6D, "MiterLimit", AttrKind::Number),
    (
        0x6E6F,
        "CornerOption",
        AttrKind::Enum(&[(0x5A15, "RoundedCorner")]),
    ),
    (0x6E70, "CornerRadius", AttrKind::Number),
    (0x551F, "GradientFillLength", AttrKind::Number),
    (0x5520, "GradientFillStart", AttrKind::Point),
    (0x5525, "GradientStrokeLength", AttrKind::Number),
    (0x5526, "GradientStrokeStart", AttrKind::Point),
    (
        0x6E6E,
        "StrokeType",
        AttrKind::Builtin("StrokeStyle/$ID/", STROKE_TYPES),
    ),
    (
        0x6E8C,
        "StrokeAlignment",
        AttrKind::Enum(&[(0, "CenterAlignment"), (1, "InsideAlignment")]),
    ),
];
/// Gradient attributes every page item has: attribute-list ID (0 for
/// none, `GradientStroke...` of groups), IDML name, value without the
/// attribute. Fill and stroke start and length are in `ITEM_ATTRS`. See
/// `docs/format/objects.md`, page item settings.
const GRADIENT_ATTRS: &[(u32, &str, &str)] = &[
    (0x5520, "GradientFillStart", "0 0"),
    (0x551F, "GradientFillLength", "0"),
    (0x551E, "GradientFillAngle", "0"),
    (0x5526, "GradientStrokeStart", "0 0"),
    (0x5525, "GradientStrokeLength", "0"),
    (0x5524, "GradientStrokeAngle", "0"),
    (0x5522, "GradientFillHiliteLength", "0"),
    (0x5523, "GradientFillHiliteAngle", "0"),
    (0x5528, "GradientStrokeHiliteLength", "0"),
    (0x5529, "GradientStrokeHiliteAngle", "0"),
];

/// IDML `HorizontalLayoutConstraints` and `VerticalLayoutConstraints` of
/// layout constraint flags (chunk 0x22228): bits 4 to 6 and 0 to 2, each
/// set for `FixedDimension`. `None` if another bit is set.
fn layout_constraints(flags: u8) -> Option<(String, String)> {
    if flags & 0x88 != 0 {
        return None;
    }
    let side = |bits: u8| {
        (0..3)
            .map(|i| {
                if bits & (1 << i) != 0 {
                    "FixedDimension"
                } else {
                    "FlexibleDimension"
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
    };
    Some((side(flags >> 4), side(flags & 7)))
}

/// Numbers separated by spaces.
fn join_numbers(v: &[u32]) -> String {
    v.iter().map(u32::to_string).collect::<Vec<_>>().join(" ")
}

/// Kinds of cell attribute values.
#[derive(Clone, Copy)]
enum CellKind {
    Number,
    Integer,
    Swatch,
    Enum(&'static [(u32, &'static str)]),
    /// A stroke style code in the first four of eight bytes; the other
    /// four are 0.
    StrokeType,
    /// A cell style UID; 0 is written as `n`.
    CellStyle,
}

/// Cell attributes of a cell attribute set: ID, IDML attributes, kind.
/// See `docs/format/tables.md`.
const CELL_ATTRS: &[(u32, &[&str], CellKind)] = &[
    (0xB62C, &["TextTopInset", "TopInset"], CellKind::Number),
    (0xB62B, &["TextLeftInset", "LeftInset"], CellKind::Number),
    (
        0xB62E,
        &["TextBottomInset", "BottomInset"],
        CellKind::Number,
    ),
    (0xB62D, &["TextRightInset", "RightInset"], CellKind::Number),
    (0xB63D, &["FillColor"], CellKind::Swatch),
    (0xB63E, &["FillTint"], CellKind::Number),
    (
        0xB677,
        &["VerticalJustification"],
        CellKind::Enum(&[(1, "CenterAlign"), (2, "BottomAlign")]),
    ),
    (
        0xB6DE,
        &["ClipContentToCell"],
        CellKind::Enum(&[(0, "false")]),
    ),
    (0xB645, &["LeftEdgeStrokeWeight"], CellKind::Number),
    (0xB64D, &["LeftEdgeStrokeType"], CellKind::StrokeType),
    (0xB649, &["LeftEdgeStrokeColor"], CellKind::Swatch),
    (0xB6A8, &["LeftEdgeStrokeTint"], CellKind::Number),
    (0xB6F9, &["LeftEdgeStrokePriority"], CellKind::Integer),
    (0xB647, &["TopEdgeStrokeWeight"], CellKind::Number),
    (0xB64F, &["TopEdgeStrokeType"], CellKind::StrokeType),
    (0xB64A, &["TopEdgeStrokeColor"], CellKind::Swatch),
    (0xB6AA, &["TopEdgeStrokeTint"], CellKind::Number),
    (0xB6FB, &["TopEdgeStrokePriority"], CellKind::Integer),
    (0xB646, &["RightEdgeStrokeWeight"], CellKind::Number),
    (0xB64E, &["RightEdgeStrokeType"], CellKind::StrokeType),
    (0xB64B, &["RightEdgeStrokeColor"], CellKind::Swatch),
    (0xB6A9, &["RightEdgeStrokeTint"], CellKind::Number),
    (0xB6FA, &["RightEdgeStrokePriority"], CellKind::Integer),
    (0xB648, &["BottomEdgeStrokeWeight"], CellKind::Number),
    (0xB650, &["BottomEdgeStrokeType"], CellKind::StrokeType),
    (0xB64C, &["BottomEdgeStrokeColor"], CellKind::Swatch),
    (0xB6AB, &["BottomEdgeStrokeTint"], CellKind::Number),
    (0xB6FC, &["BottomEdgeStrokePriority"], CellKind::Integer),
];

/// Table and table style attributes: ID, IDML attribute, kind.
/// See `docs/format/tables.md`.
const TABLE_ATTRS: &[(u32, &str, CellKind)] = &[
    (0xB662, "SpaceBefore", CellKind::Number),
    (0xB663, "SpaceAfter", CellKind::Number),
    (0xB684, "StartRowStrokeColor", CellKind::Swatch),
    (0xB690, "StartRowStrokeWeight", CellKind::Number),
    (0xB688, "StartRowStrokeType", CellKind::StrokeType),
    (
        0xB683,
        "ColumnFillsPriority",
        CellKind::Enum(&[(0, "false")]),
    ),
    (0xB67B, "StartRowFillColor", CellKind::Swatch),
    (0xB6B0, "StartRowFillTint", CellKind::Number),
    (0xB67C, "EndRowFillColor", CellKind::Swatch),
    (0xB6B1, "EndRowFillTint", CellKind::Number),
    (
        0x10457,
        "HeaderRegionSameAsBodyRegion",
        CellKind::Enum(&[(0, "false"), (1, "true")]),
    ),
    (0x10450, "HeaderRegionCellStyle", CellKind::CellStyle),
    (0x10452, "BodyRegionCellStyle", CellKind::CellStyle),
    (0x10453, "LeftColumnRegionCellStyle", CellKind::CellStyle),
    (0x10454, "RightColumnRegionCellStyle", CellKind::CellStyle),
];

/// Paragraph style of a cell style.
const CELL_STYLE_PARAGRAPH_STYLE: u32 = 0x10463;

/// Stroke style code of a cell edge that has no stroke type (IDML `n`).
const CELL_NO_STROKE_TYPE: u32 = 0x1040C;

use values::Node;
use xml::Xml;

use crate::object::{
    Cursor, f64_bytes, f64_from, i32_bytes, u16_bytes, u16_from, u32_at, u32_bytes,
};

/// Report an attribute value that the converter has no IDML value for to
/// `indd audit`: its code, or for a stroke type the code in its first four
/// bytes.
fn unknown_code(attrs: &Attrs, id: u32, v: &Value) {
    let code = v
        .as_u32()
        .or_else(|| match v {
            Value::RefOrCode(_, code) => Some(*code),
            _ => u32_at(&raw_bytes(v), 0),
        })
        .unwrap_or(u32::MAX);
    crate::audit::unknown_code(attrs.1, id, code);
}

/// The bytes of a value as stored: list values of two, four or eight bytes
/// are decoded as numbers by the attribute reader.
fn raw_bytes(v: &Value) -> Vec<u8> {
    match v {
        Value::Double(f) => f64_bytes(*f).to_vec(),
        Value::Int(i) => i32_bytes(*i).to_vec(),
        Value::Enum(e) => u16_bytes(*e).to_vec(),
        Value::Ref(r) => u32_bytes(*r).to_vec(),
        Value::Point(a, b) => [f64_bytes(*a), f64_bytes(*b)].concat(),
        Value::RefOrCode(r, _) => u32_bytes(*r).to_vec(),
        Value::Other(_, b) => b.clone(),
    }
}

/// Records of a `TabList`: u16 count, then per stop f64 position, u16
/// alignment, u16 leader length and the leader in UTF-16 code units. See
/// `docs/format/attributes.md`. `None` for an unknown alignment code.
fn tab_list(data: &[u8]) -> Option<Vec<Vec<Field>>> {
    let mut c = Cursor::new(data);
    let n = c.u16().ok()?;
    let mut out = Vec::new();
    for _ in 0..n {
        let position = c.f64().ok()?;
        let alignment = match c.u16().ok()? {
            0 => "LeftAlign",
            2 => "RightAlign",
            _ => return None,
        };
        let len = c.u16().ok()? as usize;
        let units = (0..len)
            .map(|_| c.u16())
            .collect::<Result<Vec<_>, _>>()
            .ok()?;
        out.push(vec![
            ("Alignment", "enumeration", alignment.to_string()),
            ("AlignmentCharacter", "string", ".".to_string()),
            ("Leader", "string", String::from_utf16_lossy(&units)),
            ("Position", "unit", num(position)),
        ]);
    }
    (c.remaining() == 0).then_some(out)
}

/// `BulletChar` attributes: u32 character type, u32 character value.
/// See `docs/format/attributes.md`.
fn bullet_char(data: &[u8]) -> Option<PropValue> {
    let mut c = Cursor::new(data);
    let kind = match c.u32().ok()? {
        0 => "UnicodeOnly",
        1 => "UnicodeWithFont",
        2 => "GlyphWithFont",
        _ => return None,
    };
    let value = c.u32().ok()?;
    (c.remaining() == 0).then(|| {
        PropValue::Attributes(vec![
            ("BulletCharacterType", kind.into()),
            ("BulletCharacterValue", value.to_string()),
        ])
    })
}

/// Delimiter field, repetition and inclusiveness of a nested style's
/// delimiter code (`^c`, or `(d)` / `[d]` followed by an optional count).
/// See `docs/format/attributes.md`.
fn nested_delimiter(code: &str) -> Option<(Field, u32, bool)> {
    let enumeration = |v: &str| ("Delimiter", "enumeration", v.to_string());
    if code == "^c" {
        return Some((enumeration("Dropcap"), 1, true));
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
        "^w" => enumeration("AnyWord"),
        "^?" => enumeration("AnyCharacter"),
        _ if inner.chars().count() == 1 && inner != "^" => {
            ("Delimiter", "string", inner.to_string())
        }
        _ => return None,
    };
    Some((delimiter, repetition, inclusive))
}

const PACKAGING_NS: &str = "http://ns.adobe.com/AdobeInDesign/idml/1.0/packaging";
const MIMETYPE: &str = "application/vnd.adobe.indesign-idml-package";

/// Format a number the way IDML does: shortest round-trip form. IDML
/// keeps negative zero (`1 -0 -0 1 0 0`).
pub fn num(v: f64) -> String {
    format!("{v}")
}

/// Round away binary noise from scaled values (0.8 * 100 = 80.00000000000001).
pub(crate) fn round(v: f64) -> f64 {
    (v * 1e9).round() / 1e9
}

fn nums(v: &[f64]) -> String {
    v.iter().map(|&x| num(x)).collect::<Vec<_>>().join(" ")
}

fn matrix(m: &Matrix) -> String {
    nums(&m.0)
}

/// Characters per CDATA section of embedded file data, as in InDesign's
/// IDML export.
const CDATA_SECTION: usize = 262_144;

/// Base64 (RFC 4648, with padding) in lines of 76 characters separated by
/// line feeds, as IDML writes embedded file data.
fn base64_lines(data: &[u8]) -> String {
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

fn uref(uid: Option<u32>) -> String {
    uid.map_or("n".into(), |u| format!("u{u:x}"))
}

/// Name as used in a style's `Self` and in references: `$ID/` for built-ins.
fn style_name(s: &Style) -> String {
    if s.builtin {
        format!("$ID/{}", s.name)
    } else {
        s.name.clone()
    }
}

/// Escape a style name for use in a `Self` reference (`:` separates groups).
/// A text variable name as IDML writes it: control characters (U+001B in
/// the built-in cross-reference variables) become `<?AID 00xx?>`.
fn variable_name(name: &str) -> String {
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

fn self_name(name: &str) -> String {
    name.replace('%', "%25").replace(':', "%3a")
}

/// The top-left corner of a spread's pages in spread coordinates: the
/// smallest left and top edge of any page (0, 0 without pages).
fn spread_origin(pages: &[Page]) -> (f64, f64) {
    let corners = pages.iter().map(|p| {
        let [a, b, c, d, tx, ty] = p.transform.0;
        let (l, t) = (p.bounds[0], p.bounds[1]);
        (a * l + c * t + tx, b * l + d * t + ty)
    });
    corners
        .reduce(|(l0, t0), (l1, t1)| (l0.min(l1), t0.min(t1)))
        .unwrap_or((0.0, 0.0))
}

/// UIDs of the document pages, in order.
fn document_pages(doc: &Document) -> Vec<u32> {
    doc.spreads
        .iter()
        .flat_map(|s| &s.pages)
        .map(|p| p.uid)
        .collect()
}

/// Sections in page order, each with the index of its first page and its
/// length in pages. A section whose first page is not a document page is
/// left out.
fn section_ranges(doc: &Document) -> Vec<(&Section, usize, usize)> {
    let pages = document_pages(doc);
    if pages.is_empty() {
        return Vec::new();
    }
    let mut starts: Vec<(usize, &Section)> = doc
        .sections
        .iter()
        .filter_map(|s| match s.page {
            None => Some((0, s)),
            Some(p) => pages.iter().position(|&u| u == p).map(|i| (i, s)),
        })
        .collect();
    starts.sort_by_key(|&(i, _)| i);
    starts.dedup_by_key(|&mut (i, _)| i);
    (0..starts.len())
        .map(|k| {
            let (i, s) = starts[k];
            let end = starts.get(k + 1).map_or(pages.len(), |&(j, _)| j);
            (s, i, end - i)
        })
        .collect()
}

/// The IDML `AlternateLayout` of a section: its name, with `$ID/` before
/// a built-in key.
fn alternate_layout_name(s: &Section) -> Option<String> {
    s.alternate_layout.as_ref().map(|(flag, name)| match flag {
        1 => format!("$ID/{name}"),
        _ => name.clone(),
    })
}

/// For each section of `section_ranges`, the number of pages from its
/// start to the start of the next alternate layout (a section with a
/// layout name) or the end of the document; and for each document page,
/// the section that starts its alternate layout. See
/// `docs/format/objects.md`, sections.
fn alternate_layouts(doc: &Document) -> (Vec<usize>, Vec<u32>) {
    let ranges = section_ranges(doc);
    let total: usize = ranges.iter().map(|&(_, _, n)| n).sum();
    let starts: Vec<(usize, u32)> = ranges
        .iter()
        .filter(|(s, _, _)| alternate_layout_name(s).is_some_and(|n| !n.is_empty() && n != "$ID/"))
        .map(|&(s, i, _)| (i, s.uid))
        .collect();
    let lengths = ranges
        .iter()
        .map(|&(_, i, _)| {
            starts
                .iter()
                .find(|&&(j, _)| j > i)
                .map_or(total, |&(j, _)| j)
                - i
        })
        .collect();
    let first = ranges.first().map(|(s, _, _)| s.uid);
    let pages = (0..total)
        .filter_map(|p| {
            starts
                .iter()
                .rev()
                .find(|&&(j, _)| j <= p)
                .map(|&(_, u)| u)
                .or(first)
        })
        .collect();
    (lengths, pages)
}

/// IDML `PageNumberStyle` of a section's style code.
fn number_style(code: u32) -> Option<&'static str> {
    match code {
        numbering::ARABIC => Some("Arabic"),
        numbering::LOWER_ROMAN => Some("LowerRoman"),
        numbering::KANJI => Some("Kanji"),
        _ => None,
    }
}

/// Lower-case Roman numeral of `n` (1 or more).
fn lower_roman(mut n: u32) -> String {
    const DIGITS: [(u32, &str); 13] = [
        (1000, "m"),
        (900, "cm"),
        (500, "d"),
        (400, "cd"),
        (100, "c"),
        (90, "xc"),
        (50, "l"),
        (40, "xl"),
        (10, "x"),
        (9, "ix"),
        (5, "v"),
        (4, "iv"),
        (1, "i"),
    ];
    let mut out = String::new();
    for (value, digits) in DIGITS {
        while n >= value {
            out.push_str(digits);
            n -= value;
        }
    }
    out
}

/// Chinese numeral of `n`, digit by digit (10 is 一〇), as the folios of
/// a sample with that style show.
fn kanji_digits(n: u32) -> String {
    const DIGITS: [char; 10] = ['〇', '一', '二', '三', '四', '五', '六', '七', '八', '九'];
    n.to_string()
        .bytes()
        .map(|b| DIGITS[usize::from(b - b'0')])
        .collect()
}

/// The name of each document page: its number (see `page_numbers`) in
/// its section's style. Styles other than lower-case Roman and Kanji are
/// written as Arabic numbers.
fn page_names(doc: &Document) -> Vec<String> {
    let numbers = page_numbers(doc);
    let mut out: Vec<String> = numbers.iter().map(u32::to_string).collect();
    for (s, first, length) in section_ranges(doc) {
        let name: fn(u32) -> String = match s.style {
            numbering::LOWER_ROMAN => lower_roman,
            numbering::KANJI => kanji_digits,
            _ => continue,
        };
        for i in (first..first + length).filter(|&i| numbers[i] > 0) {
            out[i] = name(numbers[i]);
        }
    }
    out
}

/// The number shown on each document page, from the sections. Pages
/// not covered by a section are numbered by their position.
/// The section and page number of each document page.
fn page_sections(doc: &Document) -> Vec<(Section, u32)> {
    let numbers = page_numbers(doc);
    let mut out = Vec::new();
    for (s, first, length) in section_ranges(doc) {
        for &n in numbers.iter().skip(first).take(length) {
            out.push((s.clone(), n));
        }
    }
    out
}

fn page_numbers(doc: &Document) -> Vec<u32> {
    let count = document_pages(doc).len();
    let mut out: Vec<u32> = (1..=count as u32).collect();
    let mut next: Option<u32> = None;
    for (s, first, length) in section_ranges(doc) {
        let mut number = match next {
            Some(n) if s.continue_numbering => n,
            None if s.continue_numbering => first as u32 + 1,
            _ => s.start,
        };
        for n in out.iter_mut().skip(first).take(length) {
            *n = number;
            number += 1;
        }
        next = Some(number);
    }
    out
}

struct Writer<'a> {
    doc: &'a Document,
    dom: String,
    /// Document file name.
    name: String,
    /// Names of the enclosing style groups of each style or group UID.
    group_path: std::collections::HashMap<u32, Vec<String>>,
    /// Values left out while writing, reported with the model's warnings.
    warnings: std::cell::RefCell<Vec<String>>,
    /// For each document page, the section that starts its alternate
    /// layout (`alternate_layouts`).
    page_layouts: Vec<u32>,
    /// For each document page, its section and page number.
    page_sections: Vec<(Section, u32)>,
    /// Observed attributes by element path (`values::element_attrs`).
    observed: std::cell::RefCell<std::collections::HashMap<String, Observed>>,
}

/// Attributes observed on an element path (`values::element_attrs`).
type Observed = std::rc::Rc<Vec<(String, String)>>;

fn group_paths(doc: &Document) -> std::collections::HashMap<u32, Vec<String>> {
    fn walk(
        doc: &Document,
        g: &StyleGroup,
        path: &[String],
        out: &mut std::collections::HashMap<u32, Vec<String>>,
    ) {
        for &c in &g.children {
            out.insert(c, path.to_vec());
            if let Some(sub) = doc.style_groups.get(&c) {
                let mut p = path.to_vec();
                p.push(sub.name.clone());
                out.insert(c, p.clone());
                walk(doc, sub, &p, out);
            }
        }
    }
    let mut out = std::collections::HashMap::new();
    for g in doc.style_groups.values().filter(|g| g.root.is_some()) {
        walk(doc, g, &[], &mut out);
    }
    out
}

impl Writer<'_> {
    /// The attributes every IDML of the document's version has on the
    /// elements on `path` (`docs/format/idml-values.md`).
    fn observed(&self, path: &str) -> Observed {
        self.observed
            .borrow_mut()
            .entry(path.to_string())
            .or_insert_with(|| {
                std::rc::Rc::new(values::element_attrs(path, self.doc.version.major))
            })
            .clone()
    }

    fn package_root(&self, x: &mut Xml, kind: &str) {
        x.start(&format!("idPkg:{kind}"))
            .attr("xmlns:idPkg", PACKAGING_NS)
            .attr("DOMVersion", &self.dom);
    }

    fn style_ref(&self, uid: Option<u32>, paragraph: bool) -> String {
        let prefix = if paragraph {
            "ParagraphStyle"
        } else {
            "CharacterStyle"
        };
        match uid.and_then(|u| self.doc.styles.get(&u)) {
            Some(s) => {
                let mut parts = self.group_path.get(&s.uid).cloned().unwrap_or_default();
                parts.push(style_name(s));
                format!("{prefix}/{}", self_name(&parts.join(":")))
            }
            None if paragraph => "ParagraphStyle/$ID/NormalParagraphStyle".into(),
            None => "CharacterStyle/$ID/[No character style]".into(),
        }
    }

    fn designmap(&self, name: &str) -> String {
        let doc = self.doc;
        let mut x = Xml::new();
        x.pi(&format!(
            "aid style=\"50\" type=\"document\" readerVersion=\"6.0\" featureSet=\"257\" product=\"{}\" ",
            self.dom
        ));
        // The stories, then the backing story.
        let stories: Vec<String> = doc
            .stories
            .iter()
            .chain(&doc.xml.story)
            .map(|s| uref(Some(s.uid)))
            .collect();
        x.start("Document")
            .attr("xmlns:idPkg", PACKAGING_NS)
            .attr("DOMVersion", &self.dom)
            .attr("Self", "d")
            .attr("StoryList", stories.join(" "))
            .attr("Name", name)
            .attr("ZeroPoint", "0 0");
        if let Some(l) = doc.active_layer {
            x.attr("ActiveLayer", uref(Some(l)));
        }
        self.languages(&mut x);
        x.empty("idPkg:Graphic", &[("src", "Resources/Graphic.xml".into())]);
        x.empty("idPkg:Fonts", &[("src", "Resources/Fonts.xml".into())]);
        // Kinsoku tables, then mojikumi tables, as the schema orders them.
        for mojikumi in [false, true] {
            for t in doc.cjk_tables.iter().filter(|t| t.mojikumi == mojikumi) {
                let tag = if mojikumi {
                    "MojikumiTable"
                } else {
                    "KinsokuTable"
                };
                x.start(tag)
                    .attr("Self", format!("{tag}/{}", self_name(&t.name)))
                    .attr("Name", &t.name);
                if let Some([begin, end, _, hanging, together]) = &t.chars {
                    x.attr("CantBeginLineChars", begin)
                        .attr("CantEndLineChars", end)
                        .attr("HangingPunctuationChars", hanging)
                        .attr("CantBeSeparatedChars", together);
                }
                x.end();
            }
        }
        x.empty("idPkg:Styles", &[("src", "Resources/Styles.xml".into())]);
        // Present, with these values, in every corpus IDML; the root
        // paragraph style refers to it. See docs/format/idml-values.md.
        x.empty(
            "NumberingList",
            &[
                ("Self", "NumberingList/$ID/[Default]".into()),
                ("Name", "$ID/[Default]".into()),
                ("ContinueNumbersAcrossStories", "false".into()),
                ("ContinueNumbersAcrossDocuments", "false".into()),
            ],
        );
        let major = doc.version.major;
        // Elements every IDML of the version has, with the values they
        // all have (idml-values.md), in the order of the IDML files.
        let singleton = |x: &mut Xml, tag: &str| {
            if let Some(n) = values::present(&format!("Document/{tag}"), major) {
                n.write(x);
            }
        };
        singleton(&mut x, "ConditionalTextPreference");
        x.empty(
            "idPkg:Preferences",
            &[("src", "Resources/Preferences.xml".into())],
        );
        for tag in [
            "EndnoteOption",
            "TextFrameFootnoteOptionsObject",
            "LinkedStoryOption",
            "LinkedPageItemOption",
            "WatermarkPreference",
            "TaggedPDFPreference",
            "AdjustLayoutPreference",
            "HTMLFXLExportPreference",
            "PublishExportPreference",
        ] {
            singleton(&mut x, tag);
        }
        self.text_variables(&mut x);
        x.empty("idPkg:Tags", &[("src", "XML/Tags.xml".into())]);
        for l in doc.layers.iter().filter(|l| !l.internal) {
            x.start("Layer")
                .attr("Self", uref(Some(l.uid)))
                .attr("Name", &l.name)
                .attr("Visible", l.visible.to_string())
                .attr("Locked", l.locked.to_string());
            let mut color = None;
            if let Some(st) = &l.settings {
                x.attr("IgnoreWrap", st.ignore_wrap.to_string())
                    .attr("LockGuides", st.lock_guides.to_string())
                    .attr("UI", st.ui.to_string())
                    .attr("Printable", st.printable.to_string());
                color = st.color.and_then(|c| ui_color_property("LayerColor", c));
            }
            x.attrs_missing(self.observed("Layer").iter());
            if let Some(c) = color {
                Self::properties_with(&mut x, &[], &[c]);
            }
            x.end();
        }
        for s in &doc.master_spreads {
            x.empty(
                "idPkg:MasterSpread",
                &[(
                    "src",
                    format!("MasterSpreads/MasterSpread_u{:x}.xml", s.uid),
                )],
            );
        }
        for s in &doc.spreads {
            x.empty(
                "idPkg:Spread",
                &[("src", format!("Spreads/Spread_u{:x}.xml", s.uid))],
            );
        }
        let pages = document_pages(doc);
        let (layout_lengths, _) = alternate_layouts(doc);
        for (k, (section, first, length)) in section_ranges(doc).into_iter().enumerate() {
            x.start("Section")
                .attr("Self", uref(Some(section.uid)))
                .attr("Length", length.to_string())
                .attr("Name", "")
                .attr("ContinueNumbering", section.continue_numbering.to_string())
                .attr("IncludeSectionPrefix", "false")
                .attr("Marker", &section.marker)
                .attr("PageStart", uref(Some(pages[first])));
            if doc.version.major >= 8
                && let Some(name) = alternate_layout_name(section)
            {
                x.attr("AlternateLayoutLength", layout_lengths[k].to_string())
                    .attr("AlternateLayout", name);
            }
            // IDML gives a start number only to sections that restart
            // numbering.
            if !section.continue_numbering {
                x.attr("PageNumberStart", section.start.to_string());
            }
            x.attr("SectionPrefix", &section.prefix);
            if let Some(style) = number_style(section.style) {
                x.start("Properties")
                    .start("PageNumberStyle")
                    .attr("type", "enumeration")
                    .text(style)
                    .end()
                    .end();
            }
            x.end();
        }
        // Document users. A user with flag 2 is the placeholder for an
        // unknown user, which IDML names `$ID/Unknown User Name`; the
        // colours are left out (docs/format/objects.md, document users).
        for (i, (flag, name)) in doc.users.iter().enumerate() {
            let name = if *flag == 2 {
                "$ID/Unknown User Name"
            } else {
                name.as_str()
            };
            x.empty(
                "DocumentUser",
                &[
                    ("Self", format!("dDocumentUser{i:x}")),
                    ("UserName", name.into()),
                ],
            );
        }
        self.cross_reference_formats(&mut x);
        x.empty(
            "idPkg:BackingStory",
            &[("src", "XML/BackingStory.xml".into())],
        );
        for s in &doc.stories {
            x.empty(
                "idPkg:Story",
                &[("src", format!("Stories/Story_u{:x}.xml", s.uid))],
            );
        }
        self.hyperlinks(&mut x);
        self.color_groups(&mut x);
        for (i, b) in doc.bullets.iter().enumerate() {
            const KINDS: [&str; 3] = ["UnicodeOnly", "UnicodeWithFont", "GlyphWithFont"];
            let Some(kind) = KINDS.get(b.kind as usize) else {
                continue;
            };
            let font = match b.font {
                0 => "$ID/".to_string(),
                f => match doc.fonts.get(&f) {
                    Some(f) => f.name.clone(),
                    None => continue,
                },
            };
            x.start("ABullet")
                .attr("Self", format!("dABullet{i}"))
                .attr("CharacterType", *kind)
                .attr("CharacterValue", b.value.to_string());
            Self::properties(
                &mut x,
                &[
                    ("BulletsFont", "string", font.into()),
                    ("BulletsFontStyle", "string", b.font_style.clone().into()),
                ],
            );
            x.end();
        }
        x.end();
        x.finish()
    }

    /// The document's languages, in UID order. See
    /// `docs/format/objects.md`, languages.
    fn languages(&self, x: &mut Xml) {
        let mut list: Vec<_> = self.doc.language_list.iter().collect();
        list.sort_by_key(|l| l.uid);
        for l in list {
            // The language without one is `Neutral` in the INDD.
            let neutral = l.name == "Neutral";
            let name = |s: &str| {
                if neutral {
                    "$ID/[No Language]".to_string()
                } else {
                    format!("$ID/{s}")
                }
            };
            let full = name(&l.name);
            x.start("Language")
                .attr("Self", format!("Language/{}", self_name(&full)))
                .attr("Name", &full);
            if let Some(q) = values::keyed("Language", &full) {
                for k in ["SingleQuotes", "DoubleQuotes"] {
                    if let Some(v) = q.attr(k) {
                        x.attr(k, v);
                    }
                }
            }
            x.attr("PrimaryLanguageName", name(&l.primary))
                .attr("SublanguageName", name(&l.sub))
                .attr("Id", l.id.to_string());
            if let Some([(sf, spelling), (hf, hyphenation)]) = &l.vendors {
                if *hf == 1 {
                    x.attr("HyphenationVendor", hyphenation);
                }
                if *sf == 1 {
                    x.attr("SpellingVendor", spelling);
                }
            }
            x.end();
        }
    }

    /// Colour groups and their swatches. See `docs/format/objects.md`.
    fn color_groups(&self, x: &mut Xml) {
        for (i, g) in self.doc.color_groups.iter().enumerate() {
            x.start("ColorGroup")
                .attr("Self", format!("ColorGroup/{}", self_name(&g.name)))
                .attr("Name", &g.name)
                .attr("IsRootColorGroup", (i == 0).to_string());
            for (n, s) in g.swatches.iter().enumerate() {
                if let Some(r) = self.doc.swatches.get(s) {
                    x.empty(
                        "ColorGroupSwatch",
                        &[
                            (
                                "Self",
                                format!("{}ColorGroupSwatch{n:x}", uref(Some(g.uid))),
                            ),
                            ("SwatchItemRef", r.clone()),
                        ],
                    );
                }
            }
            x.end();
        }
    }

    /// Cross-reference formats. See docs/format/cross-references.md.
    fn cross_reference_formats(&self, x: &mut Xml) {
        for f in self.doc.cross_reference_formats.values() {
            let id = uref(Some(f.uid));
            x.start("CrossReferenceFormat")
                .attr("Self", &id)
                .attr("Name", &f.name);
            if f.character_style == 0 {
                x.attr("AppliedCharacterStyle", "n");
            }
            for (i, b) in f.blocks.iter().enumerate() {
                let Some(kind) = b.type_name() else { continue };
                x.start("BuildingBlock")
                    .attr("Self", format!("{id}BuildingBlock{i}"))
                    .attr("BlockType", kind);
                if b.zero_fields {
                    x.attr("AppliedCharacterStyle", "n");
                }
                x.attr("CustomText", &b.text);
                if b.zero_fields {
                    x.attr("AppliedDelimiter", "$ID/")
                        .attr("IncludeDelimiter", "false");
                }
                x.end();
            }
            x.end();
        }
    }

    /// Hyperlink text sources written in the stories.
    fn written_sources(&self) -> std::collections::HashSet<u32> {
        self.doc
            .stories
            .iter()
            .flat_map(|s| &s.sources)
            .map(|r| r.source)
            .filter(|u| self.doc.text_sources.contains_key(u))
            .collect()
    }

    /// Destinations, hyperlinks and bookmarks, in schema order. See
    /// docs/format/hyperlinks.md.
    fn hyperlinks(&self, x: &mut Xml) {
        let doc = self.doc;
        let mut dests: Vec<_> = doc.destinations.iter().collect();
        dests.sort_by_key(|d| (matches!(d.kind, DestinationKind::Url { .. }), d.key));
        for d in &dests {
            match &d.kind {
                DestinationKind::Page { page, zoom, view } => {
                    x.start("HyperlinkPageDestination")
                        .attr("Self", d.reference())
                        .attr("Name", &d.name)
                        .attr("DestinationPage", uref(Some(*page)));
                    match view {
                        0 => {
                            x.attr("ViewSetting", "Fixed");
                        }
                        1 => {
                            x.attr("ViewSetting", "FitWindow");
                        }
                        _ => {}
                    }
                    if let Some(zoom) = zoom {
                        x.attr("ViewPercentage", num(zoom * 100.0));
                    }
                }
                DestinationKind::Url { url } => {
                    x.start("HyperlinkURLDestination")
                        .attr("Self", d.reference())
                        .attr("Name", &d.name)
                        .attr("DestinationURL", url);
                }
            }
            x.attr("Hidden", d.hidden.to_string())
                .attr("DestinationUniqueKey", d.key.to_string())
                .end();
        }
        let sources = self.written_sources();
        let mut links: Vec<_> = doc.hyperlinks.iter().collect();
        links.sort_by_key(|h| h.uid);
        for h in links {
            let Some(dest) = dests.iter().find(|d| d.key == h.key) else {
                continue;
            };
            if !sources.contains(&h.source) {
                continue;
            }
            x.start("Hyperlink")
                .attr("Self", uref(Some(h.uid)))
                .attr("Name", &h.name)
                .attr("Source", uref(Some(h.source)));
            // One value in every corpus pair; see the format notes.
            if h.as_in_samples {
                x.attr("Visible", "false")
                    .attr("Highlight", "None")
                    .attr("Width", "Thin")
                    .attr("BorderStyle", "Solid");
            }
            x.attr("Hidden", h.hidden.to_string())
                .attr("DestinationUniqueKey", h.key.to_string());
            x.start("Properties");
            if h.as_in_samples {
                x.start("BorderColor")
                    .attr("type", "enumeration")
                    .text("Black")
                    .end();
            }
            x.start("Destination")
                .attr("type", "object")
                .text(&dest.reference())
                .end();
            x.end().end();
        }
        let top = doc
            .bookmark_order
            .iter()
            .filter_map(|u| doc.bookmarks.get(u))
            .filter(|b| !doc.bookmarks.contains_key(&b.parent));
        for b in top {
            self.bookmark(x, b, 0);
        }
    }

    fn bookmark(&self, x: &mut Xml, b: &crate::model::Bookmark, depth: usize) {
        let doc = self.doc;
        let Some(dest) = doc.destinations.iter().find(|d| d.uid == b.destination) else {
            return;
        };
        x.start("Bookmark")
            .attr("Self", uref(Some(b.uid)))
            .attr("Name", &b.name)
            .attr("Destination", dest.reference());
        if depth < 64 {
            for c in b.children.iter().filter_map(|u| doc.bookmarks.get(u)) {
                self.bookmark(x, c, depth + 1);
            }
        }
        x.end();
    }

    /// `TextVariable` elements, sorted by name as in every corpus IDML.
    /// See docs/format/text-variables.md.
    fn text_variables(&self, x: &mut Xml) {
        let mut vars: Vec<_> = self.doc.text_variables.iter().collect();
        vars.sort_by(|a, b| a.name.cmp(&b.name));
        for v in vars {
            let name = variable_name(&v.name);
            x.start("TextVariable")
                .attr("Self", format!("dTextVariablen{name}"))
                .attr("Name", &name);
            if let Some(t) = v.type_name() {
                x.attr("VariableType", t);
            }
            self.variable_preference(x, v);
            x.end();
        }
    }

    /// The settings element of a text variable. Attributes other than the
    /// date format and the running header's style are not located in the
    /// INDD; they are written only for variables whose unidentified fields
    /// hold the values of every corpus sample, and then with the values
    /// those samples have in IDML.
    fn variable_preference(&self, x: &mut Xml, v: &TextVariable) {
        let same = v.as_in_samples;
        let text = |x: &mut Xml, name: &str| {
            if same {
                x.attr(name, "");
            }
        };
        match v.kind {
            0xCAA1 | 0xCAAB | 0xCAAC => {
                x.start("DateVariablePreference");
                text(x, "TextBefore");
                x.attr("Format", &v.text);
                text(x, "TextAfter");
                x.end();
            }
            0xCAAA => {
                x.start("MatchParagraphStylePreference");
                text(x, "TextBefore");
                text(x, "TextAfter");
                x.attr("AppliedParagraphStyle", self.style_ref(v.style, true));
                if same {
                    x.attr("SearchStrategy", "FirstOnPage")
                        .attr("ChangeCase", "None")
                        .attr("DeleteEndPunctuation", "false");
                }
                x.end();
            }
            _ if !same => {}
            0xCAA3 => {
                x.start("CustomTextVariablePreference")
                    .start("Properties")
                    .start("Contents")
                    .attr("type", "string")
                    .end()
                    .end()
                    .end();
            }
            0xCAA6 => {
                x.empty(
                    "FileNameVariablePreference",
                    &[
                        ("TextBefore", String::new()),
                        ("IncludePath", "false".into()),
                        ("IncludeExtension", "false".into()),
                        ("TextAfter", String::new()),
                    ],
                );
            }
            0xCAA8 => {
                x.empty(
                    "PageNumberVariablePreference",
                    &[
                        ("TextBefore", String::new()),
                        ("Format", "Current".into()),
                        ("TextAfter", String::new()),
                        ("Scope", "SectionScope".into()),
                    ],
                );
            }
            0xCAA9 => {
                x.empty(
                    "ChapterNumberVariablePreference",
                    &[
                        ("TextBefore", String::new()),
                        ("Format", "Current".into()),
                        ("TextAfter", String::new()),
                    ],
                );
            }
            0xCAC0 => {
                x.empty(
                    "CaptionMetadataVariablePreference",
                    &[
                        ("TextBefore", String::new()),
                        ("MetadataProviderName", "$ID/#LinkInfoNameStr".into()),
                        ("TextAfter", String::new()),
                    ],
                );
            }
            _ => {}
        }
    }

    /// A text variable instance in place of its U+0018.
    fn variable_instance(&self, x: &mut Xml, v: &Instance) {
        let name = variable_name(&v.name);
        x.start("TextVariableInstance")
            .attr("Self", uref(Some(v.uid)))
            .attr("Name", &name);
        // The displayed text is not stored. A file name variable with the
        // sample settings shows the document's name without extension.
        let def = self.doc.text_variables.iter().find(|d| d.name == v.name);
        if let Some(d) = def
            && d.kind == 0xCAA6
            && d.as_in_samples
        {
            let stem = self.name.strip_suffix(".indd").unwrap_or(&self.name);
            x.attr("ResultText", stem);
        }
        x.attr("AssociatedTextVariable", format!("dTextVariablen{name}"));
        x.end();
    }

    /// `SwatchColorGroupReference` of each swatch in a colour group: the
    /// `ColorGroupSwatch` that names it. See `docs/format/objects.md`.
    fn group_swatches(&self) -> std::collections::HashMap<String, String> {
        let mut out = std::collections::HashMap::new();
        for g in &self.doc.color_groups {
            for (n, s) in g.swatches.iter().enumerate() {
                if let Some(r) = self.doc.swatches.get(s) {
                    out.entry(r.clone())
                        .or_insert_with(|| format!("{}ColorGroupSwatch{n:x}", uref(Some(g.uid))));
                }
            }
        }
        out
    }

    fn graphic(&self) -> String {
        let mut x = Xml::new();
        self.package_root(&mut x, "Graphic");
        let groups = self.group_swatches();
        // IDML names the colour group swatch of every swatch from DOM 12
        // on (`n` for none).
        let group_ref = |x: &mut Xml, reference: &str| {
            if self.doc.version.major >= 12 {
                x.attr(
                    "SwatchColorGroupReference",
                    groups.get(reference).map_or("n", String::as_str),
                );
            }
        };
        for c in &self.doc.colors {
            let name = c.idml_name();
            let mut attrs = vec![("Self", c.reference())];
            if let Some(model) = c.model_name() {
                attrs.push(("Model", model.into()));
            }
            attrs.extend([
                ("Space", c.space_name().into()),
                ("ColorValue", nums(&c.idml_values())),
                ("ColorOverride", c.override_name().into()),
                ("Name", name),
                ("ColorEditable", c.editable.to_string()),
                ("ColorRemovable", c.removable.to_string()),
                ("Visible", c.visible.to_string()),
            ]);
            if let Some((space, values)) = c.idml_alternate() {
                attrs.push(("AlternateSpace", space.into()));
                attrs.push(("AlternateColorValue", nums(&values)));
            }
            if let Some(id) = c.creator {
                attrs.push(("SwatchCreatorID", id.to_string()));
            }
            if self.doc.version.major >= 16 {
                let hsb = c.space == crate::model::color::Space::Hsb;
                attrs.push(("ConvertToHsb", hsb.to_string()));
            }
            x.start("Color");
            for (k, v) in &attrs {
                x.attr(k, v);
            }
            group_ref(&mut x, &c.reference());
            x.end();
        }
        // The schema puts inks after the colours.
        for i in &self.doc.inks {
            let mut attrs = vec![
                ("Self", format!("Ink/{}", self_name(&i.name))),
                ("Name", i.name.clone()),
                ("Angle", num(i.angle)),
                // These three are the same in every Ink of the corpus
                // IDML files (idml-values.md).
                ("ConvertToProcess", "false".into()),
                ("Frequency", num(i.frequency)),
            ];
            if let Some(d) = i.neutral_density {
                attrs.push(("NeutralDensity", num(d)));
            }
            attrs.extend([
                ("PrintInk", "true".into()),
                ("TrapOrder", i.trap_order.to_string()),
                ("InkType", "Normal".into()),
            ]);
            x.empty("Ink", &attrs);
        }
        for (t, reference, name) in &self.doc.tints {
            let base = self.doc.swatches.get(&t.base).cloned().unwrap_or_default();
            x.start("Tint")
                .attr("Self", reference)
                .attr("TintValue", num(t.value))
                .attr("BaseColor", base)
                .attr("Name", name)
                .attr("ColorOverride", t.override_name());
            group_ref(&mut x, reference);
            x.attrs_missing(self.observed("Tint").iter());
            x.end();
        }
        x.start("Swatch")
            .attr("Self", "Swatch/None")
            .attr("Name", "None")
            .attr("ColorEditable", "false")
            .attr("ColorRemovable", "false")
            .attr("Visible", "true");
        group_ref(&mut x, "Swatch/None");
        x.attrs_missing(self.observed("Swatch").iter());
        x.end();
        // The schema requires gradients after the swatches.
        for g in &self.doc.gradients {
            x.start("Gradient")
                .attr("Self", g.reference())
                .attr("Type", if g.kind == 2 { "Radial" } else { "Linear" })
                .attr("Name", g.idml_name())
                .attr("ColorEditable", g.editable.to_string())
                .attr("ColorRemovable", g.removable.to_string())
                .attr("Visible", g.visible.to_string());
            group_ref(&mut x, &g.reference());
            x.attrs_missing(self.observed("Gradient").iter());
            for (i, stop) in g.stops.iter().enumerate() {
                let color = self
                    .doc
                    .swatches
                    .get(&stop.color)
                    .cloned()
                    .unwrap_or_else(|| "Color/Black".into());
                x.start("GradientStop")
                    .attr("Self", format!("{}GradientStop{i}", uref(Some(g.uid))))
                    .attr("StopColor", color)
                    .attr("Location", num(round(stop.location * 100.0)));
                if let Some(m) = g.idml_midpoint(i) {
                    x.attr("Midpoint", num(m));
                }
                x.end();
            }
            x.end();
        }
        for name in BUILTIN_STROKE_STYLES {
            x.empty(
                "StrokeStyle",
                &[
                    ("Self", format!("StrokeStyle/$ID/{name}")),
                    ("Name", format!("$ID/{name}")),
                ],
            );
        }
        x.end();
        x.finish()
    }

    /// IDML attributes and properties for a text attribute list.
    fn text_attrs(&self, attrs: &Attrs) -> (Vec<(&'static str, String)>, Vec<Property>) {
        let mut plain = Vec::new();
        let mut props = Vec::new();
        for &(id, name, kind, in_props) in TEXT_ATTRS {
            let Some(v) = attrs.get(id) else { continue };
            let out: Option<(&'static str, PropValue)> = match kind {
                TextKind::TabList => tab_list(&raw_bytes(v)).map(|l| ("list", PropValue::List(l))),
                TextKind::NestedStyles => self
                    .nested_styles(&raw_bytes(v))
                    .map(|l| ("list", PropValue::List(l))),
                TextKind::BulletChar => bullet_char(&raw_bytes(v)).map(|a| ("", a)),
                _ => self.text_value(kind, v).map(|(t, s)| (t, s.into())),
            };
            if out.is_none() && matches!(kind, TextKind::Enum(_)) {
                unknown_code(attrs, id, v);
            }
            if let Some((ty, value)) = out {
                if in_props {
                    props.push((name, ty, value));
                } else if let PropValue::Text(text) = value {
                    plain.push((name, text));
                }
            }
        }
        (plain, props)
    }

    /// IDML type and text of a single-valued text attribute.
    fn text_value(&self, kind: TextKind, v: &Value) -> Option<(&'static str, String)> {
        let swatch = |u: u32| self.doc.swatches.get(&u).cloned();
        match kind {
            TextKind::Number => v
                .as_f64()
                .or(v.as_u32().map(f64::from))
                .map(|f| ("unit", num(f))),
            TextKind::Point => match raw_bytes(v).as_slice() {
                b if b.len() == 16 => {
                    let f = |o: usize| f64_from(b[o..o + 8].try_into().unwrap());
                    Some(("unit", nums(&[f(0), f(8)])))
                }
                _ => None,
            },
            TextKind::Percent => v.as_f64().map(|f| ("unit", num(round(f * 100.0)))),
            TextKind::Scale(k) => v.as_f64().map(|f| ("unit", num(round(f * k)))),
            TextKind::Kerning => v
                .as_f64()
                .filter(|&f| f != KERNING_NONE)
                .map(|f| ("unit", num(round(f * 1000.0)))),
            TextKind::Bool(t) => v.as_u32().map(|u| ("boolean", (u == t).to_string())),
            TextKind::Enum(map) => v
                .as_u32()
                .and_then(|u| map.iter().find(|(k, _)| *k == u))
                .map(|(_, n)| ("enumeration", n.to_string())),
            TextKind::Swatch => v.as_u32().and_then(swatch).map(|s| ("object", s)),
            TextKind::SwatchOrText => match v.as_u32() {
                Some(0) => Some(("string", "Text Color".into())),
                Some(u) => swatch(u).map(|s| ("object", s)),
                None => None,
            },
            TextKind::Font => v
                .as_u32()
                .and_then(|u| self.doc.fonts.get(&u))
                .map(|f| ("string", f.name.clone())),
            TextKind::FontStyle => v.as_string().map(|s| ("string", s)),
            TextKind::Leading => v.as_f64().map(|f| {
                if f < 0.0 {
                    ("enumeration", "Auto".into())
                } else {
                    ("unit", num(f))
                }
            }),
            TextKind::Language => v
                .as_u32()
                .and_then(|u| self.doc.languages.get(&u))
                .map(|l| ("string", format!("$ID/{l}"))),
            TextKind::NumberOr(code, name, ty) => v.as_f64().map(|f| {
                if f == code {
                    ("enumeration", name.to_string())
                } else {
                    (ty, num(f))
                }
            }),
            TextKind::CharacterStyle => v
                .as_u32()
                .map(|u| ("object", self.style_ref(Some(u).filter(|&u| u != 0), false))),
            TextKind::FontOrNone => match v.as_u32() {
                Some(0) => Some(("string", "$ID/".into())),
                Some(u) => self.doc.fonts.get(&u).map(|f| ("string", f.name.clone())),
                None => None,
            },
            TextKind::NonZero => v
                .as_u32()
                .filter(|&u| u != 0)
                .map(|u| ("long", u.to_string())),
            TextKind::Text => {
                let b = raw_bytes(v);
                let mut c = Cursor::new(&b);
                let n = c.u32().ok()? as usize;
                c.segments(n)
                    .ok()
                    .filter(|t| !t.is_empty())
                    .map(|t| ("string", t))
            }
            TextKind::StringOrNothing => v.as_string().map(|s| {
                if s.is_empty() {
                    ("enumeration", "Nothing".into())
                } else {
                    ("string", s)
                }
            }),
            TextKind::TabList | TextKind::NestedStyles | TextKind::BulletChar => None,
        }
    }

    /// Records of an `AllNestedStyles` list. See `docs/format/attributes.md`.
    fn nested_styles(&self, data: &[u8]) -> Option<Vec<Vec<Field>>> {
        let mut c = Cursor::new(data);
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
            out.push(vec![
                (
                    "AppliedCharacterStyle",
                    "object",
                    self.style_ref(Some(style).filter(|&u| u != 0), false),
                ),
                delimiter,
                ("Repetition", "long", repetition.to_string()),
                ("Inclusive", "boolean", inclusive.to_string()),
            ]);
        }
        (c.remaining() == 0).then_some(out)
    }

    fn properties(x: &mut Xml, props: &[Property]) {
        Self::properties_with(x, props, &[]);
    }

    /// `<Properties>` with `props`, then the elements in `extra`.
    fn properties_with(x: &mut Xml, props: &[Property], extra: &[Node]) {
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

    /// Observed values of a root style (see `values`) that are not in
    /// `written` (attributes) or `props` (Properties children): attributes,
    /// Properties children and other child elements.
    fn root_values(
        &self,
        tag: &str,
        written: &[&str],
        props: &[Property],
    ) -> (Vec<(String, String)>, Vec<Node>, Vec<Node>) {
        let node = values::root_style(tag, self.doc.version.major);
        let attrs = node
            .attrs
            .into_iter()
            .filter(|(k, _)| !written.contains(&k.as_str()))
            .collect();
        let mut extra_props = Vec::new();
        let mut children = Vec::new();
        for c in node.children {
            if c.tag == "Properties" {
                extra_props.extend(
                    c.children
                        .into_iter()
                        .filter(|p| !props.iter().any(|(n, _, _)| *n == p.tag)),
                );
            } else {
                children.push(c);
            }
        }
        (attrs, extra_props, children)
    }

    /// A cell or table style other than the root style.
    fn table_style_element(
        &self,
        x: &mut Xml,
        tag: &str,
        s: &crate::model::TableStyle,
        styles: &BTreeMap<u32, crate::model::TableStyle>,
    ) {
        x.start(tag)
            .attr("Self", Self::table_style_ref(tag, s))
            .attr("Name", Self::table_style_name(s));
        let attrs = if tag == "CellStyle" {
            self.cell_attrs(&s.attrs)
        } else {
            self.table_attrs(&s.attrs)
        };
        for (k, v) in attrs {
            x.attr(k, v);
        }
        if tag == "CellStyle"
            && let Some(p) = s
                .attrs
                .get(CELL_STYLE_PARAGRAPH_STYLE)
                .and_then(Value::as_u32)
            && self.doc.styles.contains_key(&p)
        {
            x.attr("AppliedParagraphStyle", self.style_ref(Some(p), true));
        }
        if let Some(base) = s.based_on.and_then(|b| styles.get(&b)) {
            // A root style is written as a string.
            let prop = if base.builtin && base.based_on.is_none() {
                ("BasedOn", "string", Self::table_style_name(base).into())
            } else {
                ("BasedOn", "object", Self::table_style_ref(tag, base).into())
            };
            Self::properties(x, &[prop]);
        }
        x.end();
    }

    /// A root cell or table style, from observed values only.
    fn root_table_style(&self, x: &mut Xml, tag: &str, name: &str) {
        let (attrs, props, children) = self.root_values(tag, &[], &[]);
        x.start(tag)
            .attr("Self", format!("{tag}/$ID/{name}"))
            .attr("Name", format!("$ID/{name}"));
        for (k, v) in &attrs {
            x.attr(k, v);
        }
        Self::properties_with(x, &[], &props);
        for c in &children {
            c.write(x);
        }
        x.end();
    }

    /// Write page item attributes from the item's attribute list.
    fn item_attrs(&self, x: &mut Xml, attrs: &Attrs) {
        for (name, text) in self.item_attr_values(attrs) {
            x.attr(name, text);
        }
    }

    /// Page item attributes from an attribute list, as IDML values.
    fn item_attr_values(&self, attrs: &Attrs) -> Vec<(&'static str, String)> {
        let mut out = Vec::new();
        for &(id, name, kind) in ITEM_ATTRS {
            let Some(v) = attrs.get(id) else { continue };
            let text = match kind {
                AttrKind::Number => v.as_f64().map(num),
                AttrKind::Swatch => v.as_ref().and_then(|u| self.doc.swatches.get(&u).cloned()),
                AttrKind::Point => match v {
                    Value::Point(x, y) => Some(nums(&[*x, *y])),
                    _ => None,
                },
                AttrKind::Enum(map) => v
                    .as_u32()
                    .and_then(|u| map.iter().find(|(k, _)| *k == u))
                    .map(|(_, n)| n.to_string()),
                AttrKind::Builtin(prefix, map) => match v {
                    Value::RefOrCode(0, code) => map
                        .iter()
                        .find(|(k, _)| k == code)
                        .map(|(_, n)| format!("{prefix}{n}")),
                    _ => None,
                },
            };
            match text {
                Some(t) => out.push((name, t)),
                None if matches!(kind, AttrKind::Enum(_) | AttrKind::Builtin(..)) => {
                    unknown_code(attrs, id, v)
                }
                None => {}
            }
        }
        out
    }

    /// `CellStyle/...` or `TableStyle/...` reference of a style.
    fn table_style_ref(tag: &str, s: &crate::model::TableStyle) -> String {
        format!("{tag}/{}", self_name(&Self::table_style_name(s)))
    }

    fn table_style_name(s: &crate::model::TableStyle) -> String {
        if s.builtin {
            format!("$ID/{}", s.name)
        } else {
            s.name.clone()
        }
    }

    /// The reference for an applied cell style UID; 0 is `[None]`.
    fn cell_style_ref(&self, uid: u32) -> Option<String> {
        if uid == 0 {
            return Some("CellStyle/$ID/[None]".into());
        }
        self.doc
            .cell_styles
            .get(&uid)
            .map(|s| Self::table_style_ref("CellStyle", s))
    }

    /// IDML attributes of a table or table style attribute list.
    fn table_attrs(&self, attrs: &Attrs) -> Vec<(&'static str, String)> {
        TABLE_ATTRS
            .iter()
            .filter_map(|&(id, name, kind)| {
                let v = attrs.get(id)?;
                let t = match kind {
                    CellKind::CellStyle => match v.as_u32()? {
                        0 => Some("n".to_string()),
                        u => self
                            .doc
                            .cell_styles
                            .get(&u)
                            .map(|s| Self::table_style_ref("CellStyle", s)),
                    },
                    _ => self.table_value(v, kind),
                };
                if t.is_none() && matches!(kind, CellKind::Enum(_) | CellKind::StrokeType) {
                    unknown_code(attrs, id, v);
                }
                Some((name, t?))
            })
            .collect()
    }

    /// IDML attributes of a cell attribute set.
    fn cell_attrs(&self, attrs: &Attrs) -> Vec<(&'static str, String)> {
        let mut out = Vec::new();
        for &(id, names, kind) in CELL_ATTRS {
            let Some(v) = attrs.get(id) else { continue };
            match self.table_value(v, kind) {
                Some(t) => {
                    for &name in names {
                        out.push((name, t.clone()));
                    }
                }
                None if matches!(kind, CellKind::Enum(_) | CellKind::StrokeType) => {
                    unknown_code(attrs, id, v)
                }
                None => {}
            }
        }
        out
    }

    fn table_value(&self, v: &Value, kind: CellKind) -> Option<String> {
        {
            match kind {
                CellKind::Number => v.as_f64().map(num),
                CellKind::Integer => v.as_u32().map(|u| u.to_string()),
                CellKind::Swatch => v.as_ref().and_then(|u| self.doc.swatches.get(&u).cloned()),
                CellKind::Enum(map) => v
                    .as_u32()
                    .and_then(|u| map.iter().find(|(k, _)| *k == u))
                    .map(|(_, n)| n.to_string()),
                CellKind::StrokeType => {
                    let b = raw_bytes(v);
                    let word = |i: usize| u32_at(&b, i);
                    match (word(0), word(4)) {
                        (Some(CELL_NO_STROKE_TYPE), Some(0)) => Some("n".to_string()),
                        (Some(code), Some(0)) => STROKE_TYPES
                            .iter()
                            .find(|(k, _)| *k == code)
                            .map(|(_, n)| format!("StrokeStyle/$ID/{n}")),
                        _ => None,
                    }
                }
                CellKind::CellStyle => v.as_u32().and_then(|u| self.cell_style_ref(u)),
            }
        }
    }

    /// Font families and their fonts. See docs/format/fonts.md.
    fn fonts(&self) -> String {
        let mut x = Xml::new();
        self.package_root(&mut x, "Fonts");
        for f in self.doc.fonts.values() {
            let id = format!("di{:x}", f.uid);
            x.start("FontFamily")
                .attr("Self", &id)
                .attr("Name", &f.name);
            for font in &f.fonts {
                let name = format!("{} {}", f.name, font.style);
                x.start("Font")
                    .attr("Self", format!("{id}Fontn{name}"))
                    .attr("FontFamily", &f.name)
                    .attr("Name", &name)
                    .attr("PostScriptName", &font.postscript_name)
                    .attr("FontStyleName", &font.style);
                if let Some(t) = font.type_name() {
                    x.attr("FontType", t);
                }
                // The schema's `xsd:int`; one file stores 0xFFFFFFFF (fonts.md).
                if let Ok(w) = i32::try_from(f.writing_script) {
                    x.attr("WritingScript", w.to_string());
                }
                x.attr("FullName", &font.full_name)
                    .attr("FullNameNative", &font.full_name_native)
                    .attr("FontStyleNameNative", &font.style_native)
                    // `$ID/` in every Font of the corpus IDML files.
                    .attr("PlatformName", "$ID/")
                    .attr("Version", &font.version);
                // IDML from InDesign 7 has no TypekitID.
                if self.doc.version.major >= 12 {
                    x.attr("TypekitID", &font.typekit_id);
                }
                x.end();
            }
            x.end();
        }
        // Only the built-in composite font occurs in the corpus.
        for cf in self
            .doc
            .composite_fonts
            .iter()
            .filter(|c| c.name == "$ID/[No composite font]")
        {
            x.start("CompositeFont")
                .attr("Self", format!("CompositeFont/{}", self_name(&cf.name)))
                .attr("Name", &cf.name);
            for e in &cf.entries {
                // The four numbers are the same in every sample.
                let usual = e.numbers == [100.0, 0.0, 100.0, 100.0];
                x.start("CompositeFontEntry")
                    .attr("Self", uref(Some(e.uid)))
                    .attr("Name", &e.name)
                    .attr("FontStyle", &e.font_style);
                if usual {
                    x.attr("RelativeSize", "100")
                        .attr("HorizontalScale", "100")
                        .attr("VerticalScale", "100");
                }
                // IDML gives no characters for the Kanji entry.
                if e.name != "$ID/Kanji" {
                    x.attr("CustomCharacters", e.characters());
                }
                // In every CompositeFontEntry of the corpus IDML files.
                x.attr("Locked", "true");
                match e.scale {
                    [1, 1, 1, 1] => {
                        x.attr("ScaleOption", "true");
                    }
                    [0, 0, 0, 0] => {
                        x.attr("ScaleOption", "false");
                    }
                    _ => {}
                }
                if usual {
                    x.attr("BaselineShift", "0");
                }
                if let Some(f) = self.doc.fonts.get(&e.font_family) {
                    Self::properties(&mut x, &[("AppliedFont", "string", f.name.clone().into())]);
                }
                x.end();
            }
            x.end();
        }
        x.end();
        x.finish()
    }

    fn preferences(&self) -> String {
        let mut x = Xml::new();
        self.package_root(&mut x, "Preferences");
        // Values every exported IDML has (docs/format/idml-values.md);
        // values read from the INDD take precedence.
        let mut nodes = values::preferences(self.doc.version.major);
        // Observed values the INDD contradicts but the converter cannot
        // replace: (element, attribute).
        let mut drop: Vec<(&str, &str)> = Vec::new();
        let mut set = |tag: &str, ours: Vec<(&str, String)>| {
            let i = match nodes.iter().position(|n| n.tag == tag) {
                Some(i) => i,
                None => {
                    nodes.push(Node {
                        tag: tag.to_string(),
                        ..Node::default()
                    });
                    nodes.len() - 1
                }
            };
            let node = &mut nodes[i];
            node.attrs
                .retain(|(k, _)| !ours.iter().any(|(o, _)| o == k));
            let rest = std::mem::take(&mut node.attrs);
            node.attrs = ours.into_iter().map(|(k, v)| (k.to_string(), v)).collect();
            node.attrs.extend(rest);
        };
        if let Some(p) = &self.doc.preferences {
            const INTENT: [&str; 3] = ["PrintIntent", "WebIntent", "MobileIntent"];
            let [top, bottom, inside, outside] = p.bleed;
            let mut ours = vec![
                ("PageHeight", num(p.page_height)),
                ("PageWidth", num(p.page_width)),
                ("FacingPages", p.facing_pages.to_string()),
                ("DocumentBleedTopOffset", num(top)),
                ("DocumentBleedBottomOffset", num(bottom)),
                ("DocumentBleedInsideOrLeftOffset", num(inside)),
                ("DocumentBleedOutsideOrRightOffset", num(outside)),
            ];
            if let Some(i) = INTENT.get(p.intent as usize) {
                ours.push(("Intent", i.to_string()));
            }
            // Every sample's IDML has `LeftToRight`; a code the samples do
            // not show leaves the attribute out.
            match p.page_binding {
                0 => ours.push(("PageBinding", "LeftToRight".into())),
                1 => ours.push(("PageBinding", "RightToLeft".into())),
                _ => drop.push(("DocumentPreference", "PageBinding")),
            }
            set("DocumentPreference", ours);
        }
        // Guide locations are written measured from the spread (see
        // `guide`), so the ruler origin is stated rather than read.
        set(
            "ViewPreference",
            vec![("RulerOrigin", "SpreadOrigin".into())],
        );
        for (tag, attr) in drop {
            if let Some(n) = nodes.iter_mut().find(|n| n.tag == tag) {
                n.attrs.retain(|(k, _)| k != attr);
            }
        }
        for n in &nodes {
            n.write(&mut x);
        }
        x.end();
        x.finish()
    }

    fn styles(&self) -> String {
        let doc = self.doc;
        let mut x = Xml::new();
        self.package_root(&mut x, "Styles");
        let root_of = |kind: u32| doc.style_groups.values().find(|g| g.root == Some(kind));
        for paragraph in [false, true] {
            let (group_tag, tag, sub_tag) = if paragraph {
                (
                    "RootParagraphStyleGroup",
                    "ParagraphStyle",
                    "ParagraphStyleGroup",
                )
            } else {
                (
                    "RootCharacterStyleGroup",
                    "CharacterStyle",
                    "CharacterStyleGroup",
                )
            };
            let root = root_of(if paragraph {
                root_kind::PARAGRAPH
            } else {
                root_kind::CHARACTER
            });
            x.start(group_tag).attr(
                "Self",
                root.map_or(group_tag.to_string(), |g| uref(Some(g.uid))),
            );
            let mut written = std::collections::HashSet::new();
            if let Some(root) = root {
                self.style_group_children(&mut x, root, tag, sub_tag, &mut written);
            }
            // Styles not listed in a group (such as a built-in root
            // style). Some documents have two style objects with the same
            // name, which would give two elements with the same `Self`;
            // the first is written.
            let mut refs: std::collections::HashSet<String> = written
                .iter()
                .map(|&u| self.style_ref(Some(u), paragraph))
                .collect();
            for s in doc.styles.values().filter(|s| s.paragraph == paragraph) {
                if !written.contains(&s.uid) && refs.insert(self.style_ref(Some(s.uid), paragraph))
                {
                    self.style_element(&mut x, s, tag);
                }
            }
            x.end();
        }
        for (tag, kind, style, name) in [
            ("RootCellStyleGroup", root_kind::CELL, "CellStyle", "[None]"),
            (
                "RootTableStyleGroup",
                root_kind::TABLE,
                "TableStyle",
                "[No table style]",
            ),
        ] {
            let root = root_of(kind);
            let id = root.map_or(tag.to_string(), |g| uref(Some(g.uid)));
            x.start(tag).attr("Self", id);
            self.root_table_style(&mut x, style, name);
            let styles = if style == "CellStyle" {
                &doc.cell_styles
            } else {
                &doc.table_styles
            };
            // Styles listed in the root group first, then any others; the
            // root style is written above.
            let mut order: Vec<u32> = root.map_or(Vec::new(), |g| g.children.clone());
            order.extend(styles.keys().copied());
            let mut seen = std::collections::HashSet::new();
            for uid in order {
                let Some(s) = styles.get(&uid) else { continue };
                let is_root = s.builtin && s.based_on.is_none() && s.name == name;
                if is_root || !seen.insert(Self::table_style_ref(style, s)) {
                    continue;
                }
                self.table_style_element(&mut x, style, s, styles);
            }
            x.end();
        }
        let object_root = root_of(root_kind::OBJECT);
        x.start("RootObjectStyleGroup").attr(
            "Self",
            object_root.map_or("RootObjectStyleGroup".to_string(), |g| uref(Some(g.uid))),
        );
        for os in doc.object_styles.values() {
            let name = if os.builtin {
                format!("$ID/{}", os.name)
            } else {
                os.name.clone()
            };
            let root = os.builtin && os.name == "[None]" && os.based_on.is_none();
            let node = self.object_style_node(os, root);
            x.start("ObjectStyle")
                .attr("Self", format!("ObjectStyle/{}", self_name(&name)))
                .attr("Name", &name);
            for (k, v) in &node.attrs {
                x.attr(k, v);
            }
            let mut props: Vec<Node> = node
                .child("Properties")
                .map_or(Vec::new(), |p| p.children.clone());
            if !root && let Some(base) = os.based_on.and_then(|b| doc.object_styles.get(&b)) {
                let base_name = if base.builtin {
                    format!("$ID/{}", base.name)
                } else {
                    base.name.clone()
                };
                // The root "[None]" is written as a string.
                let (ty, text) = if base.builtin && base.name == "[None]" {
                    ("string", base_name)
                } else {
                    ("object", format!("ObjectStyle/{}", self_name(&base_name)))
                };
                props.insert(
                    0,
                    Node {
                        tag: "BasedOn".into(),
                        attrs: vec![("type".into(), ty.into())],
                        text: Some(text),
                        children: Vec::new(),
                    },
                );
            }
            if !props.is_empty() {
                x.start("Properties");
                for p in &props {
                    p.write(&mut x);
                }
                x.end();
            }
            for c in node.children.iter().filter(|c| c.tag != "Properties") {
                c.write(&mut x);
            }
            x.end();
        }
        x.end();
        // The trap presets every IDML has (idml-values.md).
        for n in values::list("TrapPreset", self.doc.version.major) {
            n.write(&mut x);
        }
        x.end();
        x.finish()
    }

    /// An object style as a values node: the values every exported IDML
    /// has on such a style (`idml-values.md`), with the values read from
    /// the INDD in their place. See `docs/format/objects.md`.
    fn object_style_node(&self, os: &crate::model::ObjectStyle, root: bool) -> Node {
        let major = self.doc.version.major;
        let mut node = if root {
            values::root_style("ObjectStyle", major)
        } else {
            values::object_style(major)
        };
        let mut attrs = self.item_attr_values(&os.attrs);
        for (id, name) in [
            (0x551E, "GradientFillAngle"),
            (0x5524, "GradientStrokeAngle"),
        ] {
            if let Some(v) = os.attrs.get(id).and_then(Value::as_f64) {
                attrs.push((name, num(v)));
            }
        }
        match os.attrs.get(0x6E6F).and_then(Value::as_u32) {
            Some(0) => attrs.push(("CornerOption", "None".into())),
            Some(0x5A16) => attrs.push(("CornerOption", "InverseRoundedCorner".into())),
            _ => {}
        }
        match os.paragraph_style {
            Some(0) => attrs.push(("AppliedParagraphStyle", "n".into())),
            Some(p) if self.doc.styles.contains_key(&p) => {
                attrs.push(("AppliedParagraphStyle", self.style_ref(Some(p), true)))
            }
            _ => {}
        }
        // Per-corner values: the two IDs of each pair are equal in every
        // sample, so the values are written only when all four agree.
        let corners = |ids: [u32; 4]| {
            let v: Vec<_> = ids.iter().map(|&id| os.attrs.get(id)).collect();
            (v[0].is_some() && v.iter().all(|x| *x == v[0])).then(|| v[0].cloned())?
        };
        if let Some(r) = corners([0x6E70, 0x6E94, 0x6E92, 0x6E93]).and_then(|v| v.as_f64()) {
            for name in [
                "TopLeftCornerRadius",
                "TopRightCornerRadius",
                "BottomLeftCornerRadius",
                "BottomRightCornerRadius",
            ] {
                attrs.push((name, num(r)));
            }
        }
        let corner_option = match corners([0x6E6F, 0x6E91, 0x6E8F, 0x6E90]).and_then(|v| v.as_u32())
        {
            Some(0) => Some("None"),
            Some(0x5A15) => Some("RoundedCorner"),
            Some(0x5A16) => Some("InverseRoundedCorner"),
            _ => None,
        };
        if let Some(o) = corner_option {
            for name in [
                "TopLeftCornerOption",
                "TopRightCornerOption",
                "BottomLeftCornerOption",
                "BottomRightCornerOption",
            ] {
                attrs.push((name, o.into()));
            }
        }
        if let Some(on) = &os.enabled {
            // Pairs of IDs that are both present or both absent in every
            // sample; either one gives the attribute.
            for (ids, names) in [
                ([0x1B933, 0x1B934], &["EnableFill", "EnableStroke"][..]),
                (
                    [0xADC8, 0x1B93E],
                    &[
                        "EnableTextFrameGeneralOptions",
                        "EnableTextFrameBaselineOptions",
                    ][..],
                ),
                ([0xADC9, 0xADCA], &["EnableTextFrameAutoSizingOptions"][..]),
            ] {
                let (a, b) = (on.contains(&ids[0]), on.contains(&ids[1]));
                if a == b {
                    for name in names {
                        attrs.push((name, a.to_string()));
                    }
                }
            }
            for (id, name) in [
                (0x1B940, "EnableStoryOptions"),
                (0x1B960, "EnableFrameFittingOptions"),
                (0xADCB, "EnableTextFrameColumnRuleOptions"),
            ] {
                attrs.push((name, on.contains(&id).to_string()));
            }
        }
        node.set(&[], attrs);
        if let Some(d) = &os.frame {
            let f = |o: usize| {
                (d.len() >= o + 8)
                    .then(|| Cursor::new(&d[o..]).f64().ok())
                    .flatten()
            };
            let u = |o: usize| {
                (d.len() >= o + 4)
                    .then(|| Cursor::new(&d[o..]).u32().ok())
                    .flatten()
            };
            let h = |o: usize| {
                (d.len() >= o + 2)
                    .then(|| Cursor::new(&d[o..]).u16().ok())
                    .flatten()
            };
            let mut tf = Vec::new();
            if let Some(n) = u(66) {
                tf.push(("TextColumnCount", n.to_string()));
            }
            if let Some(v) = f(8) {
                tf.push(("TextColumnGutter", num(v)));
            }
            if let Some(v) = f(0) {
                tf.push(("TextColumnFixedWidth", num(v)));
            }
            let mut footnote = Vec::new();
            if let (Some(span), Some(min), Some(between)) = (h(144), f(146), f(154))
                && span <= 1
            {
                let span = (span == 1).to_string();
                tf.push(("FootnotesSpanAcrossColumns", span.clone()));
                tf.push(("FootnotesMinimumSpacing", num(min)));
                tf.push(("FootnotesSpaceBetween", num(between)));
                footnote = vec![
                    ("SpanFootnotesAcross", span),
                    ("MinimumSpacingOption", num(min)),
                    ("SpaceBetweenFootnotes", num(between)),
                ];
            }
            if let (Some(width), Some(color), Some(tint)) = (f(190), u(198), f(210)) {
                let color = match color {
                    0 => Some("n".to_string()),
                    c => self.doc.swatches.get(&c).cloned(),
                };
                tf.push(("ColumnRuleStrokeWidth", num(width)));
                if let Some(c) = color {
                    tf.push(("ColumnRuleStrokeColor", c));
                }
                tf.push(("ColumnRuleStrokeTint", num(tint)));
            }
            node.set(&["TextFramePreference"], tf);
            // Only the top inset is shown apart from the others; the list is
            // written when all four are equal.
            if let (Some(a), Some(b), Some(c), Some(e)) = (f(34), f(42), f(50), f(58))
                && a == b
                && b == c
                && c == e
            {
                node.set(&["TextFramePreference", "Properties"], Vec::new());
                let props = node
                    .children
                    .iter_mut()
                    .find(|c| c.tag == "TextFramePreference")
                    .and_then(|t| t.children.iter_mut().find(|c| c.tag == "Properties"))
                    .expect("set above");
                props.children.retain(|c| c.tag != "InsetSpacing");
                props.children.push(Node {
                    tag: "InsetSpacing".into(),
                    attrs: vec![("type".into(), "list".into())],
                    text: None,
                    children: (0..4)
                        .map(|_| Node {
                            tag: "ListItem".into(),
                            attrs: vec![("type".into(), "unit".into())],
                            text: Some(num(a)),
                            children: Vec::new(),
                        })
                        .collect(),
                });
            }
            if !footnote.is_empty() && node.child("TextFrameFootnoteOptionsObject").is_some() {
                node.set(&["TextFrameFootnoteOptionsObject"], footnote);
            }
        }
        let mut story = Vec::new();
        if let Some(d) = &os.story
            && d.len() >= 16
        {
            let h = |o: usize| u16_from([d[o], d[o + 1]]);
            match h(14) {
                0 => story.push(("FrameType", "Unknown".to_string())),
                1 => story.push(("FrameType", "TextFrameType".to_string())),
                2 => story.push(("FrameType", "FrameGridType".to_string())),
                _ => {}
            }
            match h(0) {
                0 => story.push(("StoryOrientation", "Unknown".to_string())),
                1 => story.push(("StoryOrientation", "Horizontal".to_string())),
                _ => {}
            }
        }
        match os.direction {
            Some(1) => story.push(("StoryDirection", "LeftToRightDirection".into())),
            Some(0) | None => story.push(("StoryDirection", "UnknownDirection".into())),
            _ => {}
        }
        node.set(&["StoryPreference"], story);
        let mode = match os.text_wrap.as_ref().map(|w| w.mode) {
            None | Some(wrap_mode::NONE) => Some("None"),
            Some(wrap_mode::JUMP_OBJECT) => Some("JumpObjectTextWrap"),
            Some(wrap_mode::BOUNDING_BOX) => Some("BoundingBoxTextWrap"),
            Some(wrap_mode::CONTOUR) => Some("Contour"),
            Some(_) => None,
        };
        if let Some(mode) = mode {
            node.set(&["TextWrapPreference"], vec![("TextWrapMode", mode.into())]);
            let [left, top, right, bottom] = os.text_wrap.as_ref().map_or([0.0; 4], |w| w.offsets);
            node.set(
                &["TextWrapPreference", "Properties", "TextWrapOffset"],
                vec![
                    ("Top", num(top)),
                    ("Left", num(left)),
                    ("Bottom", num(bottom)),
                    ("Right", num(right)),
                ],
            );
            if os.contour_type == Some(5) {
                node.set(
                    &["TextWrapPreference", "ContourOption"],
                    vec![("ContourType", "SameAsClipping".into())],
                );
            }
        }
        let all: Vec<u32> = FITTING_ATTRS.iter().map(|(id, _)| *id).collect();
        node.set(&["FrameFittingOption"], fitting_attrs(&os.fitting, &all));
        if let Some(d) = &os.anchor {
            node.set(&["AnchoredObjectSetting"], anchored_settings(d));
        }
        node
    }

    fn style_group_children(
        &self,
        x: &mut Xml,
        g: &StyleGroup,
        tag: &str,
        sub_tag: &str,
        written: &mut std::collections::HashSet<u32>,
    ) {
        for &c in &g.children {
            if let Some(sub) = self.doc.style_groups.get(&c) {
                let path = self.group_path.get(&sub.uid).cloned().unwrap_or_default();
                x.start(sub_tag)
                    .attr(
                        "Self",
                        format!("{sub_tag}/$ID/{}", self_name(&path.join(":"))),
                    )
                    .attr("Name", format!("$ID/{}", sub.name));
                self.style_group_children(x, sub, tag, sub_tag, written);
                x.end();
            } else if let Some(s) = self.doc.styles.get(&c) {
                self.style_element(x, s, tag);
                written.insert(c);
            }
        }
    }

    fn style_element(&self, x: &mut Xml, s: &Style, tag: &str) {
        let doc = self.doc;
        let paragraph = s.paragraph;
        let name = style_name(s);
        let (mut plain, mut props) = self.text_attrs(&s.attrs);
        if paragraph {
            // The schema allows KerningValue on character styles only.
            plain.retain(|(k, _)| *k != "KerningValue");
        }
        x.start(tag)
            .attr("Self", self.style_ref(Some(s.uid), paragraph))
            .attr("Name", &name);
        if paragraph {
            x.attr("NextStyle", self.style_ref(s.next.or(Some(s.uid)), true));
        }
        for (k, v) in &plain {
            x.attr(k, v);
        }
        x.attr("Imported", s.imported.to_string());
        if let Some(id) = &s.unique_id {
            x.attr("StyleUniqueId", id);
        }
        // Root styles also get the values every exported IDML has on them;
        // values read from the INDD take precedence.
        let mut extra = Vec::new();
        if s.builtin && s.based_on.is_none() && s.name.starts_with("[No ") {
            let written: Vec<&str> = plain.iter().map(|(k, _)| *k).collect();
            let (attrs, more, _) = self.root_values(tag, &written, &props);
            for (k, v) in &attrs {
                if !x.has_attr(k) {
                    x.attr(k, v);
                }
            }
            extra = more;
        } else if let Some(n) = values::element(tag, doc.version.major) {
            // Other styles get the values every IDML has on them.
            x.attrs_missing(n.attrs.iter());
            x.attrs_missing(values::when_written(tag, doc.version.major).iter());
            if let Some(p) = n.child("Properties") {
                extra = p.children.clone();
            }
        }
        if let Some(base) = s.based_on.and_then(|b| doc.styles.get(&b)) {
            // The root "[No ... style]" is written as a string.
            let root = base.builtin && base.name.starts_with("[No ");
            if root {
                props.insert(0, ("BasedOn", "string", style_name(base).into()));
            } else {
                props.insert(
                    0,
                    (
                        "BasedOn",
                        "object",
                        self.style_ref(Some(base.uid), paragraph).into(),
                    ),
                );
            }
        }
        Self::properties_with(x, &props, &extra);
        x.end();
    }

    fn path_geometry(x: &mut Xml, paths: &[Path]) {
        if paths.is_empty() {
            return;
        }
        x.start("Properties").start("PathGeometry");
        for p in paths {
            x.start("GeometryPathType")
                .attr("PathOpen", p.open.to_string());
            x.start("PathPointArray");
            for pt in &p.points {
                x.empty(
                    "PathPointType",
                    &[
                        ("Anchor", nums(&[pt.anchor.0, pt.anchor.1])),
                        ("LeftDirection", nums(&[pt.left.0, pt.left.1])),
                        ("RightDirection", nums(&[pt.right.0, pt.right.1])),
                    ],
                );
            }
            x.end().end();
        }
        x.end().end();
    }

    fn text_frame_preference(
        x: &mut Xml,
        p: &TextFramePreferences,
        major: u32,
        swatch: impl Fn(u32) -> Option<String>,
    ) {
        const POINTS: [&str; 9] = [
            "TopLeftPoint",
            "TopCenterPoint",
            "TopRightPoint",
            "LeftCenterPoint",
            "CenterPoint",
            "RightCenterPoint",
            "BottomLeftPoint",
            "BottomCenterPoint",
            "BottomRightPoint",
        ];
        const JUSTIFY: [&str; 4] = ["TopAlign", "CenterAlign", "BottomAlign", "JustifyAlign"];
        const FIRST_BASELINE: [&str; 4] =
            ["LeadingOffset", "AscentOffset", "CapHeight", "EmboxHeight"];
        const SIZING: [&str; 5] = [
            "Off",
            "HeightOnly",
            "WidthOnly",
            "HeightAndWidth",
            "HeightAndWidthProportionally",
        ];
        x.start("TextFramePreference")
            .attr("TextColumnCount", p.column_count.to_string())
            .attr("TextColumnGutter", num(p.column_gutter))
            .attr("TextColumnFixedWidth", num(p.column_fixed_width));
        if let Some(v) = FIRST_BASELINE.get(p.first_baseline_offset as usize) {
            x.attr("FirstBaselineOffset", *v);
        }
        if let Some(v) = JUSTIFY.get(p.vertical_justification as usize) {
            x.attr("VerticalJustification", *v);
        }
        x.attr(
            "VerticalBalanceColumns",
            p.vertical_balance_columns.to_string(),
        );
        if let Some(v) = SIZING.get(p.auto_sizing_type as usize) {
            x.attr("AutoSizingType", *v);
        }
        if let Some(v) = POINTS.get(p.auto_sizing_reference_point as usize) {
            x.attr("AutoSizingReferencePoint", *v);
        }
        if let Some(w) = p.max_width {
            x.attr("TextColumnMaxWidth", num(w));
        }
        x.attr("UseFixedColumnWidth", p.use_fixed_width.to_string());
        if let Some(([use_height, use_width], [height, width], no_breaks)) = p.minimum_sizes {
            x.attr("UseMinimumHeightForAutoSizing", use_height.to_string())
                .attr("MinimumHeightForAutoSizing", num(height))
                .attr("UseMinimumWidthForAutoSizing", use_width.to_string())
                .attr("MinimumWidthForAutoSizing", num(width))
                .attr("UseNoLineBreaksForAutoSizing", no_breaks.to_string());
        }
        if let Some(v) = p.ignore_wrap {
            x.attr("IgnoreWrap", v.to_string());
        }
        // Values every IDML of the version has where it writes them; they
        // also say from which version the column rule and footnote
        // settings exist (idml-values.md).
        let written = values::when_written("TextFrame/TextFramePreference", major);
        let has = |k: &str| written.iter().any(|(n, _)| n == k);
        if has("ColumnRuleOffset") {
            let (width, color) = match p.column_rule {
                Some((w, c)) => (w, swatch(c)),
                // Without the chunk (objects.md).
                None => (1.0, Some("Color/Black".to_string())),
            };
            x.attr("ColumnRuleStrokeWidth", num(width));
            if let Some(c) = color {
                x.attr("ColumnRuleStrokeColor", c);
            }
            if p.column_rule_override != Some(true) {
                x.attr("ColumnRuleOverride", "false");
            }
        }
        if has("FootnotesEnableOverrides") {
            let [spacing, between] = p.footnotes.unwrap_or([12.0, 6.0]);
            x.attr("FootnotesMinimumSpacing", num(spacing))
                .attr("FootnotesSpaceBetween", num(between));
        }
        x.attrs_missing(written.iter());
        if major >= 11 {
            let [top, left, bottom, right] = p.inset;
            let item = |v: f64| Node {
                tag: "ListItem".into(),
                attrs: vec![("type".into(), "unit".into())],
                text: Some(num(v)),
                children: Vec::new(),
            };
            Self::properties_with(
                x,
                &[],
                &[Node {
                    tag: "InsetSpacing".into(),
                    attrs: vec![("type".into(), "list".into())],
                    text: None,
                    children: vec![item(top), item(left), item(bottom), item(right)],
                }],
            );
        }
        x.end();
    }

    /// `TextWrapPreference` from an item's text wrap chunk. An item without
    /// the chunk has no wrap. Values not identified yet are left out: the
    /// whole element for an unknown mode, and the side and inverse settings
    /// for flags other than 1.
    fn text_wrap_preference(x: &mut Xml, wrap: Option<&TextWrap>, contour_type: Option<u32>) {
        let mode = match wrap.map(|w| w.mode) {
            None | Some(wrap_mode::NONE) => "None",
            Some(wrap_mode::JUMP_OBJECT) => "JumpObjectTextWrap",
            Some(wrap_mode::BOUNDING_BOX) => "BoundingBoxTextWrap",
            Some(wrap_mode::CONTOUR) => "Contour",
            Some(_) => return,
        };
        x.start("TextWrapPreference");
        if wrap.is_none_or(|w| w.flags == 1) {
            x.attr("Inverse", "false")
                .attr("ApplyToMasterPageOnly", "false")
                .attr("TextWrapSide", "BothSides");
        }
        x.attr("TextWrapMode", mode);
        let [left, top, right, bottom] = wrap.map_or([0.0; 4], |w| w.offsets);
        x.start("Properties").empty(
            "TextWrapOffset",
            &[
                ("Top", num(top)),
                ("Left", num(left)),
                ("Bottom", num(bottom)),
                ("Right", num(right)),
            ],
        );
        x.end();
        if contour_type == Some(5) {
            x.empty("ContourOption", &[("ContourType", "SameAsClipping".into())]);
        }
        x.end();
    }

    /// `ClippingPathSettings` of an image, PDF or EPS graphic, and
    /// `ImageIOPreference` of an image. Values without an INDD field are
    /// those every exported IDML has, and a graphic without chunk 0x2C1A
    /// has the values every such graphic has in IDML. See
    /// `docs/format/objects.md`.
    fn clipping(x: &mut Xml, g: &Graphic) {
        let (high_resolution, threshold, tolerance, inset, index) = match &g.clipping {
            Some(c) if c.kind == 0 => (
                match c.high_resolution {
                    2 => Some("true"),
                    0 => Some("false"),
                    _ => None,
                },
                c.threshold.to_string(),
                num(c.tolerance),
                num(c.inset),
                c.index.to_string(),
            ),
            Some(_) => return,
            None => (
                Some("true"),
                "25".into(),
                "2".into(),
                "0".into(),
                "-1".into(),
            ),
        };
        x.start("ClippingPathSettings")
            .attr("ClippingType", "None")
            .attr("InvertPath", "false")
            .attr("IncludeInsideEdges", "false")
            .attr("RestrictToFrame", "false");
        if let Some(h) = high_resolution {
            x.attr("UseHighResolutionImage", h);
        }
        x.attr("Threshold", threshold)
            .attr("Tolerance", tolerance)
            .attr("InsetFrame", inset)
            .attr("AppliedPathName", "$ID/")
            .attr("Index", index);
        x.end();
        if g.kind != GraphicKind::Image {
            return;
        }
        x.start("ImageIOPreference");
        match g.photoshop_clipping {
            Some(1) => {
                x.attr("ApplyPhotoshopClippingPath", "true");
            }
            Some(0) => {
                x.attr("ApplyPhotoshopClippingPath", "false");
            }
            _ => {}
        }
        x.attr("AllowAutoEmbedding", "true")
            .attr("AlphaChannelName", "$ID/");
        x.end();
    }

    fn placed_graphic(x: &mut Xml, g: &Graphic) {
        let tag = match g.kind {
            GraphicKind::Image => "Image",
            GraphicKind::Pdf => "PDF",
            GraphicKind::Eps => "EPS",
            GraphicKind::Svg => "SVG",
        };
        let [left, top, right, bottom] = g.bounds;
        x.start(tag)
            .attr("Self", uref(Some(g.uid)))
            .attr("ItemTransform", matrix(&g.transform));
        x.start("Properties");
        if let Some(data) = &g.contents {
            x.start("Contents")
                .cdata(&base64_lines(data), CDATA_SECTION)
                .end();
        }
        x.empty(
            "GraphicBounds",
            &[
                ("Left", num(left)),
                ("Top", num(top)),
                ("Right", num(right)),
                ("Bottom", num(bottom)),
            ],
        );
        x.end();
        if g.kind != GraphicKind::Svg {
            Self::clipping(x, g);
        }
        Self::text_wrap_preference(x, g.text_wrap.as_ref(), g.contour_type);
        if let Some(link) = &g.link {
            x.empty(
                "Link",
                &[
                    ("Self", uref(Some(link.uid))),
                    ("LinkResourceURI", link.uri.clone()),
                    (
                        "StoredState",
                        if link.embedded { "Embedded" } else { "Normal" }.into(),
                    ),
                ],
            );
        }
        x.end();
    }

    /// The gradient attributes of a page item: its own, or, for a group,
    /// the values all its children have. See `docs/format/objects.md`.
    fn gradients(item: &PageItem) -> Vec<(&'static str, String)> {
        if item.kind == ItemKind::Group {
            let kids: Vec<_> = item.children.iter().map(Self::gradients).collect();
            return GRADIENT_ATTRS
                .iter()
                .filter_map(|&(_, name, _)| {
                    let mut values = kids
                        .iter()
                        .map(|k| k.iter().find(|(n, _)| *n == name).map(|(_, v)| v));
                    let first = values.next()??;
                    values
                        .all(|v| v == Some(first))
                        .then(|| (name, first.clone()))
                })
                .collect();
        }
        GRADIENT_ATTRS
            .iter()
            .map(|&(id, name, default)| {
                let v = match item.attrs.get(id) {
                    Some(Value::Point(x, y)) => Some(nums(&[*x, *y])),
                    Some(v) => v.as_f64().map(num),
                    None => None,
                };
                (name, v.unwrap_or_else(|| default.to_string()))
            })
            .collect()
    }

    /// The name, visibility, lock and the other settings every page item
    /// has; `nested` for an item inside another page item, which IDML
    /// writes without `Locked`. See `docs/format/objects.md`.
    fn item_settings(&self, x: &mut Xml, item: &PageItem, nested: bool) {
        let p = &item.props;
        let name = match &p.name {
            Some(n) if n.builtin => format!("$ID/{}", n.name),
            Some(n) => n.name.clone(),
            None => "$ID/".into(),
        };
        x.attr("Name", name)
            .attr("Visible", (!p.hidden).to_string());
        if !nested {
            x.attr("Locked", p.locked.to_string());
        }
        if self.doc.version.major >= 8 {
            for (name, counts) in [
                "ParentInterfaceChangeCount",
                "TargetInterfaceChangeCount",
                "LastUpdatedInterfaceChangeCount",
            ]
            .into_iter()
            .zip(&p.change_counts)
            {
                x.attr(name, join_numbers(counts));
            }
            let overridden = match &p.overridden {
                Some((master, ids)) if *master != 0 => join_numbers(ids),
                _ => String::new(),
            };
            x.attr("OverriddenPageItemProps", overridden);
            if let Some((h, v)) = layout_constraints(p.layout_constraints.unwrap_or(0x22)) {
                x.attr("HorizontalLayoutConstraints", h)
                    .attr("VerticalLayoutConstraints", v);
            }
        }
        for (name, v) in Self::gradients(item) {
            if !x.has_attr(name) {
                x.attr(name, v);
            }
        }
    }

    /// `nested` for an item inside another page item.
    fn page_item(&self, x: &mut Xml, item: &PageItem, nested: bool) {
        let tag = match &item.kind {
            ItemKind::TextFrame { .. } => "TextFrame",
            ItemKind::Group => "Group",
            ItemKind::Shape(Shape::Rectangle) => "Rectangle",
            ItemKind::Shape(Shape::Oval) => "Oval",
            ItemKind::Shape(Shape::Polygon) => "Polygon",
            ItemKind::Shape(Shape::GraphicLine) => "GraphicLine",
        };
        x.start(tag).attr("Self", uref(Some(item.uid)));
        if let ItemKind::TextFrame {
            story,
            previous,
            next,
            ..
        } = &item.kind
        {
            x.attr("ParentStory", uref(*story))
                .attr("PreviousTextFrame", uref(*previous))
                .attr("NextTextFrame", uref(*next))
                .attr("ContentType", "TextType");
        } else if let ItemKind::Shape(_) = item.kind {
            let content = if item.graphics.is_empty() {
                "Unassigned"
            } else {
                "GraphicType"
            };
            x.attr("ContentType", content);
        }
        self.item_attrs(x, &item.attrs);
        if let Some(os) = item
            .object_style
            .and_then(|u| self.doc.object_styles.get(&u))
        {
            let name = if os.builtin {
                format!("$ID/{}", os.name)
            } else {
                os.name.clone()
            };
            x.attr(
                "AppliedObjectStyle",
                format!("ObjectStyle/{}", self_name(&name)),
            );
        }
        if let Some(layer) = item.layer {
            x.attr("ItemLayer", uref(Some(layer)));
        }
        x.attr("ItemTransform", matrix(&item.transform));
        self.item_settings(x, item, nested);
        x.attrs_missing(self.observed(tag).iter());
        Self::path_geometry(x, &item.paths);
        if let ItemKind::TextFrame {
            preferences: Some(p),
            ..
        } = &item.kind
        {
            Self::text_frame_preference(x, p, self.doc.version.major, |u| match u {
                0 => Some("n".to_string()),
                u => self.doc.swatches.get(&u).cloned(),
            });
        }
        let frame = matches!(
            item.kind,
            ItemKind::Shape(Shape::Rectangle | Shape::Oval | Shape::Polygon)
        );
        if frame {
            self.frame_fitting(x, item);
        }
        // Export options every IDML of the version has on this kind of
        // item; from DOM 12 on, also those of the object styles
        // (idml-values.md).
        let major = self.doc.version.major;
        let mut export = values::element(&format!("{tag}/ObjectExportOption"), major);
        if major >= 12
            && let Some(n) = values::object_style(major).child("ObjectExportOption")
        {
            match &mut export {
                Some(e) => e.merge(n),
                None => export = Some(n.clone()),
            }
        }
        if let Some(n) = export {
            n.write(x);
        }
        if let Some(d) = &item.anchor {
            let style = item
                .object_style
                .and_then(|u| self.doc.object_styles.get(&u))
                .and_then(|s| s.anchor.as_deref());
            let attrs: Vec<_> = anchored_settings(d)
                .into_iter()
                .filter(|a| style.is_none_or(|s| !anchored_settings(s).contains(a)))
                .collect();
            if !attrs.is_empty() {
                x.empty("AnchoredObjectSetting", &attrs);
            }
        }
        Self::text_wrap_preference(x, item.text_wrap.as_ref(), None);
        if frame {
            x.empty(
                "InCopyExportOption",
                &[
                    ("IncludeGraphicProxies", "true".into()),
                    ("IncludeAllResources", "false".into()),
                ],
            );
        }
        for (effect, attr, v) in
            transparency::write(x, &item.attrs, &uref(Some(item.uid)), &self.doc.swatches)
        {
            self.warnings.borrow_mut().push(format!(
                "item {}: {effect} {attr} {v} is outside the IDML range; left out",
                item.uid
            ));
        }
        for child in &item.children {
            self.page_item(x, child, true);
        }
        for g in &item.graphics {
            Self::placed_graphic(x, g);
        }
        x.end();
    }

    /// `FrameFittingOption` of a rectangle, oval or polygon. IDML writes
    /// the local values that differ from the object style's; with none,
    /// all of the style's values unless the style is the root `[None]`.
    /// See `docs/format/objects.md`.
    fn frame_fitting(&self, x: &mut Xml, item: &PageItem) {
        let style = item
            .object_style
            .and_then(|u| self.doc.object_styles.get(&u));
        let differ: Vec<u32> = FITTING_ATTRS
            .iter()
            .map(|(id, _)| *id)
            .filter(|&id| {
                item.attrs
                    .get(id)
                    .is_some_and(|v| style.and_then(|s| s.fitting.get(id)) != Some(v))
            })
            .collect();
        let attrs = if !differ.is_empty() {
            fitting_attrs(&item.attrs, &differ)
        } else {
            match style {
                Some(s) if !(s.builtin && s.name == "[None]" && s.based_on.is_none()) => {
                    let all: Vec<u32> = FITTING_ATTRS.iter().map(|(id, _)| *id).collect();
                    fitting_attrs(&s.fitting, &all)
                }
                _ => Vec::new(),
            }
        };
        if !attrs.is_empty() {
            x.empty("FrameFittingOption", &attrs);
        }
    }

    /// A ruler guide. `origin` is the top-left corner of the spread's
    /// pages, from which IDML measures `Location` with the ruler origin
    /// `SpreadOrigin`. See `docs/format/objects.md`.
    /// `page_index` is IDML `PageIndex` (see `docs/format/objects.md`).
    fn guide(x: &mut Xml, g: &Guide, origin: (f64, f64), page_index: i64) {
        let (left, top) = origin;
        x.start("Guide")
            .attr("Self", uref(Some(g.uid)))
            .attr(
                "Orientation",
                if g.horizontal {
                    "Horizontal"
                } else {
                    "Vertical"
                },
            )
            .attr(
                "Location",
                num(g.position - if g.horizontal { top } else { left }),
            )
            .attr("FitToPage", g.fit_to_page.to_string());
        // The only stored value in the samples, with the only IDML value.
        if g.view_threshold == 0.05 {
            x.attr("ViewThreshold", "5");
        }
        if g.layer != 0 {
            x.attr("ItemLayer", uref(Some(g.layer)));
        }
        match g.guide_type {
            Some(0) => {
                x.attr("GuideType", "Ruler");
            }
            Some(1) => {
                x.attr("GuideType", "Liquid");
            }
            _ => {}
        }
        x.attr("Locked", g.locked.to_string())
            .attr("PageIndex", page_index.to_string());
        if let Some(z) = g.zone {
            x.attr("GuideZone", num(z));
        }
        if let Some((master, ids)) = &g.overridden {
            let ids = if *master == 0 {
                String::new()
            } else {
                join_numbers(ids)
            };
            x.attr("OverriddenPageItemProps", ids);
        } else if g.zone.is_some() {
            x.attr("OverriddenPageItemProps", "");
        }
        if g.color == 6 {
            Self::properties(
                x,
                &[("GuideColor", "enumeration", "Cyan".to_string().into())],
            );
        }
        x.end();
    }

    /// The settings of a page: its tab order, overridden master items,
    /// layout grid use, layout rule and colour, and the values every IDML
    /// of the version has. See `docs/format/objects.md`, page settings.
    fn page_settings(&self, x: &mut Xml, p: &Page, section: Option<&(Section, u32)>) {
        let st = &p.settings;
        let tab: Vec<String> = st.tab_order.iter().map(|&u| uref(Some(u))).collect();
        x.attr("TabOrder", tab.join(" "));
        let overrides: Vec<String> = st
            .overrides
            .iter()
            .flat_map(|&(item, with)| [uref(Some(item)), uref((with != 0).then_some(with))])
            .collect();
        x.attr("OverrideList", overrides.join(" "));
        if let Some(v) = st.use_master_grid {
            x.attr("UseMasterGrid", v.to_string());
        }
        if self.doc.version.major >= 8 {
            const RULES: [&str; 6] = [
                "",
                "Recenter",
                "ObjectBased",
                "Scale",
                "GuideBased",
                "UseMaster",
            ];
            match st.layout_rule {
                None => {
                    x.attr("LayoutRule", "Off");
                }
                Some(code) => {
                    if let Some(r) = RULES.get(code as usize).filter(|r| !r.is_empty()) {
                        x.attr("LayoutRule", *r);
                    }
                }
            }
        }
        x.attrs_missing(self.observed("Page").iter());
        let color = match st.color {
            UiColorRef::UseMaster => Some("UseMasterColor"),
            UiColorRef::Nothing => Some("Nothing"),
            _ => None,
        };
        let node = match (color, st.color) {
            (Some(name), _) => Some(Node {
                tag: "PageColor".into(),
                attrs: vec![("type".into(), "enumeration".into())],
                text: Some(name.into()),
                children: Vec::new(),
            }),
            (None, UiColorRef::Rgb(rgb)) => ui_color_property("PageColor", rgb),
            _ => None,
        };
        let mut props: Vec<Node> = node.into_iter().collect();
        // A document page describes its numbering: section prefix, style,
        // continue, include prefix, page number (from DOM 20 twice) and
        // marker.
        if let Some((s, number)) = section
            && let Some(style) = number_style(s.style)
        {
            let item = |ty: &str, text: String| Node {
                tag: "ListItem".into(),
                attrs: vec![("type".into(), ty.into())],
                text: Some(text),
                children: Vec::new(),
            };
            let mut items = vec![
                item("string", s.prefix.clone()),
                item("enumeration", style.into()),
                item("boolean", s.continue_numbering.to_string()),
                item("boolean", "false".into()),
                item("long", number.to_string()),
            ];
            if self.doc.version.major >= 20 {
                items.push(item("long", number.to_string()));
            }
            items.push(item("string", s.marker.clone()));
            props.push(Node {
                tag: "Descriptor".into(),
                attrs: vec![("type".into(), "list".into())],
                text: None,
                children: items,
            });
        }
        Self::properties_with(x, &[], &props);
    }

    /// `MarginPreference` and `GridDataInformation` of a page. Margins
    /// and columns are those in effect (see `resolve_page_layout`). The
    /// grid values that are the same in every sample are written only
    /// when the stored value is that one. See `docs/format/objects.md`.
    fn page_layout(&self, x: &mut Xml, p: &Page) {
        if let (Some(m), Some(c)) = (&p.margins, &p.columns) {
            x.empty(
                "MarginPreference",
                &[
                    ("ColumnCount", (c.positions.len() / 2).to_string()),
                    ("ColumnGutter", num(c.gutter)),
                    ("Top", num(m.top)),
                    ("Bottom", num(m.bottom)),
                    ("Left", num(m.left)),
                    ("Right", num(m.right)),
                    // Every exported IDML has this (idml-values.md).
                    ("ColumnDirection", "Horizontal".into()),
                    ("ColumnsPositions", nums(&c.positions)),
                ],
            );
        }
        let Some(g) = &p.grid else { return };
        x.start("GridDataInformation")
            .attr("FontStyle", &g.font_style);
        let [size, character_aki, line_aki, h_scale, v_scale] = g.numbers;
        for (name, value, observed, scale) in [
            ("PointSize", size, 12.0, 1.0),
            ("CharacterAki", character_aki, 0.0, 1.0),
            ("LineAki", line_aki, 9.0, 1.0),
            ("HorizontalScale", h_scale, 1.0, 100.0),
            ("VerticalScale", v_scale, 1.0, 100.0),
        ] {
            if value == observed {
                x.attr(name, num(value * scale));
            }
        }
        if g.codes == [3, 0, 3, 1] {
            x.attr("LineAlignment", "LeftOrTopLineJustify")
                .attr("GridAlignment", "AlignEmCenter")
                .attr("CharacterAlignment", "AlignEmCenter");
        }
        if let Some(f) = self.doc.fonts.get(&g.font) {
            Self::properties(x, &[("AppliedFont", "string", f.name.clone().into())]);
        }
        x.end();
    }

    /// `page_index` counts the document pages written so far; `names`
    /// gives each document page its name (see `page_names`).
    fn spread(&self, s: &Spread, master: bool, page_index: &mut usize, names: &[String]) -> String {
        let mut x = Xml::new();
        let kind = if master { "MasterSpread" } else { "Spread" };
        self.package_root(&mut x, kind);
        x.start(kind)
            .attr("Self", uref(Some(s.uid)))
            .attr("PageCount", s.pages.len().to_string());
        let origin = spread_origin(&s.pages);
        if !master {
            x.attr("BindingLocation", s.binding_location.to_string());
        }
        x.attr("ItemTransform", matrix(&s.transform));
        let (prefix, base) = s
            .master_name
            .clone()
            .unwrap_or_else(|| ("A".into(), "Master".into()));
        if master {
            x.attr("Name", format!("{prefix}-{base}"))
                .attr("NamePrefix", &prefix)
                .attr("BaseName", &base);
            match s.show_master_items {
                None => {
                    x.attr("ShowMasterItems", "true");
                }
                Some(0) => {
                    x.attr("ShowMasterItems", "false");
                }
                Some(_) => {}
            }
        } else {
            match s.shuffle {
                Some(0) => {
                    x.attr("AllowPageShuffle", "true");
                }
                Some(1) | None => {
                    x.attr("AllowPageShuffle", "false");
                }
                Some(_) => {}
            }
        }
        x.attrs_missing(self.observed(kind).iter());
        let major = self.doc.version.major;
        if !master && let Some(mut fp) = values::element("Spread/FlattenerPreference", major) {
            if let Some([line_art, gradient]) = s.flattener_resolution {
                fp.attrs
                    .insert(0, ("GradientAndMeshResolution".into(), num(gradient)));
                fp.attrs
                    .insert(0, ("LineArtAndTextResolution".into(), num(line_art)));
            }
            fp.write(&mut x);
        }
        for p in &s.pages {
            let name = if master {
                prefix.clone()
            } else {
                *page_index += 1;
                names
                    .get(*page_index - 1)
                    .cloned()
                    .unwrap_or_else(|| page_index.to_string())
            };
            // Master pages have none (`n`).
            let layout = if master {
                None
            } else {
                self.page_layouts.get(*page_index - 1).copied()
            };
            let [x0, y0, x1, y1] = p.bounds;
            x.start("Page")
                .attr("Self", uref(Some(p.uid)))
                .attr("Name", name)
                .attr("GeometricBounds", nums(&[y0, x0, y1, x1]))
                .attr("ItemTransform", matrix(&p.transform))
                // A master page can itself be based on a master.
                .attr("AppliedMaster", uref(p.master))
                .attr("MasterPageTransform", matrix(&p.master_transform));
            if self.doc.version.major >= 8 {
                x.attr("AppliedAlternateLayout", uref(layout));
            }
            let section = if master {
                None
            } else {
                self.page_sections.get(*page_index - 1)
            };
            self.page_settings(&mut x, p, section);
            // A guide belongs to a page, or to the spread; IDML writes a
            // spread's guides in its first page.
            let first = p.uid == s.pages[0].uid;
            // Pages count from the spine: left of it -1, -2, ..., right of
            // it 1, 2, ...; a master spread's spine is at its left edge.
            let position = s.pages.iter().position(|q| q.uid == p.uid).unwrap_or(0) as i64;
            let binding = if master { 0 } else { s.binding_location as i64 };
            let from_spine = if position < binding {
                position - binding
            } else {
                position - binding + 1
            };
            for g in &s.guides {
                let own_page = s.pages.iter().any(|q| q.uid == g.owner);
                if g.owner == p.uid || (first && !own_page) {
                    let index = if own_page { from_spine } else { 0 };
                    Self::guide(&mut x, g, origin, index);
                }
            }
            self.page_layout(&mut x, p);
            x.end();
        }
        for item in &s.items {
            self.page_item(&mut x, item, false);
        }
        x.end();
        x.finish()
    }

    /// The start tag of a `Story` or `XmlStory` with its attributes.
    fn story_start(&self, x: &mut Xml, tag: &str, s: &Story) {
        x.start(tag).attr("Self", uref(Some(s.uid)));
        // Values every exported IDML has on every story, from the DOM
        // version where they first appear (docs/format/idml-values.md).
        let major = self.doc.version.major;
        if major >= 12 {
            x.attr("UserText", "true");
        }
        if major >= 15 {
            x.attr("IsEndnoteStory", "false");
        }
        x.attr("TrackChanges", "false")
            .attr("StoryTitle", "$ID/")
            .attr("AppliedNamedGrid", "n");
    }

    fn xml_element_start(x: &mut Xml, e: &XmlElement) {
        x.start("XMLElement")
            .attr("Self", &e.name)
            .attr("MarkupTag", format!("XMLTag/{}", self_name(&e.tag)));
        if let Some(c) = e.content {
            x.attr("XMLContent", uref(Some(c)));
        }
    }

    fn story(&self, s: &Story) -> String {
        let mut x = Xml::new();
        self.package_root(&mut x, "Story");
        self.story_start(&mut x, "Story", s);
        x.empty(
            "StoryPreference",
            &[
                ("OpticalMarginAlignment", "false".into()),
                ("OpticalMarginSize", "12".into()),
                ("FrameType", "TextFrameType".into()),
                // Read from the story's frames; otherwise the value every
                // exported IDML has (`idml-values.md`).
                (
                    "StoryOrientation",
                    match s.orientation {
                        Some(Orientation::Vertical) => "Vertical",
                        _ => "Horizontal",
                    }
                    .into(),
                ),
                ("StoryDirection", "LeftToRightDirection".into()),
            ],
        );
        x.empty(
            "InCopyExportOption",
            &[
                ("IncludeGraphicProxies", "true".into()),
                ("IncludeAllResources", "false".into()),
            ],
        );
        let scope = uref(Some(s.uid));
        // The element whose content is the story holds all its text.
        let element = s
            .xml_element
            .and_then(|k| self.doc.xml.elements.get(&k))
            .filter(|e| e.content == Some(s.uid));
        if let Some(e) = element {
            Self::xml_element_start(&mut x, e);
        }
        self.text_ranges(&mut x, &s.runs, s, &scope);
        if element.is_some() {
            x.end();
        }
        x.end().end();
        x.finish()
    }

    /// The start tag of a character range for `r`.
    fn csr_start(&self, x: &mut Xml, r: &TextRun) {
        let (plain, props) = self.text_attrs(&r.character_attrs);
        x.start("CharacterStyleRange").attr(
            "AppliedCharacterStyle",
            self.style_ref(r.character_style, false),
        );
        for (k, v) in &plain {
            x.attr(k, v);
        }
        Self::properties(x, &props);
    }

    /// Paragraph and character ranges for `runs`. The final paragraph
    /// return (a story's last, or a table cell's terminator) is not written.
    /// `scope` is the `Self` of the enclosing story or table cell, which
    /// prefixes the `Self` of tables inside the text.
    fn text_ranges(&self, x: &mut Xml, runs: &[TextRun], story: &Story, scope: &str) {
        let mut runs: Vec<&TextRun> = runs.iter().collect();
        let mut last_text = None;
        if let Some(last) = runs.last()
            && last.text.ends_with('\r')
        {
            last_text = Some(last.text[..last.text.len() - 1].to_string());
            if last_text.as_deref() == Some("") && runs.len() > 1 {
                runs.pop();
                last_text = None;
            }
        }
        let n = runs.len();
        let text_of = |i: usize| -> &str {
            match (&last_text, i + 1 == n) {
                (Some(t), true) => t,
                _ => &runs[i].text,
            }
        };
        let mut i = 0;
        while i < n {
            let para = (runs[i].paragraph_style, &runs[i].paragraph_attrs);
            let (plain, props) = self.text_attrs(para.1);
            x.start("ParagraphStyleRange")
                .attr("AppliedParagraphStyle", self.style_ref(para.0, true));
            for (k, v) in &plain {
                x.attr(k, v);
            }
            Self::properties(x, &props);
            let mut st = TextState::default();
            while i < n && (runs[i].paragraph_style, &runs[i].paragraph_attrs) == para {
                let r = runs[i];
                self.csr_start(x, r);
                st.csr = true;
                self.run_content(x, text_of(i), r, story, scope, &mut st);
                st.close_csr(x);
                i += 1;
            }
            // Elements left open (their end is missing) end with the range.
            for _ in st.blocks.drain(..) {
                x.end();
            }
            x.end();
        }
    }

    /// Content, line breaks, anchored items, tables and XML elements of
    /// one character range `run`, whose text is `text`. A character range
    /// is open on entry (`st.csr`); XML markers can close it and open
    /// others (see `docs/format/xml.md`).
    fn run_content(
        &self,
        x: &mut Xml,
        text: &str,
        run: &TextRun,
        story: &Story,
        scope: &str,
        st: &mut TextState,
    ) {
        let offset = run.start;
        let mut buf = String::new();
        let flush = |x: &mut Xml, buf: &mut String| {
            if !buf.is_empty() {
                // INDD stores a forced line break as LF; IDML as U+2028.
                x.start("Content")
                    .text(&buf.replace('\n', "\u{2028}"))
                    .end();
                buf.clear();
            }
        };
        let mut pos = offset;
        // End offset of the open hyperlink text source.
        let mut open: Option<usize> = None;
        for ch in text.chars() {
            if open == Some(pos) {
                flush(x, &mut buf);
                x.end();
                open = None;
            }
            if ch == '\u{FEFF}'
                && let Some(&m) = story.xml_markers.get(&pos)
            {
                flush(x, &mut buf);
                // An XML marker ends the open text source.
                if open.take().is_some() {
                    x.end();
                }
                let get = |k: &XmlKey| self.doc.xml.elements.get(k);
                match m {
                    XmlMarker::Start(k) => match get(&k) {
                        Some(e) if e.block => {
                            st.close_csr(x);
                            Self::xml_element_start(x, e);
                            st.blocks.push(k);
                            self.csr_start(x, run);
                            st.csr = true;
                        }
                        Some(e) => {
                            st.open_csr(self, x, run);
                            Self::xml_element_start(x, e);
                            st.inline.push(k);
                        }
                        None => {}
                    },
                    XmlMarker::End(k) => {
                        if st.inline.last() == Some(&k) {
                            st.inline.pop();
                            x.end();
                        } else if st.blocks.last() == Some(&k) {
                            st.close_csr(x);
                            st.blocks.pop();
                            x.end();
                        }
                    }
                    XmlMarker::Placeholder(k) => {
                        if let Some(e) = get(&k) {
                            st.open_csr(self, x, run);
                            Self::xml_element_start(x, e);
                            x.end();
                            // An element with a story as content ends its
                            // character range.
                            if e.story_content {
                                st.close_csr(x);
                            }
                        }
                    }
                    XmlMarker::Hidden => {}
                }
                pos += 1;
                continue;
            }
            st.open_csr(self, x, run);
            if open.is_none()
                && let Some(r) = story.sources.iter().find(|r| r.start == pos)
                && let Some(src) = self.doc.text_sources.get(&r.source)
            {
                flush(x, &mut buf);
                x.start("HyperlinkTextSource")
                    .attr("Self", uref(Some(src.uid)))
                    .attr("Name", &src.name)
                    .attr("Hidden", src.hidden.to_string())
                    .attr(
                        "AppliedCharacterStyle",
                        match src.character_style {
                            Some(c) => self.style_ref(Some(c), false),
                            None => "n".into(),
                        },
                    );
                if src.toc_anchor {
                    x.start("Properties")
                        .start("AlternativeDestination")
                        .attr("Type", "TocTextAnchor")
                        .attr("IndexMarkerId", "0")
                        .attr("TextAnchorName", "")
                        .attr("TocEntryPageNumberString", "")
                        .attr("TocEntryLevel", "0")
                        .end()
                        .end();
                }
                open = Some(r.start + r.len);
            }
            match ch {
                '\r' => {
                    flush(x, &mut buf);
                    x.start("Br").end();
                }
                '\u{FFFC}' if story.anchors.contains_key(&pos) => {
                    flush(x, &mut buf);
                    for item in &story.anchors[&pos] {
                        self.page_item(x, item, false);
                    }
                }
                '\u{16}' if story.tables.contains_key(&pos) => {
                    flush(x, &mut buf);
                    self.table(x, &story.tables[&pos], story, scope);
                }
                // Internal table markers that follow U+0016.
                '\u{17}' => {}
                '\u{18}' if story.text_variables.contains_key(&pos) => {
                    flush(x, &mut buf);
                    self.variable_instance(x, &story.text_variables[&pos]);
                }
                c => buf.push(c),
            }
            pos += ch.len_utf16();
        }
        flush(x, &mut buf);
        if open.is_some() {
            x.end();
        }
    }

    fn table(&self, x: &mut Xml, t: &Table, story: &Story, scope: &str) {
        let id = format!("{scope}i{:x}", t.uid);
        let rows = t.rows.len() as u32;
        x.start("Table")
            .attr("Self", &id)
            .attr("HeaderRowCount", t.header_rows.to_string())
            .attr("FooterRowCount", t.footer_rows.to_string())
            .attr(
                "BodyRowCount",
                rows.saturating_sub(t.header_rows + t.footer_rows)
                    .to_string(),
            )
            .attr("ColumnCount", t.columns.len().to_string());
        if let Some(st) = t.style.and_then(|u| self.doc.table_styles.get(&u)) {
            x.attr("AppliedTableStyle", Self::table_style_ref("TableStyle", st));
        }
        for (name, v) in self.table_attrs(&t.attrs) {
            x.attr(name, v);
        }
        for (i, r) in t.rows.iter().enumerate() {
            x.start("Row")
                .attr("Self", format!("{id}Row{i}"))
                .attr("Name", i.to_string());
            if let Some(h) = r.height {
                x.attr("SingleRowHeight", num(h));
            }
            if let Some(h) = r.min_height {
                x.attr("MinimumHeight", num(h));
            }
            if r.auto_grow == Some(0) {
                x.attr("AutoGrow", "false");
            }
            x.end();
        }
        for (i, w) in t.columns.iter().enumerate() {
            x.empty(
                "Column",
                &[
                    ("Self", format!("{id}Column{i}")),
                    ("Name", i.to_string()),
                    ("SingleColumnWidth", num(*w)),
                ],
            );
        }
        for c in &t.cells {
            let cell_id = format!("{id}i{:x}", c.id);
            x.start("Cell")
                .attr("Self", &cell_id)
                .attr("Name", format!("{}:{}", c.column, c.row))
                .attr("RowSpan", c.row_span.to_string())
                .attr("ColumnSpan", c.column_span.to_string())
                .attr("CellType", "TextTypeCell");
            if let Some(f) = &c.format {
                for (name, v) in self.cell_attrs(&f.attrs) {
                    x.attr(name, v);
                }
                if let Some(r) = self.cell_style_ref(f.style) {
                    x.attr("AppliedCellStyle", r)
                        .attr("AppliedCellStylePriority", f.style_priority.to_string());
                }
            } else {
                // A cell without an attribute set has no cell style.
                x.attr("AppliedCellStyle", "CellStyle/$ID/[None]")
                    .attr("AppliedCellStylePriority", "0");
            }
            self.text_ranges(x, &c.runs, story, &cell_id);
            x.end();
        }
        x.end();
    }

    fn backing_story(&self) -> String {
        let mut x = Xml::new();
        self.package_root(&mut x, "BackingStory");
        if let Some(s) = &self.doc.xml.story {
            self.story_start(&mut x, "XmlStory", s);
            self.text_ranges(&mut x, &s.runs, s, &uref(Some(s.uid)));
            x.end();
        }
        x.end();
        x.finish()
    }

    fn tags(&self) -> String {
        let mut x = Xml::new();
        self.package_root(&mut x, "Tags");
        // IDML lists the tags by name, ignoring case.
        let mut tags: Vec<&(String, Option<[f64; 3]>)> = self.doc.xml_tags.iter().collect();
        tags.sort_by_key(|(name, _)| name.to_lowercase());
        for (name, color) in tags {
            x.start("XMLTag")
                .attr("Self", format!("XMLTag/{}", self_name(name)))
                .attr("Name", name);
            if let Some(c) = color.and_then(xml_tag_color) {
                Self::properties(&mut x, &[("TagColor", "enumeration", c.to_string().into())]);
            }
            x.end();
        }
        x.end();
        x.finish()
    }
}

/// While writing text with XML markers: whether a character range is
/// open, and the XML elements open inside it and around it.
#[derive(Default)]
struct TextState {
    csr: bool,
    /// Elements open inside the character range.
    inline: Vec<XmlKey>,
    /// Elements open between character ranges.
    blocks: Vec<XmlKey>,
}

impl TextState {
    fn open_csr(&mut self, w: &Writer, x: &mut Xml, run: &TextRun) {
        if !self.csr {
            w.csr_start(x, run);
            self.csr = true;
        }
    }

    fn close_csr(&mut self, x: &mut Xml) {
        if self.csr {
            for _ in self.inline.drain(..) {
                x.end();
            }
            x.end();
            self.csr = false;
        }
    }
}

/// Write `doc` as an IDML package. `name` is the document name (file name).
/// Write `doc` as an IDML package. Returns warnings about values left out
/// because the IDML schema does not allow them.
pub fn write(doc: &Document, name: &str, out: impl std::io::Write) -> std::io::Result<Vec<String>> {
    let w = Writer {
        doc,
        dom: format!("{}.0", doc.version.major),
        name: name.to_string(),
        group_path: group_paths(doc),
        warnings: Default::default(),
        observed: Default::default(),
        page_layouts: alternate_layouts(doc).1,
        page_sections: page_sections(doc),
    };
    let mut files: BTreeMap<String, String> = BTreeMap::new();
    files.insert(
        "META-INF/container.xml".into(),
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<container version=\"1.0\" xmlns=\"urn:oasis:names:tc:opendocument:xmlns:container\">\n\t<rootfiles>\n\t\t<rootfile full-path=\"designmap.xml\" media-type=\"text/xml\">\n\t\t</rootfile>\n\t</rootfiles>\n</container>".into(),
    );
    files.insert("designmap.xml".into(), w.designmap(name));
    files.insert("Resources/Graphic.xml".into(), w.graphic());
    files.insert("Resources/Fonts.xml".into(), w.fonts());
    files.insert("Resources/Styles.xml".into(), w.styles());
    files.insert("Resources/Preferences.xml".into(), w.preferences());
    files.insert("XML/BackingStory.xml".into(), w.backing_story());
    files.insert("XML/Tags.xml".into(), w.tags());
    let names = page_names(doc);
    let mut unused = 0;
    for s in &doc.master_spreads {
        files.insert(
            format!("MasterSpreads/MasterSpread_u{:x}.xml", s.uid),
            w.spread(s, true, &mut unused, &names),
        );
    }
    let mut page_index = 0;
    for s in &doc.spreads {
        files.insert(
            format!("Spreads/Spread_u{:x}.xml", s.uid),
            w.spread(s, false, &mut page_index, &names),
        );
    }
    for s in &doc.stories {
        files.insert(format!("Stories/Story_u{:x}.xml", s.uid), w.story(s));
    }
    let mut z = zip::ZipWriter::new(out);
    z.add("mimetype", MIMETYPE.as_bytes())?;
    for (path, content) in &files {
        z.add(path, content.as_bytes())?;
    }
    z.finish()?;
    Ok(w.warnings.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_xml_elements_at_their_markers() {
        use crate::model::{Attrs, XmlStructure};
        let element = |name: &str, content, story_content, block| XmlElement {
            name: name.into(),
            tag: "t".into(),
            content,
            story_content,
            block,
        };
        let (a, b, c, d) = ((9, 2), (9, 5), (9, 6), (9, 7));
        let markers = [
            (0, XmlMarker::Hidden),
            (1, XmlMarker::Start(a)),
            (2, XmlMarker::Placeholder(b)),
            (3, XmlMarker::Start(c)),
            (4, XmlMarker::Placeholder(d)),
            (5, XmlMarker::End(c)),
            (6, XmlMarker::End(a)),
        ];
        let story = Story {
            uid: 9,
            runs: vec![TextRun {
                start: 0,
                text: "\u{FEFF}".repeat(8) + "\r",
                paragraph_style: None,
                character_style: None,
                paragraph_attrs: Attrs::default(),
                character_attrs: Attrs::default(),
            }],
            anchors: Default::default(),
            tables: Default::default(),
            text_variables: Default::default(),
            sources: Vec::new(),
            xml_markers: markers.into_iter().collect(),
            xml_element: None,
            orientation: None,
        };
        let doc = Document {
            xml: XmlStructure {
                story: Some(story),
                elements: [
                    (a, element("di2", None, false, true)),
                    (b, element("di2i5", Some(0x10), true, false)),
                    (c, element("di2i6", None, false, false)),
                    (d, element("di2i6i7", Some(0x20), false, false)),
                ]
                .into_iter()
                .collect(),
            },
            ..Document::default()
        };
        let w = Writer {
            doc: &doc,
            dom: "20.0".into(),
            name: String::new(),
            group_path: Default::default(),
            warnings: Default::default(),
            observed: Default::default(),
            page_layouts: Vec::new(),
            page_sections: Vec::new(),
        };
        let out: String = w
            .backing_story()
            .lines()
            .map(str::trim)
            .collect::<Vec<_>>()
            .concat();
        let csr =
            "CharacterStyleRange AppliedCharacterStyle=\"CharacterStyle/$ID/[No character style]\"";
        let el = |name: &str| format!("XMLElement Self=\"{name}\" MarkupTag=\"XMLTag/t\"");
        let want = format!(
            "<{csr} /><{a}><{csr}><{b} XMLContent=\"u10\" /></CharacterStyleRange>\
             <{csr}><{c}><{d} XMLContent=\"u20\" /></XMLElement></CharacterStyleRange>\
             </XMLElement><{csr}><Content>\u{FEFF}</Content></CharacterStyleRange>",
            a = el("di2"),
            b = el("di2i5"),
            c = el("di2i6"),
            d = el("di2i6i7"),
        );
        assert!(out.contains(&want), "{out}");
    }

    #[test]
    fn numbers_pages_by_section() {
        use crate::model::{Page, Section, Spread, numbering};
        let page = |uid| Page {
            uid,
            bounds: [0.0; 4],
            transform: Matrix::IDENTITY,
            master: None,
            master_transform: Matrix::IDENTITY,
            margins: None,
            columns: None,
            grid: None,
            settings: Default::default(),
        };
        let section = |uid, page, continue_numbering, start| Section {
            uid,
            page,
            continue_numbering,
            start,
            style: numbering::ARABIC,
            prefix: String::new(),
            marker: String::new(),
            alternate_layout: None,
        };
        let doc = Document {
            spreads: vec![Spread {
                uid: 1,
                master_name: None,
                transform: Matrix::IDENTITY,
                binding_location: 0,
                pages: (10..16).map(page).collect(),
                items: Vec::new(),
                guides: Vec::new(),
                shuffle: None,
                flattener_resolution: None,
                show_master_items: None,
            }],
            // Listed out of page order, as in some samples.
            sections: vec![
                section(1, None, true, 1),
                section(3, Some(14), true, 9),
                section(2, Some(12), false, 1),
            ],
            ..Document::default()
        };
        assert_eq!(page_numbers(&doc), [1, 2, 1, 2, 3, 4]);
        let ranges: Vec<_> = section_ranges(&doc)
            .iter()
            .map(|(s, first, len)| (s.uid, *first, *len))
            .collect();
        assert_eq!(ranges, [(1, 0, 2), (2, 2, 2), (3, 4, 2)]);
        let mut doc = doc;
        doc.sections[0].style = numbering::LOWER_ROMAN;
        assert_eq!(page_names(&doc), ["i", "ii", "1", "2", "3", "4"]);
    }

    #[test]
    fn writes_wrap_offsets_by_side() {
        let wrap = TextWrap {
            mode: wrap_mode::BOUNDING_BOX,
            offsets: [1.0, 2.0, 3.0, 4.0],
            flags: 1,
        };
        let mut x = Xml::new();
        Writer::text_wrap_preference(&mut x, Some(&wrap), None);
        let out = x.finish();
        assert!(out.contains("TextWrapMode=\"BoundingBoxTextWrap\""));
        assert!(out.contains("<TextWrapOffset Top=\"2\" Left=\"1\" Bottom=\"4\" Right=\"3\" />"));
    }

    #[test]
    fn measures_guides_from_spread_origin() {
        use crate::model::Page;
        let page = |uid, tx| Page {
            uid,
            bounds: [0.0, 0.0, 612.0, 792.0],
            transform: Matrix([1.0, 0.0, 0.0, 1.0, tx, -396.0]),
            master: None,
            master_transform: Matrix::IDENTITY,
            margins: None,
            columns: None,
            grid: None,
            settings: Default::default(),
        };
        let origin = spread_origin(&[page(1, -612.0), page(2, 0.0)]);
        assert_eq!(origin, (-612.0, -396.0));
        let guide = Guide {
            uid: 0x2299,
            horizontal: true,
            position: 339.5,
            owner: 2,
            fit_to_page: true,
            view_threshold: 0.05,
            color: 6,
            guide_type: Some(0),
            layer: 0xcc,
            locked: false,
            zone: Some(1.0),
            overridden: None,
        };
        let mut x = Xml::new();
        Writer::guide(&mut x, &guide, origin, -1);
        let out = x.finish();
        assert!(
            out.contains("Orientation=\"Horizontal\" Location=\"735.5\""),
            "{out}"
        );
        assert!(
            out.contains("ItemLayer=\"ucc\" GuideType=\"Ruler\""),
            "{out}"
        );
        assert!(out.contains("<GuideColor type=\"enumeration\">Cyan</GuideColor>"));
        assert!(out.contains("PageIndex=\"-1\" GuideZone=\"1\""), "{out}");
        let vertical = Guide {
            horizontal: false,
            position: 28.0,
            ..guide
        };
        let mut x = Xml::new();
        Writer::guide(&mut x, &vertical, origin, 1);
        assert!(x.finish().contains("Location=\"640\""));
    }

    #[test]
    fn names_pages_in_lower_roman() {
        assert_eq!(lower_roman(1), "i");
        assert_eq!(lower_roman(4), "iv");
        assert_eq!(lower_roman(12), "xii");
        assert_eq!(lower_roman(49), "xlix");
        assert_eq!(lower_roman(1994), "mcmxciv");
    }

    #[test]
    fn names_pages_in_kanji_digits() {
        assert_eq!(kanji_digits(8), "八");
        assert_eq!(kanji_digits(10), "一〇");
        assert_eq!(kanji_digits(102), "一〇二");
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

    #[test]
    fn splits_cdata_sections() {
        let mut x = Xml::new();
        x.start("Contents").cdata("abcde", 2).end();
        assert!(
            x.finish()
                .ends_with("\n<Contents><![CDATA[ab]]><![CDATA[cd]]><![CDATA[e]]></Contents>")
        );
    }

    #[test]
    fn decodes_tab_stops() {
        // Two stops: 12 pt left aligned, 237.5 pt right aligned with "." leader.
        let mut b = 2u16.to_le_bytes().to_vec();
        b.extend(12f64.to_le_bytes());
        b.extend([0, 0, 0, 0]);
        b.extend(237.5f64.to_le_bytes());
        b.extend([2, 0, 1, 0, b'.', 0]);
        let stops = tab_list(&b).unwrap();
        assert_eq!(stops.len(), 2);
        assert_eq!(stops[0][0].2, "LeftAlign");
        assert_eq!(stops[0][3].2, "12");
        assert_eq!(stops[1][0].2, "RightAlign");
        assert_eq!(stops[1][2].2, ".");
        assert_eq!(stops[1][3].2, "237.5");
        assert_eq!(tab_list(&[0, 0]).unwrap().len(), 0);
        // Unknown alignment code.
        let mut b = 1u16.to_le_bytes().to_vec();
        b.extend(12f64.to_le_bytes());
        b.extend([1, 0, 0, 0]);
        assert!(tab_list(&b).is_none());
    }

    #[test]
    fn decodes_nested_style_delimiters() {
        let d = |c| nested_delimiter(c).map(|(f, r, i)| (f.1, f.2, r, i));
        assert_eq!(d("^c"), Some(("enumeration", "Dropcap".into(), 1, true)));
        assert_eq!(d("[.]"), Some(("string", ".".into(), 1, false)));
        assert_eq!(d("(:)"), Some(("string", ":".into(), 1, true)));
        assert_eq!(d("(^w)5"), Some(("enumeration", "AnyWord".into(), 5, true)));
        assert_eq!(
            d("(^?)"),
            Some(("enumeration", "AnyCharacter".into(), 1, true))
        );
        assert_eq!(d("(^x)"), None);
        assert_eq!(d("^t"), None);
    }

    #[test]
    fn decodes_bullet_char() {
        let b = [0, 0, 0, 0, 0x22, 0x20, 0, 0];
        let Some(PropValue::Attributes(a)) = bullet_char(&b) else {
            panic!("not decoded");
        };
        assert_eq!(a[0].1, "UnicodeOnly");
        assert_eq!(a[1].1, "8226");
        assert!(bullet_char(&[3, 0, 0, 0, 0x22, 0x20, 0, 0]).is_none());
    }
}
