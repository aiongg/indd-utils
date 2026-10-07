//! `Resources/Graphic.xml` (colours, tints, gradients, inks, stroke styles),
//! `Resources/Fonts.xml` and `Resources/Preferences.xml`.
//!
//! Evidence: `docs/format/objects.md` (swatches), `fonts.md`,
//! `preferences.md` and `idml-values.md`.

use super::*;

/// Preference elements of `Resources/Preferences.xml` that `model::prefs`
/// supplies values for.
pub(super) const PREFERENCE_TAGS: &[&str] = &[
    "ViewPreference",
    "MarginPreference",
    "TextFramePreference",
    "AnchoredObjectSetting",
    "GridPreference",
    "GuidePreference",
    "DocumentPreference",
    "TextPreference",
    "PasteboardPreference",
    "XMLPreference",
    "PrintPreference",
    "PrintBookletPrintPreference",
];

impl Writer<'_> {
    /// `SwatchColorGroupReference` of each swatch in a colour group: the
    /// `ColorGroupSwatch` that names it. See `docs/format/objects.md`.
    pub(super) fn group_swatches(&self) -> std::collections::HashMap<String, String> {
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

    /// `Resources/Graphic.xml`. `refs` are the references to unnamed
    /// colours and gradients in the other parts of the package: IDML
    /// writes an unnamed colour or gradient only when something refers to
    /// it or the page item defaults name it (`docs/format/objects.md`,
    /// colours).
    pub(super) fn graphic(&self, refs: &std::collections::HashSet<String>) -> String {
        use crate::model::color::class;
        let mut x = Xml::new();
        self.package_root(&mut x, "Graphic");
        let groups = self.group_swatches();
        let named = |class_id: u32| -> std::collections::HashSet<u32> {
            self.doc
                .prefs
                .item_default_entries
                .iter()
                .filter(|e| e.0 == class_id)
                .flat_map(|e| [e.1, e.2])
                .collect()
        };
        let gradients: Vec<_> = {
            let named = named(class::GRADIENT);
            self.doc
                .gradients
                .iter()
                .filter(|g| {
                    !g.name.is_empty() || named.contains(&g.uid) || refs.contains(&g.reference())
                })
                .collect()
        };
        // Colours that the stops of any gradient, written or not, and the
        // tints refer to.
        let mut used: std::collections::HashSet<u32> = named(class::COLOR);
        used.extend(
            self.doc
                .gradients
                .iter()
                .flat_map(|g| g.stops.iter().map(|s| s.color)),
        );
        used.extend(self.doc.tints.iter().map(|(t, ..)| t.base));
        let colors = self.doc.colors.iter().filter(|c| {
            !c.name.is_empty() || used.contains(&c.uid) || refs.contains(&c.reference())
        });
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
        for c in colors {
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
                ("Self", format!("Ink/{}", self_name(&i.name.idml()))),
                ("Name", i.name.idml()),
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
        // The document's constant shade, with the values every IDML has
        // on one (idml-values.md); its contents are the stored numbers in
        // big-endian order (objects.md, pasted smooth shades).
        if let Some(shade) = &self.doc.constant_shade
            && let Some(node) = values::keyed("PastedSmoothShade", "ConstantShade")
        {
            let reference = format!("PastedSmoothShade/{}", uref(Some(shade.uid)));
            let mut data = shade.count.to_be_bytes().to_vec();
            for v in shade.values {
                data.extend(v.to_be_bytes());
            }
            x.start("PastedSmoothShade")
                .attr("Self", &reference)
                .attr("ContentsType", "ConstantShade");
            if let Some((builtin, name)) = &shade.name {
                let name = if *builtin {
                    builtin_key(name)
                } else {
                    name.clone()
                };
                x.attr("Name", name);
            }
            x.attrs_missing(node.attrs.iter());
            group_ref(&mut x, &reference);
            x.start("Properties")
                .start("Contents")
                .cdata(&base64_lines(&data), CDATA_SECTION)
                .end()
                .end();
            x.end();
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
        for g in gradients {
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
                    ("Name", builtin_key(name)),
                ],
            );
        }
        x.end();
        x.finish()
    }

    /// Font families and their fonts. See docs/format/fonts.md.
    pub(super) fn fonts(&self) -> String {
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
                    x.attr("TypekitID", font.typekit_id.idml());
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
            .filter(|c| c.name.builtin && c.name.name == "[No composite font]")
        {
            let name = cf.name.idml();
            x.start("CompositeFont")
                .attr("Self", format!("CompositeFont/{}", self_name(&name)))
                .attr("Name", &name);
            for e in &cf.entries {
                // The four numbers are the same in every sample.
                let usual = e.numbers == [100.0, 0.0, 100.0, 100.0];
                x.start("CompositeFontEntry")
                    .attr("Self", uref(Some(e.uid)))
                    .attr("Name", e.name.idml())
                    .attr("FontStyle", e.font_style.idml());
                if usual {
                    x.attr("RelativeSize", "100")
                        .attr("HorizontalScale", "100")
                        .attr("VerticalScale", "100");
                }
                // IDML gives no characters for the Kanji entry.
                if !(e.name.builtin && e.name.name == "Kanji") {
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

    pub(super) fn preferences(&self) -> String {
        let mut x = Xml::new();
        self.package_root(&mut x, "Preferences");
        // Values every exported IDML has (docs/format/idml-values.md);
        // values read from the INDD take precedence.
        let mut nodes = values::preferences(self.doc.version.major);
        // DOM 7 IDML has no EPUB and HTML export preferences
        // (idml-values.md, preferences).
        if self.doc.version.major < 8 {
            nodes.retain(|n| {
                !matches!(
                    n.tag.as_str(),
                    "EPubExportPreference" | "HTMLExportPreference"
                )
            });
        }
        // Values every IDML of the version has in the larger corpus, where
        // the file above has none.
        for n in &mut nodes {
            if let Some(e) =
                values::element(&format!("Preferences/{}", n.tag), self.doc.version.major)
            {
                n.merge(&e);
            }
        }
        // Object styles named by the page item defaults must be in the
        // package.
        let styles: std::collections::HashSet<String> = self
            .doc
            .object_styles
            .values()
            .map(|os| {
                let name = if os.builtin {
                    builtin_key(&os.name)
                } else {
                    os.name.clone()
                };
                format!("ObjectStyle/{}", self_name(&name))
            })
            .collect();
        for n in nodes.iter_mut().filter(|n| n.tag == "PageItemDefault") {
            n.attrs
                .retain(|(k, v)| !k.ends_with("ObjectStyle") || styles.contains(v));
        }
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
        // Values read from the preferences object (`model::prefs`).
        let prefs = &self.doc.prefs;
        let mut ours: Vec<Node> = Vec::new();
        fn ours_of(ours: &mut Vec<Node>, tag: &str) -> usize {
            match ours.iter().position(|n| n.tag == tag) {
                Some(i) => i,
                None => {
                    ours.push(Node {
                        tag: tag.to_string(),
                        ..Node::default()
                    });
                    ours.len() - 1
                }
            }
        }
        for v in prefs
            .values
            .iter()
            .filter(|v| PREFERENCE_TAGS.contains(&v.element))
        {
            let i = ours_of(&mut ours, v.element);
            ours[i].attrs.push((v.name.to_string(), v.value.clone()));
        }
        if let Some(d) = &prefs.anchor {
            let i = ours_of(&mut ours, "AnchoredObjectSetting");
            ours[i].attrs.extend(
                anchored_settings(d)
                    .into_iter()
                    .map(|(k, v)| (k.to_string(), v)),
            );
        }
        if let Some(a) = &prefs.item_defaults {
            let i = ours_of(&mut ours, "PageItemDefault");
            let values = self.item_attr_values(a);
            ours[i]
                .attrs
                .extend(values.into_iter().map(|(k, v)| (k.to_string(), v)));
            let all: Vec<u32> = FITTING_ATTRS.iter().map(|(id, ..)| *id).collect();
            let fitting = fitting_attrs(self, a, &all);
            if !fitting.is_empty() {
                let i = ours_of(&mut ours, "FrameFittingOption");
                ours[i]
                    .attrs
                    .extend(fitting.into_iter().map(|(k, v)| (k.to_string(), v)));
            }
        }
        for (tag, data) in &prefs.print_records {
            let mut text = base64_lines(data);
            // IDML ends a last line of full length with a line feed.
            if text.rsplit('\n').next().is_some_and(|l| l.len() == 76) {
                text.push('\n');
            }
            let i = ours_of(&mut ours, tag);
            ours[i]
                .attrs
                .push(("PrintRecord".into(), builtin_key(&text)));
        }
        for (tag, name, value) in &prefs.props {
            let mut n = Node {
                tag: name.to_string(),
                ..Node::default()
            };
            match value {
                PrefProp::Text(ty, text) => {
                    n.attrs.push(("type".into(), ty.to_string()));
                    n.text = Some(text.clone());
                }
                PrefProp::Attrs(a) => {
                    n.attrs = a.iter().map(|(k, v)| (k.to_string(), v.clone())).collect();
                }
            }
            let i = ours_of(&mut ours, tag);
            match ours[i].children.iter_mut().find(|c| c.tag == "Properties") {
                Some(p) => p.children.push(n),
                None => ours[i].children.insert(
                    0,
                    Node {
                        tag: "Properties".into(),
                        children: vec![n],
                        ..Node::default()
                    },
                ),
            }
        }
        for &(tag, name, rgb) in &prefs.colors {
            if !PREFERENCE_TAGS.contains(&tag) {
                continue;
            }
            if let Some(p) = ui_color_property(name, rgb) {
                let i = ours_of(&mut ours, tag);
                match ours[i].children.iter_mut().find(|c| c.tag == "Properties") {
                    Some(props) => props.children.push(p),
                    None => ours[i].children.insert(
                        0,
                        Node {
                            tag: "Properties".into(),
                            children: vec![p],
                            ..Node::default()
                        },
                    ),
                }
            }
        }
        if let Some(a) = &prefs.text_defaults {
            let (mut plain, mut props) = self.text_attrs(a);
            // The schema allows KerningValue on character styles only.
            plain.retain(|(k, _)| *k != "KerningValue");
            super::styles::root_lists(&mut props, self.doc.version.major);
            let i = ours_of(&mut ours, "TextDefault");
            ours[i]
                .attrs
                .extend(plain.into_iter().map(|(k, v)| (k.to_string(), v)));
            if !props.is_empty() {
                ours[i].children.push(Node {
                    tag: "Properties".into(),
                    children: props.iter().map(prop_node).collect(),
                    ..Node::default()
                });
            }
        }
        for mut n in ours {
            match nodes.iter_mut().find(|m| m.tag == n.tag) {
                Some(m) => {
                    n.merge(m);
                    *m = n;
                }
                None => nodes.push(n),
            }
        }
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
}
