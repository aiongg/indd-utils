//! IDML package writer.
//!
//! `write` builds one `Writer` and asks it for each package part. The parts
//! are written by `impl Writer` blocks in the child modules: `designmap`,
//! `resources` (graphics, fonts, preferences), `styles`, `spread` and
//! `story`. `attrs` holds the attribute tables and value decoders, `format`
//! the text forms of values, `pages` page numbering, `values` the values
//! every IDML has, and `transparency` the effect settings.

mod attrs;
mod designmap;
mod format;
mod kind;
mod pages;
mod resources;
mod spread;
mod story;
mod styles;
mod transparency;
mod values;
mod xml;
pub mod zip;

use attrs::*;
pub use format::num;
pub(crate) use format::round;
use format::*;
use kind::Kind;
use pages::*;
use styles::*;

use std::collections::BTreeMap;

use crate::model::prefs::PrefProp;
use crate::model::{
    Attrs, Document, Graphic, GraphicKind, Guide, ItemKind, Matrix, Orientation, Page, PageItem,
    Path, Section, Shape, Spread, Story, Style, StyleGroup, Table, TextFramePreferences, TextRun,
    TextVariable, TextWrap, UiColorRef, Value, XmlElement, XmlMarker, hyperlink::DestinationKind,
    numbering, root_kind, variable::Instance, wrap_mode, xml::Key as XmlKey,
};

use values::Node;
use xml::Xml;

const PACKAGING_NS: &str = "http://ns.adobe.com/AdobeInDesign/idml/1.0/packaging";
const MIMETYPE: &str = "application/vnd.adobe.indesign-idml-package";

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
    use super::spread::spread_origin;
    use super::*;

    #[test]
    fn numbers_without_text_are_written_as_zero() {
        assert_eq!(num(1.5), "1.5");
        assert_eq!(num(f64::NAN), "0");
        assert_eq!(num(f64::INFINITY), "0");
        assert_eq!(num(f64::NEG_INFINITY), "0");
    }

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
            toc_style: None,
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
    fn writes_tab_stops() {
        let stop = |position, alignment, leader: &str| crate::model::attrs::TabStop {
            position,
            alignment,
            leader: leader.into(),
        };
        let stops = tab_list(&[stop(12.0, 0, ""), stop(237.5, 2, ".")]).unwrap();
        assert_eq!(stops[0][0].2, "LeftAlign");
        assert_eq!(stops[0][3].2, "12");
        assert_eq!(stops[1][0].2, "RightAlign");
        assert_eq!(stops[1][2].2, ".");
        assert_eq!(stops[1][3].2, "237.5");
        // Unknown alignment code.
        assert!(tab_list(&[stop(12.0, 1, "")]).is_none());
    }

    #[test]
    fn writes_bullet_char() {
        let Some(PropValue::Attributes(a)) = bullet_char(0, 0x2022) else {
            panic!("not written");
        };
        assert_eq!(a[0].1, "UnicodeOnly");
        assert_eq!(a[1].1, "8226");
        assert!(bullet_char(3, 0x2022).is_none());
    }
}
