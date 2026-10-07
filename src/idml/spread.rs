//! Spreads and master spreads: pages, guides, page items with their paths,
//! frame settings, text wrap and fitting, and placed graphics.
//!
//! Evidence: `docs/format/objects.md`, `attributes.md` and
//! `transparency.md`.

use super::*;

/// The top-left corner of a spread's pages in spread coordinates: the
/// smallest left and top edge of any page (0, 0 without pages).
pub(super) fn spread_origin(pages: &[Page]) -> (f64, f64) {
    let corners = pages.iter().map(|p| {
        let [a, b, c, d, tx, ty] = p.transform.0;
        let (l, t) = (p.bounds[0], p.bounds[1]);
        (a * l + c * t + tx, b * l + d * t + ty)
    });
    corners
        .reduce(|(l0, t0), (l1, t1)| (l0.min(l1), t0.min(t1)))
        .unwrap_or((0.0, 0.0))
}

impl Writer<'_> {
    pub(super) fn path_geometry(x: &mut Xml, paths: &[Path]) {
        if paths.is_empty() {
            return;
        }
        x.start("Properties").start("PathGeometry");
        for p in paths {
            x.start("GeometryPathType")
                .attr("PathOpen", p.open.to_string());
            x.start("PathPointArray");
            for pt in &p.points {
                x.empty(
                    "PathPointType",
                    &[
                        ("Anchor", nums(&[pt.anchor.0, pt.anchor.1])),
                        ("LeftDirection", nums(&[pt.left.0, pt.left.1])),
                        ("RightDirection", nums(&[pt.right.0, pt.right.1])),
                    ],
                );
            }
            x.end().end();
        }
        x.end().end();
    }

    pub(super) fn text_frame_preference(
        x: &mut Xml,
        p: &TextFramePreferences,
        major: u32,
        swatch: impl Fn(u32) -> Option<String>,
    ) {
        const POINTS: [&str; 9] = [
            "TopLeftPoint",
            "TopCenterPoint",
            "TopRightPoint",
            "LeftCenterPoint",
            "CenterPoint",
            "RightCenterPoint",
            "BottomLeftPoint",
            "BottomCenterPoint",
            "BottomRightPoint",
        ];
        const JUSTIFY: [&str; 4] = ["TopAlign", "CenterAlign", "BottomAlign", "JustifyAlign"];
        const FIRST_BASELINE: [&str; 4] =
            ["LeadingOffset", "AscentOffset", "CapHeight", "EmboxHeight"];
        const SIZING: [&str; 5] = [
            "Off",
            "HeightOnly",
            "WidthOnly",
            "HeightAndWidth",
            "HeightAndWidthProportionally",
        ];
        x.start("TextFramePreference")
            .attr("TextColumnCount", p.column_count.to_string())
            .attr("TextColumnGutter", num(p.column_gutter))
            .attr("TextColumnFixedWidth", num(p.column_fixed_width));
        if let Some(v) = FIRST_BASELINE.get(p.first_baseline_offset as usize) {
            x.attr("FirstBaselineOffset", *v);
        }
        if let Some(v) = JUSTIFY.get(p.vertical_justification as usize) {
            x.attr("VerticalJustification", *v);
        }
        x.attr(
            "VerticalBalanceColumns",
            p.vertical_balance_columns.to_string(),
        );
        if let Some(v) = SIZING.get(p.auto_sizing_type as usize) {
            x.attr("AutoSizingType", *v);
        }
        if let Some(v) = POINTS.get(p.auto_sizing_reference_point as usize) {
            x.attr("AutoSizingReferencePoint", *v);
        }
        if let Some(w) = p.max_width {
            x.attr("TextColumnMaxWidth", num(w));
        }
        x.attr("UseFixedColumnWidth", p.use_fixed_width.to_string());
        if let Some(([use_height, use_width], [height, width], no_breaks)) = p.minimum_sizes {
            x.attr("UseMinimumHeightForAutoSizing", use_height.to_string())
                .attr("MinimumHeightForAutoSizing", num(height))
                .attr("UseMinimumWidthForAutoSizing", use_width.to_string())
                .attr("MinimumWidthForAutoSizing", num(width))
                .attr("UseNoLineBreaksForAutoSizing", no_breaks.to_string());
        }
        if let Some(v) = p.ignore_wrap {
            x.attr("IgnoreWrap", v.to_string());
        }
        // Values every IDML of the version has where it writes them; they
        // also say from which version the column rule and footnote
        // settings exist (idml-values.md).
        let written = values::when_written("TextFrame/TextFramePreference", major);
        let has = |k: &str| written.iter().any(|(n, _)| n == k);
        if has("ColumnRuleOffset") {
            let (width, color) = match p.column_rule {
                Some((w, c)) => (w, swatch(c)),
                // Without the chunk (objects.md).
                None => (1.0, Some("Color/Black".to_string())),
            };
            x.attr("ColumnRuleStrokeWidth", num(width));
            if let Some(c) = color {
                x.attr("ColumnRuleStrokeColor", c);
            }
            if p.column_rule_override != Some(true) {
                x.attr("ColumnRuleOverride", "false");
            }
        }
        if has("FootnotesEnableOverrides") {
            let [spacing, between] = p.footnotes.unwrap_or([12.0, 6.0]);
            x.attr("FootnotesMinimumSpacing", num(spacing))
                .attr("FootnotesSpaceBetween", num(between));
        }
        x.attrs_missing(written.iter());
        if major >= 11 {
            let [top, left, bottom, right] = p.inset;
            let item = |v: f64| Node {
                tag: "ListItem".into(),
                attrs: vec![("type".into(), "unit".into())],
                text: Some(num(v)),
                children: Vec::new(),
            };
            Self::properties_with(
                x,
                &[],
                &[Node {
                    tag: "InsetSpacing".into(),
                    attrs: vec![("type".into(), "list".into())],
                    text: None,
                    children: vec![item(top), item(left), item(bottom), item(right)],
                }],
            );
        }
        x.end();
    }

    /// `TextWrapPreference` from an item's text wrap chunk. An item without
    /// the chunk has no wrap. Values not identified yet are left out: the
    /// whole element for an unknown mode, and the side and inverse settings
    /// for flags other than 1.
    pub(super) fn text_wrap_preference(
        x: &mut Xml,
        wrap: Option<&TextWrap>,
        contour_type: Option<u32>,
    ) {
        let mode = match wrap.map(|w| w.mode) {
            None | Some(wrap_mode::NONE) => "None",
            Some(wrap_mode::JUMP_OBJECT) => "JumpObjectTextWrap",
            Some(wrap_mode::BOUNDING_BOX) => "BoundingBoxTextWrap",
            Some(wrap_mode::CONTOUR) => "Contour",
            Some(_) => return,
        };
        x.start("TextWrapPreference");
        if wrap.is_none_or(|w| w.flags == 1) {
            x.attr("Inverse", "false")
                .attr("ApplyToMasterPageOnly", "false")
                .attr("TextWrapSide", "BothSides");
        }
        x.attr("TextWrapMode", mode);
        let [left, top, right, bottom] = wrap.map_or([0.0; 4], |w| w.offsets);
        x.start("Properties").empty(
            "TextWrapOffset",
            &[
                ("Top", num(top)),
                ("Left", num(left)),
                ("Bottom", num(bottom)),
                ("Right", num(right)),
            ],
        );
        x.end();
        if contour_type == Some(5) {
            x.empty("ContourOption", &[("ContourType", "SameAsClipping".into())]);
        }
        x.end();
    }

    /// `ClippingPathSettings` of an image, PDF or EPS graphic, and
    /// `ImageIOPreference` of an image. Values without an INDD field are
    /// those every exported IDML has, and a graphic without chunk 0x2C1A
    /// has the values every such graphic has in IDML. See
    /// `docs/format/objects.md`.
    pub(super) fn clipping(x: &mut Xml, g: &Graphic) {
        let (high_resolution, threshold, tolerance, inset, index) = match &g.clipping {
            Some(c) if c.kind == 0 => (
                match c.high_resolution {
                    2 => Some("true"),
                    0 => Some("false"),
                    _ => None,
                },
                c.threshold.to_string(),
                num(c.tolerance),
                num(c.inset),
                c.index.to_string(),
            ),
            Some(_) => return,
            None => (
                Some("true"),
                "25".into(),
                "2".into(),
                "0".into(),
                "-1".into(),
            ),
        };
        x.start("ClippingPathSettings")
            .attr("ClippingType", "None")
            .attr("InvertPath", "false")
            .attr("IncludeInsideEdges", "false")
            .attr("RestrictToFrame", "false");
        if let Some(h) = high_resolution {
            x.attr("UseHighResolutionImage", h);
        }
        x.attr("Threshold", threshold)
            .attr("Tolerance", tolerance)
            .attr("InsetFrame", inset)
            .attr("AppliedPathName", "$ID/")
            .attr("Index", index);
        x.end();
        if g.kind != GraphicKind::Image {
            return;
        }
        x.start("ImageIOPreference");
        match g.photoshop_clipping {
            Some(1) => {
                x.attr("ApplyPhotoshopClippingPath", "true");
            }
            Some(0) => {
                x.attr("ApplyPhotoshopClippingPath", "false");
            }
            _ => {}
        }
        x.attr("AllowAutoEmbedding", "true")
            .attr("AlphaChannelName", "$ID/");
        x.end();
    }

    pub(super) fn placed_graphic(x: &mut Xml, g: &Graphic) {
        let tag = match g.kind {
            GraphicKind::Image => "Image",
            GraphicKind::Pdf => "PDF",
            GraphicKind::Eps => "EPS",
            GraphicKind::Svg => "SVG",
        };
        let [left, top, right, bottom] = g.bounds;
        x.start(tag)
            .attr("Self", uref(Some(g.uid)))
            .attr("ItemTransform", matrix(&g.transform));
        x.start("Properties");
        if let Some(data) = &g.contents {
            x.start("Contents")
                .cdata(&base64_lines(data), CDATA_SECTION)
                .end();
        }
        x.empty(
            "GraphicBounds",
            &[
                ("Left", num(left)),
                ("Top", num(top)),
                ("Right", num(right)),
                ("Bottom", num(bottom)),
            ],
        );
        x.end();
        if g.kind != GraphicKind::Svg {
            Self::clipping(x, g);
        }
        Self::text_wrap_preference(x, g.text_wrap.as_ref(), g.contour_type);
        if let Some(link) = &g.link {
            x.empty(
                "Link",
                &[
                    ("Self", uref(Some(link.uid))),
                    ("LinkResourceURI", link.uri.clone()),
                    (
                        "StoredState",
                        if link.embedded { "Embedded" } else { "Normal" }.into(),
                    ),
                ],
            );
        }
        x.end();
    }

    /// The name, visibility, lock and the other settings every page item
    /// has; `nested` for an item inside another page item, which IDML
    /// writes without `Locked`. See `docs/format/objects.md`.
    pub(super) fn item_settings(&self, x: &mut Xml, item: &PageItem, nested: bool) {
        let p = &item.props;
        let name = match &p.name {
            Some(n) if n.builtin => format!("$ID/{}", n.name),
            Some(n) => n.name.clone(),
            None => "$ID/".into(),
        };
        x.attr("Name", name)
            .attr("Visible", (!p.hidden).to_string());
        if !nested {
            x.attr("Locked", p.locked.to_string());
        }
        if self.doc.version.major >= 8 {
            for (name, counts) in [
                "ParentInterfaceChangeCount",
                "TargetInterfaceChangeCount",
                "LastUpdatedInterfaceChangeCount",
            ]
            .into_iter()
            .zip(&p.change_counts)
            {
                x.attr(name, join_numbers(counts));
            }
            let overridden = match &p.overridden {
                Some((master, ids)) if *master != 0 => join_numbers(ids),
                _ => String::new(),
            };
            x.attr("OverriddenPageItemProps", overridden);
            if let Some((h, v)) = layout_constraints(p.layout_constraints.unwrap_or(0x22)) {
                x.attr("HorizontalLayoutConstraints", h)
                    .attr("VerticalLayoutConstraints", v);
            }
        }
        for (name, v) in Self::gradients(item) {
            if !x.has_attr(name) {
                x.attr(name, v);
            }
        }
    }

    /// `nested` for an item inside another page item.
    pub(super) fn page_item(&self, x: &mut Xml, item: &PageItem, nested: bool) {
        let tag = match &item.kind {
            ItemKind::TextFrame { .. } => "TextFrame",
            ItemKind::Group => "Group",
            ItemKind::Shape(Shape::Rectangle) => "Rectangle",
            ItemKind::Shape(Shape::Oval) => "Oval",
            ItemKind::Shape(Shape::Polygon) => "Polygon",
            ItemKind::Shape(Shape::GraphicLine) => "GraphicLine",
        };
        x.start(tag).attr("Self", uref(Some(item.uid)));
        if let ItemKind::TextFrame {
            story,
            previous,
            next,
            ..
        } = &item.kind
        {
            x.attr("ParentStory", uref(*story))
                .attr("PreviousTextFrame", uref(*previous))
                .attr("NextTextFrame", uref(*next))
                .attr("ContentType", "TextType");
        } else if let ItemKind::Shape(_) = item.kind {
            let content = if item.graphics.is_empty() && !item.props.graphic_frame {
                "Unassigned"
            } else {
                "GraphicType"
            };
            x.attr("ContentType", content);
        }
        self.item_attrs(x, &item.attrs);
        let style = item
            .object_style
            .and_then(|u| self.doc.object_styles.get(&u));
        // An item without a stroke weight of its own and with the object
        // style [None] has `StrokeWeight="1"` in IDML (objects.md, page
        // item settings); groups vary.
        if tag != "Group"
            && item.attrs.0.iter().all(|(id, _)| *id != 0x6E65)
            && style.is_some_and(|os| os.builtin && os.name == "[None]")
        {
            x.attr("StrokeWeight", "1");
        }
        if let Some(os) = style {
            let name = if os.builtin {
                format!("$ID/{}", os.name)
            } else {
                os.name.clone()
            };
            x.attr(
                "AppliedObjectStyle",
                format!("ObjectStyle/{}", self_name(&name)),
            );
        }
        if let Some(layer) = item.layer {
            x.attr("ItemLayer", uref(Some(layer)));
        }
        x.attr("ItemTransform", matrix(&item.transform));
        self.item_settings(x, item, nested);
        x.attrs_missing(self.observed(tag).iter());
        Self::path_geometry(x, &item.paths);
        if let ItemKind::TextFrame {
            preferences: Some(p),
            ..
        } = &item.kind
        {
            Self::text_frame_preference(x, p, self.doc.version.major, |u| match u {
                0 => Some("n".to_string()),
                u => self.doc.swatches.get(&u).cloned(),
            });
        }
        let frame = matches!(
            item.kind,
            ItemKind::Shape(Shape::Rectangle | Shape::Oval | Shape::Polygon)
        );
        if frame {
            self.frame_fitting(x, item);
        }
        // Export options every IDML of the version has on this kind of
        // item; from DOM 12 on, also those of the object styles
        // (idml-values.md).
        let major = self.doc.version.major;
        let mut export = values::element(&format!("{tag}/ObjectExportOption"), major);
        if major >= 12
            && let Some(n) = values::object_style(major).child("ObjectExportOption")
        {
            match &mut export {
                Some(e) => e.merge(n),
                None => export = Some(n.clone()),
            }
        }
        if let Some(n) = export {
            n.write(x);
        }
        if let Some(d) = &item.anchor {
            let style = item
                .object_style
                .and_then(|u| self.doc.object_styles.get(&u))
                .and_then(|s| s.anchor.as_ref());
            let attrs: Vec<_> = anchored_settings(d)
                .into_iter()
                .filter(|a| style.is_none_or(|s| !anchored_settings(s).contains(a)))
                .collect();
            if !attrs.is_empty() {
                x.empty("AnchoredObjectSetting", &attrs);
            }
        }
        Self::text_wrap_preference(x, item.text_wrap.as_ref(), None);
        if frame {
            x.empty(
                "InCopyExportOption",
                &[
                    ("IncludeGraphicProxies", "true".into()),
                    ("IncludeAllResources", "false".into()),
                ],
            );
        }
        for (effect, attr, v) in
            transparency::write(x, &item.attrs, &uref(Some(item.uid)), &self.doc.swatches)
        {
            self.warnings.borrow_mut().push(format!(
                "item {}: {effect} {attr} {v} is outside the IDML range; left out",
                item.uid
            ));
        }
        for child in &item.children {
            self.page_item(x, child, true);
        }
        for g in &item.graphics {
            Self::placed_graphic(x, g);
        }
        x.end();
    }

    /// `FrameFittingOption` of a rectangle, oval or polygon. IDML writes
    /// the local values that differ from the object style's; with none,
    /// all of the style's values unless the style is the root `[None]`.
    /// See `docs/format/objects.md`.
    pub(super) fn frame_fitting(&self, x: &mut Xml, item: &PageItem) {
        let style = item
            .object_style
            .and_then(|u| self.doc.object_styles.get(&u));
        let differ: Vec<u32> = FITTING_ATTRS
            .iter()
            .map(|(id, _)| *id)
            .filter(|&id| {
                item.attrs
                    .get(id)
                    .is_some_and(|v| style.and_then(|s| s.fitting.get(id)) != Some(v))
            })
            .collect();
        let attrs = if !differ.is_empty() {
            fitting_attrs(&item.attrs, &differ)
        } else {
            match style {
                Some(s) if !(s.builtin && s.name == "[None]" && s.based_on.is_none()) => {
                    let all: Vec<u32> = FITTING_ATTRS.iter().map(|(id, _)| *id).collect();
                    fitting_attrs(&s.fitting, &all)
                }
                _ => Vec::new(),
            }
        };
        if !attrs.is_empty() {
            x.empty("FrameFittingOption", &attrs);
        }
    }

    /// A ruler guide. `origin` is the top-left corner of the spread's
    /// pages, from which IDML measures `Location` with the ruler origin
    /// `SpreadOrigin`. See `docs/format/objects.md`.
    /// `page_index` is IDML `PageIndex` (see `docs/format/objects.md`).
    pub(super) fn guide(x: &mut Xml, g: &Guide, origin: (f64, f64), page_index: i64) {
        let (left, top) = origin;
        x.start("Guide")
            .attr("Self", uref(Some(g.uid)))
            .attr(
                "Orientation",
                if g.horizontal {
                    "Horizontal"
                } else {
                    "Vertical"
                },
            )
            .attr(
                "Location",
                num(g.position - if g.horizontal { top } else { left }),
            )
            .attr("FitToPage", g.fit_to_page.to_string());
        // The only stored value in the samples, with the only IDML value.
        if g.view_threshold == 0.05 {
            x.attr("ViewThreshold", "5");
        }
        if g.layer != 0 {
            x.attr("ItemLayer", uref(Some(g.layer)));
        }
        match g.guide_type {
            Some(0) => {
                x.attr("GuideType", "Ruler");
            }
            Some(1) => {
                x.attr("GuideType", "Liquid");
            }
            _ => {}
        }
        x.attr("Locked", g.locked.to_string())
            .attr("PageIndex", page_index.to_string());
        if let Some(z) = g.zone {
            x.attr("GuideZone", num(z));
        }
        if let Some((master, ids)) = &g.overridden {
            let ids = if *master == 0 {
                String::new()
            } else {
                join_numbers(ids)
            };
            x.attr("OverriddenPageItemProps", ids);
        } else if g.zone.is_some() {
            x.attr("OverriddenPageItemProps", "");
        }
        if g.color == 6 {
            Self::properties(
                x,
                &[("GuideColor", "enumeration", "Cyan".to_string().into())],
            );
        }
        x.end();
    }

    /// The settings of a page: its tab order, overridden master items,
    /// layout grid use, layout rule and colour, and the values every IDML
    /// of the version has. See `docs/format/objects.md`, page settings.
    pub(super) fn page_settings(&self, x: &mut Xml, p: &Page, section: Option<&(Section, u32)>) {
        let st = &p.settings;
        let tab: Vec<String> = st.tab_order.iter().map(|&u| uref(Some(u))).collect();
        x.attr("TabOrder", tab.join(" "));
        let overrides: Vec<String> = st
            .overrides
            .iter()
            .flat_map(|&(item, with)| [uref(Some(item)), uref((with != 0).then_some(with))])
            .collect();
        x.attr("OverrideList", overrides.join(" "));
        if let Some(v) = st.use_master_grid {
            x.attr("UseMasterGrid", v.to_string());
        }
        if self.doc.version.major >= 8 {
            const RULES: [&str; 6] = [
                "",
                "Recenter",
                "ObjectBased",
                "Scale",
                "GuideBased",
                "UseMaster",
            ];
            match st.layout_rule {
                None => {
                    x.attr("LayoutRule", "Off");
                }
                Some(code) => {
                    if let Some(r) = RULES.get(code as usize).filter(|r| !r.is_empty()) {
                        x.attr("LayoutRule", *r);
                    }
                }
            }
        }
        x.attrs_missing(self.observed("Page").iter());
        let mut props: Vec<Node> = page_color(&st.color).into_iter().collect();
        // A document page describes its numbering: section prefix, style,
        // continue, include prefix, page number (from DOM 20 twice) and
        // marker.
        if let Some((s, number)) = section
            && let Some(style) = number_style(s.style)
        {
            let item = |ty: &str, text: String| Node {
                tag: "ListItem".into(),
                attrs: vec![("type".into(), ty.into())],
                text: Some(text),
                children: Vec::new(),
            };
            let mut items = vec![
                item("string", s.prefix.clone()),
                item("enumeration", style.into()),
                item("boolean", s.continue_numbering.to_string()),
                item("boolean", "false".into()),
                item("long", number.to_string()),
            ];
            if self.doc.version.major >= 20 {
                items.push(item("long", number.to_string()));
            }
            items.push(item("string", s.marker.clone()));
            props.push(Node {
                tag: "Descriptor".into(),
                attrs: vec![("type".into(), "list".into())],
                text: None,
                children: items,
            });
        }
        Self::properties_with(x, &[], &props);
    }

    /// `MarginPreference` and `GridDataInformation` of a page. Margins
    /// and columns are those in effect (see `resolve_page_layout`). The
    /// grid values that are the same in every sample are written only
    /// when the stored value is that one. See `docs/format/objects.md`.
    pub(super) fn page_layout(&self, x: &mut Xml, p: &Page) {
        if let (Some(m), Some(c)) = (&p.margins, &p.columns) {
            x.empty(
                "MarginPreference",
                &[
                    ("ColumnCount", (c.positions.len() / 2).to_string()),
                    ("ColumnGutter", num(c.gutter)),
                    ("Top", num(m.top)),
                    ("Bottom", num(m.bottom)),
                    ("Left", num(m.left)),
                    ("Right", num(m.right)),
                    // Every exported IDML has this (idml-values.md).
                    ("ColumnDirection", "Horizontal".into()),
                    ("ColumnsPositions", nums(&c.positions)),
                ],
            );
        }
        if let Some(g) = &p.grid {
            self.grid_data(x, g);
        }
    }

    /// `GridDataInformation` of layout grid settings (chunk 0xCD02).
    pub(super) fn grid_data(&self, x: &mut Xml, g: &crate::model::GridData) {
        x.start("GridDataInformation")
            .attr("FontStyle", &g.font_style);
        let [size, character_aki, line_aki, h_scale, v_scale] = g.numbers;
        for (name, value, observed, scale) in [
            ("PointSize", size, 12.0, 1.0),
            ("CharacterAki", character_aki, 0.0, 1.0),
            ("LineAki", line_aki, 9.0, 1.0),
            ("HorizontalScale", h_scale, 1.0, 100.0),
            ("VerticalScale", v_scale, 1.0, 100.0),
        ] {
            if value == observed {
                x.attr(name, num(value * scale));
            }
        }
        if g.codes == [3, 0, 3, 1] {
            x.attr("LineAlignment", "LeftOrTopLineJustify")
                .attr("GridAlignment", "AlignEmCenter")
                .attr("CharacterAlignment", "AlignEmCenter");
        }
        if let Some(f) = self.doc.fonts.get(&g.font) {
            Self::properties(x, &[("AppliedFont", "string", f.name.clone().into())]);
        }
        x.end();
    }

    /// `page_index` counts the document pages written so far; `names`
    /// gives each document page its name (see `page_names`).
    pub(super) fn spread(
        &self,
        s: &Spread,
        master: bool,
        page_index: &mut usize,
        names: &[String],
    ) -> String {
        let mut x = Xml::new();
        let kind = if master { "MasterSpread" } else { "Spread" };
        self.package_root(&mut x, kind);
        x.start(kind)
            .attr("Self", uref(Some(s.uid)))
            .attr("PageCount", s.pages.len().to_string());
        let origin = spread_origin(&s.pages);
        if !master {
            x.attr("BindingLocation", s.binding_location.to_string());
        }
        x.attr("ItemTransform", matrix(&s.transform));
        let (prefix, base) = s
            .master_name
            .clone()
            .unwrap_or_else(|| ("A".into(), "Master".into()));
        if master {
            x.attr("Name", format!("{prefix}-{base}"))
                .attr("NamePrefix", &prefix)
                .attr("BaseName", &base);
            match s.show_master_items {
                None => {
                    x.attr("ShowMasterItems", "true");
                }
                Some(0) => {
                    x.attr("ShowMasterItems", "false");
                }
                Some(_) => {}
            }
        } else {
            match s.shuffle {
                Some(0) => {
                    x.attr("AllowPageShuffle", "true");
                }
                Some(1) | None => {
                    x.attr("AllowPageShuffle", "false");
                }
                Some(_) => {}
            }
        }
        x.attrs_missing(self.observed(kind).iter());
        // A master spread has the colour its pages share (objects.md).
        if master
            && let Some(first) = s.pages.first()
            && s.pages
                .iter()
                .all(|p| p.settings.color == first.settings.color)
            && let Some(n) = page_color(&first.settings.color)
        {
            x.start("Properties");
            n.write(&mut x);
            x.end();
        }
        let major = self.doc.version.major;
        if !master && let Some(mut fp) = values::element("Spread/FlattenerPreference", major) {
            if let Some([line_art, gradient]) = s.flattener_resolution {
                fp.attrs
                    .insert(0, ("GradientAndMeshResolution".into(), num(gradient)));
                fp.attrs
                    .insert(0, ("LineArtAndTextResolution".into(), num(line_art)));
            }
            fp.write(&mut x);
        }
        for p in &s.pages {
            let name = if master {
                prefix.clone()
            } else {
                *page_index += 1;
                names
                    .get(*page_index - 1)
                    .cloned()
                    .unwrap_or_else(|| page_index.to_string())
            };
            // Master pages have none (`n`).
            let layout = if master {
                None
            } else {
                self.page_layouts.get(*page_index - 1).copied()
            };
            let [x0, y0, x1, y1] = p.bounds;
            x.start("Page")
                .attr("Self", uref(Some(p.uid)))
                .attr("Name", name)
                .attr("GeometricBounds", nums(&[y0, x0, y1, x1]))
                .attr("ItemTransform", matrix(&p.transform))
                // A master page can itself be based on a master.
                .attr("AppliedMaster", uref(p.master))
                .attr("MasterPageTransform", matrix(&p.master_transform));
            if self.doc.version.major >= 8 {
                x.attr("AppliedAlternateLayout", uref(layout));
            }
            let section = if master {
                None
            } else {
                self.page_sections.get(*page_index - 1)
            };
            self.page_settings(&mut x, p, section);
            // A guide belongs to a page, or to the spread; IDML writes a
            // spread's guides in its first page.
            let first = p.uid == s.pages[0].uid;
            // Pages count from the spine: left of it -1, -2, ..., right of
            // it 1, 2, ...; a master spread's spine is at its left edge.
            let position = s.pages.iter().position(|q| q.uid == p.uid).unwrap_or(0) as i64;
            let binding = if master { 0 } else { s.binding_location as i64 };
            let from_spine = if position < binding {
                position - binding
            } else {
                position - binding + 1
            };
            for g in &s.guides {
                let own_page = s.pages.iter().any(|q| q.uid == g.owner);
                if g.owner == p.uid || (first && !own_page) {
                    let index = if own_page { from_spine } else { 0 };
                    Self::guide(&mut x, g, origin, index);
                }
            }
            self.page_layout(&mut x, p);
            x.end();
        }
        for item in &s.items {
            self.page_item(&mut x, item, false);
        }
        x.end();
        x.finish()
    }
}
