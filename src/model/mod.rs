//! A document model built from database objects. See
//! `docs/format/objects.md` for the class and chunk layouts used here.

pub mod attrs;
pub mod cjk;
pub mod color;
pub mod font;
pub mod hyperlink;
pub mod prefs;
pub mod table;
pub mod variable;
pub mod xml;
pub mod xref;

use std::collections::{BTreeMap, HashMap};

pub use attrs::{Attrs, Value};
pub use cjk::{CjkTable, CompositeFont, CompositeFontEntry};
pub use color::{Color, ColorGroup, Gradient, Ink, Tint};
pub use font::{Font, FontFamily};
pub use hyperlink::{Bookmark, Destination, DestinationKind, Hyperlink, SourceRange, TextSource};
pub use table::{Cell, CellFormat, Table, TableStyle};
pub use variable::TextVariable;
pub use xref::CrossReferenceFormat;

use crate::audit::List;
use crate::object::{Cursor, Object, f64_at, i16_from, u16_at, u16_from, u32_at, u32_from};
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
    pub const IMAGE: u32 = 0x1702;
    pub const PDF: u32 = 0x2501;
    pub const EPS: u32 = 0x6601;
    pub const SVG: u32 = 0x6639;
    /// A plain byte stream, such as the file of an embedded graphic.
    pub const RAW_DATA: u32 = 0x129;
    pub const FONT_FAMILY: u32 = 0x3E03;
    pub const LANGUAGE: u32 = 0x2D07;
    pub const TOC_STYLE: u32 = 0x11605;
    pub const NAMED_GRID: u32 = 0xCD12;
    pub const ASSIGNMENT: u32 = 0x1BE01;
    pub const SMOOTH_SHADE: u32 = 0x5533;
    /// Holds an item anchored in text.
    pub const ANCHOR: u32 = 0x262;
    pub const TEXT_VARIABLE_INSTANCE: u32 = 0xCA64;
    pub const TEXT_VARIABLE: u32 = 0xCAB4;
    pub const STYLE_ROOT_GROUP: u32 = 0xCA8C;
    pub const STYLE_GROUP: u32 = 0xCA8B;
    pub const OBJECT_STYLE: u32 = 0x1B901;
    pub const OBJECT_STYLE_ROOT_GROUP: u32 = 0x1B972;
    pub const CELL_STYLE_ROOT_GROUP: u32 = 0x20241;
    pub const TABLE_STYLE_ROOT_GROUP: u32 = 0x1044F;
    /// Document-wide preferences.
    pub const PREFERENCES: u32 = 0x2202;
    pub const GUIDE: u32 = 0x3301;
    pub const XML_TAG: u32 = 0xBF19;
    /// A colour used in the interface (XML tags).
    pub const UI_COLOR: u32 = 0x1F11;
    /// A colour profile: its name (chunk 0x13C).
    pub const COLOR_PROFILE: u32 = 0x7D03;
    /// Page item defaults.
    pub const ITEM_DEFAULTS: u32 = 0x6E07;
}

/// Chunk IDs.
pub mod chunk {
    pub const DOC_SPREADS: u32 = 0x501;
    pub const DOC_MASTER_SPREADS: u32 = 0x1401;
    pub const DOC_LAYERS: u32 = 0x301;
    pub const DOC_ACTIVE_LAYER: u32 = 0x313;
    pub const DOC_STORIES: u32 = 0x222;
    pub const DOC_SECTIONS: u32 = 0x4C01;
    /// Document users: u32 count, then per user a flag byte, the name
    /// and u32 colour.
    pub const DOC_USERS: u32 = 0xA443;
    pub const SPREAD_CHILDREN: u32 = 0x503;
    pub const SPREAD_TRANSFORM: u32 = 0x56E;
    pub const SPREAD_BINDING: u32 = 0x1B8;
    pub const SPREAD_LAYER_LAYER: u32 = 0x302;
    pub const SPREAD_LAYER_CHILDREN: u32 = 0x303;
    pub const LAYER_PROPS: u32 = 0x304;
    /// u16 0 if document pages may shuffle across the spread.
    pub const SPREAD_SHUFFLE: u32 = 0x1A8;
    /// Flattener settings of a spread.
    pub const SPREAD_FLATTENER: u32 = 0x10833;
    /// Tab orders: u32 count, then per page u32 page and a UID list.
    pub const SPREAD_TAB_ORDERS: u32 = 0x14580;
    /// u16 0 if a master spread hides the items of its own master.
    pub const MASTER_SHOW_ITEMS: u32 = 0x140D;
    /// Overridden master page items: u32 count, two UID lists.
    pub const PAGE_OVERRIDES: u32 = 0x1404;
    /// Layout grid use: six bytes, the last u16 1 to use the master's.
    pub const PAGE_GRID_USE: u32 = 0xCD04;
    /// Layout rule: u32, then u32 code.
    pub const PAGE_LAYOUT_RULE: u32 = 0x563;
    /// Page colour: u32 0 none, 1 the master's, else an interface colour.
    pub const PAGE_COLOR: u32 = 0x5FF;
    pub const PAGE_MASTER: u32 = 0x140F;
    pub const PAGE_TRANSFORM: u32 = 0x5CC;
    pub const PAGE_BOUNDS: u32 = 0x5DD;
    /// Page bounds in files from InDesign 3.0 and 4.0.
    pub const OLD_PAGE_BOUNDS: u32 = 0x154;
    pub const PAGE_MARGINS: u32 = 0x51A;
    pub const PAGE_COLUMNS: u32 = 0x528;
    pub const PAGE_GRID: u32 = 0xCD02;
    pub const ITEM_TRANSFORM: u32 = 0x151;
    pub const ITEM_PATHS: u32 = 0x162B;
    pub const ITEM_HIERARCHY: u32 = 0x15B;
    pub const COLUMN_FRAME_LIST: u32 = 0x220;
    pub const FRAME_LIST_FRAMES: u32 = 0x205;
    pub const STORY_STRANDS: u32 = 0x223;
    /// The table of contents (class 0x8C20) that made a story.
    pub const STORY_TOC: u32 = 0x8C40;
    /// The TOC style of a table of contents.
    pub const TOC_STYLE_OF: u32 = 0x11613;
    pub const TOC_STYLE: u32 = 0x11605;
    pub const NAMED_GRID: u32 = 0xCD28;
    pub const STRAND_DATA: u32 = 0x261;
    pub const STRAND_RUNS: u32 = 0x262;
    pub const STYLE_INFO: u32 = 0x230;
    pub const ITEM_ATTRS: u32 = 0x6E03;
    pub const STYLE_ATTRS: u32 = 0x23F;
    pub const LANGUAGE_NAME: u32 = 0x2D0F;
    pub const ANCHOR_CHILDREN: u32 = 0x2C8;
    /// Anchored object settings, of an anchor or an object style.
    pub const ANCHOR_SETTINGS: u32 = 0x2800;
    pub const MASTER_NAME: u32 = 0x1402;
    pub const STYLE_ROOT_CHILDREN: u32 = 0x28DC;
    pub const STYLE_GROUP_CHILDREN: u32 = 0x28D3;
    pub const STYLE_GROUP_NAME: u32 = 0x28D2;
    pub const OBJECT_STYLE_INFO: u32 = 0x1B907;
    pub const OBJECT_STYLE_ROOT_CHILDREN: u32 = 0x1B95A;
    pub const ITEM_OBJECT_STYLE: u32 = 0x1B916;
    /// Page item name: u8 1 if a built-in key, then an in-object string.
    pub const ITEM_NAME: u32 = 0x2C10;
    /// The same for a group.
    pub const GROUP_NAME: u32 = 0x418;
    /// u16, then u32 shape code: 1 line, 2 or 3 rectangle, 4 or 5
    /// oval, 0 or 6 to 8 polygon, 9 none.
    pub const ITEM_SHAPE: u32 = 0x6204;
    /// u32 1 if the page item is locked.
    pub const ITEM_LOCKED: u32 = 0x2C2D;
    /// u16 0 if the page item is hidden.
    pub const ITEM_VISIBLE: u32 = 0x2C32;
    /// Override of a master page item: u32 master item, then a list of
    /// attribute IDs.
    pub const ITEM_OVERRIDE: u32 = 0x1424;
    /// Interface change counts: u32 count *n*, then *n* pairs of u32.
    pub const ITEM_PARENT_CHANGES: u32 = 0x21D4E;
    pub const ITEM_TARGET_CHANGES: u32 = 0x21D50;
    pub const ITEM_UPDATED_CHANGES: u32 = 0x21D53;
    /// u8 layout constraint flags.
    pub const ITEM_LAYOUT_CONSTRAINTS: u32 = 0x22228;
    pub const ROOT_GROUP_KIND: u32 = 0x28C2;
    pub const SECTION_INFO: u32 = 0x4C02;
    pub const DOCUMENT_PREFERENCES: u32 = 0x533;
    /// Shading of a pasted smooth shade.
    pub const SMOOTH_SHADE: u32 = 0x5532;
    pub const SMOOTH_SHADE_NAME: u32 = 0x5531;
    /// Index sort groups, in the preferences object.
    pub const INDEX_GROUPS: u32 = 0x1307E;
    pub const XML_TAG_NAME: u32 = 0xBF2F;
    pub const XML_TAG_COLOR: u32 = 0x117;
    /// Bullet characters, in the preferences object.
    pub const BULLETS: u32 = 0x1A488;
    pub const FRAME_COLUMNS: u32 = 0x2D1;
    pub const FRAME_COLUMN_RULE: u32 = 0x22646;
    pub const FRAME_COLUMN_RULE_OVERRIDE: u32 = 0x2265A;
    pub const FRAME_FOOTNOTES: u32 = 0x22608;
    pub const FRAME_IGNORE_WRAP: u32 = 0x3730;
    /// Inset spacing of a text frame (on the frame, not its columns).
    pub const FRAME_INSET: u32 = 0x3723;
    pub const FRAME_JUSTIFICATION: u32 = 0x2CE;
    /// Matrix of a multi-column frame; its first four values give the
    /// text orientation.
    pub const FRAME_TEXT_TRANSFORM: u32 = 0x2DE;
    pub const GRAPHIC_BOUNDS: u32 = 0x1633;
    pub const GRAPHIC_LINK: u32 = 0x8CBC;
    pub const LINK_INFO: u32 = 0x8C9B;
    pub const LINK_RESOURCE_URI: u32 = 0x8C92;
    /// Pasted image without a link: u32 raw data object.
    pub const IMAGE_DATA: u32 = 0x8C23;
    /// Pasted PDF without a link: u32 raw data object.
    pub const PDF_DATA: u32 = 0x2521;
    pub const SWATCH_NAME: u32 = 0x1F30;
    pub const TEXT_WRAP: u32 = 0x3703;
    pub const CONTOUR_OPTION: u32 = 0x373D;
    pub const CLIPPING_PATH: u32 = 0x2C1A;
    pub const PHOTOSHOP_CLIPPING: u32 = 0x8C39;
    /// Frame fitting attributes of an object style (u16 count, records).
    pub const OBJECT_STYLE_FITTING: u32 = 0x1B956;
    /// Page item attributes of an object style (u16 count, records).
    pub const OBJECT_STYLE_ATTRS: u32 = 0x1B92B;
    pub const OBJECT_STYLE_FRAME: u32 = 0x1B924;
    pub const OBJECT_STYLE_STORY: u32 = 0x285B;
    pub const OBJECT_STYLE_DIRECTION: u32 = 0x50F28;
    pub const OBJECT_STYLE_WRAP: u32 = 0x3776;
    pub const OBJECT_STYLE_CONTOUR: u32 = 0x3777;
    pub const OBJECT_STYLE_ENABLED: u32 = 0x1B92E;
    pub const OBJECT_STYLE_PARAGRAPH_STYLE: u32 = 0x1B946;
    pub const GUIDE: u32 = 0x3308;
}

/// Kinds of strand run data (first u32 of chunk 0x262).
pub mod strand {
    pub const TEXT: u32 = 0x202;
    pub const CHARACTER_STYLE: u32 = 0x203;
    pub const PARAGRAPH_STYLE: u32 = 0x204;
    /// Objects owned by text positions (anchored items, tables, ...).
    pub const OWNED_ITEMS: u32 = 0x209;
    /// Which object (story, or table and cell) each stretch of text belongs to.
    pub const TEXT_OWNER: u32 = 0x2A4;
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
    /// The stored point type: 2 a corner without direction points, 0 or
    /// 1 a point with them.
    pub kind: u32,
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
    /// The other settings, when chunk 0x304 has the layout of the samples.
    pub settings: Option<LayerSettings>,
}

/// Layer settings in chunk 0x304. See `docs/format/objects.md`.
#[derive(Debug, Clone, PartialEq)]
pub struct LayerSettings {
    pub printable: bool,
    pub lock_guides: bool,
    pub ui: bool,
    pub ignore_wrap: bool,
    /// The layer colour (an interface colour object).
    pub color: Option<[f64; 3]>,
}

/// A colour setting that names an interface colour (class 0x1F11), or
/// one of two codes. See `docs/format/objects.md`.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum UiColorRef {
    /// Code 1, or no chunk: the colour of the master.
    #[default]
    UseMaster,
    /// Code 0.
    Nothing,
    Rgb([f64; 3]),
    /// Any other value.
    Unknown,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Page {
    pub uid: u32,
    /// Left, top, right, bottom in page coordinates.
    pub bounds: [f64; 4],
    pub transform: Matrix,
    pub master: Option<u32>,
    pub master_transform: Matrix,
    /// Margins and columns (chunks 0x51A and 0x528). After
    /// `resolve_page_layout`, these are the values in effect: the page's
    /// own, or its master page's.
    pub margins: Option<Margins>,
    pub columns: Option<Columns>,
    /// Layout grid settings (chunk 0xCD02).
    pub grid: Option<GridData>,
    pub settings: PageSettings,
}

/// Settings of a page. See `docs/format/objects.md`, page settings.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PageSettings {
    /// Master page items overridden on the page, each with its override
    /// (0 for none), from chunk 0x1404.
    pub overrides: Vec<(u32, u32)>,
    /// Chunk 0xCD04: whether the page uses its master's layout grid.
    pub use_master_grid: Option<bool>,
    /// Layout rule code (chunk 0x563); `None` without the chunk.
    pub layout_rule: Option<u32>,
    pub color: UiColorRef,
    /// Page items in tab order (the spread's chunk 0x14580).
    pub tab_order: Vec<u32>,
}

/// Page margins from chunk 0x51A: four f64 (left, top, right, bottom),
/// then u16 1 if the page has its own margins, 0 if it uses its master
/// page's.
#[derive(Debug, Clone, PartialEq)]
pub struct Margins {
    pub left: f64,
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
    pub own: bool,
}

/// Page columns from chunk 0x528: u32 count *n*, *n* f64 column edge
/// positions, f64 gutter, then u16 1 if the page has its own columns.
#[derive(Debug, Clone, PartialEq)]
pub struct Columns {
    pub positions: Vec<f64>,
    pub gutter: f64,
    pub own: bool,
}

/// Layout grid settings of a page (chunk 0xCD02): u32 font family, a flag
/// byte, the font style as an in-object string, five f64 and four u32.
/// See `docs/format/objects.md`.
#[derive(Debug, Clone, PartialEq)]
pub struct GridData {
    pub font: u32,
    pub font_style: String,
    pub numbers: [f64; 5],
    pub codes: [u32; 4],
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
    pub anchor: Option<Vec<u8>>,
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

/// Text wrap mode codes.
pub mod wrap_mode {
    pub const NONE: u32 = 0;
    pub const JUMP_OBJECT: u32 = 1;
    pub const BOUNDING_BOX: u32 = 3;
    pub const CONTOUR: u32 = 6;
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
type EmbeddedFile = Option<Vec<u8>>;

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

#[derive(Debug, Clone, PartialEq)]
pub struct Spread {
    pub uid: u32,
    /// Master spreads only: name prefix (for example "A") and base name.
    pub master_name: Option<(String, String)>,
    pub transform: Matrix,
    pub binding_location: u32,
    pub pages: Vec<Page>,
    pub items: Vec<PageItem>,
    pub guides: Vec<Guide>,
    /// Chunk 0x1A8: `None` without it.
    pub shuffle: Option<u16>,
    /// The two resolutions in chunk 0x10833, when it is there.
    pub flattener_resolution: Option<[f64; 2]>,
    /// Master spreads: chunk 0x140D.
    pub show_master_items: Option<u16>,
}

/// A ruler guide (class 0x3301, chunk 0x3308). See `docs/format/objects.md`.
#[derive(Debug, Clone, PartialEq)]
pub struct Guide {
    pub uid: u32,
    pub horizontal: bool,
    /// Position in spread coordinates: y for a horizontal guide, x for a
    /// vertical one.
    pub position: f64,
    /// The page the guide belongs to, or the spread.
    pub owner: u32,
    pub fit_to_page: bool,
    /// Stored view threshold (0.05 in every sample).
    pub view_threshold: f64,
    /// Colour code (6 in every sample).
    pub color: u32,
    /// 0 ruler guide, 1 liquid guide; none in 40-byte records.
    pub guide_type: Option<u32>,
    /// Document layer.
    pub layer: u32,
    /// Chunk 0x2C2D, as for page items.
    pub locked: bool,
    /// f64 at 44 of the 52-byte record (`GuideZone`).
    pub zone: Option<f64>,
    /// Chunk 0x1424, as for page items.
    pub overridden: Option<(u32, Vec<u32>)>,
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
    pub attrs: Attrs,
    /// The u16 after the kind, before the name (IDML `Imported`).
    pub imported: bool,
    /// The GUID string after the name in newer files (`StyleUniqueId`).
    pub unique_id: Option<String>,
}

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
    /// The XML element whose content is this story.
    pub xml_element: Option<xml::Key>,
    /// Text orientation, from the frames that show the story; `None` when
    /// it has no frame or its frames disagree.
    pub orientation: Option<Orientation>,
    /// The TOC style that made the story (chunk 0x8C40).
    pub toc_style: Option<u32>,
}

/// A pasted smooth shade with a constant shading. See
/// `docs/format/objects.md`, pasted smooth shades.
#[derive(Debug, Clone, PartialEq)]
pub struct ConstantShade {
    pub uid: u32,
    pub count: u32,
    pub values: [f64; 3],
    /// Flag (built-in key) and name.
    pub name: Option<(bool, String)>,
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

/// Document setup, from chunk 0x533 of the preferences object.
#[derive(Debug, Clone, PartialEq)]
pub struct DocumentPreferences {
    pub page_width: f64,
    pub page_height: f64,
    pub facing_pages: bool,
    /// Top, bottom, inside, outside (stored as inside, top, outside, bottom).
    pub bleed: [f64; 4],
    /// 0 print, 1 web, 2 mobile.
    pub intent: u32,
    /// Page binding (u16 at 64): 0 left to right, 1 right to left.
    pub page_binding: u16,
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

/// Values of chunk 0x28C2, the kind of a root style group.
pub mod root_kind {
    pub const PARAGRAPH: u32 = 0xCA0C;
    pub const CHARACTER: u32 = 0xCA0D;
    pub const OBJECT: u32 = 0x1B924;
    pub const TABLE: u32 = 0xB668;
    pub const CELL: u32 = 0xB669;
}

/// Numbering of a section.
#[derive(Debug, Clone, PartialEq)]
pub struct Section {
    pub uid: u32,
    /// The section's first page; `None` for the first section, which
    /// starts at the first page of the document.
    pub page: Option<u32>,
    pub continue_numbering: bool,
    pub start: u32,
    /// Page number style code (see `numbering`).
    pub style: u32,
    /// `SectionPrefix` and `Marker`; empty if the chunk is missing.
    pub prefix: String,
    pub marker: String,
    /// The alternate layout name after the numbers, with the flag byte
    /// before it (1: a built-in key); `None` if the chunk ends before it.
    pub alternate_layout: Option<(u8, String)>,
}

/// Page number style codes of sections.
pub mod numbering {
    pub const ARABIC: u32 = 0x4C15;
    pub const LOWER_ROMAN: u32 = 0x4C17;
    /// Chinese numerals, written digit by digit.
    pub const KANJI: u32 = 0x4C12;
}

/// A language object (class 0x2D07), chunk 0x2D0F. See
/// `docs/format/objects.md`, languages.
#[derive(Debug, Clone, PartialEq)]
pub struct Language {
    pub uid: u32,
    pub name: String,
    pub primary: String,
    pub sub: String,
    pub id: u16,
    /// The spelling and hyphenation vendors, each with its flag byte.
    pub vendors: Option<[(u8, String); 2]>,
}

/// A bullet character of the document's list (IDML `ABullet`): u32
/// character type, u32 character value, u32 font family (0 = none), the
/// font style as a flag byte and string, then a byte (0 in every sample).
#[derive(Debug, Clone, PartialEq)]
pub struct Bullet {
    /// 0 `UnicodeOnly`, 1 `UnicodeWithFont`, 2 `GlyphWithFont`.
    pub kind: u32,
    pub value: u32,
    pub font: u32,
    /// The IDML font style, with `$ID/` for a built-in name.
    pub font_style: String,
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

/// Reads typed objects from a database, caching them.
pub struct Reader<'a> {
    db: &'a Database<'a>,
    cache: std::cell::RefCell<HashMap<u32, std::rc::Rc<Object>>>,
    warnings: std::cell::RefCell<Vec<String>>,
    /// XML nodes found in the stories read so far.
    xml_nodes: std::cell::RefCell<BTreeMap<xml::Key, xml::Node>>,
    /// Text orientation of each text frame read so far, by story.
    frame_orientations: std::cell::RefCell<BTreeMap<u32, Vec<Option<Orientation>>>>,
}

fn uid_or_none(v: u32) -> Option<u32> {
    (v != 0).then_some(v)
}

impl<'a> Reader<'a> {
    pub fn new(db: &'a Database<'a>) -> Reader<'a> {
        Reader {
            db,
            cache: Default::default(),
            warnings: Default::default(),
            xml_nodes: Default::default(),
            frame_orientations: Default::default(),
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
        crate::audit::object_read(uid);
        self.cache.borrow_mut().insert(uid, obj.clone());
        Ok(obj)
    }

    /// Record a problem that does not stop the conversion.
    pub fn warn(&self, msg: String) {
        self.warnings.borrow_mut().push(msg);
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

    /// The text orientation shared by all frames of a story. A story
    /// whose frames disagree, or have an orientation that is not known,
    /// gets none, with a warning.
    fn story_orientation(&self, story: u32) -> Option<Orientation> {
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

    /// An interface colour (class 0x1F11): chunk 0x1F01 holds u32 space
    /// (5 = RGB), u16 count and the components as f64 fractions.
    fn ui_color(&self, uid: u32) -> Result<Option<[f64; 3]>, Error> {
        if self.class(uid) != Some(class::UI_COLOR) {
            return Ok(None);
        }
        let Some(d) = self.chunk(uid, color::chunk::COLOR_VALUE)? else {
            return Ok(None);
        };
        let mut c = Cursor::new(&d);
        if c.u32()? != 5 || c.u16()? != 3 {
            return Ok(None);
        }
        Ok(Some([c.f64()?, c.f64()?, c.f64()?]))
    }

    /// The bullet characters in the preferences object (chunk 0x1A488):
    /// u16 1, u32 count, then the bullets.
    fn bullets(&self) -> Result<Vec<Bullet>, Error> {
        let Some(&(uid, _)) = self
            .db
            .classes()
            .iter()
            .find(|(_, c)| *c == class::PREFERENCES)
        else {
            return Ok(Vec::new());
        };
        let Some(d) = self.chunk(uid, chunk::BULLETS)? else {
            return Ok(Vec::new());
        };
        let mut c = Cursor::new(&d);
        if c.u16()? != 1 {
            return Ok(Vec::new());
        }
        let n = c.u32()?;
        let mut out = Vec::new();
        for _ in 0..n {
            let kind = c.u32()?;
            let value = c.u32()?;
            let font = c.u32()?;
            let builtin = c.flag()? == 1;
            let style = c.string()?;
            // 0, or 1 and four bytes of unknown meaning (objects.md).
            match c.u8()? {
                0 => {}
                1 => c.skip(4)?,
                other => {
                    return Err(Error::Corrupt(format!(
                        "bullet font style followed by {other} at {}",
                        c.pos() - 1
                    )));
                }
            }
            out.push(Bullet {
                kind,
                value,
                font,
                font_style: if builtin {
                    format!("$ID/{style}")
                } else {
                    style
                },
            });
        }
        Ok(out)
    }

    /// The colour groups listed in the preferences object (chunk 0x1F61).
    fn color_group_order(&self) -> Result<Vec<u32>, Error> {
        let Some(&(uid, _)) = self
            .db
            .classes()
            .iter()
            .find(|(_, c)| *c == class::PREFERENCES)
        else {
            return Ok(Vec::new());
        };
        self.uid_list(uid, color::chunk::COLOR_GROUPS)
    }

    fn document_preferences(&self) -> Result<Option<DocumentPreferences>, Error> {
        let Some(&(uid, _)) = self
            .db
            .classes()
            .iter()
            .find(|(_, c)| *c == class::PREFERENCES)
        else {
            return Ok(None);
        };
        let Some(d) = self.chunk(uid, chunk::DOCUMENT_PREFERENCES)? else {
            return Ok(None);
        };
        if d.len() < 146 {
            self.warn(format!(
                "document preferences of {} bytes are not known; left out",
                d.len()
            ));
            return Ok(None);
        }
        let f = |o: usize| Cursor::new(&d[o..]).f64();
        let u = |o: usize| Cursor::new(&d[o..]).u32();
        let binding = Cursor::new(&d[64..]).u16()?;
        if binding > 1 {
            self.warn(format!(
                "document preferences: page binding code {binding} is not known; left out"
            ));
        }
        Ok(Some(DocumentPreferences {
            page_width: f(0)?,
            page_height: f(8)?,
            facing_pages: d[58] == 2,
            bleed: [f(78)?, f(94)?, f(70)?, f(86)?],
            intent: u(142)?,
            page_binding: binding,
        }))
    }

    /// Text orientation of a multi-column frame (chunk 0x2DE): the
    /// identity gives horizontal text, the rotation 0 1 −1 0 vertical
    /// text (`docs/format/objects.md`). `None` without the chunk;
    /// `Some(None)` for another matrix.
    fn frame_orientation(&self, mcf: u32) -> Result<Option<Option<Orientation>>, Error> {
        let Some(d) = self.chunk(mcf, chunk::FRAME_TEXT_TRANSFORM)? else {
            return Ok(None);
        };
        if d.len() < 48 {
            return Ok(Some(None));
        }
        let Matrix([a, b, c, dd, _, _]) = Matrix::read(&mut Cursor::new(&d))?;
        Ok(Some(match [a, b, c, dd] {
            [1.0, 0.0, 0.0, 1.0] => Some(Orientation::Horizontal),
            [0.0, 1.0, -1.0, 0.0] => Some(Orientation::Vertical),
            _ => None,
        }))
    }

    /// Text frame settings of frame `frame`, from its multi-column frame
    /// `mcf` and the frame itself.
    fn text_frame_preferences(
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
            let flag = |at| u16_at(&just, at).is_some_and(|v| v != 0);
            (
                [flag(26), flag(36)],
                [
                    f64_at(&just, 28).unwrap_or(0.0),
                    f64_at(&just, 38).unwrap_or(0.0),
                ],
                flag(46),
            )
        });
        let column_rule = match self.chunk(mcf, chunk::FRAME_COLUMN_RULE)? {
            Some(d) => match (f64_at(&d, 28), u32_at(&d, 36)) {
                (Some(w), Some(c)) => Some((w, c)),
                _ => None,
            },
            None => None,
        };
        let footnotes = match self.chunk(mcf, chunk::FRAME_FOOTNOTES)? {
            Some(d) => match (f64_at(&d, 4), f64_at(&d, 12)) {
                (Some(a), Some(b)) => Some([a, b]),
                _ => None,
            },
            None => None,
        };
        // The frame's chunk 0x3723: f64, u32, then four f64: left, top,
        // right, bottom.
        let inset = match self.chunk(frame, chunk::FRAME_INSET)? {
            Some(d) if d.len() >= 44 => {
                let f = |at| f64_at(&d, at).unwrap_or(0.0);
                [f(20), f(12), f(36), f(28)]
            }
            _ => [0.0; 4],
        };
        Ok(Some(TextFramePreferences {
            column_count: Cursor::new(&cols).u32()?,
            column_gutter: Cursor::new(&cols[4..]).f64()?,
            column_fixed_width: Cursor::new(&cols[14..]).f64()?,
            first_baseline_offset: Cursor::new(&just).u16()?,
            vertical_justification: Cursor::new(&just[2..]).u16()?,
            vertical_balance_columns: Cursor::new(&just[20..]).u16()? != 0,
            auto_sizing_type: Cursor::new(&just[22..]).u16()?,
            auto_sizing_reference_point: Cursor::new(&just[24..]).u16()?,
            use_fixed_width: cols.get(12).is_some_and(|&b| b != 0),
            max_width: if cols.len() >= 40 {
                f64_at(&cols, 32)
            } else {
                None
            },
            minimum_sizes,
            ignore_wrap: self
                .chunk(mcf, chunk::FRAME_IGNORE_WRAP)?
                .and_then(|d| u16_at(&d, 0))
                .map(|v| v != 0),
            column_rule,
            column_rule_override: self
                .chunk(mcf, chunk::FRAME_COLUMN_RULE_OVERRIDE)?
                .map(|d| d.iter().any(|&b| b != 0)),
            footnotes,
            inset,
        }))
    }

    fn section(&self, uid: u32) -> Result<Section, Error> {
        let mut section = Section {
            uid,
            page: None,
            continue_numbering: true,
            start: 1,
            style: numbering::ARABIC,
            prefix: String::new(),
            marker: String::new(),
            alternate_layout: None,
        };
        // u8, string (prefix), u8, string (marker), u32 first page, u32
        // page number start, u32 numbering style, u32 continue, u32, then
        // u8 and string (alternate layout).
        if let Some(d) = self.chunk(uid, chunk::SECTION_INFO)? {
            let mut c = Cursor::new(&d);
            let parsed = (|| -> Result<(String, String, [u32; 4]), Error> {
                c.u8()?;
                let prefix = c.string()?;
                c.u8()?;
                let marker = c.string()?;
                Ok((prefix, marker, [c.u32()?, c.u32()?, c.u32()?, c.u32()?]))
            })();
            if let Ok((prefix, marker, [page, start, style, cont])) = parsed {
                section.page = uid_or_none(page);
                section.start = start;
                section.style = style;
                section.continue_numbering = cont != 0;
                section.prefix = prefix;
                section.marker = marker;
                section.alternate_layout = (|| -> Result<(u8, String), Error> {
                    c.u32()?;
                    let flag = c.flag()?;
                    Ok((flag, c.string()?))
                })()
                .ok();
            }
        }
        if ![numbering::ARABIC, numbering::LOWER_ROMAN, numbering::KANJI].contains(&section.style) {
            self.warn(format!(
                "section {uid}: page number style {:#x} is not known; left out",
                section.style
            ));
        }
        Ok(section)
    }

    fn layer(&self, uid: u32) -> Result<Layer, Error> {
        let data = self.required(uid, chunk::LAYER_PROPS)?;
        let mut c = Cursor::new(&data);
        let locked = c.u16()? != 0;
        let visible = c.u16()? != 0;
        c.skip(14)?;
        // A string follows; its position varies, so search for the tag.
        let name = find_string(&data, 18)?;
        let settings = self.layer_settings(&data)?;
        Ok(Layer {
            uid,
            internal: name == "Internal_pages_layer_name",
            name,
            visible,
            locked,
            settings,
        })
    }

    /// The settings in chunk 0x304 when the name is at offset 19, as in
    /// all corpus pairs: u16 printable at 4, u16 lock guides at 6, u32
    /// colour at 10, u16 UI at 14; after the name, u16 ignore wrap.
    fn layer_settings(&self, data: &[u8]) -> Result<Option<LayerSettings>, Error> {
        if crate::object::big_endian() || data.len() < 23 || data[19] != 2 {
            return Ok(None);
        }
        let mut c = Cursor::new(&data[19..]);
        c.string()?;
        let ignore_wrap = c.u16()? != 0;
        let flag = |at: usize| u16_at(data, at).is_some_and(|v| v != 0);
        let color = match u32_at(data, 10) {
            Some(u) => self.ui_color(u)?,
            None => None,
        };
        Ok(Some(LayerSettings {
            printable: flag(4),
            lock_guides: flag(6),
            ui: flag(14),
            ignore_wrap,
            color,
        }))
    }

    /// A colour setting: 0 none, 1 the master's, else an interface colour.
    fn ui_color_ref(&self, code: u32) -> Result<UiColorRef, Error> {
        Ok(match code {
            0 => UiColorRef::Nothing,
            1 => UiColorRef::UseMaster,
            u => match self.ui_color(u)? {
                Some(rgb) => UiColorRef::Rgb(rgb),
                None => UiColorRef::Unknown,
            },
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
        let master_name = match self.chunk(uid, chunk::MASTER_NAME)? {
            Some(d) => {
                let mut c = Cursor::new(&d);
                c.u8()?;
                let prefix = c.string()?;
                c.u8()?;
                let base = c.string()?;
                Some((prefix, base))
            }
            None => None,
        };
        let children = self.required(uid, chunk::SPREAD_CHILDREN)?;
        let mut c = Cursor::new(&children);
        c.skip(8)?;
        let spread_layers = c.u32_list()?;
        let mut pages = Vec::new();
        let mut items = Vec::new();
        let mut guides = Vec::new();
        for sl in spread_layers {
            let layer = match self.chunk(sl, chunk::SPREAD_LAYER_LAYER)? {
                Some(d) => Cursor::new(&d).u32()?,
                None => 0,
            };
            for child in self.children(sl, chunk::SPREAD_LAYER_CHILDREN)? {
                match self.class(child) {
                    Some(class::PAGE) => pages.push(self.page(child)?),
                    Some(class::GUIDE) => guides.extend(self.guide(child, layer)?),
                    _ => {
                        if let Some(item) = self.page_item(child, Some(layer))? {
                            items.push(item);
                        }
                    }
                }
            }
        }
        if let Some(d) = self.chunk(uid, chunk::SPREAD_TAB_ORDERS)? {
            let mut c = Cursor::new(&d);
            let n = c.u32()?;
            for _ in 0..n {
                let page = c.u32()?;
                let order = c.u32_list()?;
                if let Some(p) = pages.iter_mut().find(|p| p.uid == page) {
                    p.settings.tab_order = order;
                }
            }
        }
        let flattener_resolution = match self.chunk(uid, chunk::SPREAD_FLATTENER)? {
            Some(d) => match (f64_at(&d, 20), f64_at(&d, 28)) {
                (Some(a), Some(b)) => Some([a, b]),
                _ => None,
            },
            None => None,
        };
        let short = |id: u32| -> Result<Option<u16>, Error> {
            Ok(self.chunk(uid, id)?.and_then(|d| u16_at(&d, 0)))
        };
        Ok(Spread {
            uid,
            master_name,
            transform,
            binding_location,
            pages,
            items,
            guides,
            shuffle: short(chunk::SPREAD_SHUFFLE)?,
            flattener_resolution,
            show_master_items: short(chunk::MASTER_SHOW_ITEMS)?,
        })
    }

    /// A ruler guide from chunk 0x3308: f64 position, u32 owner (page or
    /// spread), u16 orientation (1 horizontal), f64 view threshold, u32
    /// colour, u16 fit to page, f64, u32, u32 guide type, f64. Records of
    /// 40 bytes, from InDesign 3.0 to 7.0, end before the guide type.
    fn guide(&self, uid: u32, layer: u32) -> Result<Option<Guide>, Error> {
        let Some(d) = self.chunk(uid, chunk::GUIDE)? else {
            return Ok(None);
        };
        if d.len() != 40 && d.len() < 52 {
            self.warn(format!(
                "guide {uid}: guide record of {} bytes is not known; left out",
                d.len()
            ));
            return Ok(None);
        }
        let f = |o: usize| Cursor::new(&d[o..]).f64();
        let u = |o: usize| Cursor::new(&d[o..]).u32();
        let h = |o: usize| Cursor::new(&d[o..]).u16();
        Ok(Some(Guide {
            uid,
            position: f(0)?,
            owner: u(8)?,
            horizontal: h(12)? == 1,
            view_threshold: f(14)?,
            color: u(22)?,
            fit_to_page: h(26)? == 1,
            guide_type: if d.len() >= 52 { Some(u(40)?) } else { None },
            layer,
            locked: self
                .chunk(uid, chunk::ITEM_LOCKED)?
                .is_some_and(|d| u32_at(&d, 0) == Some(1)),
            zone: if d.len() >= 52 { Some(f(44)?) } else { None },
            overridden: match self.chunk(uid, chunk::ITEM_OVERRIDE)? {
                Some(o) => {
                    let mut c = Cursor::new(&o);
                    let master = c.u32()?;
                    Some((master, c.u32_list()?))
                }
                None => None,
            },
        }))
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
        // Files from InDesign 3.0 and 4.0 store the transform and bounds in
        // the chunks page items use (docs/format/big-endian.md).
        let transform = match self.chunk(uid, chunk::PAGE_TRANSFORM)? {
            Some(d) => d,
            None => self.required(uid, chunk::ITEM_TRANSFORM)?,
        };
        let transform = Matrix::read(&mut Cursor::new(&transform))?;
        let b = match self.chunk(uid, chunk::PAGE_BOUNDS)? {
            Some(d) => d,
            None => self.required(uid, chunk::OLD_PAGE_BOUNDS)?,
        };
        let mut c = Cursor::new(&b);
        let bounds = [c.f64()?, c.f64()?, c.f64()?, c.f64()?];
        let (master, master_transform) = match self.chunk(uid, chunk::PAGE_MASTER)? {
            Some(d) => {
                let mut c = Cursor::new(&d);
                let m = c.u32()?;
                c.skip(2)?;
                // The matrix is missing in files from InDesign 3.0 and 4.0.
                let t = match c.remaining() {
                    0 => Matrix::IDENTITY,
                    _ => Matrix::read(&mut c)?,
                };
                (uid_or_none(m), t)
            }
            None => (None, Matrix::IDENTITY),
        };
        let margins = match self.chunk(uid, chunk::PAGE_MARGINS)? {
            Some(d) if d.len() >= 34 => {
                let mut c = Cursor::new(&d);
                Some(Margins {
                    left: c.f64()?,
                    top: c.f64()?,
                    right: c.f64()?,
                    bottom: c.f64()?,
                    own: c.u16()? == 1,
                })
            }
            _ => None,
        };
        let columns = match self.chunk(uid, chunk::PAGE_COLUMNS)? {
            Some(d) => {
                let mut c = Cursor::new(&d);
                let n = c.u32()? as usize;
                if n > d.len() / 8 {
                    return Err(Error::Corrupt(format!("page {uid}: {n} column positions")));
                }
                let positions = (0..n).map(|_| c.f64()).collect::<Result<Vec<_>, _>>()?;
                Some(Columns {
                    positions,
                    gutter: c.f64()?,
                    own: c.u16()? == 1,
                })
            }
            None => None,
        };
        let grid = match self.chunk(uid, chunk::PAGE_GRID)? {
            Some(d) => {
                let mut c = Cursor::new(&d);
                let font = c.u32()?;
                c.u8()?;
                let font_style = c.string()?;
                let numbers = [c.f64()?, c.f64()?, c.f64()?, c.f64()?, c.f64()?];
                let codes = [c.u32()?, c.u32()?, c.u32()?, c.u32()?];
                Some(GridData {
                    font,
                    font_style,
                    numbers,
                    codes,
                })
            }
            None => None,
        };
        // u32 count, then (unless it is 0) the two lists.
        let overrides = match self.chunk(uid, chunk::PAGE_OVERRIDES)? {
            Some(d) if u32_at(&d, 0).is_some_and(|n| n > 0) => {
                let mut c = Cursor::new(&d[4..]);
                let items = c.u32_list()?;
                let with = c.u32_list()?;
                items.into_iter().zip(with).collect()
            }
            _ => Vec::new(),
        };
        let settings = PageSettings {
            overrides,
            use_master_grid: self
                .chunk(uid, chunk::PAGE_GRID_USE)?
                .and_then(|d| u16_at(&d, 4))
                .map(|v| v != 0),
            layout_rule: self
                .chunk(uid, chunk::PAGE_LAYOUT_RULE)?
                .and_then(|d| u32_at(&d, 4)),
            color: match self
                .chunk(uid, chunk::PAGE_COLOR)?
                .and_then(|d| u32_at(&d, 0))
            {
                Some(code) => self.ui_color_ref(code)?,
                None => UiColorRef::UseMaster,
            },
            tab_order: Vec::new(),
        };
        Ok(Page {
            uid,
            bounds,
            transform,
            master,
            master_transform,
            margins,
            columns,
            grid,
            settings,
        })
    }

    /// The document's users, from chunk 0xA443 of the document.
    fn users(&self, doc: u32) -> Result<Vec<(u8, String)>, Error> {
        let Some(d) = self.chunk(doc, chunk::DOC_USERS)? else {
            return Ok(Vec::new());
        };
        let mut c = Cursor::new(&d);
        let n = c.u32()?;
        if n as usize > d.len() / 9 {
            return Err(Error::Corrupt(format!("{n} document users")));
        }
        (0..n)
            .map(|_| {
                let flag = c.flag()?;
                let name = c.string()?;
                c.u32()?;
                Ok((flag, name))
            })
            .collect()
    }

    /// The settings every page item has (`ItemProps`).
    fn item_props(&self, uid: u32) -> Result<ItemProps, Error> {
        let name_chunk = if self.class(uid) == Some(class::GROUP) {
            chunk::GROUP_NAME
        } else {
            chunk::ITEM_NAME
        };
        let name = match self.chunk(uid, name_chunk)? {
            Some(d) => {
                let mut c = Cursor::new(&d);
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
            let mut c = Cursor::new(&d);
            let n = c.u32()? as usize;
            if n > d.len() / 8 {
                return Err(Error::Corrupt(format!("item {uid}: {n} change counts")));
            }
            (0..2 * n).map(|_| c.u32()).collect()
        };
        let overridden = match self.chunk(uid, chunk::ITEM_OVERRIDE)? {
            Some(d) => {
                let mut c = Cursor::new(&d);
                let master = c.u32()?;
                Some((master, c.u32_list()?))
            }
            None => None,
        };
        Ok(ItemProps {
            name,
            hidden: self
                .chunk(uid, chunk::ITEM_VISIBLE)?
                .is_some_and(|d| d.len() >= 2 && Cursor::new(&d).u16().ok() == Some(0)),
            locked: self
                .chunk(uid, chunk::ITEM_LOCKED)?
                .is_some_and(|d| d.len() >= 4 && Cursor::new(&d).u32().ok() == Some(1)),
            layout_constraints: self
                .chunk(uid, chunk::ITEM_LAYOUT_CONSTRAINTS)?
                .and_then(|d| d.first().copied()),
            change_counts: [
                counts(chunk::ITEM_PARENT_CHANGES)?,
                counts(chunk::ITEM_TARGET_CHANGES)?,
                counts(chunk::ITEM_UPDATED_CHANGES)?,
            ],
            overridden,
        })
    }

    /// Text wrap of a page item or graphic, from chunk 0x3703: u32 mode,
    /// u32 contour path object, four f64 offsets, u32 flags.
    fn text_wrap(&self, uid: u32) -> Result<Option<TextWrap>, Error> {
        self.wrap_chunk(uid, chunk::TEXT_WRAP)
    }

    /// A text wrap record in chunk `id` of object `uid`.
    fn wrap_chunk(&self, uid: u32, id: u32) -> Result<Option<TextWrap>, Error> {
        let Some(d) = self.chunk(uid, id)? else {
            return Ok(None);
        };
        if d.len() < 44 {
            return Ok(None);
        }
        let mut c = Cursor::new(&d);
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

    fn page_item(&self, uid: u32, layer: Option<u32>) -> Result<Option<PageItem>, Error> {
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
            Some(d) => Attrs::parse(&d, List::Item).unwrap_or_default(),
            None => Attrs::default(),
        };
        let object_style = match self.chunk(uid, chunk::ITEM_OBJECT_STYLE)? {
            Some(d) => uid_or_none(Cursor::new(&d).u32()?),
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
                .and_then(|d| u32_at(&d, 2));
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

    fn graphic(&self, uid: u32) -> Result<Option<Graphic>, Error> {
        let kind = match self.class(uid) {
            Some(class::IMAGE) => GraphicKind::Image,
            Some(class::PDF) => GraphicKind::Pdf,
            Some(class::EPS) => GraphicKind::Eps,
            Some(class::SVG) => GraphicKind::Svg,
            _ => return Ok(None),
        };
        let transform = match self.chunk(uid, chunk::ITEM_TRANSFORM)? {
            Some(d) => Matrix::read(&mut Cursor::new(&d))?,
            None => Matrix::IDENTITY,
        };
        let bounds = match self.chunk(uid, chunk::GRAPHIC_BOUNDS)? {
            Some(d) => {
                let mut c = Cursor::new(&d);
                [c.f64()?, c.f64()?, c.f64()?, c.f64()?]
            }
            None => [0.0; 4],
        };
        let link = match self.chunk(uid, chunk::GRAPHIC_LINK)? {
            Some(d) if d.len() >= 12 => self.link(Cursor::new(&d[8..]).u32()?)?,
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
                        Some(d) if d.len() >= 4 => self.raw_data(Cursor::new(&d).u32()?)?,
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
                Some(d) if d.len() >= 4 => Some(Cursor::new(&d).u32()?),
                _ => None,
            },
            clipping: match self.chunk(uid, chunk::CLIPPING_PATH)? {
                Some(d) if d.len() >= 26 => {
                    let f = |o: usize| Cursor::new(&d[o..]).f64();
                    Some(ClippingPath {
                        kind: Cursor::new(&d).u32()?,
                        tolerance: f(4)?,
                        inset: f(12)?,
                        threshold: d[20],
                        index: i16_from([d[23], d[24]]),
                        high_resolution: d[25],
                    })
                }
                _ => None,
            },
            photoshop_clipping: match self.chunk(uid, chunk::PHOTOSHOP_CLIPPING)? {
                Some(d) if d.len() >= 2 => Some(Cursor::new(&d).u16()?),
                _ => None,
            },
        }))
    }

    /// The bytes of raw data object `uid` (`None` if `uid` is not one).
    fn raw_data(&self, uid: u32) -> Result<EmbeddedFile, Error> {
        if uid == 0 || self.class(uid) != Some(class::RAW_DATA) {
            return Ok(None);
        }
        crate::audit::object_read(uid);
        self.db.object(uid)
    }

    /// A link, the URI of its resource and, for an embedded link, the
    /// embedded file.
    fn link(&self, uid: u32) -> Result<Option<(Link, EmbeddedFile)>, Error> {
        if uid == 0 || self.db.object(uid)?.is_none() {
            return Ok(None);
        }
        let Some(info) = self.chunk(uid, chunk::LINK_INFO)? else {
            return Ok(None);
        };
        if info.len() < 12 {
            return Ok(None);
        }
        let resource = Cursor::new(&info[8..]).u32()?;
        let (uri, data) = match self.chunk(resource, chunk::LINK_RESOURCE_URI)? {
            Some(d) if d.len() >= 5 => {
                let mut c = Cursor::new(&d);
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
    fn text_frame_links(&self, column: u32) -> Result<ItemKind, Error> {
        let none = ItemKind::TextFrame {
            story: None,
            previous: None,
            next: None,
            preferences: None,
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
            preferences: None,
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
        // flag byte (1 = built-in name), then an in-object string. Four
        // bytes before the flag, a u16 is 1 for paragraph styles and 0 for
        // character styles; the u16 after it is not identified.
        let Some((at, builtin, name)) = find_flagged_string(&data, 12, |_| true) else {
            return Err(Error::Corrupt(format!("style {uid}: no name")));
        };
        let paragraph = Cursor::new(&data[at - 4..]).u16()? != 0;
        let imported = Cursor::new(&data[at - 2..]).u16()? != 0;
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
        }))
    }

    fn story(&self, uid: u32) -> Result<Story, Error> {
        let data = self.required(uid, chunk::STORY_STRANDS)?;
        let mut c = Cursor::new(&data);
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
                let first = u32_from(tree[..4].try_into().unwrap());
                let pages = |uid| self.chunk(uid, hyperlink::chunk::RANGE_PAGE);
                match hyperlink::source_ranges(first, pages) {
                    Ok(r) => sources.extend(r),
                    Err(e) => self.warn(format!("story {uid}: hyperlink sources left out: {e}")),
                }
            }
            let Some(list) = self.chunk(strand, chunk::STRAND_DATA)? else {
                continue;
            };
            let mut c = Cursor::new(&list);
            let n = c.u16()?;
            let mut base = 0usize;
            for _ in 0..n {
                let data_len = c.u32()? as usize;
                let data_uid = c.u32()?;
                let runs = self.required(data_uid, chunk::STRAND_RUNS)?;
                let mut r = Cursor::new(&runs);
                let kind = r.u32()?;
                r.skip(4)?;
                let count = r.u16()?;
                let mut pos = base;
                base += data_len;
                for _ in 0..count {
                    let size = r.u32()? as usize;
                    let rec = r.bytes(size)?;
                    let mut rc = Cursor::new(rec);
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
                            let attrs =
                                Attrs::parse_text(&mut rc, n, List::Text).unwrap_or_default();
                            let list = if kind == strand::PARAGRAPH_STYLE {
                                &mut para
                            } else {
                                &mut chars
                            };
                            list.push((len, style, attrs));
                        }
                        _ => crate::audit::unknown_strand_kind(kind),
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
                let mut c = Cursor::new(&d);
                Some((c.u32()?, c.u32()?))
            }
            _ => None,
        };
        let mut anchors: BTreeMap<usize, Vec<PageItem>> = BTreeMap::new();
        let mut tables: BTreeMap<usize, Table> = BTreeMap::new();
        let mut text_variables = BTreeMap::new();
        for (pos, cls, item) in owned {
            match cls {
                class::TEXT_VARIABLE_INSTANCE => {
                    let v = variable::Instance::read(item, &*self.object(item)?)?;
                    text_variables.insert(pos, v);
                }
                class::ANCHOR => {
                    let settings = self.chunk(item, chunk::ANCHOR_SETTINGS)?;
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
        // IDML writes a text source inside one character range; keep the
        // sources that lie within one run.
        sources.sort_by_key(|r| r.start);
        let cell_runs = tables
            .values()
            .flat_map(|t| t.cells.iter().flat_map(|c| &c.runs));
        let spans: Vec<(usize, usize)> = runs
            .iter()
            .chain(cell_runs)
            .map(|r| (r.start, r.start + r.text.encode_utf16().count()))
            .collect();
        sources.retain(|r| {
            let inside = spans
                .iter()
                .any(|&(a, b)| r.start >= a && r.start + r.len <= b);
            if !inside && r.len > 0 {
                self.warn(format!(
                    "story {uid}: hyperlink source {} spans several text ranges; left out",
                    r.source
                ));
            }
            // The IDML schema allows no page item inside a text source.
            let anchored = anchors.range(r.start..r.start + r.len).next().is_some();
            if inside && anchored {
                self.warn(format!(
                    "story {uid}: hyperlink source {} holds an anchored object; left out",
                    r.source
                ));
            }
            inside && r.len > 0 && !anchored
        });
        Ok(Story {
            uid,
            runs,
            anchors,
            tables,
            text_variables,
            sources,
            xml_markers,
            xml_element,
            orientation: None,
            // The story names an object (class 0x8C20) whose chunk 0x11613
            // is the TOC style.
            toc_style: match self
                .chunk(uid, chunk::STORY_TOC)?
                .and_then(|d| u32_at(&d, 0))
                .and_then(uid_or_none)
            {
                Some(toc) => self
                    .chunk(toc, chunk::TOC_STYLE_OF)?
                    .and_then(|d| u32_at(&d, 0))
                    .and_then(uid_or_none),
                None => None,
            },
        })
    }

    /// The index sort groups of the preferences object, chunk 0x1307E: u32
    /// group count, then the groups with their sections. Each group is
    /// found by its name, a flag byte and an in-object string starting with
    /// `kIndexGroup_` or `kWRIndexGroup_`, at its first occurrence; after
    /// the name come u8 include, u8, u16 header variant. Empty unless the
    /// names found are as many as the count. See `docs/format/objects.md`.
    fn index_groups(&self) -> Vec<(String, bool, u16)> {
        let Some(&(uid, _)) = self
            .db
            .classes()
            .iter()
            .find(|(_, c)| *c == class::PREFERENCES)
        else {
            return Vec::new();
        };
        let Ok(Some(d)) = self.chunk(uid, chunk::INDEX_GROUPS) else {
            return Vec::new();
        };
        if crate::object::big_endian() {
            return Vec::new();
        }
        let Some(count) = u32_at(&d, 0) else {
            return Vec::new();
        };
        let mut out: Vec<(String, bool, u16)> = Vec::new();
        for i in 4..d.len().saturating_sub(6) {
            if d[i] != 1 || d[i + 1] != 2 {
                continue;
            }
            let mut c = Cursor::new(&d[i + 1..]);
            let Ok(name) = c.string() else { continue };
            if !(name.starts_with("kIndexGroup_") || name.starts_with("kWRIndexGroup_"))
                || out.iter().any(|(n, _, _)| *n == name)
            {
                continue;
            }
            let at = i + 1 + c.pos();
            let (Some(&include), Some(header)) = (d.get(at), u16_at(&d, at + 2)) else {
                continue;
            };
            out.push((name, include != 0, header));
        }
        if out.len() == count as usize {
            out
        } else {
            Vec::new()
        }
    }

    /// The first pasted smooth shade (class 0x5533) whose chunk 0x5532
    /// holds a constant shade: 92 bytes, with u32 28 at offset 60 and then
    /// a u32 and three f64. See `docs/format/objects.md`.
    fn constant_shade(&self) -> Option<ConstantShade> {
        let mut uids: Vec<u32> = self
            .db
            .classes()
            .iter()
            .filter(|&&(_, c)| c == class::SMOOTH_SHADE)
            .map(|&(u, _)| u)
            .collect();
        uids.sort_unstable();
        uids.into_iter().find_map(|uid| {
            let d = self.chunk(uid, chunk::SMOOTH_SHADE).ok()??;
            if d.len() != 92 || u32_at(&d, 60) != Some(28) {
                return None;
            }
            let name = self
                .chunk(uid, chunk::SMOOTH_SHADE_NAME)
                .ok()
                .flatten()
                .and_then(|n| {
                    let mut c = Cursor::new(&n);
                    let builtin = c.flag().ok()? == 1;
                    Some((builtin, c.string().ok()?))
                });
            Some(ConstantShade {
                uid,
                count: u32_at(&d, 64)?,
                values: [f64_at(&d, 68)?, f64_at(&d, 76)?, f64_at(&d, 84)?],
                name,
            })
        })
    }

    /// The named grids, in UID order.
    fn named_grids(&self) -> Vec<(bool, String)> {
        let mut out = Vec::new();
        for &(uid, cls) in self.db.classes() {
            if cls != class::NAMED_GRID {
                continue;
            }
            let read = (|| -> Result<Option<(u32, bool, String)>, Error> {
                let Some(d) = self.chunk(uid, chunk::NAMED_GRID)? else {
                    return Ok(None);
                };
                let mut c = Cursor::new(&d);
                c.u32()?;
                let builtin = c.flag()? == 1;
                Ok(Some((uid, builtin, c.string()?)))
            })();
            match read {
                Ok(Some(g)) => out.push(g),
                Ok(None) => {}
                Err(e) => self.warn(format!("named grid {uid} left out: {e}")),
            }
        }
        out.sort_by_key(|g| g.0);
        out.into_iter().map(|(_, b, n)| (b, n)).collect()
    }

    /// The table of contents styles, in UID order.
    fn toc_styles(&self) -> Vec<TocStyle> {
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

    /// Read the XML nodes stored with story `uid` and place their markers.
    /// `unit` converts a character position to a UTF-16 offset; `para` are
    /// the paragraph runs, in UTF-16 units. An element whose start and end
    /// lie in different paragraph runs is left out.
    fn story_xml(
        &self,
        uid: u32,
        marker_strand: Option<u32>,
        para: &[StyleRun],
        unit: &dyn Fn(usize) -> usize,
    ) -> Result<BTreeMap<usize, XmlMarker>, Error> {
        let mut out = BTreeMap::new();
        let first = match self.chunk(uid, xml::chunk::STORE)? {
            Some(d) if d.len() >= 4 => Cursor::new(&d).u32()?,
            _ => return Ok(out),
        };
        let nodes = xml::read_store(first, |p| self.chunk(p, xml::chunk::PAGE))?;
        if nodes.is_empty() {
            return Ok(out);
        }
        let positions = match marker_strand.map(|s| self.chunk(s, xml::chunk::MARKER_TREE)) {
            Some(r) => match r? {
                Some(d) => xml::marker_positions(&d)?,
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
    fn xml_elements(
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
                            (uid_or_none(Cursor::new(&d[4..]).u32()?), false)
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

/// The UTF-16 offset of each character of `text`, where a surrogate pair is
/// one character, followed by the length of `text`.
fn char_offsets(text: &[u16]) -> Vec<usize> {
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

/// A master page's applied master and its own margins and columns.
type MasterPageLayout = (Option<u32>, Option<Margins>, Option<Columns>);

/// Replace the margins and columns that pages take from their master page
/// (flag 0) with the values in effect on that master page. A page's master
/// page is the page at the same position, counted from the left, in its
/// applied master spread, or that spread's only page. See
/// `docs/format/objects.md`.
fn resolve_page_layout(spreads: &mut [Spread], masters: &mut [Spread]) {
    fn left_to_right(s: &Spread) -> Vec<usize> {
        let mut order: Vec<usize> = (0..s.pages.len()).collect();
        order.sort_by(|&a, &b| s.pages[a].transform.0[4].total_cmp(&s.pages[b].transform.0[4]));
        order
    }
    fn master_page(
        raw: &HashMap<u32, Vec<MasterPageLayout>>,
        master: Option<u32>,
        index: usize,
    ) -> Option<(usize, &MasterPageLayout)> {
        let pages = raw.get(&master?)?;
        let i = if pages.len() == 1 { 0 } else { index };
        pages.get(i).map(|p| (i, p))
    }
    fn effective<T: Clone>(
        raw: &HashMap<u32, Vec<MasterPageLayout>>,
        get: fn(&MasterPageLayout) -> (Option<&T>, bool),
        page: &MasterPageLayout,
        index: usize,
        depth: usize,
    ) -> Option<T> {
        let (own, is_own) = get(page);
        if is_own || depth > 8 {
            return own.cloned();
        }
        match master_page(raw, page.0, index) {
            Some((i, m)) => effective(raw, get, m, i, depth + 1).or_else(|| own.cloned()),
            None => own.cloned(),
        }
    }
    let raw: HashMap<u32, Vec<MasterPageLayout>> = masters
        .iter()
        .map(|s| {
            let pages = left_to_right(s)
                .into_iter()
                .map(|i| {
                    let p = &s.pages[i];
                    (p.master, p.margins.clone(), p.columns.clone())
                })
                .collect();
            (s.uid, pages)
        })
        .collect();
    fn margins(p: &MasterPageLayout) -> (Option<&Margins>, bool) {
        (p.1.as_ref(), p.1.as_ref().is_some_and(|m| m.own))
    }
    fn columns(p: &MasterPageLayout) -> (Option<&Columns>, bool) {
        (p.2.as_ref(), p.2.as_ref().is_some_and(|c| c.own))
    }
    for s in spreads.iter_mut().chain(masters.iter_mut()) {
        for (index, i) in left_to_right(s).into_iter().enumerate() {
            let p = &mut s.pages[i];
            let layout = (p.master, p.margins.clone(), p.columns.clone());
            p.margins = effective(&raw, margins, &layout, index, 0);
            p.columns = effective(&raw, columns, &layout, index, 0);
        }
    }
}

/// Find a flag byte (1 = built-in name) followed by an in-object string
/// that `accept` takes, at or after `from`. Returns the offset of the pair,
/// whether the flag is 1, and the string. In big-endian data the flag and
/// the string's tag (2) are swapped (`docs/format/big-endian.md`).
fn find_flagged_string(
    data: &[u8],
    from: usize,
    accept: impl Fn(&str) -> bool,
) -> Option<(usize, bool, String)> {
    let big = crate::object::big_endian();
    (from..data.len().saturating_sub(6)).find_map(|i| {
        let header = match big {
            false => data[i] <= 2 && data[i + 1] == 2,
            true => data[i] == 2 && data[i + 1] <= 2,
        };
        if !header {
            return None;
        }
        let mut c = Cursor::new(&data[i..]);
        let builtin = c.flag().ok()? == 1;
        let s = c.string().ok()?;
        accept(&s).then_some((i, builtin, s))
    })
}

/// Find the first in-object string at or after `from`.
fn find_string(data: &[u8], from: usize) -> Result<String, Error> {
    if crate::object::big_endian() {
        // The tag (2), the byte before it in little-endian data, then the
        // rest of the header (`Cursor::string`).
        for i in from..data.len().saturating_sub(5) {
            if data[i] == 2
                && data[i + 1] <= 2
                && u16_from([data[i + 3], data[i + 4]]) > 0
                && let Ok(s) = Cursor::new(&data[i..]).string()
            {
                return Ok(s);
            }
        }
        return Ok(String::new());
    }
    for i in from..data.len().saturating_sub(4) {
        if data[i] == 2 && data[i + 1] == 0 {
            let n = u16_from([data[i + 2], data[i + 3]]) as usize;
            if n > 0
                && i + 6 <= data.len()
                && (u16_from([data[i + 4], data[i + 5]]) & 0xC000) != 0
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
type StyleRun = (usize, u32, Attrs);

fn split_runs(
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

/// The IDML element of a frame or shape: by the shape code of chunk
/// 0x6204 (`code`), and where it has none (9, or no chunk), by the path.
/// See `docs/format/objects.md`, page items.
fn classify(paths: &[Path], code: Option<u32>) -> Shape {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pages_take_margins_from_master_page_at_same_position() {
        let margins = |top, own| Margins {
            left: 0.0,
            top,
            right: 0.0,
            bottom: 0.0,
            own,
        };
        let page = |uid, tx, master, m| Page {
            uid,
            bounds: [0.0; 4],
            transform: Matrix([1.0, 0.0, 0.0, 1.0, tx, 0.0]),
            master,
            master_transform: Matrix::IDENTITY,
            margins: Some(m),
            columns: None,
            grid: None,
            settings: Default::default(),
        };
        let spread = |uid, pages| Spread {
            uid,
            master_name: None,
            transform: Matrix::IDENTITY,
            binding_location: 0,
            pages,
            items: Vec::new(),
            guides: Vec::new(),
            shuffle: None,
            flattener_resolution: None,
            show_master_items: None,
        };
        // Master B (pages listed right to left) is based on master A.
        let mut masters = vec![
            spread(
                10,
                vec![
                    page(1, -100.0, None, margins(10.0, true)),
                    page(2, 0.0, None, margins(20.0, true)),
                ],
            ),
            spread(
                11,
                vec![
                    page(3, 0.0, Some(10), margins(0.0, false)),
                    page(4, -100.0, Some(10), margins(30.0, true)),
                ],
            ),
        ];
        let mut spreads = vec![spread(
            12,
            vec![
                page(5, -100.0, Some(11), margins(0.0, false)),
                page(6, 0.0, Some(11), margins(0.0, false)),
            ],
        )];
        resolve_page_layout(&mut spreads, &mut masters);
        let tops: Vec<f64> = spreads[0]
            .pages
            .iter()
            .map(|p| p.margins.as_ref().unwrap().top)
            .collect();
        assert_eq!(tops, [30.0, 20.0]);
    }

    #[test]
    fn counts_a_surrogate_pair_as_one_position() {
        let text: Vec<u16> = "a\u{1F93F}b".encode_utf16().collect();
        assert_eq!(char_offsets(&text), [0, 1, 3, 4]);
    }

    #[test]
    fn splits_runs_at_style_boundaries() {
        let text: Vec<u16> = "abcdef".encode_utf16().collect();
        let a = Attrs::default;
        let runs = split_runs(
            &text,
            &[(4, 10, a()), (2, 11, a())],
            &[(2, 20, a()), (4, 21, a())],
            &[],
        );
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
            kind: 2,
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
        assert_eq!(
            classify(std::slice::from_ref(&rect), None),
            Shape::Rectangle
        );
        // The stored shape code decides where there is one.
        assert_eq!(classify(&[rect], Some(7)), Shape::Polygon);
        let line = Path {
            points: vec![corner(0., 0.), corner(3., 1.)],
            open: true,
        };
        assert_eq!(classify(&[line], Some(9)), Shape::GraphicLine);
        let smooth = |x, y, kind| PathPoint {
            anchor: (x, y),
            left: (x - 1., y),
            right: (x + 1., y),
            kind,
        };
        let round = |kind| Path {
            points: vec![
                smooth(0., 1., kind),
                smooth(1., 0., kind),
                smooth(2., 1., kind),
                smooth(1., 2., kind),
            ],
            open: false,
        };
        assert_eq!(classify(&[round(0)], None), Shape::Oval);
        assert_eq!(classify(&[round(1)], None), Shape::Polygon);
    }
}
