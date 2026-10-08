//! Stories: text, paragraph and character runs, anchored objects, tables,
//! text variables, orientation and XML markers.
//!
//! Evidence: `docs/format/objects.md` (stories and strands),
//! `attributes.md`, `tables.md` and `xml.md`.

use super::*;

/// A stretch of story text with one paragraph style and one character style.
#[derive(Debug, Clone, PartialEq)]
pub struct TextRun {
    /// UTF-16 offset of the run in the story text.
    pub start: usize,
    pub text: String,
    pub paragraph_style: Option<u32>,
    pub character_style: Option<u32>,
    /// Local paragraph formatting.
    pub paragraph_attrs: Attrs,
    /// Local character formatting.
    pub character_attrs: Attrs,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Story {
    pub uid: u32,
    pub runs: Vec<TextRun>,
    /// Page items anchored in the text, by UTF-16 offset of their U+FFFC.
    pub anchors: BTreeMap<usize, Vec<PageItem>>,
    /// Tables, by UTF-16 offset of their U+0016.
    pub tables: BTreeMap<usize, Table>,
    /// Text variable instances, by UTF-16 offset of their U+0018.
    pub text_variables: BTreeMap<usize, variable::Instance>,
    /// Hyperlink text sources, sorted by start.
    pub sources: Vec<SourceRange>,
    /// XML markers, by UTF-16 offset of their U+FEFF.
    pub xml_markers: BTreeMap<usize, XmlMarker>,
    /// Text and paragraph destinations, by UTF-16 offset of their U+FEFF.
    pub text_destinations: BTreeMap<usize, Vec<Destination>>,
    /// The XML element whose content is this story.
    pub xml_element: Option<xml::Key>,
    /// Text orientation, from the frames that show the story; `None` when
    /// it has no frame or its frames disagree.
    pub orientation: Option<Orientation>,
    /// The TOC style that made the story (chunk 0x8C40).
    pub toc_style: Option<u32>,
}

/// Text orientation of a story (IDML `StoryOrientation`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Orientation {
    Horizontal,
    Vertical,
}

/// What an XML marker character (U+FEFF) in story text stands for.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum XmlMarker {
    /// The start of an element that holds the text up to its end marker.
    Start(xml::Key),
    End(xml::Key),
    /// An element that is a single marker; its content is elsewhere.
    Placeholder(xml::Key),
    /// A marker that IDML does not write: the start of the document node,
    /// or a marker of an element that is left out.
    Hidden,
}

/// An XML element that can be written, with its IDML values.
#[derive(Debug, Clone, PartialEq)]
pub struct XmlElement {
    /// IDML `Self`.
    pub name: String,
    /// Tag name (IDML `MarkupTag` without `XMLTag/`).
    pub tag: String,
    /// UID written as `XMLContent`.
    pub content: Option<u32>,
    /// The content is a story.
    pub story_content: bool,
    /// Written between character ranges rather than inside one: the
    /// element holds an element whose content is a story.
    pub block: bool,
}

/// The UTF-16 offset of each character of `text`, where a surrogate pair is
/// one character, followed by the length of `text`.
pub(super) fn char_offsets(text: &[u16]) -> Vec<usize> {
    let mut out = Vec::with_capacity(text.len() + 1);
    let mut i = 0;
    while i < text.len() {
        out.push(i);
        let pair = (0xD800..0xDC00).contains(&text[i])
            && text
                .get(i + 1)
                .is_some_and(|u| (0xDC00..0xE000).contains(u));
        i += if pair { 2 } else { 1 };
    }
    out.push(text.len());
    out
}

/// Combine text with paragraph-style and character-style run lengths
/// (counted in UTF-16 code units) into runs with both styles constant.
pub(super) type StyleRun = (usize, u32, Attrs);

pub(super) fn split_runs(
    utf16: &[u16],
    para: &[StyleRun],
    chars: &[StyleRun],
    extra: &[usize],
) -> Vec<TextRun> {
    let mut cuts: Vec<usize> = vec![0, utf16.len()];
    cuts.extend(extra.iter().map(|&c| c.min(utf16.len())));
    for list in [para, chars] {
        let mut pos = 0;
        for (len, _, _) in list {
            pos += len;
            cuts.push(pos.min(utf16.len()));
        }
    }
    cuts.sort_unstable();
    cuts.dedup();
    let run_at = |list: &[StyleRun], at: usize| -> Option<(Option<u32>, Attrs)> {
        let mut pos = 0;
        for (len, style, attrs) in list {
            if at < pos + len {
                return Some((uid_or_none(*style), attrs.clone()));
            }
            pos += len;
        }
        None
    };
    cuts.windows(2)
        .filter(|w| w[0] < w[1])
        .map(|w| {
            let (paragraph_style, paragraph_attrs) = run_at(para, w[0]).unwrap_or_default();
            let (character_style, character_attrs) = run_at(chars, w[0]).unwrap_or_default();
            TextRun {
                start: w[0],
                text: String::from_utf16_lossy(&utf16[w[0]..w[1]]),
                paragraph_style,
                character_style,
                paragraph_attrs,
                character_attrs,
            }
        })
        .collect()
}

impl<'a> Reader<'a> {
    /// The text orientation shared by all frames of a story. A story
    /// whose frames disagree, or have an orientation that is not known,
    /// gets none, with a warning.
    pub(super) fn story_orientation(&self, story: u32) -> Option<Orientation> {
        let map = self.frame_orientations.borrow();
        let frames = map.get(&story)?;
        let first = frames[0];
        if first.is_none() || frames.iter().any(|&o| o != first) {
            self.warn(format!(
                "story {story}: text orientation of its frames is not known or differs; written as horizontal"
            ));
            return None;
        }
        first
    }

    pub(super) fn story(&self, uid: u32) -> Result<Story, Error> {
        let data = self.required(uid, chunk::STORY_STRANDS)?;
        let mut c = self.cursor(&data);
        c.skip(6)?;
        let mut strands = vec![c.u32()?];
        strands.extend(c.u32_list()?);
        let mut text: Vec<u16> = Vec::new();
        let mut para: Vec<StyleRun> = Vec::new();
        let mut chars: Vec<StyleRun> = Vec::new();
        let mut owned: Vec<(usize, u32, u32)> = Vec::new();
        // (start, length, owner, cell) for each stretch of text.
        let mut owners: Vec<(usize, usize, u32, u32)> = Vec::new();
        let mut sources = Vec::new();
        let mut marker_strand = None;
        for strand in strands {
            // A strand without data occurs in a file from InDesign 3.0.
            if self.db.object(strand)?.is_none() {
                continue;
            }
            if self.class(strand) == Some(xml::class::MARKER_STRAND) {
                marker_strand = Some(strand);
            }
            if self.class(strand) == Some(hyperlink::class::RANGE_STRAND)
                && let Some(tree) = self.chunk(strand, hyperlink::chunk::RANGE_TREE)?
                && tree.len() >= 4
            {
                let first = self.enc().u32_from(tree[..4].try_into().unwrap());
                let pages = |uid| self.chunk(uid, hyperlink::chunk::RANGE_PAGE);
                match hyperlink::source_ranges(self.enc(), first, pages) {
                    Ok(r) => sources.extend(r),
                    Err(e) => self.warn(format!("story {uid}: hyperlink sources left out: {e}")),
                }
            }
            let Some(list) = self.chunk(strand, chunk::STRAND_DATA)? else {
                continue;
            };
            let mut c = self.cursor(&list);
            let n = c.u16()?;
            let mut base = 0usize;
            for _ in 0..n {
                let data_len = c.u32()? as usize;
                let data_uid = c.u32()?;
                let runs = self.required(data_uid, chunk::STRAND_RUNS)?;
                let mut r = self.cursor(&runs);
                let kind = r.u32()?;
                r.skip(4)?;
                let count = r.u16()?;
                let mut pos = base;
                base += data_len;
                for _ in 0..count {
                    let size = r.u32()? as usize;
                    let rec = r.bytes(size)?;
                    let mut rc = self.cursor(rec);
                    let len = rc.u32()? as usize;
                    let start = pos;
                    pos += len;
                    match kind {
                        strand::OWNED_ITEMS => {
                            let n = rc.u16()?;
                            for _ in 0..n {
                                let class = rc.u32()?;
                                let item = rc.u32()?;
                                owned.push((start, class, item));
                            }
                        }
                        strand::TEXT => {
                            text.extend(rc.segment_units(len)?);
                        }
                        strand::TEXT_OWNER => {
                            let owner = rc.u32()?;
                            let cell = rc.u32()?;
                            owners.push((start, len, owner, cell));
                        }
                        strand::PARAGRAPH_STYLE | strand::CHARACTER_STYLE => {
                            let style = rc.u32()?;
                            let n = rc.u16()? as usize;
                            let attrs = self
                                .attrs_or_warn(
                                    || format!("story {uid}, text formatting"),
                                    Attrs::parse_text(&mut rc, n, List::Text, self.db.recorder()),
                                )
                                .unwrap_or_default();
                            let list = if kind == strand::PARAGRAPH_STYLE {
                                &mut para
                            } else {
                                &mut chars
                            };
                            list.push((len, style, attrs));
                        }
                        _ => {
                            if let Some(r) = self.db.recorder() {
                                r.unknown_strand_kind(kind)
                            }
                        }
                    }
                }
            }
        }
        // Text records count UTF-16 code units, every other strand counts
        // characters (a surrogate pair is one position). Convert the
        // positions to UTF-16 offsets.
        let offsets = char_offsets(&text);
        let unit = |p: usize| match offsets.get(p) {
            Some(&u) => u,
            None => text.len() + (p + 1 - offsets.len()),
        };
        let to_units = |list: Vec<StyleRun>| -> Vec<StyleRun> {
            let mut at = 0;
            list.into_iter()
                .map(|(len, style, attrs)| {
                    let n = unit(at + len) - unit(at);
                    at += len;
                    (n, style, attrs)
                })
                .collect()
        };
        let para = to_units(para);
        let chars = to_units(chars);
        let owned: Vec<_> = owned
            .into_iter()
            .map(|(p, cls, item)| (unit(p), cls, item))
            .collect();
        let owners: Vec<_> = owners
            .into_iter()
            .map(|(p, len, owner, cell)| (unit(p), unit(p + len) - unit(p), owner, cell))
            .collect();
        for r in &mut sources {
            let start = unit(r.start);
            r.len = unit(r.start + r.len) - start;
            r.start = start;
        }
        let xml_markers = match self.story_xml(uid, marker_strand, &para, &unit) {
            Ok(m) => m,
            Err(e) => {
                self.warn(format!("story {uid}: XML structure left out: {e}"));
                BTreeMap::new()
            }
        };
        let xml_element = match self.chunk(uid, xml::chunk::NODE_REF)? {
            Some(d) if d.len() >= 8 => {
                let mut c = self.cursor(&d);
                Some((c.u32()?, c.u32()?))
            }
            _ => None,
        };
        let mut anchors: BTreeMap<usize, Vec<PageItem>> = BTreeMap::new();
        let mut tables: BTreeMap<usize, Table> = BTreeMap::new();
        let mut text_variables = BTreeMap::new();
        let mut text_destinations: BTreeMap<usize, Vec<Destination>> = BTreeMap::new();
        for (pos, cls, item) in owned {
            match cls {
                hyperlink::class::DESTINATION_OWNER => {
                    // The owner names its destination (hyperlinks.md).
                    let dest = self
                        .chunk(item, hyperlink::chunk::OWNER_DESTINATION)?
                        .and_then(|d| self.enc().u32_at(&d, 0))
                        .and_then(uid_or_none);
                    if let Some(dest) = dest
                        && let Some(d) = Destination::read(
                            dest,
                            hyperlink::class::TEXT_DESTINATION,
                            &*self.object(dest)?,
                        )?
                    {
                        text_destinations.entry(pos).or_default().push(d);
                    }
                }
                class::TEXT_VARIABLE_INSTANCE => {
                    let v = variable::Instance::read(item, &*self.object(item)?)?;
                    text_variables.insert(pos, v);
                }
                class::ANCHOR => {
                    let settings = self
                        .chunk(item, chunk::ANCHOR_SETTINGS)?
                        .map(|d| AnchorSettings::read(self.enc(), &d));
                    for child in self.children(item, chunk::ANCHOR_CHILDREN)? {
                        if let Some(mut pi) = self.page_item(child, None)? {
                            pi.anchor = settings.clone();
                            anchors.entry(pos).or_default().push(pi);
                        }
                    }
                }
                table::class::TABLE_ANCHOR => {
                    // A table that cannot be read is left out, with a
                    // warning, rather than failing the whole document.
                    match self.table_of_anchor(item) {
                        Ok(Some(t)) => {
                            tables.insert(pos, t);
                        }
                        Ok(None) => {}
                        Err(e) => self.warn(format!("story {uid}: table left out: {e}")),
                    }
                }
                _ => {}
            }
        }
        let cuts: Vec<usize> = owners.iter().map(|o| o.0).collect();
        let all = split_runs(&text, &para, &chars, &cuts);
        // Text not owned by the story belongs to table cells.
        let owner_at = |at: usize| {
            owners
                .iter()
                .find(|o| at >= o.0 && at < o.0 + o.1)
                .map(|o| (o.2, o.3))
        };
        let mut runs = Vec::new();
        for run in all {
            match owner_at(run.start) {
                Some((owner, cell)) if owner != uid => {
                    if let Some(t) = tables.values_mut().find(|t| t.uid == owner)
                        && let Some(c) = t.cells.iter_mut().find(|c| c.id == cell)
                    {
                        c.runs.push(run);
                    }
                }
                _ => runs.push(run),
            }
        }
        // IDML writes a text source inside one character range, or as a
        // child of a paragraph range around several character ranges
        // (hyperlinks.md, extent and placement).
        sources.sort_by_key(|r| r.start);
        let cell_lists = tables
            .values()
            .flat_map(|t| t.cells.iter().map(|c| c.runs.as_slice()));
        let lists: Vec<&[TextRun]> = std::iter::once(runs.as_slice()).chain(cell_lists).collect();
        sources.retain_mut(|r| {
            if r.len == 0 {
                return false;
            }
            let end = r.start + r.len;
            let run_end = |t: &TextRun| t.start + t.text.encode_utf16().count();
            // The runs that hold the first and the last character.
            let found = lists.iter().find_map(|list| {
                let a = list.iter().position(|t| r.start >= t.start && r.start < run_end(t))?;
                let b = list.iter().position(|t| end > t.start && end <= run_end(t))?;
                (a <= b).then(|| &list[a..=b])
            });
            let Some(span) = found else {
                self.warn(format!(
                    "story {uid}: hyperlink source {} spans several text ranges; left out",
                    r.source
                ));
                return false;
            };
            let first = &span[0];
            let one_paragraph = span.windows(2).all(|w| run_end(&w[0]) == w[1].start)
                && span.iter().all(|t| {
                    (t.paragraph_style, &t.paragraph_attrs)
                        == (first.paragraph_style, &first.paragraph_attrs)
                });
            if !one_paragraph {
                self.warn(format!(
                    "story {uid}: hyperlink source {} spans several paragraph style ranges; left out",
                    r.source
                ));
                return false;
            }
            // The schema allows no text directly in a cross-reference
            // source: one that holds more than a text variable instance is
            // written around a character range.
            let cross_reference = self
                .chunk(r.source, hyperlink::chunk::CROSS_REFERENCE)
                .ok()
                .flatten()
                .is_some();
            let variable_only = r.len == 1 && text_variables.contains_key(&r.start);
            r.paragraph = span.len() > 1 || cross_reference && !variable_only;
            if r.paragraph {
                if xml_markers
                    .range(r.start..end)
                    .any(|(_, m)| !matches!(m, XmlMarker::Hidden))
                {
                    self.warn(format!(
                        "story {uid}: hyperlink source {} holds an XML marker; left out",
                        r.source
                    ));
                    return false;
                }
                return true;
            }
            // The IDML schema allows no page item inside a text source
            // written in a character range.
            if anchors.range(r.start..end).next().is_some() {
                self.warn(format!(
                    "story {uid}: hyperlink source {} holds an anchored object; left out",
                    r.source
                ));
                return false;
            }
            true
        });
        Ok(Story {
            uid,
            runs,
            anchors,
            tables,
            text_variables,
            sources,
            xml_markers,
            text_destinations,
            xml_element,
            orientation: None,
            // The story names an object (class 0x8C20) whose chunk 0x11613
            // is the TOC style.
            toc_style: match self
                .chunk(uid, chunk::STORY_TOC)?
                .and_then(|d| self.enc().u32_at(&d, 0))
                .and_then(uid_or_none)
            {
                Some(toc) => self
                    .chunk(toc, chunk::TOC_STYLE_OF)?
                    .and_then(|d| self.enc().u32_at(&d, 0))
                    .and_then(uid_or_none),
                None => None,
            },
        })
    }

    /// Read the XML nodes stored with story `uid` and place their markers.
    /// `unit` converts a character position to a UTF-16 offset; `para` are
    /// the paragraph runs, in UTF-16 units. An element whose start and end
    /// lie in different paragraph runs is left out.
    pub(super) fn story_xml(
        &self,
        uid: u32,
        marker_strand: Option<u32>,
        para: &[StyleRun],
        unit: &dyn Fn(usize) -> usize,
    ) -> Result<BTreeMap<usize, XmlMarker>, Error> {
        let mut out = BTreeMap::new();
        let first = match self.chunk(uid, xml::chunk::STORE)? {
            Some(d) if d.len() >= 4 => self.cursor(&d).u32()?,
            _ => return Ok(out),
        };
        let nodes = xml::read_store(self.enc(), first, |p| self.chunk(p, xml::chunk::PAGE))?;
        if nodes.is_empty() {
            return Ok(out);
        }
        let positions = match marker_strand.map(|s| self.chunk(s, xml::chunk::MARKER_TREE)) {
            Some(r) => match r? {
                Some(d) => xml::marker_positions(self.enc(), &d)?,
                None => BTreeMap::new(),
            },
            None => BTreeMap::new(),
        };
        let para_of = |at: usize| {
            let mut end = 0;
            para.iter().position(|(len, _, _)| {
                end += len;
                at < end
            })
        };
        for node in nodes {
            let at = |m: Option<u32>| m.and_then(|m| positions.get(&m)).map(|&p| unit(p));
            let (start, end) = (at(node.start), at(node.end));
            if node.start.is_some() != start.is_some() || node.end.is_some() != end.is_some() {
                self.warn(format!(
                    "story {uid}: XML node {:?} left out: marker not found",
                    node.key
                ));
            } else if node.document {
                if let Some(s) = start {
                    out.insert(s, XmlMarker::Hidden);
                }
            } else {
                match (start, end) {
                    (Some(s), Some(e)) if para_of(s) == para_of(e) => {
                        out.insert(s, XmlMarker::Start(node.key));
                        out.insert(e, XmlMarker::End(node.key));
                    }
                    (Some(s), Some(e)) => {
                        self.warn(format!(
                            "story {uid}: XML element {:?} spans paragraphs; left out",
                            node.key
                        ));
                        out.insert(s, XmlMarker::Hidden);
                        out.insert(e, XmlMarker::Hidden);
                    }
                    (Some(s), None) => {
                        out.insert(s, XmlMarker::Placeholder(node.key));
                    }
                    _ => {}
                }
            }
            self.xml_nodes.borrow_mut().insert(node.key, node);
        }
        Ok(out)
    }

    /// The XML elements that can be written: those whose parents lead to
    /// the document node and whose tag is known. See `docs/format/xml.md`.
    pub(super) fn xml_elements(
        &self,
        tags: &HashMap<u32, String>,
    ) -> Result<BTreeMap<xml::Key, XmlElement>, Error> {
        let nodes = self.xml_nodes.borrow();
        let mut out = BTreeMap::new();
        for (key, node) in nodes.iter().filter(|(_, n)| !n.document) {
            // `Self`: "d", then "i" and the hex ID of each element from the
            // root down to this one.
            let mut path = vec![key.1];
            let mut parent = node.parent;
            let mut reached = false;
            while let Some(p) = nodes.get(&parent) {
                if p.document {
                    reached = true;
                    break;
                }
                if path.len() > nodes.len() {
                    break;
                }
                path.push(p.key.1);
                parent = p.parent;
            }
            let Some(tag) = tags.get(&node.tag) else {
                self.warn(format!(
                    "XML element {key:?} left out: tag {} unknown",
                    node.tag
                ));
                continue;
            };
            if !reached {
                self.warn(format!("XML element {key:?} left out: no path to the root"));
                continue;
            }
            let name = path.iter().rev().fold(String::from("d"), |mut s, id| {
                s.push_str(&format!("i{id:x}"));
                s
            });
            let (content, story_content) = match self.class(node.content) {
                _ if node.content == 0 => (None, false),
                Some(class::STORY) => (Some(node.content), true),
                // A content holder stands for its parent page item.
                Some(xml::class::CONTENT_HOLDER) => {
                    match self.chunk(node.content, chunk::ITEM_HIERARCHY)? {
                        Some(d) if d.len() >= 8 => {
                            (uid_or_none(self.cursor(&d[4..]).u32()?), false)
                        }
                        _ => (None, false),
                    }
                }
                c => {
                    self.warn(format!(
                        "XML element {key:?}: content {} of class {c:x?} left out",
                        node.content
                    ));
                    (None, false)
                }
            };
            out.insert(
                *key,
                XmlElement {
                    name,
                    tag: tag.clone(),
                    content,
                    story_content,
                    block: false,
                },
            );
        }
        // An element is written as a block when it holds, at any depth, an
        // element that is a single marker with a story as content.
        let holds_story = |key: &xml::Key| {
            let mut stack = vec![*key];
            let mut seen = Vec::new();
            while let Some(k) = stack.pop() {
                if seen.contains(&k) {
                    continue;
                }
                seen.push(k);
                let Some(n) = nodes.get(&k) else { continue };
                for c in &n.children {
                    if let Some(child) = nodes.get(c)
                        && child.end.is_none()
                        && out.get(c).is_some_and(|e: &XmlElement| e.story_content)
                    {
                        return true;
                    }
                    stack.push(*c);
                }
            }
            false
        };
        let blocks: Vec<xml::Key> = out.keys().filter(|k| holds_story(k)).copied().collect();
        for k in blocks {
            if let Some(e) = out.get_mut(&k) {
                e.block = true;
            }
        }
        Ok(out)
    }
}
