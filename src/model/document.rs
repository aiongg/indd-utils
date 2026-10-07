//! The `Document` model and `Reader::document`, which reads the document
//! object and every class the converter knows into it.
//!
//! Evidence: `docs/format/objects.md` (document object and classes).

use super::*;

/// The XML structure of the document.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct XmlStructure {
    /// The backing story, IDML `XmlStory`.
    pub story: Option<Story>,
    pub elements: BTreeMap<xml::Key, XmlElement>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Document {
    pub version: Version,
    pub layers: Vec<Layer>,
    pub active_layer: Option<u32>,
    /// Document users (chunk 0xA443): flag byte and name.
    pub users: Vec<(u8, String)>,
    /// Index sort groups (preferences chunk 0x1307E), in order: name,
    /// include flag and header variant.
    pub index_groups: Vec<(String, bool, u16)>,
    /// The constant shade the document lists (the class 0x5533 object of
    /// lowest UID with a constant shade in chunk 0x5532): its UID and its
    /// u32 and three f64; and its name (chunk 0x5531: a flag byte, 1 for
    /// a built-in key, and the name).
    pub constant_shade: Option<ConstantShade>,
    /// Assignment objects (class 0x1BE01), in UID order.
    pub assignments: Vec<u32>,
    /// Named grids (class 0xCD12, chunk 0xCD28: u32, a flag byte, 1 for
    /// a built-in key, and the name), in UID order.
    pub named_grids: Vec<(bool, String)>,
    pub spreads: Vec<Spread>,
    pub master_spreads: Vec<Spread>,
    pub stories: Vec<Story>,
    pub styles: BTreeMap<u32, Style>,
    pub toc_styles: Vec<TocStyle>,
    pub colors: Vec<Color>,
    /// Tint swatches, with their IDML reference and name.
    pub tints: Vec<(Tint, String, String)>,
    pub gradients: Vec<Gradient>,
    /// IDML reference (`Color/...`, `Swatch/None`) for each swatch UID.
    pub swatches: BTreeMap<u32, String>,
    /// Font families by UID.
    pub fonts: BTreeMap<u32, FontFamily>,
    /// The language objects (class 0x2D07) in UID order.
    pub language_list: Vec<Language>,
    /// Language name (IDML `AppliedLanguage` without `$ID/`) for each
    /// language UID.
    pub languages: BTreeMap<u32, String>,
    /// Style groups, including the root groups (empty name).
    pub style_groups: BTreeMap<u32, StyleGroup>,
    pub object_styles: BTreeMap<u32, ObjectStyle>,
    pub cell_styles: BTreeMap<u32, TableStyle>,
    pub table_styles: BTreeMap<u32, TableStyle>,
    pub sections: Vec<Section>,
    pub text_variables: Vec<TextVariable>,
    pub hyperlinks: Vec<Hyperlink>,
    pub text_sources: BTreeMap<u32, TextSource>,
    pub destinations: Vec<Destination>,
    pub bookmarks: BTreeMap<u32, Bookmark>,
    /// All bookmarks in document order (document chunk 0x13501).
    pub bookmark_order: Vec<u32>,
    /// Cross-reference formats, by UID.
    pub cross_reference_formats: BTreeMap<u32, CrossReferenceFormat>,
    /// Problems that did not stop the conversion (content left out).
    pub warnings: Vec<String>,
    pub preferences: Option<DocumentPreferences>,
    /// Other preference values (`prefs.rs`).
    pub prefs: prefs::Prefs,
    pub composite_fonts: Vec<CompositeFont>,
    /// Kinsoku and mojikumi tables, in UID order.
    pub cjk_tables: Vec<CjkTable>,
    /// Inks, in UID order.
    pub inks: Vec<Ink>,
    /// Colour groups in document order, the root group first.
    pub color_groups: Vec<ColorGroup>,
    /// The bullet characters offered for lists (preferences chunk 0x1A488).
    pub bullets: Vec<Bullet>,
    /// XML tags (class 0xBF19): name and colour (red, green, blue).
    pub xml_tags: Vec<(String, Option<[f64; 3]>)>,
    pub xml: XmlStructure,
}

impl<'a> Reader<'a> {
    pub fn document(&self, version: Version) -> Result<Document, Error> {
        let doc = 1;
        let layers = self
            .uid_list(doc, chunk::DOC_LAYERS)?
            .into_iter()
            .map(|uid| self.layer(uid))
            .collect::<Result<Vec<_>, _>>()?;
        let active_layer = self
            .chunk(doc, chunk::DOC_ACTIVE_LAYER)?
            .map(|d| Cursor::new(&d).u32())
            .transpose()?;
        let mut spreads = self
            .uid_list(doc, chunk::DOC_SPREADS)?
            .into_iter()
            .map(|uid| self.spread(uid))
            .collect::<Result<Vec<_>, _>>()?;
        let mut master_spreads = self
            .uid_list(doc, chunk::DOC_MASTER_SPREADS)?
            .into_iter()
            .map(|uid| self.spread(uid))
            .collect::<Result<Vec<_>, _>>()?;
        resolve_page_layout(&mut spreads, &mut master_spreads);
        let stories = self
            .uid_list(doc, chunk::DOC_STORIES)?
            .into_iter()
            .filter(|&uid| self.class(uid) == Some(class::STORY))
            .map(|uid| self.story(uid))
            .collect::<Result<Vec<_>, _>>()?;
        let mut styles = BTreeMap::new();
        let mut colors = Vec::new();
        let mut tint_objects = Vec::new();
        let mut gradients = Vec::new();
        let mut swatches = BTreeMap::new();
        let mut fonts = BTreeMap::new();
        let mut languages = BTreeMap::new();
        let mut language_list = Vec::new();
        let mut style_groups = BTreeMap::new();
        let mut object_styles = BTreeMap::new();
        let mut cell_styles = BTreeMap::new();
        let mut table_styles = BTreeMap::new();
        let mut text_variables = Vec::new();
        let mut hyperlinks = Vec::new();
        let mut text_sources = BTreeMap::new();
        let mut destinations = Vec::new();
        let mut bookmarks = BTreeMap::new();
        let mut cross_reference_formats = BTreeMap::new();
        let mut composite_fonts = Vec::new();
        let mut composite_entries = HashMap::new();
        let mut cjk_tables = Vec::new();
        let mut xml_tags = Vec::new();
        let mut xml_tag_names = HashMap::new();
        let mut inks = Vec::new();
        let mut color_groups = HashMap::new();
        for &(uid, cls) in self.db.classes() {
            if self.db.object(uid)?.is_none() {
                continue;
            }
            // An object that cannot be read is left out with a warning.
            let read = (|| -> Result<(), Error> {
                match cls {
                    class::STYLE_ROOT_GROUP
                    | class::OBJECT_STYLE_ROOT_GROUP
                    | class::CELL_STYLE_ROOT_GROUP
                    | class::TABLE_STYLE_ROOT_GROUP => {
                        let kind = match self.chunk(uid, chunk::ROOT_GROUP_KIND)? {
                            Some(d) => Cursor::new(&d).u32()?,
                            None => 0,
                        };
                        let mut children = Vec::new();
                        for id in [
                            chunk::STYLE_ROOT_CHILDREN,
                            chunk::OBJECT_STYLE_ROOT_CHILDREN,
                            table::chunk::CELL_STYLE_ROOT_CHILDREN,
                            table::chunk::TABLE_STYLE_ROOT_CHILDREN,
                        ] {
                            if children.is_empty() {
                                children = self.children(uid, id)?;
                            }
                        }
                        style_groups.insert(
                            uid,
                            StyleGroup {
                                uid,
                                name: String::new(),
                                root: Some(kind),
                                children,
                            },
                        );
                    }
                    class::STYLE_GROUP => {
                        let name = match self.chunk(uid, chunk::STYLE_GROUP_NAME)? {
                            Some(d) if d.len() > 1 => {
                                let mut c = Cursor::new(&d);
                                c.flag()?;
                                c.string()?
                            }
                            _ => String::new(),
                        };
                        let children = self.children(uid, chunk::STYLE_GROUP_CHILDREN)?;
                        style_groups.insert(
                            uid,
                            StyleGroup {
                                uid,
                                name,
                                root: None,
                                children,
                            },
                        );
                    }
                    class::OBJECT_STYLE => {
                        if let Some(d) = self.chunk(uid, chunk::OBJECT_STYLE_INFO)? {
                            let mut c = Cursor::new(&d);
                            let based_on = c.u32()?;
                            let builtin = c.flag()? == 1;
                            let name = c.string()?;
                            object_styles.insert(
                                uid,
                                ObjectStyle {
                                    uid,
                                    name,
                                    builtin,
                                    based_on: uid_or_none(based_on),
                                    fitting: match self.chunk(uid, chunk::OBJECT_STYLE_FITTING)? {
                                        Some(d) => Attrs::parse_short(&d, List::ObjectStyleFitting)
                                            .unwrap_or_default(),
                                        None => Attrs::default(),
                                    },
                                    attrs: match self.chunk(uid, chunk::OBJECT_STYLE_ATTRS)? {
                                        Some(d) => Attrs::parse_short(&d, List::ObjectStyle)
                                            .unwrap_or_default(),
                                        None => Attrs::default(),
                                    },
                                    // The layout is known for these sizes
                                    // only (`docs/format/big-endian.md`).
                                    frame: match self.chunk(uid, chunk::OBJECT_STYLE_FRAME)? {
                                        Some(d) if !matches!(d.len(), 106 | 142 | 162 | 222) => {
                                            self.warn(format!(
                                                "object style {uid}: text frame settings of {} bytes are not known; left out",
                                                d.len()
                                            ));
                                            None
                                        }
                                        d => d,
                                    },
                                    story: self.chunk(uid, chunk::OBJECT_STYLE_STORY)?,
                                    direction: match self
                                        .chunk(uid, chunk::OBJECT_STYLE_DIRECTION)?
                                    {
                                        Some(d) if d.len() >= 2 => Some(Cursor::new(&d).u16()?),
                                        _ => None,
                                    },
                                    text_wrap: self.wrap_chunk(uid, chunk::OBJECT_STYLE_WRAP)?,
                                    contour_type: match self
                                        .chunk(uid, chunk::OBJECT_STYLE_CONTOUR)?
                                    {
                                        Some(d) if d.len() >= 4 => Some(Cursor::new(&d).u32()?),
                                        _ => None,
                                    },
                                    enabled: match self.chunk(uid, chunk::OBJECT_STYLE_ENABLED)? {
                                        Some(d) => Some(Cursor::new(&d).u32_list()?),
                                        None => None,
                                    },
                                    paragraph_style: match self
                                        .chunk(uid, chunk::OBJECT_STYLE_PARAGRAPH_STYLE)?
                                    {
                                        Some(d) if d.len() >= 4 => Some(Cursor::new(&d).u32()?),
                                        _ => None,
                                    },
                                    anchor: self.chunk(uid, chunk::ANCHOR_SETTINGS)?,
                                },
                            );
                        }
                    }
                    class::STYLE => {
                        if let Some(style) = self.style(uid)? {
                            styles.insert(uid, style);
                        }
                    }
                    table::class::CELL_STYLE => {
                        if let Some(s) = self.table_style(uid, table::chunk::CELL_STYLE_ATTRS)? {
                            cell_styles.insert(uid, s);
                        }
                    }
                    table::class::TABLE_STYLE => {
                        if let Some(s) = self.table_style(uid, table::chunk::TABLE_STYLE_ATTRS)? {
                            table_styles.insert(uid, s);
                        }
                    }
                    color::class::COLOR => {
                        if self.db.object(uid)?.is_none() {
                            return Ok(());
                        }
                        let obj = self.object(uid)?;
                        if let Some(c) = Color::read(uid, &obj)? {
                            if c.model_name().is_none() {
                                self.warn(format!(
                                    "colour {uid}: colour model code {} is not known; left out",
                                    c.model
                                ));
                            }
                            swatches.insert(uid, c.reference());
                            colors.push(c);
                        } else if let Some(t) = Tint::read(uid, &obj)? {
                            tint_objects.push(t);
                        }
                    }
                    color::class::SWATCH_NONE => {
                        swatches.insert(uid, "Swatch/None".into());
                    }
                    color::class::GRADIENT => {
                        if let Some(g) = Gradient::read(uid, &*self.object(uid)?)? {
                            for i in 1..g.stops.len() {
                                if g.idml_midpoint(i).is_none() {
                                    self.warn(format!(
                                        "gradient {uid}: midpoint before stop {i} is not \
                                         within 13–87 %; left out"
                                    ));
                                }
                            }
                            swatches.insert(uid, g.reference());
                            gradients.push(g);
                        }
                    }
                    class::FONT_FAMILY => match FontFamily::read(uid, &*self.object(uid)?) {
                        Ok(Some(f)) => {
                            if i32::try_from(f.writing_script).is_err() {
                                self.warn(format!(
                                    "font family {uid}: writing script {:#x} is not an IDML \
                                     integer; left out",
                                    f.writing_script
                                ));
                            }
                            fonts.insert(uid, f);
                        }
                        Ok(None) => {}
                        // Keep the name, which text formatting refers to.
                        Err(e) => {
                            self.warn(format!("font family {uid}: fonts left out: {e}"));
                            if let Some(d) = self.chunk(uid, font::chunk::FAMILY)? {
                                fonts.insert(
                                    uid,
                                    FontFamily {
                                        uid,
                                        name: find_string(&d, 0)?,
                                        fonts: Vec::new(),
                                        writing_script: 0,
                                    },
                                );
                            }
                        }
                    },
                    class::TEXT_VARIABLE => {
                        if let Some(v) = TextVariable::read(uid, &*self.object(uid)?)? {
                            text_variables.push(v);
                        }
                    }
                    hyperlink::class::HYPERLINK => {
                        if let Some(h) = Hyperlink::read(uid, &*self.object(uid)?)? {
                            hyperlinks.push(h);
                        }
                    }
                    hyperlink::class::TEXT_SOURCE => {
                        if let Some(s) = TextSource::read(uid, &*self.object(uid)?)? {
                            text_sources.insert(uid, s);
                        }
                    }
                    hyperlink::class::PAGE_DESTINATION | hyperlink::class::URL_DESTINATION => {
                        if let Some(d) = Destination::read(uid, cls, &*self.object(uid)?)? {
                            if let hyperlink::DestinationKind::Page { zoom: None, .. } = d.kind {
                                self.warn(format!(
                                    "destination {uid}: view zoom is outside 5–4000 %; left out"
                                ));
                            }
                            destinations.push(d);
                        }
                    }
                    xref::CLASS => {
                        if let Some(f) = CrossReferenceFormat::read(uid, &*self.object(uid)?)? {
                            cross_reference_formats.insert(uid, f);
                        }
                    }
                    hyperlink::class::BOOKMARK => {
                        if let Some(b) = Bookmark::read(uid, &*self.object(uid)?)? {
                            bookmarks.insert(uid, b);
                        }
                    }
                    cjk::class::COMPOSITE_FONT => {
                        if let Some((name, entries)) = CompositeFont::read(&*self.object(uid)?)? {
                            composite_fonts.push((uid, name, entries));
                        }
                    }
                    cjk::class::COMPOSITE_FONT_ENTRY => {
                        if let Some(e) = CompositeFontEntry::read(uid, &*self.object(uid)?)? {
                            composite_entries.insert(uid, e);
                        }
                    }
                    c if c == cjk::class::MOJIKUMI || cjk::class::KINSOKU.contains(&c) => {
                        if let Some(t) = CjkTable::read(uid, c, &*self.object(uid)?)? {
                            cjk_tables.push(t);
                        }
                    }
                    color::class::INK => {
                        if let Some(i) = Ink::read(uid, &*self.object(uid)?)? {
                            if i.neutral_density.is_none() {
                                self.warn(format!(
                                    "ink {uid}: neutral density is outside 0.001–10; left out"
                                ));
                            }
                            inks.push(i);
                        }
                    }
                    color::class::COLOR_GROUP => {
                        if let Some(g) = ColorGroup::read(uid, &*self.object(uid)?)? {
                            color_groups.insert(uid, g);
                        }
                    }
                    class::XML_TAG => {
                        // Chunk 0xBF2F: u32 length, then the name as text
                        // segments; chunk 0x117: the UID of the tag's colour.
                        if let Some(d) = self.chunk(uid, chunk::XML_TAG_NAME)? {
                            let mut c = Cursor::new(&d);
                            let n = c.u32()? as usize;
                            // Files from InDesign 2.0 to 4.0 hold a flag byte and
                            // an in-object string instead.
                            let name = match c.segments(n) {
                                Ok(name) => name,
                                Err(_) => {
                                    let mut c = Cursor::new(&d);
                                    c.flag()?;
                                    c.string()?
                                }
                            };
                            let color = match self.chunk(uid, chunk::XML_TAG_COLOR)? {
                                Some(d) if d.len() >= 4 => self.ui_color(Cursor::new(&d).u32()?)?,
                                _ => None,
                            };
                            xml_tag_names.insert(uid, name.clone());
                            xml_tags.push((name, color));
                        }
                    }
                    class::LANGUAGE => {
                        if let Some(d) = self.chunk(uid, chunk::LANGUAGE_NAME)?
                            && d.len() > 1
                        {
                            let mut c = Cursor::new(&d);
                            c.flag()?;
                            let name = c.string()?;
                            languages.insert(uid, name.clone());
                            // Then the primary and secondary names, u16
                            // ID, and two vendors (flag, u32, string).
                            let rest = (|| -> Result<_, Error> {
                                c.flag()?;
                                let primary = c.string()?;
                                c.flag()?;
                                let sub = c.string()?;
                                let id = c.u16()?;
                                let mut vendor = || -> Result<(u8, String), Error> {
                                    let flag = c.u8()?;
                                    c.u32()?;
                                    Ok((flag, c.string()?))
                                };
                                let spelling = vendor()?;
                                let hyphenation = vendor()?;
                                Ok((primary, sub, id, [spelling, hyphenation]))
                            })();
                            if let Ok((primary, sub, id, vendors)) = rest {
                                language_list.push(Language {
                                    uid,
                                    name,
                                    primary,
                                    sub,
                                    id,
                                    vendors: Some(vendors),
                                });
                            }
                        }
                    }
                    _ => {}
                }
                Ok(())
            })();
            if let Err(e) = read {
                self.warn(format!("object {uid} (class {cls:#x}) left out: {e}"));
            }
        }
        // The backing story: the document's chunk 0xBF14 names it and the
        // document node.
        let xml_story = match self.chunk(doc, xml::chunk::NODE_REF)? {
            Some(d) if d.len() >= 4 => {
                let s = Cursor::new(&d).u32()?;
                match self.class(s) {
                    Some(class::STORY) => Some(self.story(s)?),
                    _ => None,
                }
            }
            _ => None,
        };
        let mut stories = stories;
        for story in &mut stories {
            story.orientation = self.story_orientation(story.uid);
        }
        let xml = XmlStructure {
            story: xml_story,
            elements: self.xml_elements(&xml_tag_names)?,
        };
        let mut tints = Vec::new();
        for t in tint_objects {
            match colors
                .iter()
                .find(|c| c.uid == t.base && !c.name.is_empty())
            {
                Some(base) => {
                    let reference = t.reference(base);
                    swatches.insert(t.uid, reference.clone());
                    tints.push((t.clone(), reference, t.idml_name(base)));
                }
                None => self.warn(format!(
                    "tint {}: left out: base colour {} has no name",
                    t.uid, t.base
                )),
            }
        }
        prune_style_groups(&mut style_groups, |m| self.warn(m));
        let preferences = self.document_preferences()?;
        let prefs = self.prefs(version.major)?;
        Ok(Document {
            version,
            layers,
            active_layer,
            spreads,
            master_spreads,
            stories,
            styles,
            colors,
            tints,
            gradients,
            swatches,
            fonts,
            languages,
            language_list,
            toc_styles: self.toc_styles(),
            named_grids: self.named_grids(),
            index_groups: self.index_groups(),
            constant_shade: self.constant_shade(),
            assignments: {
                let mut a: Vec<u32> = self
                    .db
                    .classes()
                    .iter()
                    .filter(|&&(_, c)| c == class::ASSIGNMENT)
                    .map(|&(u, _)| u)
                    .collect();
                a.sort_unstable();
                a
            },
            style_groups,
            object_styles,
            cell_styles,
            table_styles,
            users: self.users(doc).unwrap_or_else(|e| {
                self.warn(format!("document users left out: {e}"));
                Vec::new()
            }),
            sections: self
                .uid_list(doc, chunk::DOC_SECTIONS)?
                .into_iter()
                .map(|uid| self.section(uid))
                .collect::<Result<Vec<_>, _>>()?,
            text_variables,
            hyperlinks,
            text_sources,
            destinations,
            bookmarks,
            bookmark_order: match self.chunk(doc, hyperlink::chunk::DOCUMENT_LISTS)? {
                Some(d) => hyperlink::document_bookmarks(&d, version.major)?,
                None => Vec::new(),
            },
            cross_reference_formats,
            warnings: self.warnings.borrow().clone(),
            preferences,
            prefs,
            composite_fonts: composite_fonts
                .into_iter()
                .map(|(uid, name, entries)| CompositeFont {
                    uid,
                    name,
                    entries: entries
                        .iter()
                        .filter_map(|e| composite_entries.get(e).cloned())
                        .collect(),
                })
                .collect(),
            cjk_tables,
            inks,
            color_groups: self
                .color_group_order()?
                .into_iter()
                .filter_map(|u| color_groups.remove(&u))
                .collect(),
            bullets: self.bullets()?,
            xml_tags,
            xml,
        })
    }
}
