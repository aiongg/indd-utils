//! A document model built from database objects. See
//! `docs/format/objects.md` for the class and chunk layouts used here.

use std::collections::{BTreeMap, HashMap};

use crate::object::{Cursor, Object};
use crate::{Database, Error, Version};

/// Class IDs.
pub mod class {
    pub const DOCUMENT: u32 = 0xE01;
    pub const SPREAD: u32 = 0x501;
    pub const MASTER_SPREAD: u32 = 0x1401;
    pub const SPREAD_LAYER: u32 = 0x301;
    pub const LAYER: u32 = 0x302;
    pub const PAGE: u32 = 0x50F;
    pub const SPLINE_ITEM: u32 = 0x6201;
    pub const GROUP: u32 = 0x401;
    pub const MULTI_COLUMN_FRAME: u32 = 0x263;
    pub const FRAME_COLUMN: u32 = 0x227;
    pub const FRAME_LIST: u32 = 0x228;
    pub const STORY: u32 = 0x201;
    pub const STYLE: u32 = 0x205;
    pub const SECTION: u32 = 0x4C01;
}

/// Chunk IDs.
pub mod chunk {
    pub const DOC_SPREADS: u32 = 0x501;
    pub const DOC_MASTER_SPREADS: u32 = 0x1401;
    pub const DOC_LAYERS: u32 = 0x301;
    pub const DOC_ACTIVE_LAYER: u32 = 0x313;
    pub const DOC_STORIES: u32 = 0x222;
    pub const DOC_SECTIONS: u32 = 0x4C01;
    pub const SPREAD_CHILDREN: u32 = 0x503;
    pub const SPREAD_TRANSFORM: u32 = 0x56E;
    pub const SPREAD_BINDING: u32 = 0x1B8;
    pub const SPREAD_LAYER_LAYER: u32 = 0x302;
    pub const SPREAD_LAYER_CHILDREN: u32 = 0x303;
    pub const LAYER_PROPS: u32 = 0x304;
    pub const PAGE_MASTER: u32 = 0x140F;
    pub const PAGE_TRANSFORM: u32 = 0x5CC;
    pub const PAGE_BOUNDS: u32 = 0x5DD;
    pub const ITEM_TRANSFORM: u32 = 0x151;
    pub const ITEM_PATHS: u32 = 0x162B;
    pub const ITEM_HIERARCHY: u32 = 0x15B;
    pub const COLUMN_FRAME_LIST: u32 = 0x220;
    pub const FRAME_LIST_FRAMES: u32 = 0x205;
    pub const STORY_STRANDS: u32 = 0x223;
    pub const STRAND_DATA: u32 = 0x261;
    pub const STRAND_RUNS: u32 = 0x262;
    pub const STYLE_INFO: u32 = 0x230;
}

/// Kinds of strand run data (first u32 of chunk 0x262).
pub mod strand {
    pub const TEXT: u32 = 0x202;
    pub const CHARACTER_STYLE: u32 = 0x203;
    pub const PARAGRAPH_STYLE: u32 = 0x204;
}

/// A 2D affine transform `[a b c d tx ty]`, as in IDML `ItemTransform`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Matrix(pub [f64; 6]);

impl Matrix {
    pub const IDENTITY: Matrix = Matrix([1.0, 0.0, 0.0, 1.0, 0.0, 0.0]);

    fn read(c: &mut Cursor) -> Result<Matrix, Error> {
        let mut m = [0.0; 6];
        for v in &mut m {
            *v = c.f64()?;
        }
        Ok(Matrix(m))
    }
}

pub type Point = (f64, f64);

#[derive(Debug, Clone, PartialEq)]
pub struct PathPoint {
    pub anchor: Point,
    pub left: Point,
    pub right: Point,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Path {
    pub points: Vec<PathPoint>,
    pub open: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Layer {
    pub uid: u32,
    pub name: String,
    pub visible: bool,
    pub locked: bool,
    /// The hidden layer that holds pages; not written to IDML.
    pub internal: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Page {
    pub uid: u32,
    /// Left, top, right, bottom in page coordinates.
    pub bounds: [f64; 4],
    pub transform: Matrix,
    pub master: Option<u32>,
    pub master_transform: Matrix,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shape {
    Rectangle,
    Oval,
    Polygon,
    GraphicLine,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ItemKind {
    TextFrame {
        story: Option<u32>,
        previous: Option<u32>,
        next: Option<u32>,
    },
    Shape(Shape),
    Group,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PageItem {
    pub uid: u32,
    pub kind: ItemKind,
    pub transform: Matrix,
    pub paths: Vec<Path>,
    pub layer: u32,
    pub children: Vec<PageItem>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Spread {
    pub uid: u32,
    pub transform: Matrix,
    pub binding_location: u32,
    pub pages: Vec<Page>,
    pub items: Vec<PageItem>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Style {
    pub uid: u32,
    pub name: String,
    /// The name is an InDesign built-in key, written with `$ID/` in IDML.
    pub builtin: bool,
    pub paragraph: bool,
    pub based_on: Option<u32>,
    pub next: Option<u32>,
}

/// A stretch of story text with one paragraph style and one character style.
#[derive(Debug, Clone, PartialEq)]
pub struct TextRun {
    pub text: String,
    pub paragraph_style: Option<u32>,
    pub character_style: Option<u32>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Story {
    pub uid: u32,
    pub runs: Vec<TextRun>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Document {
    pub version: Version,
    pub layers: Vec<Layer>,
    pub active_layer: Option<u32>,
    pub spreads: Vec<Spread>,
    pub master_spreads: Vec<Spread>,
    pub stories: Vec<Story>,
    pub styles: BTreeMap<u32, Style>,
}

/// Reads typed objects from a database, caching them.
pub struct Reader<'a> {
    db: &'a Database<'a>,
    cache: std::cell::RefCell<HashMap<u32, std::rc::Rc<Object>>>,
}

fn uid_or_none(v: u32) -> Option<u32> {
    (v != 0).then_some(v)
}

impl<'a> Reader<'a> {
    pub fn new(db: &'a Database<'a>) -> Reader<'a> {
        Reader {
            db,
            cache: Default::default(),
        }
    }

    pub fn object(&self, uid: u32) -> Result<std::rc::Rc<Object>, Error> {
        if let Some(o) = self.cache.borrow().get(&uid) {
            return Ok(o.clone());
        }
        let obj = self
            .db
            .get(uid)?
            .ok_or_else(|| Error::Corrupt(format!("object {uid} has no data")))?;
        let obj = std::rc::Rc::new(obj);
        self.cache.borrow_mut().insert(uid, obj.clone());
        Ok(obj)
    }

    pub fn class(&self, uid: u32) -> Option<u32> {
        self.db.class_of(uid)
    }

    fn chunk(&self, uid: u32, id: u32) -> Result<Option<Vec<u8>>, Error> {
        Ok(self.object(uid)?.chunk(id).map(<[u8]>::to_vec))
    }

    fn required(&self, uid: u32, id: u32) -> Result<Vec<u8>, Error> {
        self.chunk(uid, id)?
            .ok_or_else(|| Error::Corrupt(format!("object {uid} has no chunk {id:#x}")))
    }

    fn uid_list(&self, uid: u32, id: u32) -> Result<Vec<u32>, Error> {
        match self.chunk(uid, id)? {
            Some(data) => Cursor::new(&data).u32_list(),
            None => Ok(Vec::new()),
        }
    }

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
        let spreads = self
            .uid_list(doc, chunk::DOC_SPREADS)?
            .into_iter()
            .map(|uid| self.spread(uid))
            .collect::<Result<Vec<_>, _>>()?;
        let master_spreads = self
            .uid_list(doc, chunk::DOC_MASTER_SPREADS)?
            .into_iter()
            .map(|uid| self.spread(uid))
            .collect::<Result<Vec<_>, _>>()?;
        let stories = self
            .uid_list(doc, chunk::DOC_STORIES)?
            .into_iter()
            .filter(|&uid| self.class(uid) == Some(class::STORY))
            .map(|uid| self.story(uid))
            .collect::<Result<Vec<_>, _>>()?;
        let mut styles = BTreeMap::new();
        for &(uid, cls) in self.db.classes() {
            if cls == class::STYLE
                && let Some(style) = self.style(uid)?
            {
                styles.insert(uid, style);
            }
        }
        Ok(Document {
            version,
            layers,
            active_layer,
            spreads,
            master_spreads,
            stories,
            styles,
        })
    }

    fn layer(&self, uid: u32) -> Result<Layer, Error> {
        let data = self.required(uid, chunk::LAYER_PROPS)?;
        let mut c = Cursor::new(&data);
        let locked = c.u16()? != 0;
        let visible = c.u16()? != 0;
        c.skip(14)?;
        // A string follows; its position varies, so search for the tag.
        let name = find_string(&data, 18)?;
        Ok(Layer {
            uid,
            internal: name == "Internal_pages_layer_name",
            name,
            visible,
            locked,
        })
    }

    fn spread(&self, uid: u32) -> Result<Spread, Error> {
        let transform = match self.chunk(uid, chunk::SPREAD_TRANSFORM)? {
            Some(d) => Matrix::read(&mut Cursor::new(&d))?,
            None => Matrix::IDENTITY,
        };
        let binding_location = match self.chunk(uid, chunk::SPREAD_BINDING)? {
            Some(d) => Cursor::new(&d).u32()?,
            None => 0,
        };
        let children = self.required(uid, chunk::SPREAD_CHILDREN)?;
        let mut c = Cursor::new(&children);
        c.skip(8)?;
        let spread_layers = c.u32_list()?;
        let mut pages = Vec::new();
        let mut items = Vec::new();
        for sl in spread_layers {
            let layer = match self.chunk(sl, chunk::SPREAD_LAYER_LAYER)? {
                Some(d) => Cursor::new(&d).u32()?,
                None => 0,
            };
            for child in self.children(sl, chunk::SPREAD_LAYER_CHILDREN)? {
                match self.class(child) {
                    Some(class::PAGE) => pages.push(self.page(child)?),
                    _ => {
                        if let Some(item) = self.page_item(child, layer)? {
                            items.push(item);
                        }
                    }
                }
            }
        }
        Ok(Spread {
            uid,
            transform,
            binding_location,
            pages,
            items,
        })
    }

    /// Children listed in a hierarchy chunk: parent, owner, then a u32 list.
    fn children(&self, uid: u32, id: u32) -> Result<Vec<u32>, Error> {
        match self.chunk(uid, id)? {
            Some(d) => {
                let mut c = Cursor::new(&d);
                c.skip(8)?;
                c.u32_list()
            }
            None => Ok(Vec::new()),
        }
    }

    fn page(&self, uid: u32) -> Result<Page, Error> {
        let transform = Matrix::read(&mut Cursor::new(
            &self.required(uid, chunk::PAGE_TRANSFORM)?,
        ))?;
        let b = self.required(uid, chunk::PAGE_BOUNDS)?;
        let mut c = Cursor::new(&b);
        let bounds = [c.f64()?, c.f64()?, c.f64()?, c.f64()?];
        let (master, master_transform) = match self.chunk(uid, chunk::PAGE_MASTER)? {
            Some(d) => {
                let mut c = Cursor::new(&d);
                let m = c.u32()?;
                c.skip(2)?;
                (uid_or_none(m), Matrix::read(&mut c)?)
            }
            None => (None, Matrix::IDENTITY),
        };
        Ok(Page {
            uid,
            bounds,
            transform,
            master,
            master_transform,
        })
    }

    fn paths(&self, uid: u32) -> Result<Vec<Path>, Error> {
        let Some(data) = self.chunk(uid, chunk::ITEM_PATHS)? else {
            return Ok(Vec::new());
        };
        let mut c = Cursor::new(&data);
        let n = c.u32()?;
        let mut paths = Vec::new();
        for _ in 0..n {
            let count = c.u32()?;
            let mut points = Vec::new();
            for _ in 0..count {
                let kind = c.u32()?;
                let p = match kind {
                    2 => {
                        let a = (c.f64()?, c.f64()?);
                        PathPoint {
                            anchor: a,
                            left: a,
                            right: a,
                        }
                    }
                    0 | 1 => {
                        let left = (c.f64()?, c.f64()?);
                        let anchor = (c.f64()?, c.f64()?);
                        let right = (c.f64()?, c.f64()?);
                        PathPoint {
                            anchor,
                            left,
                            right,
                        }
                    }
                    other => {
                        return Err(Error::Corrupt(format!(
                            "object {uid}: unknown path point type {other}"
                        )));
                    }
                };
                points.push(p);
            }
            let open = c.u16()? != 0;
            paths.push(Path { points, open });
        }
        Ok(paths)
    }

    fn page_item(&self, uid: u32, layer: u32) -> Result<Option<PageItem>, Error> {
        let cls = self.class(uid);
        if cls != Some(class::SPLINE_ITEM) && cls != Some(class::GROUP) {
            return Ok(None);
        }
        let transform = match self.chunk(uid, chunk::ITEM_TRANSFORM)? {
            Some(d) => Matrix::read(&mut Cursor::new(&d))?,
            None => Matrix::IDENTITY,
        };
        let child_uids = self.children(uid, chunk::ITEM_HIERARCHY)?;
        let mut children = Vec::new();
        let mut text_column = None;
        for &child in &child_uids {
            if self.class(child) == Some(class::MULTI_COLUMN_FRAME) {
                text_column = self
                    .children(child, chunk::ITEM_HIERARCHY)?
                    .into_iter()
                    .find(|&c| self.class(c) == Some(class::FRAME_COLUMN));
            } else if let Some(item) = self.page_item(child, layer)? {
                children.push(item);
            }
        }
        let paths = self.paths(uid)?;
        let kind = if cls == Some(class::GROUP) {
            ItemKind::Group
        } else if let Some(column) = text_column {
            self.text_frame_links(column)?
        } else {
            ItemKind::Shape(classify(&paths))
        };
        Ok(Some(PageItem {
            uid,
            kind,
            transform,
            paths,
            layer,
            children,
        }))
    }

    /// Story and threading of the text frame owning `column`.
    fn text_frame_links(&self, column: u32) -> Result<ItemKind, Error> {
        let none = ItemKind::TextFrame {
            story: None,
            previous: None,
            next: None,
        };
        let Some(d) = self.chunk(column, chunk::COLUMN_FRAME_LIST)? else {
            return Ok(none);
        };
        let list = Cursor::new(&d).u32()?;
        let Some(d) = self.chunk(list, chunk::FRAME_LIST_FRAMES)? else {
            return Ok(none);
        };
        let mut c = Cursor::new(&d);
        let story = uid_or_none(c.u32()?);
        let columns = c.u32_list()?;
        // Columns belong to frames: column -> multi-column frame -> spline item.
        let frame_of = |col: u32| -> Result<Option<u32>, Error> {
            let Some(h) = self.chunk(col, chunk::ITEM_HIERARCHY)? else {
                return Ok(None);
            };
            let mcf = Cursor::new(&h[4..]).u32()?;
            let Some(h) = self.chunk(mcf, chunk::ITEM_HIERARCHY)? else {
                return Ok(None);
            };
            Ok(uid_or_none(Cursor::new(&h[4..]).u32()?))
        };
        let mut frames: Vec<u32> = Vec::new();
        for col in columns.iter().copied() {
            if let Some(f) = frame_of(col)?
                && frames.last() != Some(&f)
            {
                frames.push(f);
            }
        }
        let me = frame_of(column)?;
        let pos = me.and_then(|m| frames.iter().position(|&f| f == m));
        Ok(ItemKind::TextFrame {
            story,
            previous: pos.and_then(|i| i.checked_sub(1)).map(|i| frames[i]),
            next: pos.and_then(|i| frames.get(i + 1).copied()),
        })
    }

    fn style(&self, uid: u32) -> Result<Option<Style>, Error> {
        let Some(data) = self.chunk(uid, chunk::STYLE_INFO)? else {
            return Ok(None);
        };
        let mut c = Cursor::new(&data);
        let next = c.u32()?;
        let based_on = c.u32()?;
        // The header before the name is 4 bytes shorter in files from
        // InDesign 13 and earlier, so locate the name by its structure: a
        // flag byte (1 = built-in name), then an in-object string. The u32
        // before the flag is 1 for paragraph styles and 0 for character styles.
        let Some(at) = (12..data.len().saturating_sub(6)).find(|&i| {
            data[i] <= 2 && data[i + 1] == 2 && Cursor::new(&data[i + 1..]).string().is_ok()
        }) else {
            return Err(Error::Corrupt(format!("style {uid}: no name")));
        };
        let paragraph = Cursor::new(&data[at - 4..]).u32()? != 0;
        let name = Cursor::new(&data[at + 1..]).string()?;
        Ok(Some(Style {
            uid,
            name,
            builtin: data[at] == 1,
            paragraph,
            based_on: uid_or_none(based_on),
            next: uid_or_none(next),
        }))
    }

    fn story(&self, uid: u32) -> Result<Story, Error> {
        let data = self.required(uid, chunk::STORY_STRANDS)?;
        let mut c = Cursor::new(&data);
        c.skip(6)?;
        let mut strands = vec![c.u32()?];
        strands.extend(c.u32_list()?);
        let mut text = String::new();
        let mut text_units = 0usize;
        let mut para: Vec<(usize, u32)> = Vec::new();
        let mut chars: Vec<(usize, u32)> = Vec::new();
        for strand in strands {
            let Some(list) = self.chunk(strand, chunk::STRAND_DATA)? else {
                continue;
            };
            let mut c = Cursor::new(&list);
            let n = c.u16()?;
            for _ in 0..n {
                let _len = c.u32()?;
                let data_uid = c.u32()?;
                let runs = self.required(data_uid, chunk::STRAND_RUNS)?;
                let mut r = Cursor::new(&runs);
                let kind = r.u32()?;
                r.skip(4)?;
                let count = r.u16()?;
                for _ in 0..count {
                    let size = r.u32()? as usize;
                    let rec = r.bytes(size)?;
                    let mut rc = Cursor::new(rec);
                    let len = rc.u32()? as usize;
                    match kind {
                        strand::TEXT => {
                            text.push_str(&rc.segments(len)?);
                            text_units += len;
                        }
                        strand::PARAGRAPH_STYLE => para.push((len, rc.u32()?)),
                        strand::CHARACTER_STYLE => chars.push((len, rc.u32()?)),
                        _ => {}
                    }
                }
            }
        }
        Ok(Story {
            uid,
            runs: split_runs(&text, text_units, &para, &chars),
        })
    }
}

/// Find the first in-object string at or after `from`.
fn find_string(data: &[u8], from: usize) -> Result<String, Error> {
    for i in from..data.len().saturating_sub(4) {
        if data[i] == 2 && data[i + 1] == 0 {
            let n = u16::from_le_bytes([data[i + 2], data[i + 3]]) as usize;
            if n > 0
                && i + 6 <= data.len()
                && (data[i + 5] & 0xC0) != 0
                && let Ok(s) = Cursor::new(&data[i..]).string()
            {
                return Ok(s);
            }
        }
    }
    Ok(String::new())
}

/// Combine text with paragraph-style and character-style run lengths
/// (counted in UTF-16 code units) into runs with both styles constant.
fn split_runs(
    text: &str,
    units: usize,
    para: &[(usize, u32)],
    chars: &[(usize, u32)],
) -> Vec<TextRun> {
    let utf16: Vec<u16> = text.encode_utf16().collect();
    debug_assert_eq!(utf16.len(), units);
    let mut cuts: Vec<usize> = vec![0, utf16.len()];
    for list in [para, chars] {
        let mut pos = 0;
        for &(len, _) in list {
            pos += len;
            cuts.push(pos.min(utf16.len()));
        }
    }
    cuts.sort_unstable();
    cuts.dedup();
    let style_at = |list: &[(usize, u32)], at: usize| {
        let mut pos = 0;
        for &(len, style) in list {
            if at < pos + len {
                return uid_or_none(style);
            }
            pos += len;
        }
        None
    };
    cuts.windows(2)
        .filter(|w| w[0] < w[1])
        .map(|w| TextRun {
            text: String::from_utf16_lossy(&utf16[w[0]..w[1]]),
            paragraph_style: style_at(para, w[0]),
            character_style: style_at(chars, w[0]),
        })
        .collect()
}

fn classify(paths: &[Path]) -> Shape {
    let [path] = paths else {
        return Shape::Polygon;
    };
    let corners = path
        .points
        .iter()
        .all(|p| p.left == p.anchor && p.right == p.anchor);
    if path.open && path.points.len() == 2 {
        return Shape::GraphicLine;
    }
    if !path.open && path.points.len() == 4 && corners {
        let xs: Vec<f64> = path.points.iter().map(|p| p.anchor.0).collect();
        let ys: Vec<f64> = path.points.iter().map(|p| p.anchor.1).collect();
        let distinct = |v: &[f64]| {
            let mut v: Vec<i64> = v.iter().map(|x| (x * 1e6).round() as i64).collect();
            v.sort_unstable();
            v.dedup();
            v.len()
        };
        if distinct(&xs) == 2 && distinct(&ys) == 2 {
            return Shape::Rectangle;
        }
    }
    if !path.open && path.points.len() == 4 && path.points.iter().all(|p| p.left != p.anchor) {
        return Shape::Oval;
    }
    Shape::Polygon
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_runs_at_style_boundaries() {
        let runs = split_runs("abcdef", 6, &[(4, 10), (2, 11)], &[(2, 20), (4, 21)]);
        let texts: Vec<_> = runs.iter().map(|r| r.text.as_str()).collect();
        assert_eq!(texts, ["ab", "cd", "ef"]);
        assert_eq!(runs[1].paragraph_style, Some(10));
        assert_eq!(runs[1].character_style, Some(21));
        assert_eq!(runs[2].paragraph_style, Some(11));
    }

    #[test]
    fn classifies_shapes() {
        let corner = |x, y| PathPoint {
            anchor: (x, y),
            left: (x, y),
            right: (x, y),
        };
        let rect = Path {
            points: vec![
                corner(0., 0.),
                corner(0., 1.),
                corner(1., 1.),
                corner(1., 0.),
            ],
            open: false,
        };
        assert_eq!(classify(&[rect]), Shape::Rectangle);
        let line = Path {
            points: vec![corner(0., 0.), corner(3., 1.)],
            open: true,
        };
        assert_eq!(classify(&[line]), Shape::GraphicLine);
    }
}
