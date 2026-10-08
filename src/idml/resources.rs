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
    "PrintBookletOption",
    "IndexHeaderSetting",
    "ChapterNumberPreference",
    "DictionaryPreference",
    "EPubExportPreference",
    "TransparencyPreference",
    "LayoutAdjustmentPreference",
];

/// A tree of preference values as a `Node`.
fn pref_node(n: &crate::model::prefs::PrefNode) -> Node {
    Node {
        tag: n.tag.to_string(),
        attrs: n
            .attrs
            .iter()
            .map(|(k, v)| (k.to_string(), v.clone()))
            .collect(),
        text: None,
        children: n.children.iter().map(pref_node).collect(),
    }
}

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
            ];
            if let Some(c) = i.convert_to_process {
                attrs.push(("ConvertToProcess", c.to_string()));
            }
            attrs.push(("Frequency", num(i.frequency)));
            if let Some(d) = i.neutral_density {
                attrs.push(("NeutralDensity", num(d)));
            }
            attrs.push(("TrapOrder", i.trap_order.to_string()));
            x.start("Ink");
            for (k, v) in &attrs {
                x.attr(k, v);
            }
            // The values every IDML has on an ink (idml-values.md).
            x.attrs_missing(self.observed("Ink").iter());
            x.end();
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
            let (family, base) = self.family_names(f);
            x.start("FontFamily")
                .attr("Self", &id)
                .attr("Name", &family);
            for font in &f.fonts {
                // A font with an empty style name is named `Regular`, and
                // its `Self` has the family name alone (fonts.md).
                let (name, key) = if font.style.is_empty() {
                    (format!("{family} Regular"), base.clone())
                } else {
                    (
                        format!("{family} {}", font.style),
                        format!("{base} {}", font.style),
                    )
                };
                let style = if font.style_builtin {
                    builtin_key(&font.style)
                } else {
                    font.style.clone()
                };
                x.start("Font")
                    .attr("Self", format!("{id}Fontn{key}"))
                    .attr("FontFamily", &base)
                    .attr("Name", &name)
                    .attr("PostScriptName", &font.postscript_name)
                    .attr("FontStyleName", &style);
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
                    // `$ID/` in every Font of the corpus IDML files but
                    // those of missing fonts (idml-values.md).
                    .attr(
                        "PlatformName",
                        font.platform_name.as_deref().unwrap_or("$ID/"),
                    )
                    .attr("Version", &font.version);
                // IDML from InDesign before 9.2 has no TypekitID (fonts.md).
                let v = &self.doc.version;
                if (v.major, v.minor) >= (9, 2) {
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
                    Self::properties(
                        &mut x,
                        &[("AppliedFont", "string", self.family_names(f).0.into())],
                    );
                }
                x.end();
            }
            x.end();
        }
        x.end();
        x.finish()
    }

    /// `FootnoteOption` from chunk 0x2820 (footnotes.md). The values
    /// that never vary come from the generated value files.
    fn footnote_option(&self) -> Node {
        let major = self.doc.version.major;
        let mut n = Node {
            tag: "FootnoteOption".into(),
            ..Node::default()
        };
        let attr = |n: &mut Node, k: &str, v: String| n.attrs.push((k.to_string(), v));
        let Some(f) = &self.doc.prefs.footnotes else {
            // Without the chunk every IDML has these values.
            if major >= 12 {
                attr(&mut n, "EnableStraddling", "true".into());
            }
            attr(&mut n, "FootnoteTextStyle", self.style_ref(None, true));
            return n;
        };
        let bool_text = |b: bool| b.to_string();
        if major >= 12
            && let Some(s) = f.tail.as_ref().and_then(|t| t.straddling)
        {
            attr(&mut n, "EnableStraddling", bool_text(s));
        }
        attr(&mut n, "StartAt", f.start_at.to_string());
        attr(&mut n, "Prefix", f.prefix.clone());
        attr(&mut n, "Suffix", f.suffix.clone());
        attr(
            &mut n,
            "FootnoteTextStyle",
            self.style_ref(Some(f.text_style), true),
        );
        attr(
            &mut n,
            "FootnoteMarkerStyle",
            self.style_ref(Some(f.marker_style), false),
        );
        attr(&mut n, "SeparatorText", f.separator.clone());
        attr(&mut n, "SpaceBetween", num(f.space_between));
        attr(&mut n, "Spacer", num(f.spacer));
        let mut props: Vec<Node> = Vec::new();
        let mut prop = |name: &str, ty: &str, text: String| {
            props.push(Node {
                tag: name.to_string(),
                attrs: vec![("type".into(), ty.into())],
                text: Some(text),
                ..Node::default()
            })
        };
        let numbering = match f.numbering {
            0xCA07 => Some("Arabic"),
            0xCA0A => Some("Symbols"),
            0xCA58 => Some("Asterisks"),
            _ => None,
        };
        if let Some(v) = numbering {
            prop("FootnoteNumberingStyle", "enumeration", v.into());
        }
        let restart = match f.restart {
            0 => Some("DontRestart"),
            1 => Some("PageRestart"),
            _ => None,
        };
        if let Some(v) = restart {
            prop("RestartNumbering", "enumeration", v.into());
        }
        let prefix_suffix = match f.prefix_suffix {
            0 => Some("NoPrefixSuffix"),
            1 => Some("PrefixSuffixReference"),
            3 => Some("PrefixSuffixBoth"),
            _ => None,
        };
        if let Some(v) = prefix_suffix {
            prop("ShowPrefixSuffix", "enumeration", v.into());
        }
        let marker = match f.marker {
            (1, 0) => Some("SuperscriptMarker"),
            (0, 0) => Some("NormalMarker"),
            (0, 1) => Some("RubyMarker"),
            _ => None,
        };
        if let Some(v) = marker {
            prop("MarkerPositioning", "enumeration", v.into());
        }
        if let Some(t) = &f.tail {
            attr(&mut n, "NoSplitting", bool_text(t.no_splitting));
            for (r, name) in t.rules.iter().zip(["Rule", "ContinuingRule"]) {
                attr(&mut n, &format!("{name}On"), bool_text(r.on));
                attr(&mut n, &format!("{name}LineWeight"), num(r.weight));
                attr(&mut n, &format!("{name}Tint"), num(r.tint));
                attr(&mut n, &format!("{name}GapTint"), num(r.gap_tint));
                attr(&mut n, &format!("{name}LeftIndent"), num(r.left_indent));
                attr(&mut n, &format!("{name}Width"), num(r.width));
                attr(&mut n, &format!("{name}Offset"), num(r.offset));
                if let Some((_, s)) = STROKE_TYPES
                    .iter()
                    .chain(TABLE_STROKE_TYPES)
                    .find(|(k, _)| *k == r.stroke)
                {
                    prop(
                        &format!("{name}Type"),
                        "object",
                        format!("StrokeStyle/$ID/{s}"),
                    );
                }
                for (uid, suffix) in [(r.color, "Color"), (r.gap_color, "GapColor")] {
                    if let Some(sw) = self.doc.swatches.get(&uid) {
                        prop(&format!("{name}{suffix}"), "object", sw.clone());
                    }
                }
            }
        }
        if !props.is_empty() {
            n.children.push(Node {
                tag: "Properties".into(),
                children: props,
                ..Node::default()
            });
        }
        n
    }

    /// The attributes of `IndexOptions` (`docs/format/preferences.md`,
    /// index options). A style is named by its name alone, among the
    /// styles outside style groups; without such a style, IDML names the
    /// root style.
    fn index_options(&self, o: &crate::model::prefs::IndexOptions) -> Vec<(&'static str, String)> {
        let style = |name: &str, paragraph: bool| match self.doc.styles.values().find(|s| {
            s.paragraph == paragraph
                && s.name == name
                && self.group_path.get(&s.uid).is_none_or(|p| p.is_empty())
        }) {
            Some(s) => self.style_ref(Some(s.uid), paragraph),
            None if paragraph => "ParagraphStyle/$ID/[No paragraph style]".into(),
            None => "CharacterStyle/$ID/[No character style]".into(),
        };
        let mut out = vec![
            ("Title", o.title.clone()),
            ("TitleStyle", style(&o.title_style, true)),
            ("ReplaceExistingIndex", o.replace_existing.to_string()),
            ("IncludeBookDocuments", o.include_book_documents.to_string()),
            (
                "IncludeSectionHeadings",
                o.include_section_headings.to_string(),
            ),
        ];
        for (name, text) in [
            "FollowingTopicSeparator",
            "BetweenPageNumbersSeparator",
            "BetweenEntriesSeparator",
            "BeforeCrossReferenceSeparator",
            "PageRangeSeparator",
            "EntryEndSeparator",
        ]
        .into_iter()
        .zip(&o.separators)
        {
            // IDML writes the en dash of the page range as `^=`.
            let text = if name == "PageRangeSeparator" {
                text.replace('\u{2013}', "^=")
            } else {
                text.clone()
            };
            out.push((name, text));
        }
        for (i, (name, s)) in [
            "Level1Style",
            "Level2Style",
            "Level3Style",
            "Level4Style",
            "SectionHeadingStyle",
            "PageNumberStyle",
            "CrossReferenceStyle",
            "CrossReferenceTopicStyle",
        ]
        .into_iter()
        .zip(&o.styles)
        .enumerate()
        {
            out.push((name, style(s, i < 5)));
        }
        out
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
            .keys()
            .filter_map(|&u| self.object_style_ref(u))
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
        for (tag, name, data) in &prefs.print_records {
            let mut text = base64_lines(data);
            // IDML ends a last line of full length with a line feed.
            if text.rsplit('\n').next().is_some_and(|l| l.len() == 76) {
                text.push('\n');
            }
            let i = ours_of(&mut ours, tag);
            ours[i].attrs.push((name.to_string(), builtin_key(&text)));
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
                PrefProp::Nodes(children) => {
                    n.children = children.iter().map(pref_node).collect();
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
        // Default styles (preferences.md, default styles).
        for (uid, paragraph, name) in [
            (prefs.default_styles[0], true, "AppliedParagraphStyle"),
            (prefs.default_styles[1], false, "AppliedCharacterStyle"),
        ] {
            let value = match uid {
                Some(u)
                    if self
                        .doc
                        .styles
                        .get(&u)
                        .is_some_and(|s| s.paragraph == paragraph) =>
                {
                    Some(self.style_ref(Some(u), paragraph))
                }
                Some(0) if paragraph => Some("ParagraphStyle/$ID/[No paragraph style]".into()),
                Some(0) => Some("CharacterStyle/$ID/[No character style]".into()),
                _ => None,
            };
            if let Some(v) = value {
                let i = ours_of(&mut ours, "TextDefault");
                ours[i].attrs.push((name.into(), v));
            }
        }
        if let Some(uids) = prefs.default_object_styles {
            for (u, name) in uids.into_iter().zip([
                "AppliedGraphicObjectStyle",
                "AppliedTextObjectStyle",
                "AppliedGridObjectStyle",
            ]) {
                if let Some(r) = self.object_style_ref(u) {
                    let i = ours_of(&mut ours, "PageItemDefault");
                    ours[i].attrs.push((name.into(), r));
                }
            }
        }
        for (tag, g, count) in &prefs.grids {
            let i = ours_of(&mut ours, tag);
            let n = &mut ours[i];
            n.attrs.push(("FontStyle".into(), g.font_style.clone()));
            let [size, character_aki, line_aki, h_scale, v_scale] = g.numbers;
            for (name, v) in [
                ("PointSize", size),
                ("CharacterAki", character_aki),
                ("LineAki", line_aki),
                ("HorizontalScale", h_scale * 100.0),
                ("VerticalScale", v_scale * 100.0),
            ] {
                n.attrs.push((name.into(), num(v)));
            }
            if let Some(c) = count {
                n.attrs.push(("CharacterCountSize".into(), num(*c)));
            }
            if let Some(f) = self.doc.fonts.get(&g.font) {
                let p = prop_node(&("AppliedFont", "string", self.family_names(f).0.into()));
                match n.children.iter_mut().find(|c| c.tag == "Properties") {
                    Some(props) => props.children.push(p),
                    None => n.children.insert(
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
        if let Some(o) = &prefs.index_options {
            let i = ours_of(&mut ours, "IndexOptions");
            ours[i].attrs.extend(
                self.index_options(o)
                    .into_iter()
                    .map(|(k, v)| (k.to_string(), v)),
            );
        }
        ours.push(self.footnote_option());
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
