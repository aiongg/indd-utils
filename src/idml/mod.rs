//! IDML package writer.

mod xml;
pub mod zip;

use std::collections::BTreeMap;

use crate::model::{
    Attrs, Document, Graphic, GraphicKind, ItemKind, Matrix, PageItem, Path, Section, Shape,
    Spread, Story, Style, StyleGroup, Table, TextFramePreferences, TextRun, TextWrap, Value,
    numbering, root_kind, wrap_mode,
};

#[derive(Clone, Copy)]
enum AttrKind {
    Number,
    Swatch,
    Point,
    Enum(&'static [(u32, &'static str)]),
}

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
}

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
    (
        0x1B11,
        "Capitalization",
        TextKind::Enum(&[(0, "Normal"), (2, "AllCaps")]),
        false,
    ),
    (0x1B12, "StrokeColor", TextKind::Swatch, false),
    (0x1B16, "LeftIndent", TextKind::Number, false),
    (0x1B18, "FirstLineIndent", TextKind::Number, false),
    (0x1B1A, "AutoLeading", TextKind::Percent, false),
    (0x1B1B, "Leading", TextKind::Leading, true),
    (0x1B1F, "Hyphenation", TextKind::Bool(3), false),
    (0x1B25, "HyphenationZone", TextKind::Number, false),
    (0x1B26, "SpaceBefore", TextKind::Number, false),
    (0x1B27, "SpaceAfter", TextKind::Number, false),
    (0x1B2A, "Underline", TextKind::Bool(1), false),
    (0x1B2B, "AppliedFont", TextKind::Font, true),
    (0x1B2E, "MaximumWordSpacing", TextKind::Percent, false),
    (0x1B2F, "MinimumWordSpacing", TextKind::Percent, false),
    (0x1B42, "FillTint", TextKind::Number, false),
    (0x1B4D, "RuleAboveLineWeight", TextKind::Number, false),
    (0x1B4F, "RuleAboveOffset", TextKind::Number, false),
    (0x1B54, "RuleBelowLineWeight", TextKind::Number, false),
    (0x1B55, "RuleBelowTint", TextKind::Number, false),
    (0x1B56, "RuleBelowOffset", TextKind::Number, false),
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
    (0x1BBF, "SplitColumnInsideGutter", TextKind::Number, false),
    (0x1BD2, "ParagraphShadingColor", TextKind::Swatch, true),
    (0x1BD3, "ParagraphShadingTint", TextKind::Number, false),
    (
        0x1A401,
        "BulletsAndNumberingListType",
        TextKind::Enum(&[(0, "NoList"), (1, "BulletList")]),
        false,
    ),
];

/// An attribute written as a `<Properties>` child: name, type, text.
type Property = (&'static str, &'static str, String);

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
];
use xml::Xml;

const PACKAGING_NS: &str = "http://ns.adobe.com/AdobeInDesign/idml/1.0/packaging";
const MIMETYPE: &str = "application/vnd.adobe.indesign-idml-package";

/// Format a number the way IDML does: shortest round-trip form. IDML
/// keeps negative zero (`1 -0 -0 1 0 0`).
pub fn num(v: f64) -> String {
    format!("{v}")
}

/// Round away binary noise from scaled values (0.8 * 100 = 80.00000000000001).
fn round(v: f64) -> f64 {
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
fn self_name(name: &str) -> String {
    name.replace('%', "%25").replace(':', "%3a")
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

/// The number shown on each document page, from the sections. Pages
/// not covered by a section are numbered by their position.
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
    /// Names of the enclosing style groups of each style or group UID.
    group_path: std::collections::HashMap<u32, Vec<String>>,
}

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
        let stories: Vec<String> = doc.stories.iter().map(|s| uref(Some(s.uid))).collect();
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
        x.empty("idPkg:Graphic", &[("src", "Resources/Graphic.xml".into())]);
        x.empty("idPkg:Fonts", &[("src", "Resources/Fonts.xml".into())]);
        x.empty("idPkg:Styles", &[("src", "Resources/Styles.xml".into())]);
        x.empty(
            "idPkg:Preferences",
            &[("src", "Resources/Preferences.xml".into())],
        );
        x.empty("idPkg:Tags", &[("src", "XML/Tags.xml".into())]);
        for l in doc.layers.iter().filter(|l| !l.internal) {
            x.empty(
                "Layer",
                &[
                    ("Self", uref(Some(l.uid))),
                    ("Name", l.name.clone()),
                    ("Visible", l.visible.to_string()),
                    ("Locked", l.locked.to_string()),
                ],
            );
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
        for (section, first, length) in section_ranges(doc) {
            x.start("Section")
                .attr("Self", uref(Some(section.uid)))
                .attr("Length", length.to_string())
                .attr("Name", "")
                .attr("ContinueNumbering", section.continue_numbering.to_string())
                .attr("IncludeSectionPrefix", "false")
                .attr("Marker", "")
                .attr("PageStart", uref(Some(pages[first])));
            // IDML gives a start number only to sections that restart
            // numbering.
            if !section.continue_numbering {
                x.attr("PageNumberStart", section.start.to_string());
            }
            x.attr("SectionPrefix", "");
            if section.style == numbering::ARABIC {
                x.start("Properties")
                    .start("PageNumberStyle")
                    .attr("type", "enumeration")
                    .text("Arabic")
                    .end()
                    .end();
            }
            x.end();
        }
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
        x.end();
        x.finish()
    }

    fn graphic(&self) -> String {
        let mut x = Xml::new();
        self.package_root(&mut x, "Graphic");
        for c in &self.doc.colors {
            let name = c.idml_name();
            x.empty(
                "Color",
                &[
                    ("Self", c.reference()),
                    ("Model", c.model_name().into()),
                    ("Space", c.space_name().into()),
                    ("ColorValue", nums(&c.idml_values())),
                    ("ColorOverride", c.override_name().into()),
                    ("Name", name),
                    ("ColorEditable", c.editable.to_string()),
                    ("ColorRemovable", c.removable.to_string()),
                    ("Visible", c.visible.to_string()),
                ],
            );
        }
        x.empty(
            "Swatch",
            &[
                ("Self", "Swatch/None".into()),
                ("Name", "None".into()),
                ("ColorEditable", "false".into()),
                ("ColorRemovable", "false".into()),
                ("Visible", "true".into()),
            ],
        );
        // The schema requires gradients after the swatches.
        for g in &self.doc.gradients {
            x.start("Gradient")
                .attr("Self", g.reference())
                .attr("Type", if g.kind == 2 { "Radial" } else { "Linear" })
                .attr("Name", g.idml_name())
                .attr("ColorEditable", g.editable.to_string())
                .attr("ColorRemovable", g.removable.to_string())
                .attr("Visible", g.visible.to_string());
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
                // The midpoint between stops i-1 and i is stored with stop i-1.
                if i > 0 {
                    x.attr("Midpoint", num(round(g.stops[i - 1].midpoint * 100.0)));
                }
                x.end();
            }
            x.end();
        }
        x.empty(
            "StrokeStyle",
            &[
                ("Self", "StrokeStyle/$ID/Solid".into()),
                ("Name", "$ID/Solid".into()),
            ],
        );
        x.end();
        x.finish()
    }

    /// IDML attributes and properties for a text attribute list.
    fn text_attrs(&self, attrs: &Attrs) -> (Vec<(&'static str, String)>, Vec<Property>) {
        let mut plain = Vec::new();
        let mut props = Vec::new();
        for &(id, name, kind, in_props) in TEXT_ATTRS {
            let Some(v) = attrs.get(id) else { continue };
            let swatch = |u: u32| self.doc.swatches.get(&u).cloned();
            let out: Option<(&'static str, String)> = match kind {
                TextKind::Number => v.as_f64().map(|f| ("unit", num(f))),
                TextKind::Percent => v.as_f64().map(|f| ("unit", num(round(f * 100.0)))),
                TextKind::Scale(k) => v.as_f64().map(|f| ("unit", num(round(f * k)))),
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
                    .and_then(|u| self.doc.fonts.get(&u).cloned())
                    .map(|f| ("string", f)),
                TextKind::FontStyle => v.as_string().map(|s| ("string", s)),
                TextKind::Leading => v.as_f64().map(|f| {
                    if f < 0.0 {
                        ("enumeration", "Auto".into())
                    } else {
                        ("unit", num(f))
                    }
                }),
            };
            if let Some((ty, text)) = out {
                if in_props {
                    props.push((name, ty, text));
                } else {
                    plain.push((name, text));
                }
            }
        }
        (plain, props)
    }

    fn properties(x: &mut Xml, props: &[Property]) {
        if props.is_empty() {
            return;
        }
        x.start("Properties");
        for (name, ty, text) in props {
            x.start(name).attr("type", *ty).text(text).end();
        }
        x.end();
    }

    /// Write page item attributes from the item's attribute list.
    fn item_attrs(&self, x: &mut Xml, attrs: &Attrs) {
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
            };
            if let Some(t) = text {
                x.attr(name, t);
            }
        }
    }

    fn fonts(&self) -> String {
        let mut x = Xml::new();
        self.package_root(&mut x, "Fonts");
        x.end();
        x.finish()
    }

    fn preferences(&self) -> String {
        let mut x = Xml::new();
        self.package_root(&mut x, "Preferences");
        if let Some(p) = &self.doc.preferences {
            const INTENT: [&str; 3] = ["PrintIntent", "WebIntent", "MobileIntent"];
            let [top, bottom, inside, outside] = p.bleed;
            x.start("DocumentPreference")
                .attr("PageHeight", num(p.page_height))
                .attr("PageWidth", num(p.page_width))
                .attr("FacingPages", p.facing_pages.to_string())
                .attr("DocumentBleedTopOffset", num(top))
                .attr("DocumentBleedBottomOffset", num(bottom))
                .attr("DocumentBleedInsideOrLeftOffset", num(inside))
                .attr("DocumentBleedOutsideOrRightOffset", num(outside));
            if let Some(i) = INTENT.get(p.intent as usize) {
                x.attr("Intent", *i);
            }
            x.end();
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
            for s in doc.styles.values().filter(|s| s.paragraph == paragraph) {
                if !written.contains(&s.uid) {
                    self.style_element(&mut x, s, tag);
                }
            }
            x.end();
        }
        for (tag, kind) in [
            ("RootCellStyleGroup", root_kind::CELL),
            ("RootTableStyleGroup", root_kind::TABLE),
        ] {
            let id = root_of(kind).map_or(tag.to_string(), |g| uref(Some(g.uid)));
            x.empty(tag, &[("Self", id)]);
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
            x.start("ObjectStyle")
                .attr("Self", format!("ObjectStyle/{}", self_name(&name)))
                .attr("Name", &name);
            if let Some(base) = os.based_on.and_then(|b| doc.object_styles.get(&b)) {
                let base_name = if base.builtin {
                    format!("$ID/{}", base.name)
                } else {
                    base.name.clone()
                };
                // The root "[None]" is written as a string.
                let prop = if base.builtin && base.name == "[None]" {
                    ("BasedOn", "string", base_name)
                } else {
                    (
                        "BasedOn",
                        "object",
                        format!("ObjectStyle/{}", self_name(&base_name)),
                    )
                };
                Self::properties(&mut x, &[prop]);
            }
            x.end();
        }
        x.end();
        x.end();
        x.finish()
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
        let (plain, mut props) = self.text_attrs(&s.attrs);
        x.start(tag)
            .attr("Self", self.style_ref(Some(s.uid), paragraph))
            .attr("Name", &name);
        if paragraph {
            x.attr("NextStyle", self.style_ref(s.next.or(Some(s.uid)), true));
        }
        for (k, v) in &plain {
            x.attr(k, v);
        }
        if let Some(base) = s.based_on.and_then(|b| doc.styles.get(&b)) {
            // The root "[No ... style]" is written as a string.
            let root = base.builtin && base.name.starts_with("[No ");
            if root {
                props.insert(0, ("BasedOn", "string", style_name(base)));
            } else {
                props.insert(
                    0,
                    (
                        "BasedOn",
                        "object",
                        self.style_ref(Some(base.uid), paragraph),
                    ),
                );
            }
        }
        Self::properties(x, &props);
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

    fn text_frame_preference(x: &mut Xml, p: &TextFramePreferences) {
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
        x.end();
    }

    /// `TextWrapPreference` from an item's text wrap chunk. An item without
    /// the chunk has no wrap. Values not identified yet are left out: the
    /// whole element for an unknown mode, the side and inverse settings
    /// for flags other than 1, and the offsets unless all are 0.
    fn text_wrap_preference(x: &mut Xml, wrap: Option<&TextWrap>, contour_type: Option<u32>) {
        let mode = match wrap.map(|w| w.mode) {
            None | Some(wrap_mode::NONE) => "None",
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
        if wrap.is_none_or(|w| w.offsets == [0.0; 4]) {
            x.start("Properties").empty(
                "TextWrapOffset",
                &[
                    ("Top", "0".into()),
                    ("Left", "0".into()),
                    ("Bottom", "0".into()),
                    ("Right", "0".into()),
                ],
            );
            x.end();
        }
        if contour_type == Some(5) {
            x.empty("ContourOption", &[("ContourType", "SameAsClipping".into())]);
        }
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

    fn page_item(&self, x: &mut Xml, item: &PageItem) {
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
        Self::path_geometry(x, &item.paths);
        if let ItemKind::TextFrame {
            preferences: Some(p),
            ..
        } = &item.kind
        {
            Self::text_frame_preference(x, p);
        }
        Self::text_wrap_preference(x, item.text_wrap.as_ref(), None);
        for child in &item.children {
            self.page_item(x, child);
        }
        for g in &item.graphics {
            Self::placed_graphic(x, g);
        }
        x.end();
    }

    /// `page_index` counts the document pages written so far; `numbers`
    /// gives each document page its number (see `page_numbers`).
    fn spread(&self, s: &Spread, master: bool, page_index: &mut usize, numbers: &[u32]) -> String {
        let mut x = Xml::new();
        let kind = if master { "MasterSpread" } else { "Spread" };
        self.package_root(&mut x, kind);
        x.start(kind)
            .attr("Self", uref(Some(s.uid)))
            .attr("PageCount", s.pages.len().to_string());
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
        }
        for p in &s.pages {
            let name = if master {
                prefix.clone()
            } else {
                *page_index += 1;
                numbers
                    .get(*page_index - 1)
                    .map_or_else(|| page_index.to_string(), u32::to_string)
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
            x.end();
        }
        for item in &s.items {
            self.page_item(&mut x, item);
        }
        x.end();
        x.finish()
    }

    fn story(&self, s: &Story) -> String {
        let mut x = Xml::new();
        self.package_root(&mut x, "Story");
        x.start("Story").attr("Self", uref(Some(s.uid)));
        let scope = uref(Some(s.uid));
        self.text_ranges(&mut x, &s.runs, s, &scope);
        x.end().end();
        x.finish()
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
            while i < n && (runs[i].paragraph_style, &runs[i].paragraph_attrs) == para {
                let r = runs[i];
                let (plain, props) = self.text_attrs(&r.character_attrs);
                x.start("CharacterStyleRange").attr(
                    "AppliedCharacterStyle",
                    self.style_ref(r.character_style, false),
                );
                for (k, v) in &plain {
                    x.attr(k, v);
                }
                Self::properties(x, &props);
                self.run_content(x, text_of(i), r.start, story, scope);
                x.end();
                i += 1;
            }
            x.end();
        }
    }

    /// Content, line breaks, anchored items and tables of one character
    /// range. `offset` is the UTF-16 offset of the range in the story.
    fn run_content(&self, x: &mut Xml, text: &str, offset: usize, story: &Story, scope: &str) {
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
        for ch in text.chars() {
            match ch {
                '\r' => {
                    flush(x, &mut buf);
                    x.start("Br").end();
                }
                '\u{FFFC}' if story.anchors.contains_key(&pos) => {
                    flush(x, &mut buf);
                    for item in &story.anchors[&pos] {
                        self.page_item(x, item);
                    }
                }
                '\u{16}' if story.tables.contains_key(&pos) => {
                    flush(x, &mut buf);
                    self.table(x, &story.tables[&pos], story, scope);
                }
                // Internal table markers that follow U+0016.
                '\u{17}' => {}
                // Text variables are not converted yet; leave them out
                // rather than writing a page number marker.
                '\u{18}' if story.text_variables.contains(&pos) => flush(x, &mut buf),
                c => buf.push(c),
            }
            pos += ch.len_utf16();
        }
        flush(x, &mut buf);
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
            self.text_ranges(x, &c.runs, story, &cell_id);
            x.end();
        }
        x.end();
    }

    fn backing_story(&self) -> String {
        let mut x = Xml::new();
        self.package_root(&mut x, "BackingStory");
        x.end();
        x.finish()
    }

    fn tags(&self) -> String {
        let mut x = Xml::new();
        self.package_root(&mut x, "Tags");
        x.end();
        x.finish()
    }
}

/// Write `doc` as an IDML package. `name` is the document name (file name).
pub fn write(doc: &Document, name: &str, out: impl std::io::Write) -> std::io::Result<()> {
    let w = Writer {
        doc,
        dom: format!("{}.0", doc.version.major),
        group_path: group_paths(doc),
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
    let numbers = page_numbers(doc);
    let mut unused = 0;
    for s in &doc.master_spreads {
        files.insert(
            format!("MasterSpreads/MasterSpread_u{:x}.xml", s.uid),
            w.spread(s, true, &mut unused, &numbers),
        );
    }
    let mut page_index = 0;
    for s in &doc.spreads {
        files.insert(
            format!("Spreads/Spread_u{:x}.xml", s.uid),
            w.spread(s, false, &mut page_index, &numbers),
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
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_pages_by_section() {
        use crate::model::{Page, Section, Spread, numbering};
        let page = |uid| Page {
            uid,
            bounds: [0.0; 4],
            transform: Matrix::IDENTITY,
            master: None,
            master_transform: Matrix::IDENTITY,
        };
        let section = |uid, page, continue_numbering, start| Section {
            uid,
            page,
            continue_numbering,
            start,
            style: numbering::ARABIC,
        };
        let doc = Document {
            spreads: vec![Spread {
                uid: 1,
                master_name: None,
                transform: Matrix::IDENTITY,
                binding_location: 0,
                pages: (10..16).map(page).collect(),
                items: Vec::new(),
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
}
