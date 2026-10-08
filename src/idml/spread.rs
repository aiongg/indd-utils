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

/// Page item attributes that IDML writes only when they differ from the
/// applied object style (attributes.md, page item attributes).
const STYLE_COMPARED: &[&str] = &[
    "FillColor",
    "FillTint",
    "StrokeColor",
    "StrokeWeight",
    "MiterLimit",
    "CornerOption",
    "CornerRadius",
    "TopLeftCornerRadius",
    "TopRightCornerRadius",
    "BottomLeftCornerRadius",
    "BottomRightCornerRadius",
    "TopLeftCornerOption",
    "TopRightCornerOption",
    "BottomLeftCornerOption",
    "BottomRightCornerOption",
];

/// The category of a `TextFramePreference` attribute: the ID that turns
/// it on in an object style's category list (objects.md, text frame
/// preferences).
fn frame_category(name: &str) -> Option<u32> {
    Some(match name {
        "TextColumnCount"
        | "TextColumnGutter"
        | "TextColumnFixedWidth"
        | "UseFixedColumnWidth"
        | "UseFlexibleColumnWidth"
        | "TextColumnMaxWidth"
        | "VerticalJustification"
        | "VerticalThreshold"
        | "IgnoreWrap"
        | "VerticalBalanceColumns" => 0x1B93E,
        "FirstBaselineOffset" | "MinimumFirstBaselineOffset" => 0xADC8,
        n if n.starts_with("Footnotes") => 0xADCA,
        n if n.starts_with("ColumnRule") => 0xADCB,
        n if n.contains("AutoSizing") => 0xADC9,
        _ => return None,
    })
}

/// `TextFramePreference` attributes that IDML writes on every frame of
/// the version, whatever the object style (objects.md, text frame
/// preferences). `InsetSpacing` (DOM 11 on) is written separately.
fn frame_always(name: &str, major: u32) -> bool {
    match name {
        "TextColumnMaxWidth" => major >= 8,
        "TextColumnCount" => major >= 10,
        _ => false,
    }
}

/// `AutoSizingReferencePoint` codes of frames and object styles.
pub(super) const POINTS: [&str; 9] = [
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
/// `VerticalJustification` codes.
pub(super) const JUSTIFY: [&str; 4] = ["TopAlign", "CenterAlign", "BottomAlign", "JustifyAlign"];
/// `FirstBaselineOffset` codes (objects.md, text frame preferences).
pub(super) const FIRST_BASELINE: [&str; 5] = [
    "LeadingOffset",
    "AscentOffset",
    "CapHeight",
    "EmboxHeight",
    "XHeight",
];
/// `AutoSizingType` codes.
pub(super) const SIZING: [&str; 5] = [
    "Off",
    "HeightOnly",
    "WidthOnly",
    "HeightAndWidth",
    "HeightAndWidthProportionally",
];

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

    /// `TextFramePreference` of a text frame. IDML writes an attribute
    /// only when its category is off in the frame's object style or its
    /// value differs from the style's, apart from a few attributes it
    /// always writes (objects.md, text frame preferences). `style` is the
    /// applied style's values; without a style every value is written.
    pub(super) fn text_frame_preference(
        &self,
        x: &mut Xml,
        p: &TextFramePreferences,
        style: Option<&applied::StyleValues>,
    ) {
        let v = self.doc.version;
        let major = v.major;
        let swatch = |u: u32| match u {
            0 => Some("n".to_string()),
            u => self.doc.swatches.get(&u).cloned(),
        };
        let mut attrs: Vec<(&str, String)> = vec![
            ("TextColumnCount", p.column_count.to_string()),
            ("TextColumnGutter", num(p.column_gutter)),
            ("TextColumnFixedWidth", num(p.column_fixed_width)),
        ];
        if let Some(v) = FIRST_BASELINE.get(p.first_baseline_offset as usize) {
            attrs.push(("FirstBaselineOffset", v.to_string()));
        }
        if let Some(v) = JUSTIFY.get(p.vertical_justification as usize) {
            attrs.push(("VerticalJustification", v.to_string()));
        }
        attrs.push((
            "VerticalBalanceColumns",
            p.vertical_balance_columns.to_string(),
        ));
        if let Some(v) = SIZING.get(p.auto_sizing_type as usize) {
            attrs.push(("AutoSizingType", v.to_string()));
        }
        if let Some(v) = POINTS.get(p.auto_sizing_reference_point as usize) {
            attrs.push(("AutoSizingReferencePoint", v.to_string()));
        }
        if let Some(w) = p.max_width {
            attrs.push(("TextColumnMaxWidth", num(w)));
        }
        attrs.push(("UseFixedColumnWidth", p.use_fixed_width.to_string()));
        if let Some(([use_height, use_width], [height, width], no_breaks)) = p.minimum_sizes {
            attrs.push(("UseMinimumHeightForAutoSizing", use_height.to_string()));
            attrs.push(("MinimumHeightForAutoSizing", num(height)));
            attrs.push(("UseMinimumWidthForAutoSizing", use_width.to_string()));
            attrs.push(("MinimumWidthForAutoSizing", num(width)));
            attrs.push(("UseNoLineBreaksForAutoSizing", no_breaks.to_string()));
        }
        if let Some(v) = p.ignore_wrap {
            attrs.push(("IgnoreWrap", v.to_string()));
        }
        // Values every IDML of the version has where it writes them; they
        // also say from which version the column rule and footnote
        // settings exist (idml-values.md). Footnote values from 13.1.
        let written = values::when_written("TextFrame/TextFramePreference", major);
        let has = |k: &str| written.iter().any(|(n, _)| n == k);
        let footnotes = (major, v.minor) >= (13, 1);
        if has("ColumnRuleOffset") {
            let (width, color) = match p.column_rule {
                Some((w, c)) => (w, swatch(c)),
                // Without the chunk (objects.md).
                None => (1.0, Some("Color/Black".to_string())),
            };
            attrs.push(("ColumnRuleStrokeWidth", num(width)));
            if let Some(c) = color {
                attrs.push(("ColumnRuleStrokeColor", c));
            }
            if p.column_rule_override != Some(true) {
                attrs.push(("ColumnRuleOverride", "false".into()));
            }
        }
        if has("FootnotesEnableOverrides") && footnotes {
            let [spacing, between] = p.footnotes.unwrap_or([12.0, 6.0]);
            attrs.push(("FootnotesMinimumSpacing", num(spacing)));
            attrs.push(("FootnotesSpaceBetween", num(between)));
        }
        for (k, val) in written.iter() {
            if !footnotes && k.starts_with("Footnotes") {
                continue;
            }
            if !attrs.iter().any(|(n, _)| n == k) {
                attrs.push((k.as_str(), val.clone()));
            }
        }
        if let Some(s) = style {
            attrs.retain(|(k, val)| {
                frame_always(k, major)
                    || !s
                        .enabled
                        .as_ref()
                        .is_some_and(|on| frame_category(k).is_some_and(|c| on.contains(&c)))
                    || !s.frame(k).is_some_and(|sv| applied::same_value(sv, val))
            });
        }
        if attrs.is_empty() && major < 11 {
            return;
        }
        x.start("TextFramePreference");
        for (k, val) in attrs {
            x.attr(k, val);
        }
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
        let Some(mode) = text_wrap_mode(wrap) else {
            return;
        };
        x.start("TextWrapPreference");
        if wrap.is_none_or(|w| w.flags == 1) {
            x.attr("Inverse", "false")
                .attr("ApplyToMasterPageOnly", "false")
                .attr("TextWrapSide", "BothSides");
        }
        x.attr("TextWrapMode", mode);
        x.start("Properties")
            .empty("TextWrapOffset", &text_wrap_offsets(wrap));
        x.end();
        if contour_type == Some(5) {
            x.empty("ContourOption", &[("ContourType", "SameAsClipping".into())]);
        }
        x.end();
    }

    /// The name, visibility, lock and the other settings every page item
    /// has; `nested` for an item inside another page item, which IDML
    /// writes without `Locked`. See `docs/format/objects.md`.
    pub(super) fn item_settings(&self, x: &mut Xml, item: &PageItem, nested: bool) {
        let locked = (!nested).then_some(item.props.locked);
        self.settings(x, &item.props, locked);
        for (name, v) in Self::gradients(item) {
            if !x.has_attr(name) {
                x.attr(name, v);
            }
        }
    }

    /// The settings of `ItemProps` (page items and placed graphics):
    /// `Locked` only when `locked` is given. See `docs/format/objects.md`,
    /// page item settings.
    pub(super) fn settings(&self, x: &mut Xml, p: &ItemProps, locked: Option<bool>) {
        let name = match &p.name {
            Some(n) if n.builtin => builtin_key(&n.name),
            Some(n) => n.name.clone(),
            None => "$ID/".into(),
        };
        x.attr("Name", name)
            .attr("Visible", (!p.hidden).to_string());
        if let Some(l) = locked {
            x.attr("Locked", l.to_string());
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
    }

    /// The `AppliedObjectStyle` reference of an object style UID.
    pub(super) fn object_style_ref(&self, uid: u32) -> Option<String> {
        let os = self.doc.object_styles.get(&uid)?;
        let name = self.grouped_name(os.uid, &os.name, os.builtin);
        Some(format!("ObjectStyle/{}", self_name(&name)))
    }

    /// `nested` for an item inside another page item; `outer` is the
    /// transform from the item's parent to the spread (for anchored items,
    /// to the anchor).
    pub(super) fn page_item(&self, x: &mut Xml, item: &PageItem, nested: bool, outer: &Matrix) {
        let tag = match &item.kind {
            // A frame of the endnote story (footnotes.md).
            ItemKind::TextFrame { story: Some(s), .. }
                if self.doc.stories.iter().any(|t| t.uid == *s && t.is_endnote) =>
            {
                "EndnoteTextFrame"
            }
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
        let applied = item.object_style.and_then(|u| self.style_values(u));
        for (name, v) in self.item_attr_values(&item.attrs) {
            // Fill, stroke and corner values equal to the object style's
            // are left out (attributes.md, page item attributes).
            let same = STYLE_COMPARED.contains(&name)
                && applied
                    .as_ref()
                    .and_then(|s| s.item(name))
                    .is_some_and(|s| applied::same_value(s, &v));
            if !same {
                x.attr(name, v);
            }
        }
        let style = item
            .object_style
            .and_then(|u| self.doc.object_styles.get(&u));
        // An item without a stroke weight of its own and with the object
        // style [None] has `StrokeWeight="1"` in IDML (objects.md, page
        // item settings); groups vary.
        if tag != "Group"
            && item.attrs.values.iter().all(|(id, _)| *id != 0x6E65)
            && style.is_some_and(|os| os.builtin && os.name == "[None]")
        {
            x.attr("StrokeWeight", "1");
        }
        if let Some(r) = item.object_style.and_then(|u| self.object_style_ref(u)) {
            x.attr("AppliedObjectStyle", r);
        }
        // Only items directly on a spread have `ItemLayer` (objects.md,
        // page item settings).
        if let Some(layer) = item.layer.filter(|_| !nested) {
            x.attr("ItemLayer", uref(Some(layer)));
        }
        x.attr("ItemTransform", matrix(&item.transform));
        self.item_settings(x, item, nested);
        // An endnote text frame has the values of a text frame.
        let observed = if tag == "EndnoteTextFrame" {
            "TextFrame"
        } else {
            tag
        };
        x.attrs_missing(self.observed(observed).iter());
        Self::path_geometry(x, &item.paths);
        if let ItemKind::TextFrame {
            preferences: Some(p),
            ..
        } = &item.kind
        {
            self.text_frame_preference(x, p, applied.as_deref());
        }
        let frame = matches!(
            item.kind,
            ItemKind::Shape(Shape::Rectangle | Shape::Oval | Shape::Polygon)
        );
        if frame {
            self.frame_fitting(x, item);
        }
        if let Some(n) = export::object_export_option(self.doc.version, item.export.as_ref(), false)
        {
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
        for (effect, attr, v) in transparency::write(
            self,
            x,
            &item.attrs,
            applied.as_deref(),
            &uref(Some(item.uid)),
        ) {
            self.warnings.borrow_mut().push(format!(
                "item {}: {effect} {attr} {v} is outside the IDML range; left out",
                item.uid
            ));
        }
        for child in &item.children {
            self.page_item(x, child, true, &item.transform.then(outer));
        }
        for g in &item.graphics {
            self.placed_graphic(x, g, &item.transform.then(outer));
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
            .map(|(id, ..)| *id)
            .filter(|&id| {
                item.attrs
                    .get(id)
                    .is_some_and(|v| style.and_then(|s| s.fitting.get(id)) != Some(v))
            })
            .collect();
        let attrs = if !differ.is_empty() {
            fitting_attrs(self, &item.attrs, &differ)
        } else {
            match style {
                Some(s) if !(s.builtin && s.name == "[None]" && s.based_on.is_none()) => {
                    let all: Vec<u32> = FITTING_ATTRS.iter().map(|(id, ..)| *id).collect();
                    fitting_attrs(self, &s.fitting, &all)
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
            self.page_item(&mut x, item, false, &s.transform);
        }
        x.end();
        x.finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_wrap_offsets_by_side() {
        let wrap = TextWrap {
            mode: wrap_mode::BOUNDING_BOX,
            offsets: [1.0, 2.0, 3.0, 4.0],
            flags: 1,
        };
        let mut x = Xml::new();
        Writer::text_wrap_preference(&mut x, Some(&wrap), None);
        let out = x.finish();
        assert!(out.contains("TextWrapMode=\"BoundingBoxTextWrap\""));
        assert!(out.contains("<TextWrapOffset Top=\"2\" Left=\"1\" Bottom=\"4\" Right=\"3\" />"));
    }

    #[test]
    fn measures_guides_from_spread_origin() {
        use crate::model::Page;
        let page = |uid, tx| Page {
            uid,
            bounds: [0.0, 0.0, 612.0, 792.0],
            transform: Matrix([1.0, 0.0, 0.0, 1.0, tx, -396.0]),
            master: None,
            master_transform: Matrix::IDENTITY,
            margins: None,
            columns: None,
            grid: None,
            settings: Default::default(),
        };
        let origin = spread_origin(&[page(1, -612.0), page(2, 0.0)]);
        assert_eq!(origin, (-612.0, -396.0));
        let guide = Guide {
            uid: 0x2299,
            horizontal: true,
            position: 339.5,
            owner: 2,
            fit_to_page: true,
            view_threshold: 0.05,
            color: 6,
            guide_type: Some(0),
            layer: 0xcc,
            locked: false,
            zone: Some(1.0),
            overridden: None,
        };
        let mut x = Xml::new();
        Writer::guide(&mut x, &guide, origin, -1);
        let out = x.finish();
        assert!(
            out.contains("Orientation=\"Horizontal\" Location=\"735.5\""),
            "{out}"
        );
        assert!(
            out.contains("ItemLayer=\"ucc\" GuideType=\"Ruler\""),
            "{out}"
        );
        assert!(out.contains("<GuideColor type=\"enumeration\">Cyan</GuideColor>"));
        assert!(out.contains("PageIndex=\"-1\" GuideZone=\"1\""), "{out}");
        let vertical = Guide {
            horizontal: false,
            position: 28.0,
            ..guide
        };
        let mut x = Xml::new();
        Writer::guide(&mut x, &vertical, origin, 1);
        assert!(x.finish().contains("Location=\"640\""));
    }
}
