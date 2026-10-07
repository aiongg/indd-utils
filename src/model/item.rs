//! Page items: geometry (transforms and paths), groups, frames and shapes,
//! their settings, text wrap, text frame preferences, placed graphics and
//! links.
//!
//! Evidence: `docs/format/objects.md` (page items, graphics, links),
//! `attributes.md` (page item attributes) and `transparency.md`.

use super::*;

/// A 2D affine transform `[a b c d tx ty]`, as in IDML `ItemTransform`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Matrix(pub [f64; 6]);

impl Matrix {
    pub const IDENTITY: Matrix = Matrix([1.0, 0.0, 0.0, 1.0, 0.0, 0.0]);

    pub(super) fn read(c: &mut Cursor) -> Result<Matrix, Error> {
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
    /// The stored point type: 2 a corner without direction points, 0 or
    /// 1 a point with them.
    pub kind: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Path {
    pub points: Vec<PathPoint>,
    pub open: bool,
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
        preferences: Option<TextFramePreferences>,
    },
    Shape(Shape),
    Group,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PageItem {
    pub uid: u32,
    /// Placed graphics (images, PDF, EPS, SVG) inside a frame.
    pub graphics: Vec<Graphic>,
    pub kind: ItemKind,
    pub transform: Matrix,
    pub paths: Vec<Path>,
    /// Document layer; `None` for items anchored in text.
    pub layer: Option<u32>,
    /// Local formatting (fill, stroke, corners, ...).
    pub attrs: Attrs,
    pub object_style: Option<u32>,
    pub children: Vec<PageItem>,
    /// Text wrap (chunk 0x3703); `None` if the item has none.
    pub text_wrap: Option<TextWrap>,
    /// Anchored object settings (chunk 0x2800 of the anchor), for an item
    /// anchored in text.
    pub anchor: Option<AnchorSettings>,
    pub props: ItemProps,
}

/// Settings every page item has. See `docs/format/objects.md`, page item
/// settings.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ItemProps {
    /// The name (chunk 0x2C10, of a group 0x418); `None` without the
    /// chunk.
    pub name: Option<ItemName>,
    /// Chunk 0x2C32 is 0.
    pub hidden: bool,
    /// Chunk 0x2C2D is 1.
    pub locked: bool,
    /// Layout constraint flags (chunk 0x22228).
    pub layout_constraints: Option<u8>,
    /// The parent, target and last updated interface change counts
    /// (chunks 0x21D4E, 0x21D50, 0x21D53), each a list of numbers.
    pub change_counts: [Vec<u32>; 3],
    /// The overridden master page item (0 for none) and the IDs of the
    /// attributes overridden (chunk 0x1424).
    pub overridden: Option<(u32, Vec<u32>)>,
    /// Chunk 0x1623 is 1: a frame for a graphic (`ContentType`
    /// `GraphicType` without a graphic).
    pub graphic_frame: bool,
}

/// A page item name: a built-in key or a name given by the user.
#[derive(Debug, Clone, PartialEq)]
pub struct ItemName {
    pub builtin: bool,
    pub name: String,
}

/// Text wrap settings of a page item or graphic (chunk 0x3703).
#[derive(Debug, Clone, PartialEq)]
pub struct TextWrap {
    /// Wrap mode code (see `wrap_mode`).
    pub mode: u32,
    /// The four offsets: left, top, right, bottom.
    pub offsets: [f64; 4],
    /// The u32 at offset 40; 1 in every sample whose IDML has
    /// `Inverse="false"`, `ApplyToMasterPageOnly="false"` and
    /// `TextWrapSide="BothSides"`.
    pub flags: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphicKind {
    Image,
    Pdf,
    Eps,
    Svg,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Link {
    pub uid: u32,
    /// `LinkResourceURI`, for example `file:/Users/me/image.jpg`.
    pub uri: String,
    /// The linked file is stored in the document (`StoredState="Embedded"`).
    pub embedded: bool,
}

/// The bytes of a file stored in the document, if there is one.
pub(super) type EmbeddedFile = Option<Vec<u8>>;

#[derive(Debug, Clone, PartialEq)]
pub struct Graphic {
    pub uid: u32,
    pub kind: GraphicKind,
    pub transform: Matrix,
    /// Left, top, right, bottom in the graphic's own coordinates.
    pub bounds: [f64; 4],
    pub link: Option<Link>,
    /// The graphic's file, when the document holds it: an embedded link or
    /// a graphic pasted without a link. IDML writes it as `Contents`.
    pub contents: Option<Vec<u8>>,
    pub text_wrap: Option<TextWrap>,
    /// Contour type code of the text wrap (chunk 0x373D): 5 = SameAsClipping.
    pub contour_type: Option<u32>,
    /// Clipping path settings (chunk 0x2C1A), if stored.
    pub clipping: Option<ClippingPath>,
    /// Chunk 0x8C39, u16 at 0: apply the Photoshop clipping path (1 true).
    pub photoshop_clipping: Option<u16>,
}

/// Clipping path settings of a graphic (chunk 0x2C1A, 37 bytes): u32 type
/// at 0, f64 tolerance at 4, f64 inset at 12, u8 threshold at 20, i16
/// index at 23, u8 at 25 (2 = use the high-resolution image). See
/// `docs/format/objects.md`.
#[derive(Debug, Clone, PartialEq)]
pub struct ClippingPath {
    pub kind: u32,
    pub tolerance: f64,
    pub inset: f64,
    pub threshold: u8,
    pub index: i16,
    pub high_resolution: u8,
}

/// Text frame settings, from the frame's multi-column frame object.
#[derive(Debug, Clone, PartialEq)]
pub struct TextFramePreferences {
    pub column_count: u32,
    pub column_gutter: f64,
    pub column_fixed_width: f64,
    /// First baseline offset code (chunk 0x2CE, u16 at 0): 0 LeadingOffset,
    /// 1 AscentOffset, 2 CapHeight, 3 EmboxHeight.
    pub first_baseline_offset: u16,
    pub vertical_justification: u16,
    pub vertical_balance_columns: bool,
    pub auto_sizing_type: u16,
    pub auto_sizing_reference_point: u16,
    /// Chunk 0x2D1: u8 at 12 and, in 40-byte chunks, f64 at 32.
    pub use_fixed_width: bool,
    pub max_width: Option<f64>,
    /// Chunk 0x2CE (48 bytes): u16 at 26 and f64 at 28 (minimum height),
    /// u16 at 36 and f64 at 38 (minimum width), u16 at 46 (no line
    /// breaks).
    pub minimum_sizes: Option<([bool; 2], [f64; 2], bool)>,
    /// Chunk 0x3730: u16 ignore text wrap.
    pub ignore_wrap: Option<bool>,
    /// Column rule (chunk 0x22646): f64 width at 28, u32 colour at 36.
    pub column_rule: Option<(f64, u32)>,
    /// Chunk 0x2265A: all zero in every sample but one.
    pub column_rule_override: Option<bool>,
    /// Footnote options (chunk 0x22608): u32, f64 minimum spacing, f64
    /// space between.
    pub footnotes: Option<[f64; 2]>,
    /// Inset spacing (the frame's chunk 0x3723): top, left, bottom, right.
    pub inset: [f64; 4],
}

/// Groups nested deeper than this are left out, with a warning, so that a
/// damaged file cannot exhaust the stack.
pub(super) const MAX_ITEM_DEPTH: usize = 100;

/// The IDML element of a frame or shape: by the shape code of chunk
/// 0x6204 (`code`), and where it has none (9, or no chunk), by the path.
/// See `docs/format/objects.md`, page items.
pub(super) fn classify(paths: &[Path], code: Option<u32>) -> Shape {
    match code {
        Some(1) => return Shape::GraphicLine,
        Some(2 | 3) => return Shape::Rectangle,
        Some(4 | 5) => return Shape::Oval,
        Some(0 | 6..=8) => return Shape::Polygon,
        _ => {}
    }
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
    if !path.open
        && path.points.len() == 4
        && path
            .points
            .iter()
            .all(|p| p.left != p.anchor && p.kind == 0)
    {
        return Shape::Oval;
    }
    Shape::Polygon
}

impl<'a> Reader<'a> {
    /// Text orientation of a multi-column frame (chunk 0x2DE): the
    /// identity gives horizontal text, the rotation 0 1 −1 0 vertical
    /// text (`docs/format/objects.md`). `None` without the chunk;
    /// `Some(None)` for another matrix.
    pub(super) fn frame_orientation(&self, mcf: u32) -> Result<Option<Option<Orientation>>, Error> {
        let Some(d) = self.chunk(mcf, chunk::FRAME_TEXT_TRANSFORM)? else {
            return Ok(None);
        };
        if d.len() < 48 {
            return Ok(Some(None));
        }
        let Matrix([a, b, c, dd, _, _]) = Matrix::read(&mut self.cursor(&d))?;
        Ok(Some(match [a, b, c, dd] {
            [1.0, 0.0, 0.0, 1.0] => Some(Orientation::Horizontal),
            [0.0, 1.0, -1.0, 0.0] => Some(Orientation::Vertical),
            _ => None,
        }))
    }

    /// Text frame settings of frame `frame`, from its multi-column frame
    /// `mcf` and the frame itself.
    pub(super) fn text_frame_preferences(
        &self,
        frame: u32,
        mcf: u32,
    ) -> Result<Option<TextFramePreferences>, Error> {
        let (Some(cols), Some(just)) = (
            self.chunk(mcf, chunk::FRAME_COLUMNS)?,
            self.chunk(mcf, chunk::FRAME_JUSTIFICATION)?,
        ) else {
            return Ok(None);
        };
        if cols.len() < 22 || just.len() < 28 {
            return Ok(None);
        }
        let minimum_sizes = (just.len() >= 48).then(|| {
            let flag = |at| self.enc().u16_at(&just, at).is_some_and(|v| v != 0);
            (
                [flag(26), flag(36)],
                [
                    self.enc().f64_at(&just, 28).unwrap_or(0.0),
                    self.enc().f64_at(&just, 38).unwrap_or(0.0),
                ],
                flag(46),
            )
        });
        let column_rule = match self.chunk(mcf, chunk::FRAME_COLUMN_RULE)? {
            Some(d) => match (self.enc().f64_at(&d, 28), self.enc().u32_at(&d, 36)) {
                (Some(w), Some(c)) => Some((w, c)),
                _ => None,
            },
            None => None,
        };
        let footnotes = match self.chunk(mcf, chunk::FRAME_FOOTNOTES)? {
            Some(d) => match (self.enc().f64_at(&d, 4), self.enc().f64_at(&d, 12)) {
                (Some(a), Some(b)) => Some([a, b]),
                _ => None,
            },
            None => None,
        };
        // The frame's chunk 0x3723: f64, u32, then four f64: left, top,
        // right, bottom.
        let inset = match self.chunk(frame, chunk::FRAME_INSET)? {
            Some(d) if d.len() >= 44 => {
                let f = |at| self.enc().f64_at(&d, at).unwrap_or(0.0);
                [f(20), f(12), f(36), f(28)]
            }
            _ => [0.0; 4],
        };
        Ok(Some(TextFramePreferences {
            column_count: self.cursor(&cols).u32()?,
            column_gutter: self.cursor(&cols[4..]).f64()?,
            column_fixed_width: self.cursor(&cols[14..]).f64()?,
            first_baseline_offset: self.cursor(&just).u16()?,
            vertical_justification: self.cursor(&just[2..]).u16()?,
            vertical_balance_columns: self.cursor(&just[20..]).u16()? != 0,
            auto_sizing_type: self.cursor(&just[22..]).u16()?,
            auto_sizing_reference_point: self.cursor(&just[24..]).u16()?,
            use_fixed_width: cols.get(12).is_some_and(|&b| b != 0),
            max_width: if cols.len() >= 40 {
                self.enc().f64_at(&cols, 32)
            } else {
                None
            },
            minimum_sizes,
            ignore_wrap: self
                .chunk(mcf, chunk::FRAME_IGNORE_WRAP)?
                .and_then(|d| self.enc().u16_at(&d, 0))
                .map(|v| v != 0),
            column_rule,
            column_rule_override: self
                .chunk(mcf, chunk::FRAME_COLUMN_RULE_OVERRIDE)?
                .map(|d| d.iter().any(|&b| b != 0)),
            footnotes,
            inset,
        }))
    }

    /// The settings every page item has (`ItemProps`).
    pub(super) fn item_props(&self, uid: u32) -> Result<ItemProps, Error> {
        let name_chunk = if self.class(uid) == Some(class::GROUP) {
            chunk::GROUP_NAME
        } else {
            chunk::ITEM_NAME
        };
        let name = match self.chunk(uid, name_chunk)? {
            Some(d) => {
                let mut c = self.cursor(&d);
                let builtin = c.flag()? == 1;
                Some(ItemName {
                    builtin,
                    name: c.string()?,
                })
            }
            None => None,
        };
        let counts = |id: u32| -> Result<Vec<u32>, Error> {
            let Some(d) = self.chunk(uid, id)? else {
                return Ok(Vec::new());
            };
            let mut c = self.cursor(&d);
            let n = c.u32()? as usize;
            if n > d.len() / 8 {
                return Err(Error::Corrupt(format!("item {uid}: {n} change counts")));
            }
            (0..2 * n).map(|_| c.u32()).collect()
        };
        let overridden = match self.chunk(uid, chunk::ITEM_OVERRIDE)? {
            Some(d) => {
                let mut c = self.cursor(&d);
                let master = c.u32()?;
                Some((master, c.u32_list()?))
            }
            None => None,
        };
        Ok(ItemProps {
            name,
            hidden: self
                .chunk(uid, chunk::ITEM_VISIBLE)?
                .is_some_and(|d| d.len() >= 2 && self.cursor(&d).u16().ok() == Some(0)),
            locked: self
                .chunk(uid, chunk::ITEM_LOCKED)?
                .is_some_and(|d| d.len() >= 4 && self.cursor(&d).u32().ok() == Some(1)),
            layout_constraints: self
                .chunk(uid, chunk::ITEM_LAYOUT_CONSTRAINTS)?
                .and_then(|d| d.first().copied()),
            change_counts: [
                counts(chunk::ITEM_PARENT_CHANGES)?,
                counts(chunk::ITEM_TARGET_CHANGES)?,
                counts(chunk::ITEM_UPDATED_CHANGES)?,
            ],
            overridden,
            graphic_frame: self
                .chunk(uid, chunk::ITEM_CONTENT)?
                .is_some_and(|d| d.len() >= 2 && self.cursor(&d).u16().ok() == Some(1)),
        })
    }

    /// Text wrap of a page item or graphic, from chunk 0x3703: u32 mode,
    /// u32 contour path object, four f64 offsets, u32 flags.
    pub(super) fn text_wrap(&self, uid: u32) -> Result<Option<TextWrap>, Error> {
        self.wrap_chunk(uid, chunk::TEXT_WRAP)
    }

    /// A text wrap record in chunk `id` of object `uid`.
    pub(super) fn wrap_chunk(&self, uid: u32, id: u32) -> Result<Option<TextWrap>, Error> {
        let Some(d) = self.chunk(uid, id)? else {
            return Ok(None);
        };
        if d.len() < 44 {
            return Ok(None);
        }
        let mut c = self.cursor(&d);
        let mode = c.u32()?;
        c.skip(4)?;
        let offsets = [c.f64()?, c.f64()?, c.f64()?, c.f64()?];
        let flags = c.u32()?;
        if !matches!(
            mode,
            wrap_mode::NONE | wrap_mode::JUMP_OBJECT | wrap_mode::BOUNDING_BOX | wrap_mode::CONTOUR
        ) {
            self.warn(format!(
                "item {uid}: text wrap mode {mode} is not known; left out"
            ));
        }
        Ok(Some(TextWrap {
            mode,
            offsets,
            flags,
        }))
    }

    pub(super) fn paths(&self, uid: u32) -> Result<Vec<Path>, Error> {
        let Some(data) = self.chunk(uid, chunk::ITEM_PATHS)? else {
            return Ok(Vec::new());
        };
        let mut c = self.cursor(&data);
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
                            kind,
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
                            kind,
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

    /// A page item and the items it contains. An item that contains
    /// itself, or one nested more than [`MAX_ITEM_DEPTH`] deep, is left
    /// out with a warning.
    pub(super) fn page_item(
        &self,
        uid: u32,
        layer: Option<u32>,
    ) -> Result<Option<PageItem>, Error> {
        let cls = self.class(uid);
        if cls != Some(class::SPLINE_ITEM) && cls != Some(class::GROUP) {
            return Ok(None);
        }
        {
            let path = self.item_path.borrow();
            if path.contains(&uid) {
                drop(path);
                self.warn(format!("item {uid} contains itself; left out"));
                return Ok(None);
            }
            if path.len() >= MAX_ITEM_DEPTH {
                drop(path);
                self.warn(format!(
                    "item {uid} is nested more than {MAX_ITEM_DEPTH} deep; left out"
                ));
                return Ok(None);
            }
        }
        self.item_path.borrow_mut().push(uid);
        let item = self.page_item_contents(uid, cls, layer);
        self.item_path.borrow_mut().pop();
        item
    }

    pub(super) fn page_item_contents(
        &self,
        uid: u32,
        cls: Option<u32>,
        layer: Option<u32>,
    ) -> Result<Option<PageItem>, Error> {
        // Groups without chunk 0x151 have their transform in chunk 0x40D.
        let transform = match self.chunk(uid, chunk::ITEM_TRANSFORM)? {
            Some(d) => Matrix::read(&mut self.cursor(&d))?,
            None => match self.chunk(uid, chunk::GROUP_TRANSFORM)? {
                Some(d) if cls == Some(class::GROUP) && d.len() >= 48 => {
                    Matrix::read(&mut self.cursor(&d))?
                }
                _ => Matrix::IDENTITY,
            },
        };
        let child_uids = self.children(uid, chunk::ITEM_HIERARCHY)?;
        let mut children = Vec::new();
        let mut graphics = Vec::new();
        let mut text_column = None;
        let mut frame_prefs = None;
        let mut frame_mcf = None;
        for &child in &child_uids {
            if let Some(g) = self.graphic(child)? {
                graphics.push(g);
            } else if self.class(child) == Some(class::MULTI_COLUMN_FRAME) {
                frame_prefs = self.text_frame_preferences(uid, child)?;
                frame_mcf = Some(child);
                text_column = self
                    .children(child, chunk::ITEM_HIERARCHY)?
                    .into_iter()
                    .find(|&c| self.class(c) == Some(class::FRAME_COLUMN));
            } else if let Some(item) = self.page_item(child, layer)? {
                children.push(item);
            }
        }
        let paths = self.paths(uid)?;
        let attrs = match self.chunk(uid, chunk::ITEM_ATTRS)? {
            Some(d) => {
                Attrs::parse(self.enc(), &d, List::Item, self.db.recorder()).unwrap_or_default()
            }
            None => Attrs::default(),
        };
        let object_style = match self.chunk(uid, chunk::ITEM_OBJECT_STYLE)? {
            Some(d) => uid_or_none(self.cursor(&d).u32()?),
            None => None,
        };
        let kind = if cls == Some(class::GROUP) {
            ItemKind::Group
        } else if let Some(column) = text_column {
            let mut kind = self.text_frame_links(column)?;
            if let ItemKind::TextFrame {
                preferences, story, ..
            } = &mut kind
            {
                *preferences = frame_prefs;
                if let (Some(story), Some(mcf)) = (*story, frame_mcf)
                    && let Some(o) = self.frame_orientation(mcf)?
                {
                    self.frame_orientations
                        .borrow_mut()
                        .entry(story)
                        .or_default()
                        .push(o);
                }
            }
            kind
        } else {
            let code = self
                .chunk(uid, chunk::ITEM_SHAPE)?
                .and_then(|d| self.enc().u32_at(&d, 2));
            ItemKind::Shape(classify(&paths, code))
        };
        Ok(Some(PageItem {
            uid,
            graphics,
            kind,
            transform,
            paths,
            layer,
            attrs,
            object_style,
            children,
            text_wrap: self.text_wrap(uid)?,
            anchor: None,
            props: self.item_props(uid).unwrap_or_else(|e| {
                self.warn(format!("item {uid}: settings left out: {e}"));
                ItemProps::default()
            }),
        }))
    }

    pub(super) fn graphic(&self, uid: u32) -> Result<Option<Graphic>, Error> {
        let kind = match self.class(uid) {
            Some(class::IMAGE) => GraphicKind::Image,
            Some(class::PDF) => GraphicKind::Pdf,
            Some(class::EPS) => GraphicKind::Eps,
            Some(class::SVG) => GraphicKind::Svg,
            _ => return Ok(None),
        };
        let transform = match self.chunk(uid, chunk::ITEM_TRANSFORM)? {
            Some(d) => Matrix::read(&mut self.cursor(&d))?,
            None => Matrix::IDENTITY,
        };
        let bounds = match self.chunk(uid, chunk::GRAPHIC_BOUNDS)? {
            Some(d) => {
                let mut c = self.cursor(&d);
                [c.f64()?, c.f64()?, c.f64()?, c.f64()?]
            }
            None => [0.0; 4],
        };
        let link = match self.chunk(uid, chunk::GRAPHIC_LINK)? {
            Some(d) if d.len() >= 12 => self.link(self.cursor(&d[8..]).u32()?)?,
            _ => None,
        };
        let (link, contents) = match link {
            Some((link, data)) => (Some(link), data),
            None => {
                let id = match kind {
                    GraphicKind::Image => Some(chunk::IMAGE_DATA),
                    GraphicKind::Pdf => Some(chunk::PDF_DATA),
                    _ => None,
                };
                let data = match id {
                    Some(id) => match self.chunk(uid, id)? {
                        Some(d) if d.len() >= 4 => self.raw_data(self.cursor(&d).u32()?)?,
                        _ => None,
                    },
                    None => None,
                };
                (None, data)
            }
        };
        Ok(Some(Graphic {
            uid,
            kind,
            transform,
            bounds,
            link,
            contents,
            text_wrap: self.text_wrap(uid)?,
            contour_type: match self.chunk(uid, chunk::CONTOUR_OPTION)? {
                Some(d) if d.len() >= 4 => Some(self.cursor(&d).u32()?),
                _ => None,
            },
            clipping: match self.chunk(uid, chunk::CLIPPING_PATH)? {
                Some(d) if d.len() >= 26 => {
                    let f = |o: usize| self.cursor(&d[o..]).f64();
                    Some(ClippingPath {
                        kind: self.cursor(&d).u32()?,
                        tolerance: f(4)?,
                        inset: f(12)?,
                        threshold: d[20],
                        index: self.enc().i16_from([d[23], d[24]]),
                        high_resolution: d[25],
                    })
                }
                _ => None,
            },
            photoshop_clipping: match self.chunk(uid, chunk::PHOTOSHOP_CLIPPING)? {
                Some(d) if d.len() >= 2 => Some(self.cursor(&d).u16()?),
                _ => None,
            },
        }))
    }

    /// The bytes of raw data object `uid` (`None` if `uid` is not one).
    pub(super) fn raw_data(&self, uid: u32) -> Result<EmbeddedFile, Error> {
        if uid == 0 || self.class(uid) != Some(class::RAW_DATA) {
            return Ok(None);
        }
        if let Some(r) = self.db.recorder() {
            r.object_read(uid);
        }
        self.db.object(uid)
    }

    /// A link, the URI of its resource and, for an embedded link, the
    /// embedded file.
    pub(super) fn link(&self, uid: u32) -> Result<Option<(Link, EmbeddedFile)>, Error> {
        if uid == 0 || self.db.object(uid)?.is_none() {
            return Ok(None);
        }
        let Some(info) = self.chunk(uid, chunk::LINK_INFO)? else {
            return Ok(None);
        };
        if info.len() < 12 {
            return Ok(None);
        }
        let resource = self.cursor(&info[8..]).u32()?;
        let (uri, data) = match self.chunk(resource, chunk::LINK_RESOURCE_URI)? {
            Some(d) if d.len() >= 5 => {
                let mut c = self.cursor(&d);
                c.u8()?;
                let n = c.u32()? as usize;
                let uri = String::from_utf8_lossy(c.bytes(n.min(c.remaining()))?).into_owned();
                // After the URI: 12 bytes, then the UID of the embedded
                // file's raw data object (0 for a normal link).
                let data = if c.remaining() >= 16 {
                    c.skip(12)?;
                    self.raw_data(c.u32()?)?
                } else {
                    None
                };
                (uri, data)
            }
            _ => (String::new(), None),
        };
        let link = Link {
            uid,
            uri,
            embedded: data.is_some(),
        };
        Ok(Some((link, data)))
    }

    /// Story and threading of the text frame owning `column`.
    pub(super) fn text_frame_links(&self, column: u32) -> Result<ItemKind, Error> {
        let none = ItemKind::TextFrame {
            story: None,
            previous: None,
            next: None,
            preferences: None,
        };
        let Some(d) = self.chunk(column, chunk::COLUMN_FRAME_LIST)? else {
            return Ok(none);
        };
        let list = self.cursor(&d).u32()?;
        let Some(d) = self.chunk(list, chunk::FRAME_LIST_FRAMES)? else {
            return Ok(none);
        };
        let mut c = self.cursor(&d);
        let story = uid_or_none(c.u32()?);
        let columns = c.u32_list()?;
        // Columns belong to frames: column -> multi-column frame -> spline item.
        let frame_of = |col: u32| -> Result<Option<u32>, Error> {
            let Some(h) = self.chunk(col, chunk::ITEM_HIERARCHY)? else {
                return Ok(None);
            };
            let mcf = self.cursor(&h[4..]).u32()?;
            let Some(h) = self.chunk(mcf, chunk::ITEM_HIERARCHY)? else {
                return Ok(None);
            };
            Ok(uid_or_none(self.cursor(&h[4..]).u32()?))
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
            preferences: None,
        })
    }
}
