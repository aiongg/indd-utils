//! Stories: paragraph and character ranges, anchored objects, tables,
//! notes, XML markers; and `XML/BackingStory.xml` and `XML/Tags.xml`.
//!
//! Evidence: `docs/format/objects.md` (stories), `attributes.md`,
//! `tables.md` and `xml.md`.

use super::*;

/// While writing text with XML markers: whether a character range is
/// open, and the XML elements open inside it and around it.
#[derive(Default)]
pub(super) struct TextState {
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

impl Writer<'_> {
    /// The start tag of a `Story` or `XmlStory` with its attributes.
    pub(super) fn story_start(&self, x: &mut Xml, tag: &str, s: &Story) {
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
        // The TOC style that made the story, if any (objects.md).
        let toc = s
            .toc_style
            .and_then(|u| self.doc.toc_styles.iter().find(|t| t.uid == u));
        x.attr(
            "AppliedTOCStyle",
            toc.map_or("n".into(), Self::toc_style_ref),
        );
    }

    pub(super) fn xml_element_start(x: &mut Xml, e: &XmlElement) {
        x.start("XMLElement")
            .attr("Self", &e.name)
            .attr("MarkupTag", format!("XMLTag/{}", self_name(&e.tag)));
        if let Some(c) = e.content {
            x.attr("XMLContent", uref(Some(c)));
        }
    }

    pub(super) fn story(&self, s: &Story) -> String {
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
    pub(super) fn csr_start(&self, x: &mut Xml, r: &TextRun) {
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
    pub(super) fn text_ranges(&self, x: &mut Xml, runs: &[TextRun], story: &Story, scope: &str) {
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
    pub(super) fn run_content(
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

    pub(super) fn table(&self, x: &mut Xml, t: &Table, story: &Story, scope: &str) {
        let id = format!("{scope}i{:x}", t.uid);
        let rows = t.rows.len() as u32;
        x.start("Table")
            .attr("Self", &id)
            .attr("HeaderRowCount", t.header_rows.to_string())
            .attr("FooterRowCount", t.footer_rows.to_string())
            .attr(
                "BodyRowCount",
                rows.saturating_sub(t.header_rows.saturating_add(t.footer_rows))
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

    pub(super) fn backing_story(&self) -> String {
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

    pub(super) fn tags(&self) -> String {
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
