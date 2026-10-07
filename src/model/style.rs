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
    /// Text frame settings (chunk 0x1B924); see `docs/format/objects.md`.
    pub frame: Option<Vec<u8>>,
    /// Story settings (chunk 0x285B).
    pub story: Option<Vec<u8>>,
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
    pub anchor: Option<Vec<u8>>,
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
        let mut c = Cursor::new(&data);
        let next = c.u32()?;
        let based_on = c.u32()?;
        // The header before the name is 4 bytes shorter in files from
        // InDesign 13 and earlier, so locate the name by its structure: a
        // flag byte (1 = built-in name), then an in-object string. Four
        // bytes before the flag, a u16 is 1 for paragraph styles and 0 for
        // character styles; the u16 after it is not identified.
        let Some((at, builtin, name)) = find_flagged_string(&data, 12, |_| true) else {
            return Err(Error::Corrupt(format!("style {uid}: no name")));
        };
        let paragraph = Cursor::new(&data[at - 4..]).u16()? != 0;
        let imported = Cursor::new(&data[at - 2..]).u16()? != 0;
        let shortcut = if at >= 10 {
            Some((
                Cursor::new(&data[at - 10..]).u32()?,
                data[at - 6],
                data[at - 5],
            ))
        } else {
            None
        };
        // A 36-character in-object string after the name: a GUID.
        const GUID: [u8; 6] = [2, 0, 36, 0, 36, 0x40];
        let unique_id = if crate::object::big_endian() {
            None
        } else {
            data[at..]
                .windows(GUID.len())
                .position(|w| w == GUID)
                .and_then(|i| Cursor::new(&data[at + i..]).string().ok())
        };
        let attrs = match self.chunk(uid, chunk::STYLE_ATTRS)? {
            Some(d) if d.len() >= 2 => {
                let mut c = Cursor::new(&d);
                let n = c.u16()? as usize;
                Attrs::parse_text(&mut c, n, List::Style).unwrap_or_default()
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
                let mut c = Cursor::new(&d);
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
