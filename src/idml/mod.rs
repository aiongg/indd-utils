//! IDML package writer.

mod xml;
pub mod zip;

use std::collections::BTreeMap;

use crate::model::{Document, ItemKind, Matrix, PageItem, Path, Shape, Spread, Story, Style};
use xml::Xml;

const PACKAGING_NS: &str = "http://ns.adobe.com/AdobeInDesign/idml/1.0/packaging";
const MIMETYPE: &str = "application/vnd.adobe.indesign-idml-package";

/// Format a number the way IDML does: shortest round-trip form, no `-0`.
pub fn num(v: f64) -> String {
    if v == 0.0 {
        return "0".into();
    }
    format!("{v}")
}

fn nums(v: &[f64]) -> String {
    v.iter().map(|&x| num(x)).collect::<Vec<_>>().join(" ")
}

fn matrix(m: &Matrix) -> String {
    nums(&m.0)
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

struct Writer<'a> {
    doc: &'a Document,
    dom: String,
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
            Some(s) => format!("{prefix}/{}", self_name(&style_name(s))),
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
        let color = |x: &mut Xml, name: &str, model: &str, value: &str, editable: bool| {
            x.empty(
                "Color",
                &[
                    ("Self", format!("Color/{name}")),
                    ("Model", model.into()),
                    ("Space", "CMYK".into()),
                    ("ColorValue", value.into()),
                    ("Name", name.into()),
                    ("ColorEditable", editable.to_string()),
                    ("ColorRemovable", "false".into()),
                    ("Visible", "true".into()),
                ],
            );
        };
        color(&mut x, "Black", "Process", "0 0 0 100", false);
        color(&mut x, "Paper", "Process", "0 0 0 0", true);
        color(
            &mut x,
            "Registration",
            "Registration",
            "100 100 100 100",
            false,
        );
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

    fn fonts(&self) -> String {
        let mut x = Xml::new();
        self.package_root(&mut x, "Fonts");
        x.end();
        x.finish()
    }

    fn preferences(&self) -> String {
        let mut x = Xml::new();
        self.package_root(&mut x, "Preferences");
        x.end();
        x.finish()
    }

    fn styles(&self) -> String {
        let doc = self.doc;
        let mut x = Xml::new();
        self.package_root(&mut x, "Styles");
        for paragraph in [false, true] {
            let group = if paragraph {
                "RootParagraphStyleGroup"
            } else {
                "RootCharacterStyleGroup"
            };
            let tag = if paragraph {
                "ParagraphStyle"
            } else {
                "CharacterStyle"
            };
            x.start(group).attr("Self", group);
            let mut have_default = false;
            for s in doc.styles.values().filter(|s| s.paragraph == paragraph) {
                let name = style_name(s);
                have_default |=
                    name == "$ID/[No paragraph style]" || name == "$ID/[No character style]";
                x.start(tag)
                    .attr("Self", format!("{tag}/{}", self_name(&name)))
                    .attr("Name", &name);
                if paragraph {
                    x.attr("NextStyle", self.style_ref(s.next.or(Some(s.uid)), true));
                }
                if let Some(base) = s.based_on.and_then(|b| doc.styles.get(&b)) {
                    x.start("Properties");
                    x.start("BasedOn")
                        .attr("type", "object")
                        .text(&format!("{tag}/{}", self_name(&style_name(base))))
                        .end();
                    x.end();
                }
                x.end();
            }
            if !have_default && !paragraph {
                x.empty(
                    tag,
                    &[
                        ("Self", "CharacterStyle/$ID/[No character style]".into()),
                        ("Name", "$ID/[No character style]".into()),
                    ],
                );
            }
            x.end();
        }
        for group in [
            "RootCellStyleGroup",
            "RootTableStyleGroup",
            "RootObjectStyleGroup",
        ] {
            x.empty(group, &[("Self", group.into())]);
        }
        x.end();
        x.finish()
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
        } = &item.kind
        {
            x.attr("ParentStory", uref(*story))
                .attr("PreviousTextFrame", uref(*previous))
                .attr("NextTextFrame", uref(*next))
                .attr("ContentType", "TextType");
        }
        x.attr("ItemLayer", uref(Some(item.layer)))
            .attr("ItemTransform", matrix(&item.transform));
        Self::path_geometry(x, &item.paths);
        for child in &item.children {
            self.page_item(x, child);
        }
        x.end();
    }

    fn spread(&self, s: &Spread, master: bool, page_number: &mut usize) -> String {
        let mut x = Xml::new();
        let kind = if master { "MasterSpread" } else { "Spread" };
        self.package_root(&mut x, kind);
        x.start(kind)
            .attr("Self", uref(Some(s.uid)))
            .attr("PageCount", s.pages.len().to_string())
            .attr("BindingLocation", s.binding_location.to_string())
            .attr("ItemTransform", matrix(&s.transform));
        if master {
            x.attr("Name", format!("M-Master {}", s.uid))
                .attr("NamePrefix", "M")
                .attr("BaseName", format!("Master {}", s.uid));
        }
        for p in &s.pages {
            let name = if master {
                "M".to_string()
            } else {
                *page_number += 1;
                page_number.to_string()
            };
            let [x0, y0, x1, y1] = p.bounds;
            x.start("Page")
                .attr("Self", uref(Some(p.uid)))
                .attr("Name", name)
                .attr("GeometricBounds", nums(&[y0, x0, y1, x1]))
                .attr("ItemTransform", matrix(&p.transform));
            if !master {
                x.attr("AppliedMaster", uref(p.master))
                    .attr("MasterPageTransform", matrix(&p.master_transform));
            }
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
        // Group runs by paragraph style, then emit character runs inside.
        let mut runs: Vec<_> = s
            .runs
            .iter()
            .map(|r| (r.paragraph_style, r.character_style, r.text.clone()))
            .collect();
        // The story's final paragraph return is implicit in IDML.
        if let Some(last) = runs.last_mut()
            && last.2.ends_with('\r')
        {
            last.2.pop();
        }
        let mut i = 0;
        while i < runs.len() {
            let para = runs[i].0;
            x.start("ParagraphStyleRange")
                .attr("AppliedParagraphStyle", self.style_ref(para, true));
            while i < runs.len() && runs[i].0 == para {
                let (_, chr, text) = &runs[i];
                x.start("CharacterStyleRange")
                    .attr("AppliedCharacterStyle", self.style_ref(*chr, false));
                let mut parts = text.split('\r').peekable();
                while let Some(part) = parts.next() {
                    if !part.is_empty() {
                        x.start("Content").text(part).end();
                    }
                    if parts.peek().is_some() {
                        x.start("Br").end();
                    }
                }
                x.end();
                i += 1;
            }
            x.end();
        }
        x.end().end();
        x.finish()
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
    let mut unused = 0;
    for s in &doc.master_spreads {
        files.insert(
            format!("MasterSpreads/MasterSpread_u{:x}.xml", s.uid),
            w.spread(s, true, &mut unused),
        );
    }
    let mut page_number = 0;
    for s in &doc.spreads {
        files.insert(
            format!("Spreads/Spread_u{:x}.xml", s.uid),
            w.spread(s, false, &mut page_number),
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
    fn formats_numbers_like_idml() {
        assert_eq!(num(205.2), "205.2");
        assert_eq!(num(-0.0), "0");
        assert_eq!(num(1.0), "1");
        assert_eq!(num(-89.99999999999999), "-89.99999999999999");
    }
}
