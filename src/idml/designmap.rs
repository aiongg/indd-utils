//! `designmap.xml`: the document element and the document-level elements
//! written there (languages, colour groups, TOC styles, cross-reference
//! formats, hyperlinks, bookmarks, text variables, index options, sections).
//!
//! Evidence: `docs/format/objects.md`, `hyperlinks.md`,
//! `cross-references.md`, `text-variables.md` and `idml-values.md`.

use super::*;

/// IDML `HeaderType` of an index group by its name and header variant.
/// See `docs/format/objects.md`, index sort options.
pub(super) fn index_header_type(group: &str, variant: u16) -> Option<&'static str> {
    Some(match (group, variant) {
        ("kIndexGroup_Alphabet", 0) => "BasicLatin",
        ("kIndexGroup_Alphabet", 3) => "Spanish",
        ("kIndexGroup_Alphabet", 5) => "Czech",
        ("kWRIndexGroup_CyrillicAlphabet", 2) => "Russian",
        ("kIndexGroup_Kana", 0) => "HiraganaAll",
        ("kIndexGroup_Chinese", 0) => "ChinesePinyin",
        ("kIndexGroup_Korean", 0) => "KoreanConsonant",
        (
            "kIndexGroup_Symbol"
            | "kIndexGroup_Numeric"
            | "kWRIndexGroup_GreekAlphabet"
            | "kWRIndexGroup_ArabicAlphabet"
            | "kWRIndexGroup_HebrewAlphabet",
            0,
        ) => "Nothing",
        _ => return None,
    })
}

impl Writer<'_> {
    pub(super) fn designmap(&self, name: &str) -> String {
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
            .attr("StoryList", stories.join(" "));
        // `Name` from DOM 13 (idml-values.md, document attributes).
        if doc.version.major >= 13 {
            x.attr("Name", name);
        }
        x.attr("ZeroPoint", "0 0");
        if let Some(l) = doc.active_layer {
            x.attr("ActiveLayer", uref(Some(l)));
        }
        // Colour settings (`model::prefs`).
        for v in doc.prefs.values.iter().filter(|v| v.element == "Document") {
            x.attr(v.name, &v.value);
        }
        // `false` in every corpus IDML (docs/format/idml-values.md).
        x.attr("AccurateLABSpots", "false");
        if !doc.label.is_empty() {
            x.start("Properties").start("Label");
            for (k, v) in &doc.label {
                x.empty("KeyValuePair", &[("Key", k.clone()), ("Value", v.clone())]);
            }
            x.end();
            x.end();
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
                let name = t.name.idml();
                x.start(tag)
                    .attr("Self", format!("{tag}/{}", self_name(&name)))
                    .attr("Name", &name);
                if let Some([begin, end, _, hanging, together]) = &t.chars {
                    x.attr("CantBeginLineChars", begin)
                        .attr("CantEndLineChars", end)
                        .attr("HangingPunctuationChars", hanging)
                        .attr("CantBeSeparatedChars", together);
                }
                if let Some(m) = &t.custom_mojikumi {
                    custom_mojikumi(&mut x, m);
                }
                x.end();
            }
        }
        x.empty("idPkg:Styles", &[("src", "Resources/Styles.xml".into())]);
        // The numbering lists, the default first (objects.md, numbering
        // lists). Without them, the default list every corpus IDML has
        // (idml-values.md), which the root paragraph style refers to.
        let default = [crate::model::NumberingList {
            uid: 0,
            name: "$ID/[Default]".into(),
            across_stories: false,
            across_documents: false,
        }];
        let lists = if doc.numbering_lists.is_empty() {
            &default[..]
        } else {
            &doc.numbering_lists[..]
        };
        for l in lists {
            x.empty(
                "NumberingList",
                &[
                    ("Self", format!("NumberingList/{}", self_name(&l.name))),
                    ("Name", l.name.clone()),
                    ("ContinueNumbersAcrossStories", l.across_stories.to_string()),
                    (
                        "ContinueNumbersAcrossDocuments",
                        l.across_documents.to_string(),
                    ),
                ],
            );
        }
        let major = doc.version.major;
        // Elements every IDML of the version has, with the values they
        // all have (idml-values.md), in the order of the IDML files.
        let singleton = |x: &mut Xml, tag: &str| {
            // Every IDML from 7.5 has the tagged PDF settings, none of 7.0
            // (preferences.md, tagged PDF).
            let present = values::present(&format!("Document/{tag}"), major).or_else(|| {
                (tag == "TaggedPDFPreference" && (major, doc.version.minor) >= (7, 5)).then(|| {
                    Node {
                        tag: tag.to_string(),
                        ..Node::default()
                    }
                })
            });
            if let Some(n) = present {
                // Values read from the INDD (`model::prefs`) first.
                let mut ours = match tag {
                    "EndnoteOption" => self.endnote_option(),
                    _ => None,
                }
                .unwrap_or_else(|| Node {
                    tag: n.tag.clone(),
                    ..Node::default()
                });
                for v in doc.prefs.values.iter().filter(|v| v.element == tag) {
                    ours.attrs.push((v.name.to_string(), v.value.clone()));
                }
                let colors: Vec<Node> = doc
                    .prefs
                    .colors
                    .iter()
                    .filter(|c| c.0 == tag)
                    .filter_map(|&(_, name, rgb)| ui_color_property(name, rgb))
                    .collect();
                if !colors.is_empty() {
                    ours.children.push(Node {
                        tag: "Properties".into(),
                        children: colors,
                        ..Node::default()
                    });
                }
                ours.merge(&n);
                ours.write(x);
            }
        };
        // A named grid without settings of its own has those of the
        // document's layout grid (objects.md, named grids).
        let layout_grid = doc
            .prefs
            .grids
            .iter()
            .find(|(tag, ..)| *tag == "LayoutGridDataInformation")
            .map(|(_, g, _)| g);
        for ng in &doc.named_grids {
            x.start("NamedGrid")
                .attr("Self", Self::named_grid_self(ng))
                .attr("Name", Self::named_grid_name(ng));
            if let Some(g) = ng.grid.as_ref().or(layout_grid) {
                self.grid_data(&mut x, g);
            }
            x.end();
        }
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
        if let Some(index) = &doc.index {
            index_xml(&mut x, index);
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
                    Some(f) => self.family_names(f).0,
                    None => continue,
                },
            };
            x.start("ABullet")
                .attr("Self", format!("dABullet{i:x}"))
                .attr("CharacterType", *kind)
                .attr("CharacterValue", b.value.to_string());
            Self::properties(
                &mut x,
                &[
                    ("BulletsFont", "string", font.into()),
                    ("BulletsFontStyle", "string", b.font_style.idml().into()),
                ],
            );
            x.end();
        }
        // Index sort options: the groups in their order, with the header
        // type each group and header variant has in IDML (objects.md).
        for (i, (name, include, variant)) in doc.index_groups.iter().enumerate() {
            x.start("IndexingSortOption")
                .attr("Self", format!("dIndexingSortOptionn{name}"))
                .attr("Name", builtin_key(name))
                .attr("Include", include.to_string())
                .attr("Priority", i.to_string());
            if let Some(h) = index_header_type(name, *variant) {
                x.attr("HeaderType", h);
            }
            x.end();
        }
        // InCopy assignments: the values every IDML has on them; the name
        // depends on the computer that exported the IDML (objects.md).
        if let Some(n) = values::element("Assignment", doc.version.major) {
            for &uid in &doc.assignments {
                let mut a = n.clone();
                a.attrs.insert(0, ("Self".into(), uref(Some(uid))));
                a.write(&mut x);
            }
        }
        x.end();
        x.finish()
    }

    /// The IDML reference of a TOC style.
    fn named_grid_name(ng: &crate::model::NamedGrid) -> String {
        if ng.builtin {
            builtin_key(&ng.name)
        } else {
            ng.name.clone()
        }
    }

    fn named_grid_self(ng: &crate::model::NamedGrid) -> String {
        format!("NamedGrid/{}", self_name(&Self::named_grid_name(ng)))
    }

    /// `AppliedNamedGrid` of a story or object style: `n` for none, the
    /// grid's `Self` for a grid of the document, `None` otherwise
    /// (objects.md, named grids).
    pub(super) fn named_grid_ref(&self, applied: Option<Option<u32>>) -> Option<String> {
        match applied? {
            None => Some("n".into()),
            Some(uid) => self
                .doc
                .named_grids
                .iter()
                .find(|g| g.uid == uid)
                .map(Self::named_grid_self),
        }
    }

    pub(super) fn toc_style_ref(t: &crate::model::TocStyle) -> String {
        let name = if t.builtin {
            builtin_key(&t.name)
        } else {
            t.name.clone()
        };
        format!("TOCStyle/{}", self_name(&name))
    }

    /// Table of contents styles, without their entries. See
    /// `docs/format/objects.md`, table of contents styles.
    pub(super) fn toc_styles(&self, x: &mut Xml) {
        for t in &self.doc.toc_styles {
            let name = if t.builtin {
                builtin_key(&t.name)
            } else {
                t.name.clone()
            };
            x.start("TOCStyle")
                .attr("Self", Self::toc_style_ref(t))
                .attr(
                    "TitleStyle",
                    self.style_ref((t.title_style != 0).then_some(t.title_style), true),
                )
                .attr("Title", &t.title)
                .attr("Name", &name);
            let major = self.doc.version.major;
            let flag = |i: usize| t.flags.get(i).copied();
            match flag(0) {
                Some(0) => {
                    x.attr("SetStoryDirection", "Horizontal");
                }
                Some(1) => {
                    x.attr("SetStoryDirection", "Vertical");
                }
                _ => {}
            }
            match flag(1) {
                Some(0) => {
                    x.attr("NumberedParagraphs", "IncludeFullParagraph");
                }
                Some(2) => {
                    x.attr("NumberedParagraphs", "ExcludeNumbers");
                }
                _ => {}
            }
            if major >= 9
                && let Some(v) = flag(2)
            {
                x.attr("MakeAnchor", (v != 0).to_string());
            }
            if major >= 13
                && let Some(v) = flag(3)
            {
                x.attr("RemoveForcedLineBreak", (v != 0).to_string());
            }
            x.attrs_missing(self.observed("TOCStyle").iter());
            x.attrs_missing(values::when_written("TOCStyle", major).iter());
            x.end();
        }
    }

    /// The document's languages, in UID order. See
    /// `docs/format/objects.md`, languages.
    pub(super) fn languages(&self, x: &mut Xml) {
        let mut list: Vec<_> = self.doc.language_list.iter().collect();
        list.sort_by_key(|l| l.uid);
        for l in list {
            // The language without one is `Neutral` in the INDD.
            let neutral = l.name == "Neutral";
            let name = |s: &str| {
                if neutral {
                    "$ID/[No Language]".to_string()
                } else {
                    builtin_key(s)
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
    pub(super) fn color_groups(&self, x: &mut Xml) {
        let v = self.doc.version;
        for (i, g) in self.doc.color_groups.iter().enumerate() {
            // Named by UID before 11.3 (objects.md, colour groups).
            let id = if (v.major, v.minor) < (11, 3) {
                uref(Some(g.uid))
            } else {
                format!("ColorGroup/{}", self_name(&g.name))
            };
            x.start("ColorGroup")
                .attr("Self", id)
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
    pub(super) fn cross_reference_formats(&self, x: &mut Xml) {
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
                x.attr("CustomText", b.text.idml());
                if b.zero_fields {
                    x.attr("AppliedDelimiter", "$ID/")
                        .attr("IncludeDelimiter", "false");
                }
                x.end();
            }
            x.end();
        }
    }

    /// `EndnoteOption` from chunk 0x2261E (footnotes.md).
    fn endnote_option(&self) -> Option<Node> {
        let mut n = Node {
            tag: "EndnoteOption".into(),
            ..Node::default()
        };
        let Some(e) = self.doc.prefs.endnotes.as_ref() else {
            // Without the chunk the styles are the defaults (149 of 149).
            // The separator and the marker position follow the language
            // of the last session; the title differs between languages and
            // versions (footnotes.md, endnote options).
            n.attrs
                .push(("EndnoteMarkerStyle".into(), self.style_ref(None, false)));
            n.attrs
                .push(("EndnoteTextStyle".into(), self.style_ref(None, true)));
            if let Some(japanese) = self.japanese_session() {
                let (separator, position) = if japanese {
                    ("\u{3000}", "RubyMarker")
                } else {
                    ("\t", "SuperscriptMarker")
                };
                n.attrs
                    .push(("EndnoteSeparatorText".into(), separator.into()));
                n.children.push(Node {
                    tag: "Properties".into(),
                    children: vec![Node {
                        tag: "EndnoteMarkerPositioning".into(),
                        attrs: vec![("type".into(), "enumeration".into())],
                        text: Some(position.into()),
                        ..Node::default()
                    }],
                    ..Node::default()
                });
            }
            return Some(n);
        };
        let mut attr = |k: &str, v: String| n.attrs.push((k.to_string(), v));
        attr("EndnoteTitle", e.title.clone());
        attr(
            "EndnoteTitleStyle",
            self.style_ref(Some(e.title_style), true),
        );
        attr("StartEndnoteNumberAt", e.start_at.to_string());
        attr(
            "EndnoteMarkerStyle",
            self.style_ref(Some(e.marker_style), false),
        );
        attr("EndnoteTextStyle", self.style_ref(Some(e.text_style), true));
        attr("EndnoteSeparatorText", e.separator.clone());
        let mut props = Vec::new();
        let mut prop = |name: &str, v: &str| {
            props.push(Node {
                tag: name.to_string(),
                attrs: vec![("type".into(), "enumeration".into())],
                text: Some(v.to_string()),
                ..Node::default()
            })
        };
        if e.numbering == 0xCA07 {
            prop("EndnoteNumberingStyle", "Arabic");
        }
        if e.restart == 0 {
            prop("RestartEndnoteNumbering", "Continuous");
        }
        match e.positioning {
            1 => prop("EndnoteMarkerPositioning", "SuperscriptMarker"),
            3 => prop("EndnoteMarkerPositioning", "RubyMarker"),
            _ => {}
        }
        // The last fields hold one value in every sample: the generated
        // value file supplies it (`idml-values.md`).
        n.children.push(Node {
            tag: "Properties".into(),
            children: props,
            ..Node::default()
        });
        Some(n)
    }

    /// Hyperlink text sources written in the stories.
    pub(super) fn written_sources(&self) -> std::collections::HashSet<u32> {
        self.doc
            .stories
            .iter()
            .flat_map(|s| &s.sources)
            .map(|r| r.source)
            .filter(|u| self.doc.text_sources.contains_key(u))
            .collect()
    }

    /// UIDs of the text and paragraph destinations written in stories.
    pub(super) fn written_text_destinations(&self) -> std::collections::HashSet<u32> {
        self.doc
            .stories
            .iter()
            .flat_map(|s| s.text_destinations.values().flatten())
            .map(|d| d.uid)
            .collect()
    }

    /// UIDs of the page items written: on spreads and master spreads, in
    /// groups and anchored in stories.
    pub(super) fn written_items(&self) -> std::collections::HashSet<u32> {
        let doc = self.doc;
        let mut out = std::collections::HashSet::new();
        let mut stack: Vec<&PageItem> = doc
            .spreads
            .iter()
            .chain(&doc.master_spreads)
            .flat_map(|s| &s.items)
            .chain(
                doc.stories
                    .iter()
                    .flat_map(|s| s.anchors.values().flatten()),
            )
            .collect();
        while let Some(item) = stack.pop() {
            out.insert(item.uid);
            stack.extend(&item.children);
        }
        out
    }

    /// Destinations, hyperlinks and bookmarks, in schema order. See
    /// docs/format/hyperlinks.md.
    pub(super) fn hyperlinks(&self, x: &mut Xml) {
        let doc = self.doc;
        // Text and paragraph destinations are written in the stories, at
        // their position; only those found there can be referred to.
        let in_text = self.written_text_destinations();
        let mut dests: Vec<_> = doc
            .destinations
            .iter()
            .filter(|d| {
                !matches!(d.kind, DestinationKind::Text | DestinationKind::Paragraph)
                    || in_text.contains(&d.uid)
            })
            .collect();
        // Schema order: page, URL, external page destinations; each sorted
        // by key.
        dests.sort_by_key(|d| {
            let rank = match d.kind {
                DestinationKind::Page { .. } => 0,
                DestinationKind::Url { .. } => 1,
                _ => 2,
            };
            (rank, d.key)
        });
        let view_setting = |x: &mut Xml, view: u32, zoom: Option<f64>| {
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
        };
        for d in &dests {
            let mut bounds = None;
            match &d.kind {
                DestinationKind::Page {
                    page,
                    zoom,
                    view,
                    bounds: b,
                } => {
                    // NameManually is true on every corpus page
                    // destination (hyperlinks.md).
                    x.start("HyperlinkPageDestination")
                        .attr("Self", d.reference())
                        .attr("Name", &d.name)
                        .attr("NameManually", "true")
                        .attr("DestinationPage", uref(Some(*page)));
                    view_setting(x, *view, *zoom);
                    bounds = *b;
                }
                DestinationKind::Url { url } => {
                    x.start("HyperlinkURLDestination")
                        .attr("Self", d.reference())
                        .attr("Name", &d.name)
                        .attr("DestinationURL", url);
                }
                DestinationKind::ExternalPage {
                    page_index,
                    zoom,
                    view,
                    bounds: b,
                    file_name,
                    ..
                } => {
                    x.start("HyperlinkExternalPageDestination")
                        .attr("Self", d.reference());
                    // The name is not stored; IDML makes it from the file
                    // name and the page (shown for the Fixed view only).
                    if let (0, Some(f)) = (view, file_name) {
                        x.attr("Name", format!("{f} - Page {} [Fixed]", page_index + 1));
                    }
                    x.attr("DestinationPageIndex", (page_index + 1).to_string());
                    view_setting(x, *view, *zoom);
                    bounds = *b;
                }
                DestinationKind::Text | DestinationKind::Paragraph => continue,
            }
            x.attr("Hidden", d.hidden.to_string())
                .attr_opt("DestinationUniqueKey", d.key.map(|k| k.to_string()));
            if let Some([left, top, right, bottom]) = bounds {
                // IDML writes the unset bounds (1e256) in exponent form.
                let n = |v: f64| if v == 1e256 { "1e+256".into() } else { num(v) };
                x.start("Properties")
                    .start("ViewBounds")
                    .attr("Top", n(top))
                    .attr("Left", n(left))
                    .attr("Bottom", n(bottom))
                    .attr("Right", n(right))
                    .end()
                    .end();
            }
            x.end();
        }
        let items = self.written_items();
        let mut sources = self.written_sources();
        for s in &doc.page_item_sources {
            if !items.contains(&s.item) {
                continue;
            }
            x.start("HyperlinkPageItemSource")
                .attr("Self", uref(Some(s.uid)))
                .attr("Name", &s.name)
                .attr("SourcePageItem", uref(Some(s.item)))
                .attr("Hidden", s.hidden.to_string())
                .end();
            sources.insert(s.uid);
        }
        let mut links: Vec<_> = doc.hyperlinks.iter().collect();
        links.sort_by_key(|h| h.uid);
        for h in links {
            // A hyperlink into another document has a destination that is
            // not converted: IDML writes it as a list, which is left out.
            // Otherwise the destination with the same key; `n` for a
            // hyperlink without one (hyperlinks.md).
            let dest = if h.other_document {
                None
            } else if let Some(d) = dests.iter().find(|d| match h.destination {
                Some(u) => d.uid == u,
                None => h.key.is_some() && d.key == h.key,
            }) {
                Some(d.reference())
            } else if h.kind_is_none() {
                Some("n".to_string())
            } else {
                continue;
            };
            if h.source == 0 || !sources.contains(&h.source) {
                continue;
            }
            x.start("Hyperlink")
                .attr("Self", uref(Some(h.uid)))
                .attr("Name", &h.name)
                .attr("Source", uref(Some(h.source)));
            if h.appearance_known {
                x.attr("Visible", "false");
                match h.highlight {
                    Some(0) => {
                        x.attr("Highlight", "None");
                    }
                    Some(1) => {
                        x.attr("Highlight", "Invert");
                    }
                    _ => {}
                }
                x.attr("Width", "Thin").attr("BorderStyle", "Solid");
            }
            x.attr("Hidden", h.hidden.to_string())
                .attr_opt("DestinationUniqueKey", h.key.map(|k| k.to_string()));
            x.start("Properties");
            // Black on every corpus hyperlink with the known appearance,
            // apart from those to a page of another document.
            if h.appearance_known && !h.to_external_page() {
                x.start("BorderColor")
                    .attr("type", "enumeration")
                    .text("Black")
                    .end();
            }
            if let Some(d) = dest {
                x.start("Destination").attr("type", "object").text(&d).end();
            }
            x.end().end();
        }
        let top = doc
            .bookmark_order
            .iter()
            .filter_map(|u| doc.bookmarks.get(u))
            .filter(|b| !doc.bookmarks.contains_key(&b.parent));
        for b in top {
            self.bookmark(x, b, 0, &in_text);
        }
    }

    pub(super) fn bookmark(
        &self,
        x: &mut Xml,
        b: &crate::model::Bookmark,
        depth: usize,
        in_text: &std::collections::HashSet<u32>,
    ) {
        let doc = self.doc;
        let Some(dest) = doc.destinations.iter().find(|d| d.uid == b.destination) else {
            return;
        };
        if matches!(
            dest.kind,
            DestinationKind::Text | DestinationKind::Paragraph
        ) && !in_text.contains(&dest.uid)
        {
            return;
        }
        x.start("Bookmark")
            .attr("Self", uref(Some(b.uid)))
            .attr("Name", &b.name)
            .attr("Destination", dest.reference());
        if depth < 64 {
            for c in b.children.iter().filter_map(|u| doc.bookmarks.get(u)) {
                self.bookmark(x, c, depth + 1, in_text);
            }
        }
        x.end();
    }

    /// `TextVariable` elements, sorted by name as in every corpus IDML.
    /// See docs/format/text-variables.md.
    pub(super) fn text_variables(&self, x: &mut Xml) {
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
    pub(super) fn variable_preference(&self, x: &mut Xml, v: &TextVariable) {
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
    pub(super) fn variable_instance(&self, x: &mut Xml, v: &Instance) {
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
}

/// The `Self` of a topic: its parent's `Self`, `Topicn` and its name
/// (`docs/format/index.md`).
fn topic_self(parent: &str, t: &crate::model::index::Topic) -> String {
    format!("{parent}Topicn{}", t.name)
}

/// The `Index` element and its topics.
fn index_xml(x: &mut Xml, index: &crate::model::index::Index) {
    fn topic(x: &mut Xml, parent: &str, t: &crate::model::index::Topic) {
        let me = topic_self(parent, t);
        x.start("Topic")
            .attr("Self", &me)
            .attr("SortOrder", &t.sort_order)
            .attr("Name", &t.name);
        for c in &t.children {
            topic(x, &me, c);
        }
        x.end();
    }
    let me = uref(Some(index.uid));
    x.start("Index").attr("Self", &me);
    for t in &index.topics {
        topic(x, &me, t);
    }
    x.end();
}

/// The `Self` of the topic of each page reference, by page reference UID.
pub(super) fn topic_refs(doc: &Document) -> std::collections::HashMap<u32, String> {
    fn walk(
        out: &mut std::collections::HashMap<u32, String>,
        parent: &str,
        t: &crate::model::index::Topic,
    ) {
        let me = topic_self(parent, t);
        for &r in &t.refs {
            out.insert(r, me.clone());
        }
        for c in &t.children {
            walk(out, &me, c);
        }
    }
    let mut out = std::collections::HashMap::new();
    if let Some(index) = &doc.index {
        let me = uref(Some(index.uid));
        for t in &index.topics {
            walk(&mut out, &me, t);
        }
    }
    out
}

/// `BasedOnMojikumiSet` and the spacing entries of a custom mojikumi
/// table (objects.md, custom mojikumi tables). A code without a known
/// table name leaves the attribute out.
fn custom_mojikumi(x: &mut Xml, m: &crate::model::cjk::MojikumiSettings) {
    let based_on = match m.based_on {
        0 => Some("Nothing"),
        n => super::kind::builtin_cjk_table(&format!("kMojikumiDefaultName{n}")),
    };
    if let Some(b) = based_on {
        x.attr("BasedOnMojikumiSet", b);
    }
    x.start("Properties").start("OverrideMojikumiAkiList");
    for e in &m.entries {
        x.empty(
            "OverrideMojikumiAkiType",
            &[
                ("TargetMojikumiClass", e.target_class.to_string()),
                ("SideMojikumiClass", e.side_class.to_string()),
                ("SideIsAfterTarget", e.side_is_after_target.to_string()),
                ("Minimum", num(e.minimum)),
                ("Desired", num(e.desired)),
                ("Maximum", num(e.maximum)),
                ("CompressionPriority", e.compression_priority.to_string()),
                ("AkiDoesNotFloat", e.aki_does_not_float.to_string()),
            ],
        );
    }
    x.end();
    x.end();
}
