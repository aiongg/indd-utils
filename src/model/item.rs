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

    /// This transform followed by `outer` (points are row vectors, as
    /// in IDML).
    pub fn then(&self, outer: &Matrix) -> Matrix {
        let [a, b, c, d, e, f] = self.0;
        let [p, q, r, s, t, u] = outer.0;
        Matrix([
            a * p + b * r,
            a * q + b * s,
            c * p + d * r,
            c * q + d * s,
            e * p + f * r + t,
            e * q + f * s + u,
        ])
    }

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
    /// Text kept from a placed EPS or PDF graphic (class 0x660B).
    EpsText(EpsText),
}

/// The settings of EPS text (`docs/format/objects.md`, EPS text).
#[derive(Debug, Clone, PartialEq)]
pub struct EpsText {
    /// Chunk 0x6611 as IDML writes it (`EPSTextData`): big-endian, with
    /// two fields left out.
    pub data: Vec<u8>,
    /// Chunk 0x154: left, top, right, bottom (`PathBoundingBox`).
    pub path_bounds: [f64; 4],
    /// The first four f64 of chunk 0x6612, in the same order
    /// (`EPSTextAttributeBounds`).
    pub attr_bounds: [f64; 4],
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
    /// Export options (chunk 0x1E206); `None` without the chunk.
    pub export: Option<ExportOptions>,
    /// Text on the item's path (chunk 0xB30A).
    pub text_paths: Vec<TextPath>,
}

/// Text on the path of a shape or frame (class 0xB320). See
/// `docs/format/objects.md`, text on a path.
#[derive(Debug, Clone, PartialEq)]
pub struct TextPath {
    pub uid: u32,
    pub story: Option<u32>,
    pub previous: Option<u32>,
    pub next: Option<u32>,
    /// `FlipPathEffect`.
    pub flipped: bool,
    pub start: f64,
    pub end: f64,
    /// The three codes at 8 of chunk 0xB30A are those of every sample
    /// (`CenterPathAlignment`, `BaselineTextAlignment`,
    /// `RainbowPathEffect`, `PathSpacing` 0).
    pub default_codes: bool,
}

/// Alternative text and tagging settings of a page item (chunk 0x1E206).
/// See `docs/format/objects.md`, object export options.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ExportOptions {
    /// Source codes: 0 custom, 5 XML structure, 6 XMP (actual text only),
    /// 8 decorative image (alternative text only).
    pub alt_source: u32,
    pub alt_text: Name,
    /// The namespace prefix and property path of the metadata property.
    pub alt_metadata: [Name; 2],
    pub actual_source: u32,
    pub actual_text: Name,
    pub actual_metadata: [Name; 2],
    /// 0 tag from structure, 1 artifact.
    pub tag_type: u32,
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
    /// `AllowOverrides`, for items of master spreads only: `false` when
    /// chunk 0x142D is 1 (`docs/format/objects.md`).
    pub allow_overrides: Option<bool>,
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
    /// A placed InDesign page (class 0x6607).
    ImportedPage,
}

/// A link (class 0x8C42) and its link resource (class 0x8C41). See
/// `docs/format/objects.md`, links.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Link {
    pub uid: u32,
    /// `LinkResourceURI`, for example `file:/Users/me/image.jpg`.
    pub uri: String,
    /// The linked file is stored in the document (`StoredState="Embedded"`).
    pub embedded: bool,
    /// Chunk 0x8C9B, u32 at 12: 0 when the linked file was modified.
    pub modified: Option<bool>,
    /// Chunk 0x8C9B, u16 at 20: 1 when the link is shown.
    pub shown: Option<bool>,
    /// The import stamp (`file <time> <size>`); `None` when empty.
    pub stamp: Option<String>,
    /// The file's modification time and the time it was placed or
    /// updated, as FILETIME (100 ns since 1601-01-01 UTC).
    pub times: Option<[u64; 2]>,
    /// Chunk 0x1B6 of the link.
    pub pdf_identifier: Option<u32>,
    /// The file size of the resource: high and low u32.
    pub size: Option<[u32; 2]>,
    /// The format name of the resource (`JPEG`, `Photoshop`, ...).
    pub format: Option<String>,
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
    /// Name, visibility, change counts and the other page item settings.
    pub props: ItemProps,
    /// Chunk 0x1B916.
    pub object_style: Option<u32>,
    /// The attribute list (chunk 0x6E03).
    pub attrs: Attrs,
    /// Image properties (chunk 0x1708 of an image).
    pub image: Option<ImageProperties>,
    /// Colour profile code (chunk 0x7C0F, u32 at 0).
    pub profile: Option<u32>,
    /// Grey, RGB, grey again and CMYK vector policy codes of a PDF or EPS
    /// (chunk 0x7C42, four u32); only the RGB and CMYK codes are known.
    pub vector_policies: Option<[u32; 4]>,
    /// PDF placement (chunk 0x251B): page number, transparent background
    /// byte, crop code.
    pub pdf: Option<(u32, u8, u32)>,
    /// Layers of an image, PDF or imported page (chunk 0x177A).
    pub layers: Option<GraphicLayers>,
    /// Applied layer comp (chunk 0x9209, i32 at 4).
    pub layer_comp: Option<i32>,
    /// Image import options (chunk 0x1714): apply the Photoshop clipping
    /// path, alpha channel name.
    pub import: Option<(bool, Name)>,
    /// The index of a placed InDesign page (chunk 0x2505, u32 at 0).
    pub page_index: Option<u32>,
}

/// Image properties (chunk 0x1708): u32 count, then records of u32 key,
/// u32 length *n*, u8, *n* bytes. See `docs/format/objects.md`, image
/// properties.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ImageProperties {
    /// Key 0x67: 1 grey, 2 RGB, 4 CMYK.
    pub color_space: Option<u32>,
    /// Key 0x6F (a colour table) is present.
    pub indexed: bool,
    /// Keys 0x6C and 0x6D, 16.16 fixed point.
    pub resolution: Option<[f64; 2]>,
}

/// Layers of a placed graphic (chunk 0x177A).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct GraphicLayers {
    /// The u16 at 0 is 1.
    pub option: bool,
    /// In IDML order.
    pub layers: Vec<GraphicLayer>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GraphicLayer {
    pub name: Name,
    pub id: u32,
    pub original_visibility: bool,
    pub current_visibility: bool,
    /// The parent layer's ID, −1 for a top-level layer.
    pub parent: i32,
    /// 0x01 separator, 0x04 effects layer, 0x08 locked.
    pub flags: u32,
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
    /// A frame grid (chunk 0xCD41 starts with u16 1).
    pub frame_grid: bool,
}

/// Groups nested deeper than this are left out, with a warning, so that a
/// damaged file cannot exhaust the stack.
pub(super) const MAX_ITEM_DEPTH: usize = 100;

/// The IDML element of a frame or shape. `shape` is the u16 flag and the
/// u32 shape code of chunk 0x6204: with flag 1 the code decides; with
/// flag 0 or without the chunk, the path. See `docs/format/objects.md`,
/// page items.
pub(super) fn classify(paths: &[Path], shape: Option<(u16, u32)>) -> Shape {
    if let Some((1, code)) = shape {
        return match code {
            1 => Shape::GraphicLine,
            2 | 3 => Shape::Rectangle,
            4 | 5 => Shape::Oval,
            _ => Shape::Polygon,
        };
    }
    let [path] = paths else {
        return Shape::Polygon;
    };
    let corners = path
        .points
        .iter()
        .all(|p| p.left == p.anchor && p.right == p.anchor);
    if path.open && path.points.len() == 2 && corners {
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
        && oval_anchors(path)
    {
        return Shape::Oval;
    }
    Shape::Polygon
}

/// Whether the second and fourth anchors of a four-point path lie on the
/// midpoint of the first and third, along the axis on which those two
/// differ (within 1e-8).
fn oval_anchors(path: &Path) -> bool {
    let a: Vec<(f64, f64)> = path.points.iter().map(|p| p.anchor).collect();
    let [p0, p1, p2, p3] = a[..] else {
        return false;
    };
    let on = |m: f64, u: f64, v: f64| (u - m).abs() <= 1e-8 && (v - m).abs() <= 1e-8;
    if (p0.0 - p2.0).abs() < (p0.1 - p2.1).abs() {
        on((p0.1 + p2.1) / 2.0, p1.1, p3.1)
    } else {
        on((p0.0 + p2.0) / 2.0, p1.0, p3.0)
    }
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
            frame_grid: self
                .chunk(mcf, chunk::FRAME_GRID)?
                .and_then(|d| self.enc().u16_at(&d, 0))
                == Some(1),
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
        let on_master = self
            .chunk(uid, chunk::ITEM_HIERARCHY)?
            .and_then(|d| self.enc().u32_at(&d, 0))
            .is_some_and(|s| self.class(s) == Some(class::MASTER_SPREAD));
        let allow_overrides = if on_master {
            Some(
                self.chunk(uid, chunk::ITEM_ALLOW_OVERRIDES)?
                    .and_then(|d| self.enc().u32_at(&d, 0))
                    != Some(1),
            )
        } else {
            None
        };
        Ok(ItemProps {
            name,
            allow_overrides,
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

    /// The export options of a page item (chunk 0x1E206): the alternative
    /// text and actual text, each a u32 source and three flagged strings
    /// (text, metadata namespace prefix, property path), then the u32
    /// tagging type. The rest of the chunk is not decoded.
    pub(super) fn export_options(&self, uid: u32) -> Result<Option<ExportOptions>, Error> {
        let Some(d) = self.chunk(uid, chunk::ITEM_EXPORT)? else {
            return Ok(None);
        };
        let mut c = self.cursor(&d);
        let alt_source = c.u32()?;
        let alt_text = c.name()?;
        let alt_metadata = [c.name()?, c.name()?];
        let actual_source = c.u32()?;
        let actual_text = c.name()?;
        let actual_metadata = [c.name()?, c.name()?];
        Ok(Some(ExportOptions {
            alt_source,
            alt_text,
            alt_metadata,
            actual_source,
            actual_text,
            actual_metadata,
            tag_type: c.u32()?,
        }))
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

    /// A page item and the items it contains. An item that cannot be
    /// read, contains itself or is nested more than [`MAX_ITEM_DEPTH`]
    /// deep is left out with a warning.
    pub(super) fn page_item(
        &self,
        uid: u32,
        layer: Option<u32>,
    ) -> Result<Option<PageItem>, Error> {
        let cls = self.class(uid);
        if cls != Some(class::SPLINE_ITEM)
            && cls != Some(class::GROUP)
            && cls != Some(class::EPS_TEXT)
        {
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
        // An item that cannot be read is left out, like other objects.
        Ok(item.unwrap_or_else(|e| {
            self.warn(format!("item {uid} left out: {e}"));
            None
        }))
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
            Some(d) => self
                .attrs_or_warn(
                    || format!("item {uid}"),
                    Attrs::parse(self.enc(), &d, List::Item, self.db.recorder()),
                )
                .unwrap_or_default(),
            None => Attrs::default(),
        };
        let object_style = match self.chunk(uid, chunk::ITEM_OBJECT_STYLE)? {
            Some(d) => uid_or_none(self.cursor(&d).u32()?),
            None => None,
        };
        let kind = if cls == Some(class::GROUP) {
            ItemKind::Group
        } else if cls == Some(class::EPS_TEXT) {
            ItemKind::EpsText(self.eps_text(uid)?)
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
            let shape = self
                .chunk(uid, chunk::ITEM_SHAPE)?
                .and_then(|d| Some((self.enc().u16_at(&d, 0)?, self.enc().u32_at(&d, 2)?)));
            ItemKind::Shape(classify(&paths, shape))
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
            export: self.export_options(uid).unwrap_or_else(|e| {
                self.warn(format!("item {uid}: export options left out: {e}"));
                None
            }),
            text_paths: self
                .text_path(uid)
                .unwrap_or_else(|e| {
                    self.warn(format!("item {uid}: text on its path left out: {e}"));
                    None
                })
                .into_iter()
                .collect(),
        }))
    }

    /// The text on the path of an item (chunk 0xB30A): u32 text path UID,
    /// u32, three codes (8 bytes), u8 flip at 16, f64 start at 18 and end
    /// at 26. The story and thread come from the column (class 0xB318) of
    /// the text path's multi-column frame (chunk 0xB334). See
    /// `docs/format/objects.md`, text on a path.
    fn text_path(&self, uid: u32) -> Result<Option<TextPath>, Error> {
        let Some(d) = self.chunk(uid, chunk::ITEM_TEXT_PATH)? else {
            return Ok(None);
        };
        let mut c = self.cursor(&d);
        let path = c.u32()?;
        if path == 0 || self.class(path) != Some(class::TEXT_PATH) {
            return Ok(None);
        }
        c.u32()?;
        let codes = c.bytes(8)?;
        let default_codes = self.enc().u16_at(codes, 0) == Some(1)
            && self.enc().u16_at(codes, 2) == Some(4)
            && self.enc().u32_at(codes, 4) == Some(3);
        let flipped = c.u8()? == 1;
        c.u8()?;
        let start = c.f64()?;
        let end = c.f64()?;
        let column = self
            .children(path, chunk::TEXT_PATH_FRAME)?
            .into_iter()
            .filter(|&m| self.class(m) == Some(class::MULTI_COLUMN_FRAME))
            .map(|m| self.children(m, chunk::ITEM_HIERARCHY))
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .flatten()
            .find(|&col| self.class(col) == Some(class::TEXT_PATH_COLUMN));
        let (story, previous, next) = match column.map(|col| self.text_frame_links(col)) {
            Some(Ok(ItemKind::TextFrame {
                story,
                previous,
                next,
                ..
            })) => (story, previous, next),
            Some(Err(e)) => return Err(e),
            _ => (None, None, None),
        };
        Ok(Some(TextPath {
            uid: path,
            story,
            previous,
            next,
            flipped,
            start,
            end,
            default_codes,
        }))
    }

    /// The settings of EPS text (class 0x660B). See
    /// `docs/format/objects.md`, EPS text.
    fn eps_text(&self, uid: u32) -> Result<EpsText, Error> {
        let four = |id: u32| -> Result<[f64; 4], Error> {
            let d = self
                .chunk(uid, id)?
                .ok_or_else(|| Error::Corrupt(format!("EPS text {uid}: no chunk {id:#x}")))?;
            let mut c = self.cursor(&d);
            Ok([c.f64()?, c.f64()?, c.f64()?, c.f64()?])
        };
        let data = self
            .chunk(uid, chunk::EPS_TEXT_DATA)?
            .ok_or_else(|| Error::Corrupt(format!("EPS text {uid}: no text record")))?;
        Ok(EpsText {
            data: eps_text_data(self.enc(), &data)?,
            path_bounds: four(chunk::OLD_PAGE_BOUNDS)?,
            attr_bounds: four(chunk::EPS_TEXT_ATTR_BOUNDS)?,
        })
    }

    pub(super) fn graphic(&self, uid: u32) -> Result<Option<Graphic>, Error> {
        let kind = match self.class(uid) {
            Some(class::IMAGE) => GraphicKind::Image,
            Some(class::PDF) => GraphicKind::Pdf,
            Some(class::EPS) => GraphicKind::Eps,
            Some(class::SVG) => GraphicKind::Svg,
            Some(class::IMPORTED_PAGE) => GraphicKind::ImportedPage,
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
            props: self.item_props(uid).unwrap_or_else(|e| {
                self.warn(format!("graphic {uid}: settings left out: {e}"));
                ItemProps::default()
            }),
            object_style: match self.chunk(uid, chunk::ITEM_OBJECT_STYLE)? {
                Some(d) if d.len() >= 4 => uid_or_none(self.cursor(&d).u32()?),
                _ => None,
            },
            attrs: match self.chunk(uid, chunk::ITEM_ATTRS)? {
                Some(d) => self
                    .attrs_or_warn(
                        || format!("graphic {uid}"),
                        Attrs::parse(self.enc(), &d, List::Item, self.db.recorder()),
                    )
                    .unwrap_or_default(),
                None => Attrs::default(),
            },
            image: match self.chunk(uid, chunk::IMAGE_PROPERTIES)? {
                Some(d) if kind == GraphicKind::Image => self.image_properties(&d),
                _ => None,
            },
            profile: match self.chunk(uid, chunk::IMAGE_PROFILE)? {
                Some(d) if d.len() >= 4 => Some(self.cursor(&d).u32()?),
                _ => None,
            },
            vector_policies: match self.chunk(uid, chunk::VECTOR_POLICIES)? {
                Some(d) if d.len() >= 16 => {
                    let mut c = self.cursor(&d);
                    Some([c.u32()?, c.u32()?, c.u32()?, c.u32()?])
                }
                _ => None,
            },
            pdf: match self.chunk(uid, chunk::PDF_PLACEMENT)? {
                Some(d) if d.len() >= 12 => {
                    Some((self.cursor(&d).u32()?, d[6], self.cursor(&d[8..]).u32()?))
                }
                _ => None,
            },
            layers: match self.chunk(uid, chunk::GRAPHIC_LAYERS)? {
                Some(d) => self.graphic_layers(&d).unwrap_or_else(|e| {
                    self.warn(format!("graphic {uid}: layers left out: {e}"));
                    None
                }),
                None => None,
            },
            layer_comp: match self.chunk(uid, chunk::LAYER_COMP)? {
                Some(d) if d.len() >= 8 => Some(self.cursor(&d[4..]).i32()?),
                _ => None,
            },
            import: match self.chunk(uid, chunk::IMAGE_IMPORT)? {
                Some(d) if d.len() >= 9 => {
                    let mut c = self.cursor(&d[4..]);
                    let clipping = c.u16()?;
                    c.u16()?;
                    match (clipping, c.name()) {
                        (0 | 1, Ok(name)) => Some((clipping == 1, name)),
                        _ => None,
                    }
                }
                _ => None,
            },
            page_index: match self.chunk(uid, chunk::IMPORTED_PAGE_INFO)? {
                Some(d) if kind == GraphicKind::ImportedPage => self.enc().u32_at(&d, 0),
                _ => None,
            },
        }))
    }

    /// Image properties from chunk 0x1708; `None` if the record list is
    /// cut short.
    fn image_properties(&self, d: &[u8]) -> Option<ImageProperties> {
        let mut c = self.cursor(d);
        let n = c.u32().ok()?;
        let mut p = ImageProperties::default();
        let mut resolution = [None, None];
        for _ in 0..n {
            let key = c.u32().ok()?;
            let len = c.u32().ok()? as usize;
            c.u8().ok()?;
            let data = c.bytes(len).ok()?;
            let value = || (len >= 4).then(|| self.cursor(data).u32().ok()).flatten();
            match key {
                0x67 => p.color_space = value(),
                0x6C => resolution[0] = value().map(|v| f64::from(v) / 65536.0),
                0x6D => resolution[1] = value().map(|v| f64::from(v) / 65536.0),
                0x6F => p.indexed = true,
                _ => {}
            }
        }
        if let [Some(h), Some(v)] = resolution {
            p.resolution = Some([h, v]);
        }
        Some(p)
    }

    /// Layers of a placed graphic (chunk 0x177A): u16 option flag, 8
    /// bytes, u32 count, then per layer a flagged name, u32 ID, u32
    /// original and current visibility, i32 parent ID and u32 flags.
    fn graphic_layers(&self, d: &[u8]) -> Result<Option<GraphicLayers>, Error> {
        let mut c = self.cursor(d);
        let option = c.u16()? == 1;
        c.skip(8)?;
        let n = c.u32()? as usize;
        if n > d.len() / 20 {
            return Err(Error::Corrupt(format!("{n} graphic layers")));
        }
        let mut layers = Vec::with_capacity(n);
        for _ in 0..n {
            let name = c.name()?;
            layers.push(GraphicLayer {
                name,
                id: c.u32()?,
                original_visibility: c.u32()? == 1,
                current_visibility: c.u32()? == 1,
                parent: c.i32()?,
                flags: c.u32()?,
            });
        }
        Ok(Some(GraphicLayers { option, layers }))
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
    /// embedded file. Fields after the URI that are cut short are left
    /// out (`None`).
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
        let mut link = Link {
            uid,
            ..Link::default()
        };
        let mut data = None;
        if let Some(d) = self.chunk(resource, chunk::LINK_RESOURCE_URI)?
            && d.len() >= 5
        {
            let mut c = self.cursor(&d);
            c.u8()?;
            let n = c.u32()? as usize;
            link.uri = String::from_utf8_lossy(c.bytes(n.min(c.remaining()))?).into_owned();
            // After the URI: 12 bytes, then the UID of the embedded
            // file's raw data object (0 for a normal link).
            if c.remaining() >= 16 {
                c.skip(12)?;
                data = self.raw_data(c.u32()?)?;
                // Then the stamp, the modification time, the file size,
                // a byte and the format name.
                let rest = (|| -> Result<_, Error> {
                    let n = c.u32()? as usize;
                    c.segments(n)?;
                    c.skip(8)?;
                    let size = [c.u32()?, c.u32()?];
                    let format = c.name()?.name;
                    Ok((size, format))
                })();
                if let Ok((size, format)) = rest {
                    link.size = Some(size);
                    link.format = Some(format);
                }
            }
        }
        link.embedded = data.is_some();
        let mut c = self.cursor(&info);
        let head = (|| -> Result<_, Error> {
            c.skip(12)?;
            let modified = c.u32()?;
            c.skip(4)?;
            let shown = c.u16()?;
            c.skip(10)?;
            Ok((modified, shown))
        })();
        if let Ok((modified, shown)) = head {
            link.modified = match modified {
                0 => Some(true),
                1 => Some(false),
                _ => None,
            };
            link.shown = match shown {
                0 => Some(false),
                1 => Some(true),
                _ => None,
            };
            let stamp = (|| -> Result<_, Error> {
                let n = c.u32()? as usize;
                let stamp = c.segments(n)?;
                let mut time = || -> Result<u64, Error> {
                    let high = c.u32()?;
                    Ok(u64::from(high) << 32 | u64::from(c.u32()?))
                };
                let times = [time()?, time()?];
                Ok((stamp, times))
            })();
            if let Ok((stamp, times)) = stamp {
                link.stamp = (!stamp.is_empty()).then_some(stamp);
                link.times = Some(times);
            }
        }
        link.pdf_identifier = match self.chunk(uid, chunk::LINK_PDF_IDENTIFIER)? {
            Some(d) if d.len() >= 4 => Some(self.cursor(&d).u32()?),
            _ => None,
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

/// The text record of EPS text (chunk 0x6611) as IDML `EPSTextData`
/// writes it: in big-endian byte order, with the u32 at 72 and at 78 of
/// the 210-byte tail left out. Layout: u32 version, flagged in-object
/// string (font name), nine f64, flagged in-object string (text), tail.
/// See `docs/format/objects.md`, EPS text.
pub(super) fn eps_text_data(enc: Encoding, d: &[u8]) -> Result<Vec<u8>, Error> {
    let mut c = enc.cursor(d);
    let mut out = c.u32()?.to_be_bytes().to_vec();
    let string = |c: &mut crate::object::Cursor, out: &mut Vec<u8>| -> Result<(), Error> {
        let (flag, tag) = if enc.big_endian() {
            let tag = c.u8()?;
            (c.u8()?, tag)
        } else {
            let flag = c.u8()?;
            (flag, c.u8()?)
        };
        if tag != enc.string_tag() {
            return Err(Error::Corrupt(format!("EPS text: string tag {tag}")));
        }
        let script = c.u8()?;
        let n = c.u16()? as usize;
        out.extend([tag, flag, script]);
        out.extend((n as u16).to_be_bytes());
        let mut left = n;
        while left > 0 {
            let header = c.u16()?;
            let count = (header & 0x3FFF) as usize;
            if count == 0 || count > left {
                return Err(Error::Corrupt("EPS text: bad text segment".into()));
            }
            out.extend(header.to_be_bytes());
            match header & 0xC000 {
                0x4000 => out.extend(c.bytes(count)?),
                0x8000 => {
                    for _ in 0..count {
                        out.extend(c.u16()?.to_be_bytes());
                    }
                }
                _ => return Err(Error::Corrupt("EPS text: bad text segment".into())),
            }
            left -= count;
        }
        Ok(())
    };
    string(&mut c, &mut out)?;
    for _ in 0..9 {
        out.extend(c.f64()?.to_be_bytes());
    }
    string(&mut c, &mut out)?;
    if c.remaining() != 210 {
        return Err(Error::Corrupt(format!(
            "EPS text: tail of {} bytes, expected 210",
            c.remaining()
        )));
    }
    let tail = c.bytes(210)?;
    let mut t = enc.cursor(tail);
    out.extend(t.u32()?.to_be_bytes());
    out.extend(&tail[4..72]);
    t.skip(72)?;
    out.extend(t.u16()?.to_be_bytes());
    out.extend(&tail[82..]);
    Ok(out)
}

#[cfg(test)]
mod eps_text_tests {
    use super::*;
    use crate::database::synthetic::flagged_string;

    #[test]
    fn writes_the_eps_text_record_big_endian() {
        let enc = Encoding::default();
        let mut d = 0x0001_0002u32.to_le_bytes().to_vec();
        d.extend(flagged_string(enc, 1, "Helv"));
        for i in 0..9 {
            d.extend((i as f64).to_le_bytes());
        }
        d.extend(flagged_string(enc, 1, "Hi"));
        let mut tail = vec![0u8; 210];
        tail[0] = 16;
        tail[72] = 0xAA; // left out
        tail[76] = 1;
        tail[78] = 14; // left out
        tail[100] = 0x55;
        d.extend(&tail);
        let out = eps_text_data(enc, &d).unwrap();
        assert_eq!(out.len(), d.len() - 8);
        assert_eq!(&out[..4], &[0, 1, 0, 2]);
        assert_eq!(
            &out[4..15],
            &[2, 1, 0, 0, 4, 0x40, 4, b'H', b'e', b'l', b'v']
        );
        assert_eq!(&out[15..23], &0f64.to_be_bytes());
        assert_eq!(&out[23..31], &1f64.to_be_bytes());
        let t = &out[out.len() - 202..];
        assert_eq!(&t[..4], &[0, 0, 0, 16]);
        assert_eq!(&t[72..74], &[0, 1]);
        assert_eq!(t[74 + 100 - 82], 0x55);
        assert!(eps_text_data(enc, &d[..d.len() - 1]).is_err());
    }
}
