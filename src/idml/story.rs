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
    /// A character written without its `Change` and deleted text, which
    /// were written before the text source that starts there.
    bare: Option<usize>,
}

/// The insertion entry of a change run of inserted text that starts where
/// a text source of `len` characters starts, at `pos`, and ends before
/// the source does.
fn insertion_at_source(story: &Story, pos: usize, len: usize) -> Option<&ChangeEntry> {
    let i = story.changes.partition_point(|c| c.start < pos);
    let c = story
        .changes
        .get(i)
        .filter(|c| c.start == pos && c.len < len)?;
    c.entries.iter().find(|e| e.is_insertion())
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
        // `UserText` from version 11.2, with the value every IDML has
        // (docs/format/idml-values.md, stories).
        let v = &self.doc.version;
        let major = v.major;
        if (major, v.minor) >= (11, 2) {
            x.attrs_missing(values::when_written(tag, major).iter());
        }
        // From DOM 13 (footnotes.md).
        if major >= 13 {
            x.attr("IsEndnoteStory", s.is_endnote.to_string());
        }
        // Without chunk 0xA44C the title is `$ID/` (objects.md, stories).
        let title = s.settings.title.as_deref().unwrap_or("$ID/");
        x.attr("StoryTitle", title);
        // The TOC style that made the story, if any (objects.md).
        let toc = s
            .toc_style
            .and_then(|u| self.doc.toc_styles.iter().find(|t| t.uid == u));
        x.attr(
            "AppliedTOCStyle",
            toc.map_or("n".into(), Self::toc_style_ref),
        );
        if let Some(g) = self.named_grid_ref(s.settings.named_grid) {
            x.attr("AppliedNamedGrid", g);
        }
        // The values every IDML has on every story (idml-values.md).
        x.attrs_missing(self.observed(tag).iter());
    }

    pub(super) fn xml_element_start(x: &mut Xml, e: &XmlElement) {
        x.start("XMLElement")
            .attr("Self", &e.name)
            .attr("MarkupTag", format!("XMLTag/{}", self_name(&e.tag)));
        if let Some(c) = e.content {
            x.attr("XMLContent", uref(Some(c)));
        }
        // The attributes come first in the element (xml.md, attributes).
        for (name, value) in &e.attributes {
            x.empty(
                "XMLAttribute",
                &[
                    ("Self", format!("{}XMLAttributen{name}", e.name)),
                    ("Name", name.clone()),
                    ("Value", value.clone()),
                ],
            );
        }
    }

    pub(super) fn story(&self, s: &Story) -> String {
        let mut x = Xml::new();
        self.package_root(&mut x, "Story");
        self.story_start(&mut x, "Story", s);
        // Optical margin and direction from the story's chunks; without
        // them, the values every story without the chunks has (objects.md,
        // stories).
        let (size, alignment) = match s.settings.optical_margin {
            Some((size, 1)) => (num(size), Some("true")),
            Some((size, 0)) => (num(size), Some("false")),
            Some((size, _)) => (num(size), None),
            None => ("12".into(), Some("false")),
        };
        let direction = match s.settings.direction {
            Some(1) => Some("RightToLeftDirection"),
            None => Some("LeftToRightDirection"),
            Some(_) => None,
        };
        let mut prefs: Vec<(&str, String)> = Vec::new();
        if let Some(a) = alignment {
            prefs.push(("OpticalMarginAlignment", a.into()));
        }
        prefs.push(("OpticalMarginSize", size));
        // Frame type and orientation from chunk 0x2EE; without it, the
        // values of every story without the chunk. For another orientation
        // code, the orientation of the story's frames (objects.md, story
        // settings).
        let (frame_type, orientation) = story_layout(s.settings.layout, s.orientation);
        if let Some(t) = frame_type {
            prefs.push(("FrameType", t.into()));
        }
        match orientation {
            Some(Orientation::Vertical) => prefs.push(("StoryOrientation", "Vertical".into())),
            Some(Orientation::Horizontal) => prefs.push(("StoryOrientation", "Horizontal".into())),
            None => {}
        }
        if let Some(d) = direction {
            prefs.push(("StoryDirection", d.into()));
        }
        x.start("StoryPreference");
        for (k, v) in &prefs {
            x.attr(k, v);
        }
        x.attrs_missing(self.observed("Story/StoryPreference").iter());
        x.end();
        // The layout grid of a frame grid story (objects.md, frame grids).
        if frame_type == Some("FrameGridType")
            && let Some(g) = &s.settings.grid
        {
            self.grid_data(&mut x, g);
        }
        x.start("InCopyExportOption");
        x.attrs_missing(self.observed("Story/InCopyExportOption").iter());
        x.end();
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
        let run_end = |t: &TextRun| t.start + t.text.encode_utf16().count();
        // Sources written around character ranges: their runs are split at
        // the source's ends (hyperlinks.md, extent and placement).
        let (lo, hi) = match (runs.first(), runs.last()) {
            (Some(a), Some(b)) => (a.start, run_end(b)),
            _ => (0, 0),
        };
        let para: Vec<&SourceRange> = story
            .sources
            .iter()
            .filter(|r| r.paragraph && r.start >= lo && r.start < hi)
            .filter(|r| {
                runs.iter()
                    .any(|t| t.start <= r.start && r.start < run_end(t))
            })
            .collect();
        let split;
        let runs = if para.is_empty() {
            runs
        } else {
            let cuts: Vec<usize> = para
                .iter()
                .flat_map(|r| [r.start, r.start + r.len])
                .collect();
            split = split_runs_at(runs, &cuts);
            &split[..]
        };
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
        let sources = para;
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
            // End offset of the open paragraph-level source.
            let mut open: Option<usize> = None;
            while i < n && (runs[i].paragraph_style, &runs[i].paragraph_attrs) == para {
                let r = runs[i];
                if open.is_none()
                    && let Some(s) = sources.iter().find(|s| s.start == r.start)
                    && let Some(src) = self.doc.text_sources.get(&s.source)
                {
                    // A source that starts with inserted text and goes on
                    // past it follows a character range with the deleted
                    // text there and an empty `Change` (hyperlinks.md,
                    // sources at inserted text).
                    if let Some(e) = insertion_at_source(story, r.start, s.len) {
                        self.csr_start(x, r);
                        self.deleted_text(x, story, r.start, scope);
                        x.start("Change");
                        self.change_attrs(x, e, "InsertedText");
                        x.end();
                        x.end();
                        st.bare = Some(r.start);
                    }
                    self.source_start(x, src);
                    open = Some(s.start + s.len);
                }
                self.csr_start(x, r);
                st.csr = true;
                self.run_content(x, text_of(i), r, story, scope, &mut st);
                st.close_csr(x);
                if open.is_some_and(|end| end <= run_end(r)) {
                    x.end();
                    open = None;
                }
                i += 1;
            }
            if open.is_some() {
                x.end();
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
        // End offset of the open endnote range.
        let mut endnote: Option<usize> = None;
        // The open `Change` of inserted text: the index of its change run.
        // It is always the innermost open element, so the others can close
        // around it; it is opened again where its text continues.
        let mut inserted: Option<usize> = None;
        let close_inserted = |x: &mut Xml, buf: &mut String, inserted: &mut Option<usize>| {
            if inserted.take().is_some() {
                flush(x, buf);
                x.end();
            }
        };
        for ch in text.chars() {
            if open == Some(pos) {
                close_inserted(x, &mut buf, &mut inserted);
                flush(x, &mut buf);
                x.end();
                open = None;
            }
            if endnote == Some(pos) {
                close_inserted(x, &mut buf, &mut inserted);
                flush(x, &mut buf);
                if open.take().is_some() {
                    x.end();
                }
                x.end();
                endnote = None;
            }
            if endnote.is_none()
                && let Some(r) = story.endnote_ranges.iter().find(|r| r.start == pos)
            {
                // Around the endnote's text, inside the character range
                // (footnotes.md).
                close_inserted(x, &mut buf, &mut inserted);
                flush(x, &mut buf);
                if open.take().is_some() {
                    x.end();
                }
                st.open_csr(self, x, run);
                x.start("EndnoteRange")
                    .attr("Self", uref(Some(r.uid)))
                    .attr("SourceEndnote", uref(Some(r.endnote)));
                endnote = Some(r.start + r.len);
            }
            // Deleted text of tracked changes, before the character that
            // follows it (objects.md, tracked changes).
            if st.bare != Some(pos) && story.deletions.contains_key(&pos) {
                close_inserted(x, &mut buf, &mut inserted);
                flush(x, &mut buf);
                st.open_csr(self, x, run);
                self.deleted_text(x, story, pos, scope);
            }
            // A character-level text source opens before its first
            // character, which can be an element (a note, a text
            // destination). The schema allows no paragraph destination in
            // it: one at the start stays before the source.
            let paragraph_destination = story
                .text_destinations
                .get(&pos)
                .is_some_and(|d| d.iter().any(|d| d.kind == DestinationKind::Paragraph));
            if open.is_none()
                && endnote.is_none()
                && !story.xml_markers.contains_key(&pos)
                && !paragraph_destination
                && let Some(r) = story
                    .sources
                    .iter()
                    .find(|r| r.start == pos && !r.paragraph)
                && let Some(src) = self.doc.text_sources.get(&r.source)
            {
                close_inserted(x, &mut buf, &mut inserted);
                flush(x, &mut buf);
                st.open_csr(self, x, run);
                // A source that starts with inserted text and goes on past
                // it follows an empty `Change`, and its first character has
                // none (hyperlinks.md, sources at inserted text).
                if let Some(e) = insertion_at_source(story, pos, r.len) {
                    x.start("Change");
                    self.change_attrs(x, e, "InsertedText");
                    x.end();
                    st.bare = Some(pos);
                }
                self.source_start(x, src);
                open = Some(r.start + r.len);
            }
            // Inserted text: inside a `Change`, except a table, an anchored
            // item and an XML marker, which close it (the schema allows no
            // page item in a `Change`).
            let want = story.changes.partition_point(|c| c.start + c.len <= pos);
            let want = story
                .changes
                .get(want)
                .filter(|c| c.start <= pos && c.entries.iter().any(|e| e.is_insertion()))
                .map(|_| want)
                .filter(|_| {
                    !matches!(ch, '\u{16}' | '\u{FFFC}')
                        && !story.xml_markers.contains_key(&pos)
                        && st.bare != Some(pos)
                });
            if inserted != want {
                close_inserted(x, &mut buf, &mut inserted);
                if let Some(i) = want
                    && let Some(e) = story.changes[i].entries.iter().find(|e| e.is_insertion())
                {
                    flush(x, &mut buf);
                    st.open_csr(self, x, run);
                    x.start("Change");
                    self.change_attrs(x, e, "InsertedText");
                    inserted = Some(i);
                }
            }
            // An index marker is written as a `PageReference` in place of
            // its character (index.md, page references).
            if ch == '\u{FEFF}'
                && !story.xml_markers.contains_key(&pos)
                && let Some(m) = story.index_markers.get(&pos)
            {
                flush(x, &mut buf);
                st.open_csr(self, x, run);
                x.start("PageReference").attr("Self", uref(Some(m.uid)));
                if m.current_page {
                    x.attr("PageReferenceType", "CurrentPage");
                }
                if let Some(t) = self.topics.get(&m.uid) {
                    x.attr("ReferencedTopic", t);
                }
                if let Some(id) = m.id {
                    x.attr("Id", id.to_string());
                }
                x.end();
                pos += 1;
                continue;
            }
            if ch == '\u{FEFF}'
                && let Some(n) = story.notes.get(&pos)
            {
                flush(x, &mut buf);
                st.open_csr(self, x, run);
                self.note(x, n, story, scope);
                pos += 1;
                continue;
            }
            if ch == '\u{FEFF}'
                && let Some(dests) = story.text_destinations.get(&pos)
            {
                // Written as empty elements in place of the character.
                flush(x, &mut buf);
                st.open_csr(self, x, run);
                for d in dests {
                    x.start(match d.kind {
                        DestinationKind::Paragraph => "ParagraphDestination",
                        _ => "HyperlinkTextDestination",
                    })
                    .attr("Self", d.reference())
                    .attr("Name", &d.name)
                    .attr("Hidden", d.hidden.to_string())
                    .attr_opt("DestinationUniqueKey", d.key.map(|k| k.to_string()))
                    .end();
                }
                if !story.xml_markers.contains_key(&pos) {
                    if paragraph_destination
                        && open.is_none()
                        && endnote.is_none()
                        && let Some(r) = story
                            .sources
                            .iter()
                            .find(|r| r.start == pos && !r.paragraph)
                        && let Some(src) = self.doc.text_sources.get(&r.source)
                    {
                        close_inserted(x, &mut buf, &mut inserted);
                        self.source_start(x, src);
                        open = Some(r.start + r.len);
                    }
                    pos += 1;
                    continue;
                }
            }
            if ch == '\u{FEFF}'
                && let Some(&m) = story.xml_markers.get(&pos)
            {
                flush(x, &mut buf);
                close_inserted(x, &mut buf, &mut inserted);
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
            match ch {
                '\r' => {
                    flush(x, &mut buf);
                    x.start("Br").end();
                }
                '\u{FFFC}' if story.anchors.contains_key(&pos) => {
                    flush(x, &mut buf);
                    for item in &story.anchors[&pos] {
                        self.page_item(x, item, false, &Matrix::IDENTITY);
                    }
                }
                '\u{4}' if story.endnotes.contains_key(&pos) => {
                    let (e, r) = story.endnotes[&pos];
                    flush(x, &mut buf);
                    x.start("Endnote")
                        .attr("Self", uref(Some(e)))
                        .attr("EndnoteTextRange", uref(Some(r)))
                        .end();
                }
                '\u{4}' if story.footnotes.contains_key(&pos) => {
                    // The footnote's text, in place of its reference.
                    flush(x, &mut buf);
                    x.start("Footnote");
                    self.text_ranges(x, &story.footnotes[&pos].runs, story, scope);
                    x.end();
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
        close_inserted(x, &mut buf, &mut inserted);
        if open.is_some() {
            x.end();
        }
        if endnote.is_some() {
            x.end();
        }
    }

    /// The deleted text of tracked changes owned by the character at
    /// `pos`, each in a `Change` (objects.md, tracked changes).
    fn deleted_text(&self, x: &mut Xml, story: &Story, pos: usize, scope: &str) {
        let Some(ds) = story.deletions.get(&pos) else {
            return;
        };
        let mut entries = story
            .changes
            .iter()
            .filter(|c| c.start == pos)
            .flat_map(|c| &c.entries)
            .filter(|e| e.is_deletion());
        for d in ds {
            x.start("Change");
            if let Some(e) = entries.next() {
                self.change_attrs(x, e, "DeletedText");
            } else {
                x.attr("ChangeType", "DeletedText");
            }
            self.text_ranges(x, &d.runs, story, scope);
            x.end();
        }
    }

    /// The attributes of a `Change`, in IDML's order (objects.md, tracked
    /// changes).
    fn change_attrs(&self, x: &mut Xml, e: &ChangeEntry, kind: &str) {
        let (name, user) = self.document_user(&e.user);
        x.attr("Date", link_time(e.time, &self.doc.xmp_dates))
            .attr("ChangeType", kind)
            .attr("UserName", name)
            .attr("AppliedDocumentUser", user);
    }

    /// A `Note` with its text (objects.md, notes).
    fn note(&self, x: &mut Xml, n: &Note, story: &Story, scope: &str) {
        let (name, user) = self.document_user(&n.user);
        x.start("Note")
            .attr("Collapsed", "false")
            .attr("CreationDate", link_time(n.created, &self.doc.xmp_dates))
            .attr(
                "ModificationDate",
                link_time(n.modified, &self.doc.xmp_dates),
            )
            .attr("UserName", name)
            .attr("AppliedDocumentUser", user);
        self.text_ranges(x, &n.runs, story, scope);
        x.end();
    }

    /// `UserName` and `AppliedDocumentUser` for a user name stored with a
    /// note or a change: the first document user with that name (`n` if
    /// none), whose name IDML replaces when it is the unknown user.
    pub(super) fn document_user(&self, name: &str) -> (String, String) {
        match self.doc.users.iter().position(|(_, u)| u == name) {
            Some(i) => {
                let shown = if self.doc.users[i].0 == 2 {
                    "$ID/Unknown User Name".to_string()
                } else {
                    name.to_string()
                };
                (shown, format!("dDocumentUser{i:x}"))
            }
            None => (name.to_string(), "n".into()),
        }
    }

    /// The start tag of a text source, `HyperlinkTextSource` or
    /// `CrossReferenceSource`, with its attributes and properties.
    fn source_start(&self, x: &mut Xml, src: &TextSource) {
        let style = match src.character_style {
            Some(c) => self.style_ref(Some(c), false),
            None => "n".into(),
        };
        match src.format {
            Some(f) => {
                x.start("CrossReferenceSource")
                    .attr("Self", uref(Some(src.uid)))
                    .attr("AppliedFormat", uref(Some(f)));
            }
            None => {
                x.start("HyperlinkTextSource")
                    .attr("Self", uref(Some(src.uid)));
            }
        }
        x.attr("Name", &src.name)
            .attr("Hidden", src.hidden.to_string())
            .attr("AppliedCharacterStyle", style);
        Self::alternative_destination(x, src.alternative.as_ref());
    }

    /// `Properties/AlternativeDestination` of a text source, for the one
    /// type seen (hyperlinks.md, text sources).
    pub(super) fn alternative_destination(x: &mut Xml, a: Option<&Alternative>) {
        let Some(a) = a.filter(|a| a.is_toc_anchor()) else {
            return;
        };
        // A control character in the page number is written as a
        // processing instruction in the text of the attribute.
        let mut page = String::new();
        for c in a.page_number.chars() {
            if (c as u32) < 0x20 {
                page.push_str(&format!("<?AID {:04x}?>", c as u32));
            } else {
                page.push(c);
            }
        }
        x.start("Properties")
            .start("AlternativeDestination")
            .attr("Type", "TocTextAnchor")
            .attr("IndexMarkerId", a.index_marker.to_string())
            .attr("TextAnchorName", &a.anchor_name)
            .attr("TocEntryPageNumberString", page)
            .attr("TocEntryLevel", a.level.to_string())
            .end()
            .end();
    }

    /// The text cell values IDML writes on every table, row, column and
    /// cell from DOM 11, with the value in effect (tables.md).
    fn text_cell_values<'a>(
        &self,
        value: impl Fn(u32) -> Option<&'a Value>,
    ) -> Vec<(&'static str, String)> {
        if self.doc.version.major < 11 {
            return Vec::new();
        }
        TEXT_CELL_ATTRS
            .iter()
            .filter_map(|&(id, name, kind)| Some((name, self.value_text(kind, value(id)?)?)))
            .collect()
    }

    pub(super) fn table(&self, x: &mut Xml, t: &Table, story: &Story, scope: &str) {
        let styles = TableStyles::new(&self.doc.cell_styles, &self.doc.table_styles);
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
            x.attr("AppliedTableStyle", self.table_style_ref("TableStyle", st));
        }
        x.attr(
            "TableDirection",
            if t.right_to_left {
                "RightToLeftDirection"
            } else {
                "LeftToRightDirection"
            },
        );
        for (name, v) in self.table_attrs(&t.attrs, false) {
            x.attr(name, v);
        }
        for (name, v) in self.text_cell_values(|a| t.value(a, &styles)) {
            x.attr(name, v);
        }
        // A row (column) has the values of the first cell that starts in
        // it.
        let first_cell = |of: &dyn Fn(&Cell) -> bool| -> Vec<(&'static str, String)> {
            match t.cells.iter().find(|c| of(c)) {
                Some(c) => self.text_cell_values(|a| t.cell_value(c, a, &styles)),
                None => self.text_cell_values(|a| t.value(a, &styles)),
            }
        };
        for (i, r) in t.rows.iter().enumerate() {
            x.start("Row")
                .attr("Self", format!("{id}Row{i:x}"))
                .attr("Name", i.to_string());
            for (name, v) in first_cell(&|c| c.row == i) {
                x.attr(name, v);
            }
            if let Some(h) = r.height {
                x.attr("SingleRowHeight", num(h));
            }
            if let Some(h) = r.min_height {
                x.attr("MinimumHeight", num(h));
            }
            for (name, v) in self.attr_values(&r.attrs, ROW_ATTRS) {
                x.attr(name, v);
            }
            x.end();
        }
        for (i, w) in t.columns.iter().enumerate() {
            x.start("Column")
                .attr("Self", format!("{id}Column{i:x}"))
                .attr("Name", i.to_string());
            for (name, v) in first_cell(&|c| c.column == i) {
                x.attr(name, v);
            }
            x.attr("SingleColumnWidth", num(*w));
            x.end();
        }
        for c in &t.cells {
            let cell_id = format!("{id}i{:x}", c.id);
            x.start("Cell")
                .attr("Self", &cell_id)
                .attr("Name", format!("{}:{}", c.column, c.row))
                .attr("RowSpan", c.row_span.to_string())
                .attr("ColumnSpan", c.column_span.to_string());
            // From DOM 11 (tables.md).
            if self.doc.version.major >= 11 {
                x.attr(
                    "CellType",
                    match c.kind {
                        CellKind::Text => "TextTypeCell",
                        CellKind::Graphic => "GraphicTypeCell",
                    },
                );
            }
            for (name, v) in self.text_cell_values(|a| t.cell_value(c, a, &styles)) {
                x.attr(name, v);
            }
            for (name, v) in self.cell_edge_attrs(t, c) {
                x.attr(name, v);
            }
            if let Some(f) = t.format(c) {
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
            // Deleted text owned by the cell's terminator, which no range
            // holds, ends the cell (objects.md, tracked changes).
            if let Some(r) = c.runs.last()
                && r.text.ends_with('\r')
            {
                let end = r.start + r.text.encode_utf16().count() - 1;
                self.deleted_text(x, story, end, &cell_id);
            }
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

/// `runs` with each run that holds one of `cuts` strictly inside split
/// there into two runs with the same styles.
fn split_runs_at(runs: &[TextRun], cuts: &[usize]) -> Vec<TextRun> {
    let mut out = Vec::with_capacity(runs.len() + cuts.len());
    for r in runs {
        let units: Vec<u16> = r.text.encode_utf16().collect();
        let end = r.start + units.len();
        let mut at: Vec<usize> = cuts
            .iter()
            .copied()
            .filter(|&c| c > r.start && c < end)
            .collect();
        at.sort_unstable();
        at.dedup();
        let mut from = r.start;
        for c in at.into_iter().chain([end]) {
            out.push(TextRun {
                start: from,
                text: String::from_utf16_lossy(&units[from - r.start..c - r.start]),
                ..r.clone()
            });
            from = c;
        }
    }
    out
}

/// `FrameType` and the orientation of a story from chunk 0x2EE (u16
/// orientation, u16 frame type); without the chunk, `TextFrameType` and
/// horizontal. For another orientation code, the orientation of the
/// story's frames. See `docs/format/objects.md`, story settings.
fn story_layout(
    layout: Option<(u16, u16)>,
    frames: Option<Orientation>,
) -> (Option<&'static str>, Option<Orientation>) {
    let (orientation, frame_type) = layout.unwrap_or((0, 0));
    let frame_type = match frame_type {
        0 => Some("TextFrameType"),
        1 => Some("FrameGridType"),
        _ => None,
    };
    let orientation = match orientation {
        0 => Some(Orientation::Horizontal),
        1 => Some(Orientation::Vertical),
        _ => frames,
    };
    (frame_type, orientation)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn story_layout_comes_from_chunk_0x2ee() {
        use Orientation::*;
        let v = Some(Vertical);
        assert_eq!(
            story_layout(None, v),
            (Some("TextFrameType"), Some(Horizontal))
        );
        assert_eq!(story_layout(Some((1, 1)), None), (Some("FrameGridType"), v));
        assert_eq!(
            story_layout(Some((0, 1)), v),
            (Some("FrameGridType"), Some(Horizontal))
        );
        assert_eq!(story_layout(Some((7, 9)), v), (None, v));
    }

    #[test]
    fn writes_paragraph_level_sources_around_split_ranges() {
        use crate::model::{Attrs, TextSource};
        let run = |start, text: &str, style| TextRun {
            start,
            text: text.into(),
            paragraph_style: None,
            character_style: style,
            paragraph_attrs: Attrs::default(),
            character_attrs: Attrs::default(),
        };
        let story = Story {
            uid: 9,
            runs: vec![run(0, "ab", None), run(2, "cd\r", Some(0x30))],
            anchors: Default::default(),
            tables: Default::default(),
            text_variables: Default::default(),
            sources: vec![SourceRange {
                start: 1,
                len: 2,
                source: 0x40,
                paragraph: true,
            }],
            xml_markers: Default::default(),
            footnotes: Default::default(),
            notes: Default::default(),
            deletions: Default::default(),
            changes: Vec::new(),
            index_markers: Default::default(),
            endnotes: Default::default(),
            is_endnote: false,
            endnote_ranges: Vec::new(),
            text_destinations: Default::default(),
            xml_element: None,
            orientation: None,
            toc_style: None,
            settings: Default::default(),
        };
        let src = TextSource {
            uid: 0x40,
            name: "s".into(),
            hidden: false,
            character_style: None,
            alternative: None,
            format: None,
        };
        let doc = Document {
            text_sources: [(0x40, src)].into_iter().collect(),
            ..Document::default()
        };
        let w = Writer::for_test(&doc);
        let out: String = w.story(&story).lines().map(str::trim).collect();
        // Tags and text only.
        let shape: String = out
            .split('<')
            .skip_while(|t| !t.starts_with("ParagraphStyleRange"))
            .map(|t| {
                let (tag, rest) = t.split_once('>').unwrap_or((t, ""));
                let name = tag.split(' ').next().unwrap_or("");
                format!("<{name}>{rest}")
            })
            .collect();
        assert!(
            shape.starts_with(
                "<ParagraphStyleRange><CharacterStyleRange><Content>a</Content>\
                 </CharacterStyleRange><HyperlinkTextSource><CharacterStyleRange>\
                 <Content>b</Content></CharacterStyleRange><CharacterStyleRange>\
                 <Content>c</Content></CharacterStyleRange></HyperlinkTextSource>\
                 <CharacterStyleRange><Content>d</Content></CharacterStyleRange>\
                 </ParagraphStyleRange>"
            ),
            "{shape}"
        );
    }

    #[test]
    fn writes_text_destinations_in_place_of_their_character() {
        use crate::model::{Attrs, Destination};
        let dest = Destination {
            uid: 5,
            name: "a:b".into(),
            hidden: false,
            key: Some(7),
            kind: DestinationKind::Text,
        };
        let story = Story {
            uid: 9,
            runs: vec![TextRun {
                start: 0,
                text: "x\u{FEFF}y\r".into(),
                paragraph_style: None,
                character_style: None,
                paragraph_attrs: Attrs::default(),
                character_attrs: Attrs::default(),
            }],
            anchors: Default::default(),
            tables: Default::default(),
            text_variables: Default::default(),
            sources: Vec::new(),
            xml_markers: Default::default(),
            footnotes: Default::default(),
            notes: Default::default(),
            deletions: Default::default(),
            changes: Vec::new(),
            index_markers: Default::default(),
            endnotes: Default::default(),
            is_endnote: false,
            endnote_ranges: Vec::new(),
            text_destinations: [(1, vec![dest])].into_iter().collect(),
            xml_element: None,
            orientation: None,
            toc_style: None,
            settings: Default::default(),
        };
        let doc = Document::default();
        let w = Writer::for_test(&doc);
        let out: String = w.story(&story).lines().map(str::trim).collect();
        assert!(
            out.contains(
                "<Content>x</Content><HyperlinkTextDestination \
                 Self=\"HyperlinkTextDestination/a%3ab\" Name=\"a:b\" Hidden=\"false\" \
                 DestinationUniqueKey=\"7\" /><Content>y</Content>"
            ),
            "{out}"
        );
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
            attributes: Vec::new(),
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
            footnotes: Default::default(),
            notes: Default::default(),
            deletions: Default::default(),
            changes: Vec::new(),
            index_markers: Default::default(),
            endnotes: Default::default(),
            is_endnote: false,
            endnote_ranges: Vec::new(),
            text_destinations: Default::default(),
            xml_element: None,
            orientation: None,
            toc_style: None,
            settings: Default::default(),
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
        let w = Writer::for_test(&doc);
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
}
