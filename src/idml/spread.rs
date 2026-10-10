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

/// The bounding box of a spread's pages (min x, min y, max x, max y), from
/// the corners of each page through its transform.
fn pages_extent(pages: &[Page]) -> Option<[f64; 4]> {
    pages
        .iter()
        .flat_map(|p| {
            let [a, b, c, d, tx, ty] = p.transform.0;
            let [l, t, r, bo] = p.bounds;
            [(l, t), (r, t), (l, bo), (r, bo)]
                .map(|(x, y)| (a * x + c * y + tx, b * x + d * y + ty))
        })
        .fold(None, |acc: Option<[f64; 4]>, (x, y)| {
            Some(match acc {
                None => [x, y, x, y],
                Some([x0, y0, x1, y1]) => [x0.min(x), y0.min(y), x1.max(x), y1.max(y)],
            })
        })
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
    "StrokeType",
    "StrokeAlignment",
    "StrokeTint",
    "GapColor",
    "GapTint",
    "EndCap",
    "EndJoin",
    "LeftLineEnd",
    "RightLineEnd",
    "ArrowHeadAlignment",
];

/// Whether an object style whose category list is `on` turns off the
/// category of a fill, stroke or corner attribute: `EnableFill` (0x1B933)
/// for the fill, `EnableStroke` (0x1B934) for the stroke colour,
/// `EnableStrokeAndCornerOptions` (0x1B935 and 0x1B936) for the other
/// stroke and corner attributes, both for the stroke type, tint and gap
/// (attributes.md, values an item does not store).
fn category_off(name: &str, on: &[u32]) -> bool {
    let off = |id: u32| !on.contains(&id);
    let corners = off(0x1B935) && off(0x1B936);
    match name {
        "FillColor" | "FillTint" => off(0x1B933),
        "StrokeColor" => off(0x1B934),
        "StrokeType" | "StrokeTint" | "GapColor" | "GapTint" => off(0x1B934) && corners,
        _ => corners,
    }
}

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
/// `PageTransitionType` codes of spreads (objects.md, spreads).
const TRANSITION_TYPES: [(u32, &str); 6] = [
    (0, "None"),
    (0x205C8, "BlindsTransition"),
    (0x205CA, "CombTransition"),
    (0x205CC, "DissolveTransition"),
    (0x205CF, "PushTransition"),
    (0x205D0, "SplitTransition"),
];
/// `PageTransitionDirection` codes of spreads.
const TRANSITION_DIRECTIONS: [(u32, &str); 4] = [
    (0, "NotApplicable"),
    (1, "Down"),
    (0xB, "Horizontal"),
    (0xD, "HorizontalIn"),
];
/// `BaselineFrameGridRelativeOption` codes (objects.md, baseline frame
/// grid of text frames).
pub(super) const BASELINE_RELATIVE: [Option<&str>; 4] = [
    Some("TopOfPage"),
    Some("TopOfMargin"),
    Some("TopOfFrame"),
    Some("TopOfInset"),
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
    /// A `TextPath` (`docs/format/objects.md`, text on a path).
    fn text_path(&self, x: &mut Xml, t: &crate::model::TextPath) {
        x.start("TextPath")
            .attr("Self", uref(Some(t.uid)))
            .attr("ParentStory", uref(t.story));
        if t.default_codes {
            x.attr("PathAlignment", "CenterPathAlignment")
                .attr("TextAlignment", "BaselineTextAlignment")
                .attr("PathEffect", "RainbowPathEffect");
        }
        x.attr(
            "FlipPathEffect",
            if t.flipped { "Flipped" } else { "NotFlipped" },
        );
        if t.default_codes {
            x.attr("PathSpacing", "0");
        }
        x.attr("StartBracket", num(t.start))
            .attr("EndBracket", num(t.end))
            .attr("PreviousTextFrame", uref(t.previous))
            .attr("NextTextFrame", uref(t.next));
        x.end();
    }

    /// The `Properties` of EPS text: `PathBoundingBox`, `EPSTextData` and
    /// `EPSTextAttributeBounds` (`docs/format/objects.md`, EPS text).
    fn eps_text_properties(x: &mut Xml, e: &crate::model::EpsText) {
        let [left, top, right, bottom] = e.path_bounds;
        x.start("Properties");
        x.empty(
            "PathBoundingBox",
            &[
                ("Left", num(left)),
                ("Top", num(top)),
                ("Right", num(right)),
                ("Bottom", num(bottom)),
            ],
        );
        x.start("EPSTextData")
            .cdata(&base64_lines(&e.data), CDATA_SECTION)
            .end();
        let [left, top, right, bottom] = e.attr_bounds;
        x.empty(
            "EPSTextAttributeBounds",
            &[
                ("Top", num(top)),
                ("Left", num(left)),
                ("Bottom", num(bottom)),
                ("Right", num(right)),
            ],
        );
        x.end();
    }

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
        if let Some(v) = p.auto_sizing_type.and_then(|c| SIZING.get(c as usize)) {
            attrs.push(("AutoSizingType", v.to_string()));
        }
        if let Some(v) = p
            .auto_sizing_reference_point
            .and_then(|c| POINTS.get(c as usize))
        {
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
        let footnotes = self.saved_by((13, 1));
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
            attrs.push(("FootnotesSpanAcrossColumns", p.footnote_span.to_string()));
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
        // A category is off when the applied style's list lacks its ID, or
        // the frame has no style (objects.md, text frame preferences).
        let off =
            |id: u32| style.is_none_or(|s| !s.enabled.as_ref().is_some_and(|on| on.contains(&id)));
        let footnote_options = self.frame_footnote_options(p, style, off(0xADCA));
        let baseline = self.frame_baseline_grid(p, style, off(0xADC8));
        // Before DOM 11 IDML gives a frame without values no element.
        if !attrs.is_empty() || major >= 11 {
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
        if let Some(n) = baseline {
            n.write(x);
        }
        if let Some(o) = footnote_options {
            x.empty("TextFrameFootnoteOptionsObject", &o);
        }
    }

    /// `TextFrameFootnoteOptionsObject` of a text frame: from DOM 12, when
    /// the footnote category is off or the frame's footnote values differ
    /// from its style's (footnotes.md, text frame footnote options).
    fn frame_footnote_options(
        &self,
        p: &TextFramePreferences,
        style: Option<&applied::StyleValues>,
        off: bool,
    ) -> Option<[(&'static str, String); 4]> {
        if self.doc.version.major < 12 {
            return None;
        }
        // Without chunk 0x22608 the values are false, 12 and 6.
        let [min, between] = p.footnotes.unwrap_or([12.0, 6.0]);
        let (span, smin, sbetween) = style.and_then(|s| s.footnote).unwrap_or((false, 12.0, 6.0));
        let differ = p.footnote_span != span
            || !applied::same_value(&num(min), &num(smin))
            || !applied::same_value(&num(between), &num(sbetween));
        (off || differ).then(|| {
            [
                ("EnableOverrides", "false".to_string()),
                ("SpanFootnotesAcross", p.footnote_span.to_string()),
                ("MinimumSpacingOption", num(min)),
                ("SpaceBetweenFootnotes", num(between)),
            ]
        })
    }

    /// `BaselineFrameGridOption` of a text frame: every value when the
    /// baseline category is off, else the values that differ from the
    /// style's; `None` when there is nothing to write (objects.md,
    /// baseline frame grid of text frames).
    fn frame_baseline_grid(
        &self,
        p: &TextFramePreferences,
        style: Option<&applied::StyleValues>,
        all: bool,
    ) -> Option<Node> {
        use crate::model::BaselineGrid;
        let g = p.baseline_grid.unwrap_or(BaselineGrid::DEFAULT);
        let sg = style
            .and_then(|s| s.baseline)
            .unwrap_or(BaselineGrid::DEFAULT);
        let mut n = Node {
            tag: "BaselineFrameGridOption".into(),
            ..Node::default()
        };
        let mut push = |differs: bool, k: &str, v: String| {
            if all || differs {
                n.attrs.push((k.into(), v));
            }
        };
        push(
            g.use_custom != sg.use_custom,
            "UseCustomBaselineFrameGrid",
            g.use_custom.to_string(),
        );
        push(
            !applied::same_value(&num(g.start), &num(sg.start)),
            "StartingOffsetForBaselineFrameGrid",
            num(g.start),
        );
        if let Some(r) = BASELINE_RELATIVE
            .get(g.relative as usize)
            .copied()
            .flatten()
        {
            push(
                g.relative != sg.relative,
                "BaselineFrameGridRelativeOption",
                r.to_string(),
            );
        }
        push(
            !applied::same_value(&num(g.increment), &num(sg.increment)),
            "BaselineFrameGridIncrement",
            num(g.increment),
        );
        if all || g.color != sg.color {
            let color = if g.color == 0 {
                self.doc
                    .prefs
                    .baseline_frame_grid_color
                    .and_then(frame_grid_color)
            } else {
                p.baseline_rgb
                    .and_then(|rgb| ui_color_property("BaselineFrameGridColor", rgb))
            };
            if let Some(c) = color {
                set_property(&mut n, c);
            }
        }
        (!n.attrs.is_empty() || !n.children.is_empty()).then_some(n)
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
        if let Some(a) = p.allow_overrides {
            x.attr("AllowOverrides", a.to_string());
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
            ItemKind::EpsText(_) => "EPSText",
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
        // A category that the item's object style (not the root) turns
        // off: IDML writes the item's values of that category even where
        // they equal the style's (attributes.md, values an item does not
        // store).
        let on = match (&applied, item.object_style) {
            (Some(a), Some(u))
                if !self
                    .doc
                    .object_styles
                    .get(&u)
                    .is_some_and(Self::is_root_object_style) =>
            {
                a.enabled.as_deref()
            }
            _ => None,
        };
        let forced = |name: &str| on.is_some_and(|on| category_off(name, on));
        // A stored tint of 100 is written as −1 but differs from a style
        // value of −1 (attributes.md, strokes).
        let full_tint = |name: &str| {
            let id = match name {
                "StrokeTint" => 0x6E66,
                "GapTint" => 0x6E8A,
                _ => return false,
            };
            item.attrs.get(id).and_then(Value::as_f64) == Some(100.0)
        };
        for (name, v) in self.item_attr_values(&item.attrs) {
            // Fill, stroke and corner values equal to the object style's
            // are left out (attributes.md, page item attributes).
            let same = STYLE_COMPARED.contains(&name)
                && !forced(name)
                && !full_tint(name)
                && applied
                    .as_ref()
                    .and_then(|s| s.item(name))
                    .is_some_and(|s| applied::same_value(s, &v));
            if !same {
                x.attr(name, v);
            }
        }
        // A fill, stroke or corner attribute that the item does not store
        // has the value of the document's base list; IDML writes it where
        // it differs from the object style's, and always for an item
        // without an object style (attributes.md, values an item does not
        // store). Groups vary.
        if tag != "Group"
            && (applied.is_some() || item.object_style.is_none())
            && let Some(base) = &self.doc.prefs.item_base
        {
            let mut rest = base.clone();
            rest.values.retain(|(id, _)| item.attrs.get(*id).is_none());
            for (name, v) in self.item_attr_values(&rest) {
                // IDML writes no corner attributes for EPS text.
                if tag == "EPSText" && name.contains("Corner") {
                    continue;
                }
                let write = match &applied {
                    Some(a) => a
                        .item(name)
                        .is_some_and(|s| forced(name) || !applied::same_value(s, &v)),
                    None => true,
                };
                if STYLE_COMPARED.contains(&name) && write {
                    x.attr(name, v);
                }
            }
        }
        // An item without an object style refers to none, `n`
        // (attributes.md, values an item does not store).
        let style_ref = item.object_style.and_then(|u| self.object_style_ref(u));
        x.attr(
            "AppliedObjectStyle",
            style_ref.unwrap_or_else(|| "n".into()),
        );
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
        // A frame grid (objects.md, frame grids).
        if let ItemKind::TextFrame {
            preferences: Some(p),
            ..
        } = &item.kind
            && p.frame_grid
        {
            let count = self
                .doc
                .prefs
                .grids
                .iter()
                .find(|(tag, ..)| *tag == "StoryGridDataInformation")
                .and_then(|(_, _, c)| *c);
            if let Some(c) = count {
                x.empty(
                    "GridDataInformation",
                    &[
                        ("GridView", "GridViewEnum".into()),
                        ("CharacterCountLocation", "BottomAlign".into()),
                        ("CharacterCountSize", num(c)),
                    ],
                );
            }
        }
        for t in &item.text_paths {
            self.text_path(x, t);
        }
        if let ItemKind::EpsText(e) = &item.kind {
            Self::eps_text_properties(x, e);
        }
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
        // EPS text has no export options (objects.md, EPS text).
        if !matches!(item.kind, ItemKind::EpsText(_))
            && let Some(n) =
                export::object_export_option(self.doc.version, item.export.as_ref(), false)
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
            // The values every IDML has (idml-values.md, page items).
            let observed = self.observed(&format!("{tag}/InCopyExportOption"));
            if !observed.is_empty() {
                x.start("InCopyExportOption");
                x.attrs_missing(observed.iter());
                x.end();
            }
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

    /// `FrameFittingOption` of a rectangle, oval or polygon. When the
    /// object style, the root `[None]` included, turns the fitting
    /// category off, IDML writes all seven values: the item's where it
    /// stores them, else the style's. Otherwise it writes the item's values
    /// that differ from the style's; a style other than the root without a
    /// category list (older files) gives all its values when none differ.
    /// See `docs/format/objects.md`.
    pub(super) fn frame_fitting(&self, x: &mut Xml, item: &PageItem) {
        let style = item
            .object_style
            .and_then(|u| self.doc.object_styles.get(&u));
        let all: Vec<u32> = FITTING_ATTRS.iter().map(|(id, ..)| *id).collect();
        let off = style
            .and_then(|s| s.enabled.as_ref())
            .is_some_and(|on| !on.contains(&0x1B960));
        let attrs = if let Some(s) = style.filter(|_| off) {
            let mut merged = s.fitting.clone();
            merged
                .values
                .retain(|(id, _)| item.attrs.get(*id).is_none());
            merged.values.extend(
                item.attrs
                    .values
                    .iter()
                    .filter(|(id, _)| all.contains(id))
                    .cloned(),
            );
            fitting_attrs(self, &merged, &all)
        } else {
            let differ: Vec<u32> = all
                .iter()
                .copied()
                .filter(|&id| {
                    item.attrs
                        .get(id)
                        .is_some_and(|v| style.and_then(|s| s.fitting.get(id)) != Some(v))
                })
                .collect();
            if !differ.is_empty() {
                fitting_attrs(self, &item.attrs, &differ)
            } else {
                match style {
                    Some(s) if s.enabled.is_none() && !Self::is_root_object_style(s) => {
                        fitting_attrs(self, &s.fitting, &all)
                    }
                    _ => Vec::new(),
                }
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
        // Codes 0 and 4 only; the others are in no pair (objects.md, page
        // settings).
        match st.grid_start {
            Some(0) => {
                x.attr("GridStartingPoint", "TopOutside");
            }
            Some(4) => {
                x.attr("GridStartingPoint", "CenterVertical");
            }
            _ => {}
        }
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
                // IDML writes the prefix again in place of this style
                // (objects.md, master spread names and sections).
                if s.style == crate::model::numbering::SINGLE_LEADING_ZEROS {
                    item("string", s.prefix.clone())
                } else {
                    item("enumeration", style.into())
                },
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
            let mut attrs = vec![
                ("ColumnCount", (c.positions.len() / 2).to_string()),
                ("ColumnGutter", num(c.gutter)),
                ("Top", num(m.top)),
                ("Bottom", num(m.bottom)),
                ("Left", num(m.left)),
                ("Right", num(m.right)),
            ];
            // The last u16 of the columns chunk (objects.md, page settings).
            match c.direction {
                Some(0) => attrs.push(("ColumnDirection", "Horizontal".into())),
                Some(1) => attrs.push(("ColumnDirection", "Vertical".into())),
                _ => {}
            }
            attrs.push(("ColumnsPositions", nums(&c.positions)));
            x.empty("MarginPreference", &attrs);
        }
        if let Some(g) = &p.grid {
            self.grid_data(x, g);
        }
    }

    /// `GridDataInformation` of layout grid settings (chunk 0xCD02).
    pub(super) fn grid_data(&self, x: &mut Xml, g: &crate::model::GridData) {
        x.start("GridDataInformation")
            .attr("FontStyle", &g.font_style);
        // `PointSize`, `CharacterAki` and `LineAki` are located; the
        // scales are written only when they have the value of every sample
        // (objects.md, layout grid).
        let [size, character_aki, line_aki, h_scale, v_scale] = g.numbers;
        for (name, value, observed, scale) in [
            ("PointSize", size, None, 1.0),
            ("CharacterAki", character_aki, None, 1.0),
            ("LineAki", line_aki, None, 1.0),
            ("HorizontalScale", h_scale, Some(1.0), 100.0),
            ("VerticalScale", v_scale, Some(1.0), 100.0),
        ] {
            if observed.is_none_or(|o| value == o) {
                x.attr(name, num(value * scale));
            }
        }
        if g.codes == [3, 0, 3, 1] {
            x.attr("LineAlignment", "LeftOrTopLineJustify")
                .attr("GridAlignment", "AlignEmCenter")
                .attr("CharacterAlignment", "AlignEmCenter");
        }
        if let Some(f) = self.doc.fonts.get(&g.font) {
            Self::properties(
                x,
                &[("AppliedFont", "string", self.family_names(f).0.into())],
            );
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
        let major = self.doc.version.major;
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
            // Page transitions (objects.md, spreads); without the chunk,
            // none.
            let (ty, dir) = s.transition.unwrap_or((0, 0));
            if let Some(t) = TRANSITION_TYPES.iter().find(|(c, _)| *c == ty) {
                x.attr("PageTransitionType", t.1);
            }
            if let Some(d) = TRANSITION_DIRECTIONS.iter().find(|(c, _)| *c == dir) {
                x.attr("PageTransitionDirection", d.1);
            }
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
        if master
            && major >= 8
            && let Some(f) = primary_text_frame(s)
        {
            x.attr("PrimaryTextFrame", f);
        }
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
        if !master && let Some(mut fp) = values::element("Spread/FlattenerPreference", major) {
            // Without the flattener chunk, IDML has 400 and 400 after a
            // session of a Japanese or Chinese edition, otherwise 300 and
            // 150 (`objects.md`, flattener settings).
            let res = s.flattener_resolution.or(match self.japanese_session() {
                Some(true) => Some([400.0, 400.0]),
                Some(false) => Some([300.0, 150.0]),
                None => None,
            });
            if let Some([line_art, gradient]) = res {
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
            let extent = pages_extent(&s.pages);
            for g in &s.guides {
                let own_page = s.pages.iter().any(|q| q.uid == g.owner);
                // IDML leaves out a guide of the spread that lies outside
                // the spread's pages (objects.md, guides).
                let outside = !own_page
                    && extent.is_some_and(|[x0, y0, x1, y1]| {
                        let (lo, hi) = if g.horizontal { (y0, y1) } else { (x0, x1) };
                        g.position < lo - 1e-6 || g.position > hi + 1e-6
                    });
                if outside {
                    continue;
                }
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

/// `PrimaryTextFrame` of a master spread: `n` without a primary story,
/// otherwise the spread's first text frame of that story; `None` if the
/// spread has no such frame (objects.md, primary text frame).
fn primary_text_frame(s: &Spread) -> Option<String> {
    fn find(items: &[PageItem], story: u32) -> Option<u32> {
        items.iter().find_map(|i| match i.kind {
            ItemKind::TextFrame {
                story: Some(st),
                previous: None,
                ..
            } if st == story => Some(i.uid),
            _ => find(&i.children, story),
        })
    }
    match s.primary_story {
        None | Some(0) => Some("n".into()),
        Some(story) => find(&s.items, story).map(|u| uref(Some(u))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stroke_categories_turn_off_their_own_attributes() {
        // Only EnableStrokeAndCornerOptions off.
        let on = [0x1B933, 0x1B934];
        assert!(category_off("StrokeWeight", &on));
        assert!(category_off("TopLeftCornerRadius", &on));
        assert!(!category_off("StrokeColor", &on));
        assert!(!category_off("StrokeType", &on));
        assert!(!category_off("FillColor", &on));
        // Fill and both stroke categories off.
        assert!(category_off("StrokeType", &[]));
        assert!(category_off("FillTint", &[]));
    }

    #[test]
    fn primary_text_frame_needs_a_first_frame_of_the_story() {
        let mut s = Spread {
            uid: 1,
            master_name: None,
            transform: Matrix::IDENTITY,
            binding_location: 0,
            pages: Vec::new(),
            items: Vec::new(),
            guides: Vec::new(),
            shuffle: None,
            flattener_resolution: None,
            show_master_items: None,
            primary_story: None,
            transition: None,
        };
        assert_eq!(primary_text_frame(&s).as_deref(), Some("n"));
        s.primary_story = Some(0);
        assert_eq!(primary_text_frame(&s).as_deref(), Some("n"));
        // A story without a frame on the spread.
        s.primary_story = Some(5);
        assert_eq!(primary_text_frame(&s), None);
    }

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
