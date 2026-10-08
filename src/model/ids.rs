//! Class, chunk and strand IDs and the enumeration codes the model reads.
//!
//! Evidence: each constant's section in `docs/format/` (mostly
//! `objects.md`).

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
    /// A footnote, owned by a story at its reference (U+0004).
    pub const FOOTNOTE: u32 = 0x24F;
    /// An InDesign note, owned by a story at its anchor (U+FEFF).
    pub const NOTE: u32 = 0xA429;
    /// Deleted text of a tracked change, owned by the character after the
    /// deletion.
    pub const DELETED_TEXT: u32 = 0xA40A;
    /// An index marker, owned by a story at its U+FEFF.
    pub const INDEX_MARKER: u32 = 0x13006;
    /// The endnote story (`IsEndnoteStory`).
    pub const ENDNOTE_STORY: u32 = 0x2801;
    /// An endnote, owned by a story at its reference (U+0004).
    pub const ENDNOTE: u32 = 0x2805;
    /// The text range of an endnote in the endnote story.
    pub const ENDNOTE_RANGE: u32 = 0x2804;
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
    /// Note: user name and times.
    pub const NOTE: u32 = 0xA412;
    /// Index marker: its `Id` in the last u32.
    pub const INDEX_MARKER: u32 = 0x13009;
    /// Endnote: the UID of its range.
    pub const ENDNOTE_RANGE_OF: u32 = 0x22616;
    /// Endnote range: the UID of its endnote.
    pub const RANGE_ENDNOTE: u32 = 0x2261A;
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
    /// A group's transform when it has no chunk 0x151.
    pub const GROUP_TRANSFORM: u32 = 0x40D;
    /// u16 1 for a frame meant for a graphic.
    pub const ITEM_CONTENT: u32 = 0x1623;
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
    /// Export options: alternative text, actual text, tagging.
    pub const ITEM_EXPORT: u32 = 0x1E206;
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
    /// u32 PDF identifier of a link.
    pub const LINK_PDF_IDENTIFIER: u32 = 0x1B6;
    /// Pasted image without a link: u32 raw data object.
    pub const IMAGE_DATA: u32 = 0x8C23;
    /// Pasted PDF without a link: u32 raw data object.
    pub const PDF_DATA: u32 = 0x2521;
    pub const SWATCH_NAME: u32 = 0x1F30;
    pub const TEXT_WRAP: u32 = 0x3703;
    pub const CONTOUR_OPTION: u32 = 0x373D;
    pub const CLIPPING_PATH: u32 = 0x2C1A;
    /// Image properties: record list (pixels, colour space, resolution).
    pub const IMAGE_PROPERTIES: u32 = 0x1708;
    /// Colour profile of an image: u32 code, then a name.
    pub const IMAGE_PROFILE: u32 = 0x7C0F;
    /// Image import options: clipping path flag, alpha channel name.
    pub const IMAGE_IMPORT: u32 = 0x1714;
    /// Vector colour policies of a PDF or EPS: four u32.
    pub const VECTOR_POLICIES: u32 = 0x7C42;
    /// PDF placement: page number, transparent background, crop.
    pub const PDF_PLACEMENT: u32 = 0x251B;
    /// Layers of an image, PDF or imported page.
    pub const GRAPHIC_LAYERS: u32 = 0x177A;
    /// Applied layer comp of an image (i32 at 4).
    pub const LAYER_COMP: u32 = 0x9209;
    /// Frame fitting attributes of an object style (u16 count, records).
    pub const OBJECT_STYLE_FITTING: u32 = 0x1B956;
    /// Page item attributes of an object style (u16 count, records).
    pub const OBJECT_STYLE_ATTRS: u32 = 0x1B92B;
    pub const OBJECT_STYLE_TRANSPARENCY: u32 = 0x1B92C;
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
    /// Tracked changes: user, kind and time of each change.
    pub const CHANGES: u32 = 0xA466;
}

/// Text wrap mode codes.
pub mod wrap_mode {
    pub const NONE: u32 = 0;
    pub const JUMP_OBJECT: u32 = 1;
    pub const BOUNDING_BOX: u32 = 3;
    pub const CONTOUR: u32 = 6;
}

/// Values of chunk 0x28C2, the kind of a root style group.
pub mod root_kind {
    pub const PARAGRAPH: u32 = 0xCA0C;
    pub const CHARACTER: u32 = 0xCA0D;
    pub const OBJECT: u32 = 0x1B924;
    pub const TABLE: u32 = 0xB668;
    pub const CELL: u32 = 0xB669;
}

/// Page number style codes of sections.
pub mod numbering {
    pub const ARABIC: u32 = 0x4C15;
    pub const LOWER_ROMAN: u32 = 0x4C17;
    /// Chinese numerals, written digit by digit.
    pub const KANJI: u32 = 0x4C12;
}
