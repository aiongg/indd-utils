//! Attribute tables: which IDML attribute or property each INDD attribute
//! ID of a text, page item, cell or table attribute list becomes, the kind
//! of its value, and the decoders for values stored as raw bytes (tab
//! lists, nested styles, bullet characters, anchored object settings).
//!
//! Evidence: `docs/format/attributes.md`, `tables.md` (cell and table
//! attributes) and `objects.md` (anchored object settings).

use super::*;

#[derive(Clone, Copy)]
pub(super) enum AttrKind {
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
pub(super) const FITTING_ATTRS: [(u32, &str); 7] = [
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
pub(super) fn fitting_value(id: u32, v: &Value) -> Option<String> {
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

/// Anchored object settings from chunk 0x2800 (of an anchor, an object
/// style or the preferences): f64 `AnchorYoffset` at 0, u16
/// `VerticalAlignment` at 52, and two groups of u16 fields that change
/// together. See `docs/format/objects.md`.
pub(super) fn anchored_settings(d: &[u8]) -> Vec<(&'static str, String)> {
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
    if d.len() >= 58 {
        let u = |o: usize| u16_from([d[o], d[o + 1]]);
        // Fields that change together in every sample: only the observed
        // combinations are written.
        match (u(46), u(50), u(56)) {
            (0, 2, 1) => {
                out.push(("AnchorPoint", "BottomRightAnchor".into()));
                out.push(("PinPosition", "true".into()));
            }
            (2, 0, 0) => {
                out.push(("AnchorPoint", "TopLeftAnchor".into()));
                out.push(("PinPosition", "false".into()));
            }
            _ => {}
        }
        match (u(40), u(48)) {
            (0, 2) => {
                out.push(("AnchoredPosition", "InlinePosition".into()));
                out.push(("HorizontalAlignment", "LeftAlign".into()));
            }
            (2, 1) => {
                out.push(("AnchoredPosition", "AboveLine".into()));
                out.push(("HorizontalAlignment", "CenterAlign".into()));
            }
            _ => {}
        }
    }
    out
}

/// Frame fitting attributes of `attrs` with IDs in `ids`, in IDML order.
pub(super) fn fitting_attrs(attrs: &Attrs, ids: &[u32]) -> Vec<(&'static str, String)> {
    FITTING_ATTRS
        .iter()
        .filter(|(id, _)| ids.contains(id))
        .filter_map(|&(id, name)| Some((name, fitting_value(id, attrs.get(id)?)?)))
        .collect()
}

pub(super) const STROKE_TYPES: &[(u32, &str)] = &[
    (0x5A29, "Solid"),
    (0x5A38, "Canned Dashed 3x2"),
    (0x5A39, "Canned Dotted"),
    (0xB004, "ThinThin"),
    (0xB01A, "Triple_Stroke"),
];

/// The stroke styles every corpus IDML lists in `Graphic.xml`, in order.
/// See `docs/format/idml-values.md`.
pub(super) const BUILTIN_STROKE_STYLES: &[&str] = &[
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
pub(super) enum TextKind {
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
    /// A kinsoku or mojikumi set: 0 `Nothing`, a built-in table (written
    /// as its enumeration value) or a custom kinsoku table (an object).
    CjkSet,
}

/// Value of the kerning attribute (0x1B13) in every root paragraph style;
/// IDML writes no `KerningValue` for it.
pub(super) const KERNING_NONE: f64 = 1e8;

/// Text attributes: ID, IDML name, kind, written in `<Properties>`.
/// See `docs/format/attributes.md` for the evidence behind each entry.
pub(super) const TEXT_ATTRS: &[(u32, &str, TextKind, bool)] = &[
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
        0x42C0,
        "TreatIdeographicSpaceAsSpace",
        TextKind::Bool(1),
        false,
    ),
    (
        0x50F18,
        "DiacriticPosition",
        TextKind::Enum(&[(4, "OpentypePosition"), (5, "OpentypePositionFromBaseline")]),
        false,
    ),
    (0x4221, "Mojikumi", TextKind::CjkSet, true),
    (0x4224, "KinsokuSet", TextKind::CjkSet, true),
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
pub(super) type Property = (&'static str, &'static str, PropValue);

/// One field of a record in a list property: name, type, text.
pub(super) type Field = (&'static str, &'static str, String);

/// The value of a `<Properties>` child.
pub(super) enum PropValue {
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
pub(super) const ITEM_ATTRS: &[(u32, &str, AttrKind)] = &[
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
pub(super) const GRADIENT_ATTRS: &[(u32, &str, &str)] = &[
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
pub(super) fn layout_constraints(flags: u8) -> Option<(String, String)> {
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
pub(super) fn join_numbers(v: &[u32]) -> String {
    v.iter().map(u32::to_string).collect::<Vec<_>>().join(" ")
}

/// Kinds of cell attribute values.
#[derive(Clone, Copy)]
pub(super) enum CellKind {
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
pub(super) const CELL_ATTRS: &[(u32, &[&str], CellKind)] = &[
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
pub(super) const TABLE_ATTRS: &[(u32, &str, CellKind)] = &[
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
pub(super) const CELL_STYLE_PARAGRAPH_STYLE: u32 = 0x10463;

/// Stroke style code of a cell edge that has no stroke type (IDML `n`).
pub(super) const CELL_NO_STROKE_TYPE: u32 = 0x1040C;

/// Report an attribute value that the converter has no IDML value for to
/// `indd audit`: its code, or for a stroke type the code in its first four
/// bytes.
pub(super) fn unknown_code(attrs: &Attrs, id: u32, v: &Value) {
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
pub(super) fn raw_bytes(v: &Value) -> Vec<u8> {
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
pub(super) fn tab_list(data: &[u8]) -> Option<Vec<Vec<Field>>> {
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
pub(super) fn bullet_char(data: &[u8]) -> Option<PropValue> {
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
pub(super) fn nested_delimiter(code: &str) -> Option<(Field, u32, bool)> {
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

impl Writer<'_> {
    /// IDML attributes and properties for a text attribute list.
    pub(super) fn text_attrs(&self, attrs: &Attrs) -> (Vec<(&'static str, String)>, Vec<Property>) {
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
    pub(super) fn text_value(&self, kind: TextKind, v: &Value) -> Option<(&'static str, String)> {
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
            TextKind::CjkSet => {
                let u = v.as_u32()?;
                if u == 0 {
                    return Some(("enumeration", "Nothing".into()));
                }
                let t = self.doc.cjk_tables.iter().find(|t| t.uid == u)?;
                // Built-in tables observed in the corpus (attributes.md).
                const BUILTIN: [(&str, &str); 6] = [
                    ("$ID/kHardKinsokuName", "HardKinsoku"),
                    ("$ID/kSoftKinsokuName", "SoftKinsoku"),
                    ("$ID/kKoreanKinsokuName", "KoreanKinsoku"),
                    ("$ID/kSimpChineseKinsokuName", "SimplifiedChineseKinsoku"),
                    ("$ID/kMojikumiDefaultName1", "LineEndAllOneHalfEmEnum"),
                    ("$ID/kMojikumiDefaultName16", "SimpChineseDefault"),
                ];
                if let Some((_, e)) = BUILTIN.iter().find(|(k, _)| *k == t.name) {
                    return Some(("enumeration", e.to_string()));
                }
                if t.name.starts_with("$ID/") || t.mojikumi {
                    return None;
                }
                Some(("object", format!("KinsokuTable/{}", self_name(&t.name))))
            }
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
    pub(super) fn nested_styles(&self, data: &[u8]) -> Option<Vec<Vec<Field>>> {
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

    /// Write page item attributes from the item's attribute list.
    pub(super) fn item_attrs(&self, x: &mut Xml, attrs: &Attrs) {
        for (name, text) in self.item_attr_values(attrs) {
            x.attr(name, text);
        }
    }

    /// Page item attributes from an attribute list, as IDML values.
    pub(super) fn item_attr_values(&self, attrs: &Attrs) -> Vec<(&'static str, String)> {
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

    /// IDML attributes of a table or table style attribute list.
    pub(super) fn table_attrs(&self, attrs: &Attrs) -> Vec<(&'static str, String)> {
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
    pub(super) fn cell_attrs(&self, attrs: &Attrs) -> Vec<(&'static str, String)> {
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

    pub(super) fn table_value(&self, v: &Value, kind: CellKind) -> Option<String> {
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

    /// The gradient attributes of a page item: its own, or, for a group,
    /// the values all its children have. See `docs/format/objects.md`.
    pub(super) fn gradients(item: &PageItem) -> Vec<(&'static str, String)> {
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
}
