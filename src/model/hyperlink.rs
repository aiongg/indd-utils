//! Hyperlinks, their text sources and destinations, and bookmarks. See
//! `docs/format/hyperlinks.md`.

use std::collections::BTreeMap;

use crate::Error;
use crate::object::{Cursor, Object};

pub mod class {
    pub const HYPERLINK: u32 = 0x13501;
    pub const TEXT_SOURCE: u32 = 0x13502;
    pub const PAGE_DESTINATION: u32 = 0x13505;
    pub const URL_DESTINATION: u32 = 0x13506;
    pub const BOOKMARK: u32 = 0x1354C;
    /// A story strand that holds text ranges in a tree.
    pub const RANGE_STRAND: u32 = 0xCA1C;
    /// A node of a range tree.
    pub const RANGE_NODE: u32 = 0xCA1E;
}

pub mod chunk {
    /// Document: lists of text sources, hyperlinks and bookmarks.
    pub const DOCUMENT_LISTS: u32 = 0x13501;
    pub const HYPERLINK: u32 = 0x13502;
    /// Hyperlink: appearance, not identified.
    pub const HYPERLINK_APPEARANCE: u32 = 0x13553;
    pub const TEXT_SOURCE: u32 = 0x13504;
    pub const TEXT_SOURCE_RANGE: u32 = 0x1352E;
    /// Text source: alternative destination (sources made by a table of
    /// contents).
    pub const TEXT_SOURCE_ALTERNATIVE: u32 = 0x135B7;
    pub const PAGE_DESTINATION: u32 = 0x13507;
    pub const PAGE_DESTINATION_VIEW: u32 = 0x13527;
    pub const URL_DESTINATION: u32 = 0x13509;
    /// URL destination: the URL.
    pub const URL: u32 = 0x100B;
    pub const BOOKMARK: u32 = 0x13547;
    /// Range strand: the first page of its tree.
    pub const RANGE_TREE: u32 = 0x16126;
    /// Range tree page: links and nodes.
    pub const RANGE_PAGE: u32 = 0x16127;
}

#[derive(Debug, Clone, PartialEq)]
pub struct Hyperlink {
    pub uid: u32,
    pub name: String,
    pub source: u32,
    pub hidden: bool,
    /// `DestinationUniqueKey`, shared with the destination.
    pub key: u32,
    /// The fields not identified hold values seen in the corpus pairs, so
    /// the appearance attributes are those of the samples.
    pub as_in_samples: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TextSource {
    pub uid: u32,
    pub name: String,
    pub hidden: bool,
    pub character_style: Option<u32>,
    /// Chunk 0x135B7 holds the one value seen in the corpus, which IDML
    /// writes as a table of contents `AlternativeDestination`.
    pub toc_anchor: bool,
}

/// The only value of chunk 0x135B7 in the corpus.
const TOC_ANCHOR: [u8; 21] = [
    2, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
];

#[derive(Debug, Clone, PartialEq)]
pub enum DestinationKind {
    Page {
        page: u32,
        /// View zoom as a fraction (1 = 100 %).
        zoom: f64,
        /// View setting code.
        view: u32,
    },
    Url {
        url: String,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Destination {
    pub uid: u32,
    pub name: String,
    pub hidden: bool,
    pub key: u32,
    pub kind: DestinationKind,
}

impl Destination {
    /// The IDML `Self` (and reference) of the destination.
    pub fn reference(&self) -> String {
        let escaped = self.name.replace('%', "%25").replace(':', "%3a");
        match self.kind {
            DestinationKind::Page { .. } => format!("HyperlinkPageDestination/{escaped}"),
            DestinationKind::Url { .. } => format!("HyperlinkURLDestination/{escaped}"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Bookmark {
    pub uid: u32,
    pub name: String,
    /// The parent bookmark, or the document (UID 1).
    pub parent: u32,
    pub children: Vec<u32>,
    pub destination: u32,
}

/// A flag byte, then an in-object string.
fn flagged_string(c: &mut Cursor) -> Result<String, Error> {
    c.u8()?;
    c.string()
}

/// Values of the unidentified hyperlink fields in the corpus pairs:
/// chunk 0x13553, and the three u32 at offset 12 of chunk 0x13502.
const SAMPLE_APPEARANCE: &[[u8; 18]] = &[
    [0, 0, 1, 0, 0, 0, 0x21, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [0, 0, 1, 0, 0, 0, 0x1B, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
];

impl Hyperlink {
    pub fn read(uid: u32, obj: &Object) -> Result<Option<Hyperlink>, Error> {
        let Some(d) = obj.chunk(chunk::HYPERLINK) else {
            return Ok(None);
        };
        let mut c = Cursor::new(d);
        let source = c.u32()?;
        c.skip(2)?;
        let hidden = c.u16()? != 0;
        let key = c.u32()?;
        let (a, b, k) = (c.u32()?, c.u32()?, c.u32()?);
        let name = flagged_string(&mut c)?;
        let appearance = obj.chunk(chunk::HYPERLINK_APPEARANCE);
        let as_in_samples = matches!(a, 2001 | 2017)
            && b == 2007
            && k == 0x13501
            && appearance.is_some_and(|ap| SAMPLE_APPEARANCE.iter().any(|s| s[..] == *ap));
        Ok(Some(Hyperlink {
            uid,
            name,
            source,
            hidden,
            key,
            as_in_samples,
        }))
    }
}

impl TextSource {
    pub fn read(uid: u32, obj: &Object) -> Result<Option<TextSource>, Error> {
        let Some(d) = obj.chunk(chunk::TEXT_SOURCE) else {
            return Ok(None);
        };
        let mut c = Cursor::new(d);
        let hidden = c.u8()? != 0;
        c.skip(5)?;
        let name = flagged_string(&mut c)?;
        let character_style = match obj.chunk(chunk::TEXT_SOURCE_RANGE) {
            Some(r) => {
                let mut c = Cursor::new(r);
                c.skip(8)?;
                Some(c.u32()?).filter(|&s| s != 0)
            }
            None => None,
        };
        Ok(Some(TextSource {
            uid,
            name,
            hidden,
            character_style,
            toc_anchor: obj.chunk(chunk::TEXT_SOURCE_ALTERNATIVE) == Some(&TOC_ANCHOR[..]),
        }))
    }
}

impl Destination {
    pub fn read(uid: u32, class: u32, obj: &Object) -> Result<Option<Destination>, Error> {
        let id = if class == class::PAGE_DESTINATION {
            chunk::PAGE_DESTINATION
        } else {
            chunk::URL_DESTINATION
        };
        let Some(d) = obj.chunk(id) else {
            return Ok(None);
        };
        let mut c = Cursor::new(d);
        let hidden = c.u8()? != 0;
        c.skip(1)?;
        let name = flagged_string(&mut c)?;
        let key = c.u32()?;
        let kind = if class == class::PAGE_DESTINATION {
            let Some(v) = obj.chunk(chunk::PAGE_DESTINATION_VIEW) else {
                return Ok(None);
            };
            let mut c = Cursor::new(v);
            let page = c.u32()?;
            let zoom = c.f64()?;
            let view = c.u32()?;
            DestinationKind::Page { page, zoom, view }
        } else {
            let url = match obj.chunk(chunk::URL) {
                Some(u) => flagged_string(&mut Cursor::new(u))?,
                None => name.clone(),
            };
            DestinationKind::Url { url }
        };
        Ok(Some(Destination {
            uid,
            name,
            hidden,
            key,
            kind,
        }))
    }
}

impl Bookmark {
    pub fn read(uid: u32, obj: &Object) -> Result<Option<Bookmark>, Error> {
        let Some(d) = obj.chunk(chunk::BOOKMARK) else {
            return Ok(None);
        };
        let mut c = Cursor::new(d);
        let name = flagged_string(&mut c)?;
        c.skip(4)?;
        let parent = c.u32()?;
        let children = c.u32_list()?;
        let destination = c.u32()?;
        Ok(Some(Bookmark {
            uid,
            name,
            parent,
            children,
            destination,
        }))
    }
}

/// Document chunk 0x13501: u32, u16, then UID lists of text sources,
/// hyperlinks and bookmarks.
pub fn document_bookmarks(data: &[u8]) -> Result<Vec<u32>, Error> {
    let mut c = Cursor::new(data);
    c.skip(6)?;
    c.u32_list()?;
    c.u32_list()?;
    c.u32_list()
}

/// A stretch of story text that is a hyperlink text source.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SourceRange {
    /// UTF-16 offset in the story text.
    pub start: usize,
    pub len: usize,
    pub source: u32,
}

/// One node of a range tree.
struct Node {
    parent: u32,
    left: u32,
    right: u32,
    value: u32,
    len: u32,
    source: u32,
}

/// Read the nodes of one range tree page (chunk 0x16127) into `nodes`, by
/// node index. Returns the next page's UID (0 for none).
fn read_page(data: &[u8], nodes: &mut BTreeMap<u32, Node>) -> Result<u32, Error> {
    let mut c = Cursor::new(data);
    let next = c.u32()?;
    c.skip(12)?;
    let n = c.u32()? as usize;
    c.skip(4 * n)?;
    let count = c.u32()?;
    for _ in 0..count {
        let class = c.u32()?;
        let size = c.u32()? as usize;
        let body = c.bytes(size)?;
        if class != class::RANGE_NODE || size != 60 {
            continue;
        }
        let mut b = Cursor::new(body);
        b.skip(12)?;
        let mut refs = [0u32; 4];
        for r in &mut refs {
            let object = b.u32()?;
            let index = b.u32()?;
            *r = if object == 0 { 0 } else { index };
        }
        let [parent, me, left, right] = refs;
        nodes.insert(
            me,
            Node {
                parent,
                left,
                right,
                value: b.u32()?,
                len: b.u32()?,
                source: b.u32()?,
            },
        );
    }
    Ok(next)
}

/// The text ranges held in the range tree of a strand. Each node's start
/// is stored relative to its parent: a left child starts `value` before
/// its parent, a right child `value` after it; the root's value is its
/// start.
pub fn source_ranges(
    first_page: u32,
    mut page_data: impl FnMut(u32) -> Result<Option<Vec<u8>>, Error>,
) -> Result<Vec<SourceRange>, Error> {
    let mut nodes = BTreeMap::new();
    let mut page = first_page;
    let mut seen = Vec::new();
    while page != 0 && !seen.contains(&page) {
        seen.push(page);
        let Some(data) = page_data(page)? else { break };
        page = read_page(&data, &mut nodes)?;
    }
    let mut out = Vec::new();
    let roots: Vec<u32> = nodes
        .iter()
        .filter(|(_, n)| n.parent == 0)
        .map(|(&k, _)| k)
        .collect();
    let bad = || Error::Corrupt("range tree is inconsistent".into());
    for root in roots {
        let mut stack = vec![(root, nodes[&root].value as i64)];
        while let Some((k, pos)) = stack.pop() {
            if out.len() > nodes.len() {
                return Err(bad());
            }
            let n = nodes.get(&k).ok_or_else(bad)?;
            if pos < 0 {
                return Err(bad());
            }
            out.push(SourceRange {
                start: pos as usize,
                len: n.len as usize,
                source: n.source,
            });
            if n.left != 0 {
                let l = nodes.get(&n.left).ok_or_else(bad)?;
                stack.push((n.left, pos - l.value as i64));
            }
            if n.right != 0 {
                let r = nodes.get(&n.right).ok_or_else(bad)?;
                stack.push((n.right, pos + r.value as i64));
            }
        }
    }
    out.sort_by_key(|r| r.start);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(
        parent: u32,
        me: u32,
        left: u32,
        right: u32,
        value: u32,
        len: u32,
        src: u32,
    ) -> Vec<u8> {
        let mut v = class::RANGE_NODE.to_le_bytes().to_vec();
        v.extend(60u32.to_le_bytes());
        v.extend([1, 0, 0xE4, 0xCA, 0, 0, 0x32, 0, 0, 0, 0, 0]);
        for r in [parent, me, left, right] {
            v.extend((if r == 0 { 0u32 } else { 0x61 }).to_le_bytes());
            v.extend(r.to_le_bytes());
        }
        for x in [value, len, src, 0] {
            v.extend(x.to_le_bytes());
        }
        v
    }

    #[test]
    fn positions_are_relative_to_the_parent() {
        // Root at 260; left child 138 before it, right child 120 after.
        let mut page = Vec::new();
        for x in [0u32, 0, 0x61, 185, 3, 1, 2, 3, 3] {
            page.extend(x.to_le_bytes());
        }
        page.extend(node(2, 1, 0, 0, 138, 22, 0xA));
        page.extend(node(0, 2, 1, 3, 260, 38, 0xB));
        page.extend(node(2, 3, 0, 0, 120, 15, 0xC));
        let ranges = source_ranges(7, |_| Ok(Some(page.clone()))).unwrap();
        let got: Vec<_> = ranges.iter().map(|r| (r.start, r.len, r.source)).collect();
        assert_eq!(got, [(122, 22, 0xA), (260, 38, 0xB), (380, 15, 0xC)]);
    }
}
