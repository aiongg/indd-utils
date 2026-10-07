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
mod export;
mod format;
mod graphic;
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
    Attrs, Document, Graphic, GraphicKind, Guide, ItemKind, ItemProps, Link, Matrix, Orientation,
    Page, PageItem, Path, Section, Shape, Spread, Story, Style, StyleGroup, Table,
    TextFramePreferences, TextRun, TextVariable, TextWrap, UiColorRef, Value, XmlElement,
    XmlMarker, hyperlink::DestinationKind, numbering, root_kind, variable::Instance, wrap_mode,
    xml::Key as XmlKey,
};

use crate::object::builtin_key;
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

/// Write `doc` as an IDML package. `name` is the document name (file
/// name). Returns warnings about values left out because the IDML schema
/// does not allow them.
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
impl<'a> Writer<'a> {
    /// A writer for `doc` with no derived state, for unit tests.
    pub(super) fn for_test(doc: &'a Document) -> Writer<'a> {
        Writer {
            doc,
            dom: "20.0".into(),
            name: String::new(),
            group_path: Default::default(),
            warnings: Default::default(),
            observed: Default::default(),
            page_layouts: Vec::new(),
            page_sections: Vec::new(),
        }
    }
}
