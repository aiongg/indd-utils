//! Attribute tables: which IDML attribute or property each INDD attribute
//! ID of a text, page item, cell or table attribute list becomes, and the
//! kind of its value; and the IDML form of structured values the model
//! decodes (tab lists, nested styles, bullet characters, anchored object
//! settings).
//!
//! Evidence: `docs/format/attributes.md`, `tables.md` (cell and table
//! attributes) and `objects.md` (anchored object settings).

use super::*;
use crate::model::AnchorSettings;
use crate::model::attrs::{Delimiter, NestedStyle, TabStop};

/// Frame fitting attributes of page items and object styles, in the
/// order IDML writes them (`docs/format/objects.md`).
pub(super) const FITTING_ATTRS: [(u32, &str, Kind); 7] = [
    (0x6E83, "AutoFit", Kind::Enum(&[(0, "false")])),
    (0x6E7E, "LeftCrop", Kind::Number),
    (0x6E7F, "TopCrop", Kind::Number),
    (0x6E80, "RightCrop", Kind::Number),
    (0x6E81, "BottomCrop", Kind::Number),
    (
        0x6E7C,
        "FittingOnEmptyFrame",
        Kind::Enum(&[
            (0, "None"),
            (1, "ContentToFrame"),
            (2, "Proportionally"),
            (3, "FillProportionally"),
        ]),
    ),
    (
        0x6E7D,
        "FittingAlignment",
        Kind::Enum(&[(0, "TopLeftAnchor"), (4, "CenterAnchor")]),
    ),
];

/// IDML attributes of anchored object settings: `VerticalAlignment`,
/// `AnchorYoffset`, and the combinations of the other fields observed in
/// every sample. See `docs/format/objects.md`.
pub(super) fn anchored_settings(a: &AnchorSettings) -> Vec<(&'static str, String)> {
    let mut out = Vec::new();
    let Some((y, align)) = a.offset else {
        return out;
    };
    let align = match align {
        0 => Some("TopAlign"),
        1 => Some("CenterAlign"),
        2 => Some("BottomAlign"),
        _ => None,
    };
    if let Some(a) = align {
        out.push(("VerticalAlignment", a.to_string()));
    }
    out.push(("AnchorYoffset", num(y)));
    if let Some((point, position)) = a.fields {
        match point {
            [0, 2, 1] => {
                out.push(("AnchorPoint", "BottomRightAnchor".into()));
                out.push(("PinPosition", "true".into()));
            }
            [2, 0, 0] => {
                out.push(("AnchorPoint", "TopLeftAnchor".into()));
                out.push(("PinPosition", "false".into()));
            }
            _ => {}
        }
        match position {
            [0, 2] => {
                out.push(("AnchoredPosition", "InlinePosition".into()));
                out.push(("HorizontalAlignment", "LeftAlign".into()));
            }
            [2, 1] => {
                out.push(("AnchoredPosition", "AboveLine".into()));
                out.push(("HorizontalAlignment", "CenterAlign".into()));
            }
            _ => {}
        }
    }
    out
}

/// Frame fitting attributes of `attrs` with IDs in `ids`, in IDML order.
pub(super) fn fitting_attrs(w: &Writer, attrs: &Attrs, ids: &[u32]) -> Vec<(&'static str, String)> {
    FITTING_ATTRS
        .iter()
        .filter(|(id, ..)| ids.contains(id))
        .filter_map(|&(id, name, kind)| Some((name, w.value_text(kind, attrs.get(id)?)?)))
        .collect()
}

/// Codes of built-in stroke styles. See `docs/format/attributes.md`.
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

/// Value of the kerning attribute (0x1B13) in every root paragraph style;
/// IDML writes no `KerningValue` for it.
pub(super) const KERNING_NONE: f64 = 1e8;

/// Text attributes: ID, IDML name, kind, written in `<Properties>`.
/// See `docs/format/attributes.md` for the evidence behind each entry.
pub(super) const TEXT_ATTRS: &[(u32, &str, Kind, bool)] = &[
    (0x1B01, "FillColor", Kind::Swatch, false),
    (0x1B02, "FontStyle", Kind::String, false),
    (0x1B03, "PointSize", Kind::Number, false),
    (0x1B06, "HorizontalScale", Kind::Percent, false),
    (0x1B08, "Ligatures", Kind::Equals(1), false),
    (
        0x1B07,
        "KerningMethod",
        Kind::Enum(&[(15972, "$ID/Metrics"), (79875, "$ID/Optical")]),
        false,
    ),
    (0x1B0A, "StrokeWeight", Kind::Number, false),
    (0x1B0B, "Tracking", Kind::Scale(1000.0), false),
    (0x1B13, "KerningValue", Kind::Kerning, false),
    (
        0x1B0C,
        "Composer",
        Kind::Enum(&[
            (0x2001, "HL Single"),
            (0x2002, "HL Composer"),
            (0x2078, "HL Composer Optyca"),
        ]),
        false,
    ),
    (0x1B0D, "DropCapCharacters", Kind::Number, false),
    (0x1B0E, "DropCapLines", Kind::Number, false),
    (0x1B10, "BaselineShift", Kind::Number, false),
    (
        0x1B11,
        "Capitalization",
        Kind::Enum(&[
            (0, "Normal"),
            (1, "SmallCaps"),
            (2, "AllCaps"),
            (3, "CapToSmallCap"),
        ]),
        false,
    ),
    (0x1B12, "StrokeColor", Kind::Swatch, false),
    (0x1B15, "VerticalScale", Kind::Percent, false),
    (0x1B16, "LeftIndent", Kind::Number, false),
    (0x1B17, "RightIndent", Kind::Number, false),
    (0x1B18, "FirstLineIndent", Kind::Number, false),
    (0x1B1A, "AutoLeading", Kind::Percent, false),
    (0x1B1B, "Leading", Kind::Leading, true),
    (0x1B1D, "AppliedLanguage", Kind::Language, false),
    (0x1B1F, "Hyphenation", Kind::Equals(3), false),
    (0x1B24, "NoBreak", Kind::Equals(1), false),
    (0x1B25, "HyphenationZone", Kind::Number, false),
    (0x1B26, "SpaceBefore", Kind::Number, false),
    (0x1B27, "SpaceAfter", Kind::Number, false),
    (0x1B29, "TabList", Kind::TabList, true),
    (0x1B2A, "Underline", Kind::Equals(1), false),
    (0x1B2B, "AppliedFont", Kind::Font, true),
    (
        0x1B2C,
        "OTFFigureStyle",
        Kind::Enum(&[
            (1, "ProportionalOldstyle"),
            (2, "ProportionalLining"),
            (4, "Default"),
        ]),
        false,
    ),
    (0x1B2E, "MaximumWordSpacing", Kind::Percent, false),
    (0x1B2F, "MinimumWordSpacing", Kind::Percent, false),
    (0x1B31, "MaximumLetterSpacing", Kind::Percent, false),
    (0x1B32, "MinimumLetterSpacing", Kind::Percent, false),
    (
        0x1B37,
        "StartParagraph",
        Kind::Enum(&[(0, "Anywhere"), (2, "NextPage")]),
        false,
    ),
    (
        0x1B3C,
        "Position",
        Kind::Enum(&[(0, "Normal"), (5, "OTNumerator")]),
        false,
    ),
    (0x1B40, "KeepLinesTogether", Kind::Equals(1), false),
    (0x1B42, "FillTint", Kind::Number, false),
    (0x1B46, "GradientFillAngle", Kind::Number, false),
    (0x1B48, "GradientFillLength", Kind::Number, false),
    (0x1B4A, "GradientFillStart", Kind::Point, false),
    (0x1B4D, "RuleAboveLineWeight", Kind::Number, false),
    (0x1B4F, "RuleAboveOffset", Kind::Number, false),
    (0x1B50, "RuleAboveLeftIndent", Kind::Number, false),
    (0x1B51, "RuleAboveRightIndent", Kind::Number, false),
    (
        0x1B52,
        "RuleAboveWidth",
        Kind::Enum(&[(1, "ColumnWidth"), (2, "TextWidth")]),
        false,
    ),
    (0x1B53, "RuleBelowColor", Kind::SwatchOrText, true),
    (0x1B54, "RuleBelowLineWeight", Kind::Number, false),
    (0x1B55, "RuleBelowTint", Kind::Number, false),
    (0x1B56, "RuleBelowOffset", Kind::Number, false),
    (0x1B5D, "RuleBelow", Kind::Equals(1), false),
    (
        0x1B6A,
        "ParagraphBreakType",
        Kind::Enum(&[(0, "Anywhere"), (1, "NextColumn")]),
        false,
    ),
    (
        0x1B6B,
        "SingleWordJustification",
        Kind::Enum(&[(0, "LeftAlign"), (3, "FullyJustified")]),
        false,
    ),
    (0x1B75, "AllNestedStyles", Kind::NestedStyles, true),
    (
        0x42C0,
        "TreatIdeographicSpaceAsSpace",
        Kind::Equals(1),
        false,
    ),
    (
        0x50F18,
        "DiacriticPosition",
        Kind::Enum(&[(4, "OpentypePosition"), (5, "OpentypePositionFromBaseline")]),
        false,
    ),
    (0x4221, "Mojikumi", Kind::CjkSet, true),
    (0x4224, "KinsokuSet", Kind::CjkSet, true),
    (
        0x1B7E,
        "Justification",
        Kind::Enum(&[
            (0, "LeftAlign"),
            (1, "CenterAlign"),
            (2, "RightAlign"),
            (4, "LeftJustified"),
            (5, "CenterJustified"),
        ]),
        false,
    ),
    (0x1B80, "DropcapDetail", Kind::Number, false),
    (0x1B8C, "OTFContextualAlternate", Kind::Equals(1), false),
    (0x1B8D, "UnderlineColor", Kind::SwatchOrText, true),
    (0x1B91, "UnderlineOffset", Kind::Number, false),
    (0x1B94, "UnderlineWeight", Kind::Number, false),
    (0x1BB7, "MiterLimit", Kind::Number, false),
    (
        0x1BB9,
        "EndJoin",
        Kind::Enum(&[(0, "MiterEndJoin"), (1, "RoundEndJoin")]),
        false,
    ),
    (
        0x1BBD,
        "SpanColumnType",
        Kind::Enum(&[(0, "SingleColumn"), (1, "SpanColumns")]),
        false,
    ),
    (
        0x1BBE,
        "SpanSplitColumnCount",
        Kind::NumberOr(1.0, "All", "short"),
        true,
    ),
    (0x1BBF, "SplitColumnInsideGutter", Kind::Number, false),
    (0x1BC4, "SpanColumnMinSpaceAfter", Kind::Number, false),
    (0x1BD2, "ParagraphShadingColor", Kind::Swatch, true),
    (0x1BD3, "ParagraphShadingTint", Kind::Number, false),
    (0x1BD6, "ParagraphShadingOn", Kind::Equals(1), false),
    (0x1BDB, "ParagraphShadingTopOffset", Kind::Number, false),
    (0x1BDC, "ParagraphShadingBottomOffset", Kind::Number, false),
    (0x1BF6, "ParagraphBorderColor", Kind::Swatch, true),
    (0x1BF9, "ParagraphBorderOn", Kind::Equals(1), false),
    (0x1DF03, "ParagraphBorderTopOffset", Kind::Number, false),
    (0x1DF04, "ParagraphBorderBottomOffset", Kind::Number, false),
    (
        0x1DF21,
        "SameParaStyleSpacing",
        Kind::NumberOr(-1.0, "SetIgnore", "unit"),
        true,
    ),
    (0x4265, "GridAlignFirstLineOnly", Kind::Equals(1), false),
    (0x425E, "Tatechuyoko", Kind::Equals(1), false),
    (0x4279, "ShataiDegreeAngle", Kind::Scale(100.0), false),
    (0x427A, "ShataiAdjustTsume", Kind::Equals(1), false),
    (0x427B, "ShataiAdjustRotation", Kind::Equals(1), false),
    (0x422D, "RubyFlag", Kind::NonZero, false),
    (0x422E, "RubyString", Kind::Text, false),
    (
        0x4266,
        "GridAlignment",
        Kind::Enum(&[(0, "None"), (1, "AlignBaseline")]),
        false,
    ),
    (
        0x1A401,
        "BulletsAndNumberingListType",
        Kind::Enum(&[(0, "NoList"), (1, "BulletList")]),
        false,
    ),
    (0x1A406, "BulletChar", Kind::BulletChar, true),
    (0x1A413, "BulletsFont", Kind::FontOrNone, true),
    (0x1A414, "BulletsFontStyle", Kind::StringOrNothing, true),
    (0x1A419, "NumberingContinue", Kind::Equals(1), false),
    (0x1A41F, "BulletsCharacterStyle", Kind::CharacterStyle, true),
    (
        0x1A420,
        "NumberingCharacterStyle",
        Kind::CharacterStyle,
        true,
    ),
    (0x1A423, "NumberingExpression", Kind::String, false),
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
pub(super) const ITEM_ATTRS: &[(u32, &str, Kind)] = &[
    (0x6E68, "FillColor", Kind::Swatch),
    (0x6E69, "FillTint", Kind::Number),
    (0x6E64, "StrokeColor", Kind::Swatch),
    (0x6E65, "StrokeWeight", Kind::Number),
    (0x6E6D, "MiterLimit", Kind::Number),
    (
        0x6E6F,
        "CornerOption",
        Kind::Enum(&[(0x5A15, "RoundedCorner")]),
    ),
    (0x6E70, "CornerRadius", Kind::Number),
    (0x551F, "GradientFillLength", Kind::Number),
    (0x5520, "GradientFillStart", Kind::Point),
    (0x5525, "GradientStrokeLength", Kind::Number),
    (0x5526, "GradientStrokeStart", Kind::Point),
    (
        0x6E6E,
        "StrokeType",
        Kind::Builtin("StrokeStyle/$ID/", STROKE_TYPES),
    ),
    (
        0x6E8C,
        "StrokeAlignment",
        Kind::Enum(&[(0, "CenterAlignment"), (1, "InsideAlignment")]),
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

/// Cell attributes of a cell attribute set: ID, IDML attributes, kind.
/// See `docs/format/tables.md`.
pub(super) const CELL_ATTRS: &[(u32, &[&str], Kind)] = &[
    (0xB62C, &["TextTopInset", "TopInset"], Kind::Number),
    (0xB62B, &["TextLeftInset", "LeftInset"], Kind::Number),
    (0xB62E, &["TextBottomInset", "BottomInset"], Kind::Number),
    (0xB62D, &["TextRightInset", "RightInset"], Kind::Number),
    (0xB63D, &["FillColor"], Kind::Swatch),
    (0xB63E, &["FillTint"], Kind::Number),
    (
        0xB677,
        &["VerticalJustification"],
        Kind::Enum(&[(1, "CenterAlign"), (2, "BottomAlign")]),
    ),
    (0xB6DE, &["ClipContentToCell"], Kind::Enum(&[(0, "false")])),
    (0xB645, &["LeftEdgeStrokeWeight"], Kind::Number),
    (0xB64D, &["LeftEdgeStrokeType"], Kind::StrokeType),
    (0xB649, &["LeftEdgeStrokeColor"], Kind::Swatch),
    (0xB6A8, &["LeftEdgeStrokeTint"], Kind::Number),
    (0xB6F9, &["LeftEdgeStrokePriority"], Kind::Integer),
    (0xB647, &["TopEdgeStrokeWeight"], Kind::Number),
    (0xB64F, &["TopEdgeStrokeType"], Kind::StrokeType),
    (0xB64A, &["TopEdgeStrokeColor"], Kind::Swatch),
    (0xB6AA, &["TopEdgeStrokeTint"], Kind::Number),
    (0xB6FB, &["TopEdgeStrokePriority"], Kind::Integer),
    (0xB646, &["RightEdgeStrokeWeight"], Kind::Number),
    (0xB64E, &["RightEdgeStrokeType"], Kind::StrokeType),
    (0xB64B, &["RightEdgeStrokeColor"], Kind::Swatch),
    (0xB6A9, &["RightEdgeStrokeTint"], Kind::Number),
    (0xB6FA, &["RightEdgeStrokePriority"], Kind::Integer),
    (0xB648, &["BottomEdgeStrokeWeight"], Kind::Number),
    (0xB650, &["BottomEdgeStrokeType"], Kind::StrokeType),
    (0xB64C, &["BottomEdgeStrokeColor"], Kind::Swatch),
    (0xB6AB, &["BottomEdgeStrokeTint"], Kind::Number),
    (0xB6FC, &["BottomEdgeStrokePriority"], Kind::Integer),
];

/// Table and table style attributes: ID, IDML attribute, kind.
/// See `docs/format/tables.md`.
pub(super) const TABLE_ATTRS: &[(u32, &str, Kind)] = &[
    (0xB662, "SpaceBefore", Kind::Number),
    (0xB663, "SpaceAfter", Kind::Number),
    (0xB684, "StartRowStrokeColor", Kind::Swatch),
    (0xB690, "StartRowStrokeWeight", Kind::Number),
    (0xB688, "StartRowStrokeType", Kind::StrokeType),
    (0xB683, "ColumnFillsPriority", Kind::Enum(&[(0, "false")])),
    (0xB67B, "StartRowFillColor", Kind::Swatch),
    (0xB6B0, "StartRowFillTint", Kind::Number),
    (0xB67C, "EndRowFillColor", Kind::Swatch),
    (0xB6B1, "EndRowFillTint", Kind::Number),
    (
        0x10457,
        "HeaderRegionSameAsBodyRegion",
        Kind::Enum(&[(0, "false"), (1, "true")]),
    ),
    (0x10450, "HeaderRegionCellStyle", Kind::CellStyle),
    (0x10452, "BodyRegionCellStyle", Kind::CellStyle),
    (0x10453, "LeftColumnRegionCellStyle", Kind::CellStyle),
    (0x10454, "RightColumnRegionCellStyle", Kind::CellStyle),
];

/// Paragraph style of a cell style.
pub(super) const CELL_STYLE_PARAGRAPH_STYLE: u32 = 0x10463;

/// Stroke style code of a cell edge that has no stroke type (IDML `n`).
pub(super) const CELL_NO_STROKE_TYPE: u32 = 0x1040C;

/// Records of a `TabList`. `None` for an alignment code without evidence.
/// See `docs/format/attributes.md`.
pub(super) fn tab_list(stops: &[TabStop]) -> Option<Vec<Vec<Field>>> {
    stops
        .iter()
        .map(|t| {
            let alignment = match t.alignment {
                0 => "LeftAlign",
                2 => "RightAlign",
                _ => return None,
            };
            Some(vec![
                ("Alignment", "enumeration", alignment.to_string()),
                ("AlignmentCharacter", "string", ".".to_string()),
                ("Leader", "string", t.leader.clone()),
                ("Position", "unit", num(t.position)),
            ])
        })
        .collect()
}

/// `BulletChar` attributes. See `docs/format/attributes.md`.
pub(super) fn bullet_char(kind: u32, value: u32) -> Option<PropValue> {
    let kind = match kind {
        0 => "UnicodeOnly",
        1 => "UnicodeWithFont",
        2 => "GlyphWithFont",
        _ => return None,
    };
    Some(PropValue::Attributes(vec![
        ("BulletCharacterType", kind.into()),
        ("BulletCharacterValue", value.to_string()),
    ]))
}

/// The IDML field of a nested style delimiter.
pub(super) fn delimiter_field(d: &Delimiter) -> Field {
    let enumeration = |v: &str| ("Delimiter", "enumeration", v.to_string());
    match d {
        Delimiter::Dropcap => enumeration("Dropcap"),
        Delimiter::AnyWord => enumeration("AnyWord"),
        Delimiter::AnyCharacter => enumeration("AnyCharacter"),
        Delimiter::Character(c) => ("Delimiter", "string", c.clone()),
    }
}

impl Writer<'_> {
    /// IDML attributes and properties for a text attribute list.
    pub(super) fn text_attrs(&self, attrs: &Attrs) -> (Vec<(&'static str, String)>, Vec<Property>) {
        let mut plain = Vec::new();
        let mut props = Vec::new();
        for &(id, name, kind, in_props) in TEXT_ATTRS {
            let Some(v) = attrs.get(id) else { continue };
            let out = self.value(kind, v);
            if out.is_none() && kind.is_code() {
                attrs.unknown_code(id, v);
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

    /// Records of an `AllNestedStyles` list. See `docs/format/attributes.md`.
    pub(super) fn nested_styles(&self, styles: &[NestedStyle]) -> Vec<Vec<Field>> {
        styles
            .iter()
            .map(|n| {
                vec![
                    (
                        "AppliedCharacterStyle",
                        "object",
                        self.style_ref(Some(n.style).filter(|&u| u != 0), false),
                    ),
                    delimiter_field(&n.delimiter),
                    ("Repetition", "long", n.repetition.to_string()),
                    ("Inclusive", "boolean", n.inclusive.to_string()),
                ]
            })
            .collect()
    }

    /// Write page item attributes from the item's attribute list.
    pub(super) fn item_attrs(&self, x: &mut Xml, attrs: &Attrs) {
        for (name, text) in self.item_attr_values(attrs) {
            x.attr(name, text);
        }
    }

    /// Page item attributes from an attribute list, as IDML values.
    pub(super) fn item_attr_values(&self, attrs: &Attrs) -> Vec<(&'static str, String)> {
        self.attr_values(attrs, ITEM_ATTRS)
    }

    /// IDML attributes of a table or table style attribute list.
    pub(super) fn table_attrs(&self, attrs: &Attrs) -> Vec<(&'static str, String)> {
        self.attr_values(attrs, TABLE_ATTRS)
    }

    /// IDML attributes of a cell attribute set. Some cell attributes are
    /// written as two IDML attributes.
    pub(super) fn cell_attrs(&self, attrs: &Attrs) -> Vec<(&'static str, String)> {
        let mut out = Vec::new();
        for &(id, names, kind) in CELL_ATTRS {
            let Some(v) = attrs.get(id) else { continue };
            match self.value_text(kind, v) {
                Some(t) => {
                    for &name in names {
                        out.push((name, t.clone()));
                    }
                }
                None if kind.is_code() => attrs.unknown_code(id, v),
                None => {}
            }
        }
        out
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
