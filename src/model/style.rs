//! Text, object and TOC styles and style groups.
//!
//! Evidence: `docs/format/objects.md` (styles, style groups, object
//! styles, TOC styles) and `attributes.md`.

use super::*;

#[derive(Debug, Clone, PartialEq)]
pub struct Style {
    pub uid: u32,
    pub name: String,
    /// The name is an InDesign built-in key, written with `$ID/` in IDML.
    pub builtin: bool,
    pub paragraph: bool,
    pub based_on: Option<u32>,
    pub next: Option<u32>,
    pub attrs: Attrs,
    /// The u16 after the kind, before the name (IDML `Imported`).
    pub imported: bool,
    /// The GUID string after the name in newer files (`StyleUniqueId`).
    pub unique_id: Option<String>,
    /// Keyboard shortcut: u32 key 10 bytes before the name's flag byte,
    /// then the two bytes 6 and 5 before it (`docs/format/objects.md`).
    pub shortcut: Option<(u32, u8, u8)>,
}

/// A table of contents style (class 0x11605), from chunk 0x11605: a flag
/// byte and the name, three u32 (the third the title style), a flag byte
/// and the title. See `docs/format/objects.md`.
#[derive(Debug, Clone, PartialEq)]
pub struct TocStyle {
    pub uid: u32,
    pub name: String,
    pub builtin: bool,
    pub title: String,
    pub title_style: u32,
    /// After a flag byte and a string: u16 at 2 numbered paragraphs, at 4
    /// make anchor and at 6 remove forced line breaks, where the chunk
    /// has them.
    pub flags: Vec<u16>,
}

/// A style group: a root group of styles, or a named group in it.
#[derive(Debug, Clone, PartialEq)]
pub struct StyleGroup {
    pub uid: u32,
    /// Empty for root groups.
    pub name: String,
    /// Root groups: what they hold, from chunk 0x28C2 (see `root_kind`).
    pub root: Option<u32>,
    pub children: Vec<u32>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ObjectStyle {
    pub uid: u32,
    pub name: String,
    pub builtin: bool,
    pub based_on: Option<u32>,
    /// Frame fitting attributes (chunk 0x1B956), same IDs as on page items.
    pub fitting: Attrs,
    /// Page item attributes (chunk 0x1B92B): fill, stroke, corners.
    pub attrs: Attrs,
    /// Text frame settings (chunk 0x1B924).
    pub frame: Option<ObjectStyleFrame>,
    /// Story settings (chunk 0x285B).
    pub story: Option<StorySettings>,
    /// Story direction (chunk 0x50F28, u16).
    pub direction: Option<u16>,
    /// Text wrap (chunk 0x3776, the layout of chunk 0x3703).
    pub text_wrap: Option<TextWrap>,
    /// Contour type of the text wrap (chunk 0x3777).
    pub contour_type: Option<u32>,
    /// IDs of the setting categories the style turns on (chunk 0x1B92E).
    pub enabled: Option<Vec<u32>>,
    /// Paragraph style applied to text frames (chunk 0x1B946).
    pub paragraph_style: Option<u32>,
    /// Anchored object settings (chunk 0x2800).
    pub anchor: Option<AnchorSettings>,
}

/// Style groups nested deeper than this are left out, with a warning.
pub(super) const MAX_GROUP_DEPTH: usize = 100;

/// Make the style groups a tree under their root groups, so that walking
/// them ends: a group listed a second time (in a cycle, or under a second
/// parent) or nested deeper than [`MAX_GROUP_DEPTH`] is removed from the
/// children of the group that lists it, with a warning.
pub(super) fn prune_style_groups(groups: &mut BTreeMap<u32, StyleGroup>, warn: impl Fn(String)) {
    let roots: Vec<u32> = groups
        .iter()
        .filter(|(_, g)| g.root.is_some())
        .map(|(&uid, _)| uid)
        .collect();
    let mut seen: std::collections::HashSet<u32> = roots.iter().copied().collect();
    let mut stack: Vec<(u32, usize)> = roots.iter().map(|&r| (r, 0)).collect();
    while let Some((uid, depth)) = stack.pop() {
        let Some(g) = groups.get_mut(&uid) else {
            continue;
        };
        let children = std::mem::take(&mut g.children);
        let mut kept = Vec::with_capacity(children.len());
        for c in children {
            if groups.contains_key(&c) {
                if !seen.insert(c) {
                    warn(format!(
                        "style group {c} is listed twice; left out of group {uid}"
                    ));
                    continue;
                }
                if depth + 1 > MAX_GROUP_DEPTH {
                    warn(format!(
                        "style group {c} is nested more than {MAX_GROUP_DEPTH} deep; left out"
                    ));
                    continue;
                }
                stack.push((c, depth + 1));
            }
            kept.push(c);
        }
        if let Some(g) = groups.get_mut(&uid) {
            g.children = kept;
        }
    }
}

impl<'a> Reader<'a> {
    pub(super) fn style(&self, uid: u32) -> Result<Option<Style>, Error> {
        let Some(data) = self.chunk(uid, chunk::STYLE_INFO)? else {
            return Ok(None);
        };
        let mut c = self.cursor(&data);
        let next = c.u32()?;
        let based_on = c.u32()?;
        // The header before the name is 4 bytes shorter in files from
        // InDesign 13 and earlier, so locate the name by its structure: a
        // flag byte (1 = built-in name), then an in-object string. Four
        // bytes before the flag, a u16 is 1 for paragraph styles and 0 for
        // character styles; the u16 after it is not identified.
        let Some((at, builtin, name)) = find_flagged_string(self.enc(), &data, 12, |_| true) else {
            return Err(Error::Corrupt(format!("style {uid}: no name")));
        };
        let paragraph = self.cursor(&data[at - 4..]).u16()? != 0;
        let imported = self.cursor(&data[at - 2..]).u16()? != 0;
        let shortcut = if at >= 10 {
            Some((
                self.cursor(&data[at - 10..]).u32()?,
                data[at - 6],
                data[at - 5],
            ))
        } else {
            None
        };
        // A 36-character in-object string after the name: a GUID.
        const GUID: [u8; 6] = [2, 0, 36, 0, 36, 0x40];
        let unique_id = if self.enc().big_endian() {
            None
        } else {
            data[at..]
                .windows(GUID.len())
                .position(|w| w == GUID)
                .and_then(|i| self.cursor(&data[at + i..]).string().ok())
        };
        let attrs = match self.chunk(uid, chunk::STYLE_ATTRS)? {
            Some(d) if d.len() >= 2 => {
                let mut c = self.cursor(&d);
                let n = c.u16()? as usize;
                self.attrs_or_warn(
                    || format!("style {uid}"),
                    Attrs::parse_text(&mut c, n, List::Style, self.db.recorder()),
                )
                .unwrap_or_default()
            }
            _ => Attrs::default(),
        };
        Ok(Some(Style {
            uid,
            name,
            builtin,
            paragraph,
            based_on: uid_or_none(based_on),
            next: uid_or_none(next),
            attrs,
            imported,
            unique_id,
            shortcut,
        }))
    }

    /// The table of contents styles, in UID order.
    pub(super) fn toc_styles(&self) -> Vec<TocStyle> {
        let mut out = Vec::new();
        for &(uid, cls) in self.db.classes() {
            if cls != class::TOC_STYLE {
                continue;
            }
            let read = (|| -> Result<Option<TocStyle>, Error> {
                let Some(d) = self.chunk(uid, chunk::TOC_STYLE)? else {
                    return Ok(None);
                };
                let mut c = self.cursor(&d);
                let builtin = c.flag()? == 1;
                let name = c.string()?;
                c.skip(8)?;
                let title_style = c.u32()?;
                c.flag()?;
                let title = c.string()?;
                c.flag()?;
                c.string()?;
                let mut flags = Vec::new();
                while flags.len() < 4 && c.remaining() >= 2 {
                    flags.push(c.u16()?);
                }
                Ok(Some(TocStyle {
                    uid,
                    name,
                    builtin,
                    title,
                    title_style,
                    flags,
                }))
            })();
            match read {
                Ok(Some(t)) => out.push(t),
                Ok(None) => {}
                Err(e) => self.warn(format!("TOC style {uid} left out: {e}")),
            }
        }
        out.sort_by_key(|t| t.uid);
        out
    }
}

impl<'a> Reader<'a> {
    /// A root style group (the root of paragraph and character, object,
    /// cell or table styles): its kind (chunk 0x28C2) and children.
    pub(super) fn style_root_group(&self, uid: u32) -> Result<StyleGroup, Error> {
        let kind = match self.chunk(uid, chunk::ROOT_GROUP_KIND)? {
            Some(d) => self.cursor(&d).u32()?,
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
        Ok(StyleGroup {
            uid,
            name: String::new(),
            root: Some(kind),
            children,
        })
    }

    /// A named style group: a flag byte and the name, and its children.
    pub(super) fn style_group(&self, uid: u32) -> Result<StyleGroup, Error> {
        let name = match self.chunk(uid, chunk::STYLE_GROUP_NAME)? {
            Some(d) if d.len() > 1 => {
                let mut c = self.cursor(&d);
                c.flag()?;
                c.string()?
            }
            _ => String::new(),
        };
        let children = self.children(uid, chunk::STYLE_GROUP_CHILDREN)?;
        Ok(StyleGroup {
            uid,
            name,
            root: None,
            children,
        })
    }

    /// An object style. `None` if it has no info chunk (based-on style,
    /// flag byte and name).
    pub(super) fn object_style(&self, uid: u32) -> Result<Option<ObjectStyle>, Error> {
        let Some(d) = self.chunk(uid, chunk::OBJECT_STYLE_INFO)? else {
            return Ok(None);
        };
        let mut c = self.cursor(&d);
        let based_on = c.u32()?;
        let builtin = c.flag()? == 1;
        let name = c.string()?;
        let u32_chunk = |id: u32| -> Result<Option<u32>, Error> {
            match self.chunk(uid, id)? {
                Some(d) if d.len() >= 4 => Ok(Some(self.cursor(&d).u32()?)),
                _ => Ok(None),
            }
        };
        Ok(Some(ObjectStyle {
            uid,
            name,
            builtin,
            based_on: uid_or_none(based_on),
            fitting: match self.chunk(uid, chunk::OBJECT_STYLE_FITTING)? {
                Some(d) => self
                    .attrs_or_warn(
                        || format!("object style {uid}, frame fitting"),
                        Attrs::parse_short(
                            self.enc(),
                            &d,
                            List::ObjectStyleFitting,
                            self.db.recorder(),
                        ),
                    )
                    .unwrap_or_default(),
                None => Attrs::default(),
            },
            attrs: match self.chunk(uid, chunk::OBJECT_STYLE_ATTRS)? {
                Some(d) => self
                    .attrs_or_warn(
                        || format!("object style {uid}"),
                        Attrs::parse_short(self.enc(), &d, List::ObjectStyle, self.db.recorder()),
                    )
                    .unwrap_or_default(),
                None => Attrs::default(),
            },
            // The layout is known for these sizes only
            // (`docs/format/big-endian.md`).
            frame: match self.chunk(uid, chunk::OBJECT_STYLE_FRAME)? {
                Some(d) if !matches!(d.len(), 106 | 142 | 162 | 222) => {
                    self.warn(format!(
                        "object style {uid}: text frame settings of {} bytes are not known; left out",
                        d.len()
                    ));
                    None
                }
                d => d.map(|d| ObjectStyleFrame::read(self.enc(), &d)),
            },
            story: self
                .chunk(uid, chunk::OBJECT_STYLE_STORY)?
                .and_then(|d| StorySettings::read(self.enc(), &d)),
            direction: match self.chunk(uid, chunk::OBJECT_STYLE_DIRECTION)? {
                Some(d) if d.len() >= 2 => Some(self.cursor(&d).u16()?),
                _ => None,
            },
            text_wrap: self.wrap_chunk(uid, chunk::OBJECT_STYLE_WRAP)?,
            contour_type: u32_chunk(chunk::OBJECT_STYLE_CONTOUR)?,
            enabled: match self.chunk(uid, chunk::OBJECT_STYLE_ENABLED)? {
                Some(d) => Some(self.cursor(&d).u32_list()?),
                None => None,
            },
            paragraph_style: u32_chunk(chunk::OBJECT_STYLE_PARAGRAPH_STYLE)?,
            anchor: self
                .chunk(uid, chunk::ANCHOR_SETTINGS)?
                .map(|d| AnchorSettings::read(self.enc(), &d)),
        }))
    }
}

/// Anchored object settings (chunk 0x2800, of an anchor, an object style
/// or the preferences). See `docs/format/objects.md`, anchored objects.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct AnchorSettings {
    /// f64 `AnchorYoffset` at 0 and u16 vertical alignment at 52, if the
    /// chunk has 54 bytes.
    pub offset: Option<(f64, u16)>,
    /// The u16 at 46, 50 and 56 (anchor point and pin position) and at
    /// 40 and 48 (anchored position and horizontal alignment), if the
    /// chunk has 58 bytes. Each group changes together in every sample.
    pub fields: Option<([u16; 3], [u16; 2])>,
}

impl AnchorSettings {
    pub(super) fn read(enc: Encoding, d: &[u8]) -> AnchorSettings {
        let u = |o: usize| enc.u16_at(d, o).unwrap_or_default();
        AnchorSettings {
            offset: (d.len() >= 54).then(|| (enc.f64_at(d, 0).unwrap_or_default(), u(52))),
            fields: (d.len() >= 58).then(|| ([u(46), u(50), u(56)], [u(40), u(48)])),
        }
    }
}

/// Text frame settings of an object style (chunk 0x1B924), at the
/// offsets known for its sizes (106, 142, 162 and 222 bytes). Each field
/// is `None` if the chunk is too short for it. See
/// `docs/format/objects.md`, object styles.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ObjectStyleFrame {
    /// f64 at 0.
    pub column_fixed_width: Option<f64>,
    /// f64 at 8.
    pub column_gutter: Option<f64>,
    /// f64 insets at 34, 42, 50 and 58.
    pub insets: Option<[f64; 4]>,
    /// u32 at 66.
    pub column_count: Option<u32>,
    /// Footnotes: u16 span across columns at 144, f64 minimum spacing at
    /// 146 and f64 space between at 154.
    pub footnotes: Option<(u16, f64, f64)>,
    /// Column rule: f64 stroke width at 190, u32 colour at 198 (0 for
    /// none) and f64 tint at 210.
    pub column_rule: Option<(f64, u32, f64)>,
}

impl ObjectStyleFrame {
    pub(super) fn read(enc: Encoding, d: &[u8]) -> ObjectStyleFrame {
        let f = |o: usize| enc.f64_at(d, o);
        ObjectStyleFrame {
            column_fixed_width: f(0),
            column_gutter: f(8),
            insets: (|| Some([f(34)?, f(42)?, f(50)?, f(58)?]))(),
            column_count: enc.u32_at(d, 66),
            footnotes: (|| Some((enc.u16_at(d, 144)?, f(146)?, f(154)?)))(),
            column_rule: (|| Some((f(190)?, enc.u32_at(d, 198)?, f(210)?)))(),
        }
    }
}

/// Story settings of an object style (chunk 0x285B), if the chunk has 16
/// bytes: u16 story orientation at 0 and u16 frame type at 14.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StorySettings {
    pub orientation: u16,
    pub frame_type: u16,
}

impl StorySettings {
    pub(super) fn read(enc: Encoding, d: &[u8]) -> Option<StorySettings> {
        Some(StorySettings {
            orientation: enc.u16_at(d, 0)?,
            frame_type: enc.u16_at(d, 14)?,
        })
    }
}
