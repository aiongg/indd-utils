//! `Resources/Styles.xml`: paragraph, character, object, table and cell
//! styles, their style groups, and the root styles every IDML has.
//!
//! Evidence: `docs/format/objects.md` (styles and style groups),
//! `attributes.md`, `tables.md` and `idml-values.md`.

use super::spread::{FIRST_BASELINE, JUSTIFY, POINTS, SIZING};
use super::*;

/// `KeyboardShortcut` and `ExtendedKeyboardShortcut` of a style, from its
/// stored key (`docs/format/objects.md`, style shortcuts). `None` where the
/// samples do not show the value.
pub(super) fn style_shortcut(s: &crate::model::Style) -> (Option<String>, Option<String>) {
    let Some((key, low, high)) = s.shortcut else {
        return (None, None);
    };
    if key == 0 {
        return (Some("0 0".into()), Some("0 0 0".into()));
    }
    let ch = key & 0xFFFF;
    match key >> 16 {
        0xC000 if (0x30..=0x39).contains(&ch) => {
            let d = ch - 0x30;
            let code = match (high, s.paragraph) {
                (0, _) => Some(82 + d + u32::from(d >= 8)),
                (1, true) => Some(96 + d),
                _ => None,
            };
            let mods = u32::from(low) + 256 * u32::from(high);
            (code.map(|c| format!("{mods} {c}")), Some("0 0 0".into()))
        }
        0x8000 => (Some("0 0".into()), None),
        _ => (None, None),
    }
}

pub(super) fn group_paths(doc: &Document) -> std::collections::HashMap<u32, Vec<String>> {
    fn walk(
        doc: &Document,
        g: &StyleGroup,
        path: &[String],
        out: &mut std::collections::HashMap<u32, Vec<String>>,
    ) {
        for &c in &g.children {
            out.insert(c, path.to_vec());
            if let Some(sub) = doc.style_groups.get(&c) {
                let mut p = path.to_vec();
                p.push(sub.name.clone());
                out.insert(c, p.clone());
                walk(doc, sub, &p, out);
            }
        }
    }
    let mut out = std::collections::HashMap::new();
    for g in doc.style_groups.values().filter(|g| g.root.is_some()) {
        walk(doc, g, &[], &mut out);
    }
    out
}

/// Leaves out the list properties that IDML does not write on the root
/// paragraph style and on `TextDefault`: an empty `AllNestedStyles`, and
/// an empty `TabList` before DOM 8 (attributes.md, root styles and text
/// defaults).
pub(super) fn root_lists(props: &mut Vec<Property>, major: u32) {
    props.retain(|(n, _, v)| {
        let empty = matches!(v, PropValue::List(l) if l.is_empty());
        !(empty && (*n == "AllNestedStyles" || (*n == "TabList" && major < 8)))
    });
}

impl Writer<'_> {
    /// Observed values of a root style (see `values`) that are not in
    /// `written` (attributes) or `props` (Properties children): attributes,
    /// Properties children and other child elements.
    pub(super) fn root_values(
        &self,
        tag: &str,
        written: &[&str],
        props: &[Property],
    ) -> (Vec<(String, String)>, Vec<Node>, Vec<Node>) {
        let node = values::root_style(tag, self.doc.version.major);
        let attrs = node
            .attrs
            .into_iter()
            .filter(|(k, _)| !written.contains(&k.as_str()))
            .collect();
        let mut extra_props = Vec::new();
        let mut children = Vec::new();
        for c in node.children {
            if c.tag == "Properties" {
                extra_props.extend(
                    c.children
                        .into_iter()
                        .filter(|p| !props.iter().any(|(n, _, _)| *n == p.tag)),
                );
            } else {
                children.push(c);
            }
        }
        (attrs, extra_props, children)
    }

    /// A cell or table style other than the root style.
    pub(super) fn table_style_element(
        &self,
        x: &mut Xml,
        tag: &str,
        s: &crate::model::TableStyle,
        styles: &BTreeMap<u32, crate::model::TableStyle>,
    ) {
        x.start(tag)
            .attr("Self", Self::table_style_ref(tag, s))
            .attr("Name", Self::table_style_name(s));
        let attrs = if tag == "CellStyle" {
            self.cell_attrs(&s.attrs)
        } else {
            self.table_attrs(&s.attrs)
        };
        for (k, v) in attrs {
            x.attr(k, v);
        }
        // Values every IDML has on these styles (idml-values.md).
        if let Some(n) = values::element(tag, self.doc.version.major) {
            x.attrs_missing(n.attrs.iter());
        }
        if tag == "CellStyle"
            && let Some(p) = s
                .attrs
                .get(CELL_STYLE_PARAGRAPH_STYLE)
                .and_then(Value::as_u32)
            && self.doc.styles.contains_key(&p)
        {
            x.attr("AppliedParagraphStyle", self.style_ref(Some(p), true));
        }
        if let Some(base) = s.based_on.and_then(|b| styles.get(&b)) {
            // A root style is written as a string.
            let prop = if base.builtin && base.based_on.is_none() {
                ("BasedOn", "string", Self::table_style_name(base).into())
            } else {
                ("BasedOn", "object", Self::table_style_ref(tag, base).into())
            };
            Self::properties(x, &[prop]);
        }
        x.end();
    }

    /// A root cell or table style, from observed values only.
    pub(super) fn root_table_style(&self, x: &mut Xml, tag: &str, name: &str) {
        let (attrs, props, children) = self.root_values(tag, &[], &[]);
        x.start(tag)
            .attr("Self", format!("{tag}/$ID/{name}"))
            .attr("Name", builtin_key(name));
        for (k, v) in &attrs {
            x.attr(k, v);
        }
        Self::properties_with(x, &[], &props);
        for c in &children {
            c.write(x);
        }
        x.end();
    }

    /// `CellStyle/...` or `TableStyle/...` reference of a style.
    pub(super) fn table_style_ref(tag: &str, s: &crate::model::TableStyle) -> String {
        format!("{tag}/{}", self_name(&Self::table_style_name(s)))
    }

    pub(super) fn table_style_name(s: &crate::model::TableStyle) -> String {
        if s.builtin {
            builtin_key(&s.name)
        } else {
            s.name.clone()
        }
    }

    /// The reference for an applied cell style UID; 0 is `[None]`.
    pub(super) fn cell_style_ref(&self, uid: u32) -> Option<String> {
        if uid == 0 {
            return Some("CellStyle/$ID/[None]".into());
        }
        self.doc
            .cell_styles
            .get(&uid)
            .map(|s| Self::table_style_ref("CellStyle", s))
    }

    pub(super) fn styles(&self) -> String {
        let doc = self.doc;
        let mut x = Xml::new();
        self.package_root(&mut x, "Styles");
        let root_of = |kind: u32| doc.style_groups.values().find(|g| g.root == Some(kind));
        for paragraph in [false, true] {
            let (group_tag, tag, sub_tag) = if paragraph {
                (
                    "RootParagraphStyleGroup",
                    "ParagraphStyle",
                    "ParagraphStyleGroup",
                )
            } else {
                (
                    "RootCharacterStyleGroup",
                    "CharacterStyle",
                    "CharacterStyleGroup",
                )
            };
            let root = root_of(if paragraph {
                root_kind::PARAGRAPH
            } else {
                root_kind::CHARACTER
            });
            x.start(group_tag).attr(
                "Self",
                root.map_or(group_tag.to_string(), |g| uref(Some(g.uid))),
            );
            let mut written = std::collections::HashSet::new();
            if let Some(root) = root {
                self.style_group_children(&mut x, root, tag, sub_tag, &mut written);
            }
            // Styles not listed in a group (such as a built-in root
            // style). Some documents have two style objects with the same
            // name, which would give two elements with the same `Self`;
            // the first is written.
            let mut refs: std::collections::HashSet<String> = written
                .iter()
                .map(|&u| self.style_ref(Some(u), paragraph))
                .collect();
            for s in doc.styles.values().filter(|s| s.paragraph == paragraph) {
                if !written.contains(&s.uid) && refs.insert(self.style_ref(Some(s.uid), paragraph))
                {
                    self.style_element(&mut x, s, tag);
                }
            }
            x.end();
        }
        self.toc_styles(&mut x);
        for (tag, kind, style, name) in [
            ("RootCellStyleGroup", root_kind::CELL, "CellStyle", "[None]"),
            (
                "RootTableStyleGroup",
                root_kind::TABLE,
                "TableStyle",
                "[No table style]",
            ),
        ] {
            let root = root_of(kind);
            let id = root.map_or(tag.to_string(), |g| uref(Some(g.uid)));
            x.start(tag).attr("Self", id);
            self.root_table_style(&mut x, style, name);
            let styles = if style == "CellStyle" {
                &doc.cell_styles
            } else {
                &doc.table_styles
            };
            // Styles listed in the root group first, then any others; the
            // root style is written above.
            let mut order: Vec<u32> = root.map_or(Vec::new(), |g| g.children.clone());
            order.extend(styles.keys().copied());
            let mut seen = std::collections::HashSet::new();
            for uid in order {
                let Some(s) = styles.get(&uid) else { continue };
                let is_root = s.builtin && s.based_on.is_none() && s.name == name;
                if is_root || !seen.insert(Self::table_style_ref(style, s)) {
                    continue;
                }
                self.table_style_element(&mut x, style, s, styles);
            }
            x.end();
        }
        let object_root = root_of(root_kind::OBJECT);
        x.start("RootObjectStyleGroup").attr(
            "Self",
            object_root.map_or("RootObjectStyleGroup".to_string(), |g| uref(Some(g.uid))),
        );
        for os in doc.object_styles.values() {
            let name = if os.builtin {
                builtin_key(&os.name)
            } else {
                os.name.clone()
            };
            let root = os.builtin && os.name == "[None]" && os.based_on.is_none();
            let node = self.object_style_node(os, root);
            x.start("ObjectStyle")
                .attr("Self", format!("ObjectStyle/{}", self_name(&name)))
                .attr("Name", &name);
            for (k, v) in &node.attrs {
                x.attr(k, v);
            }
            let mut props: Vec<Node> = node
                .child("Properties")
                .map_or(Vec::new(), |p| p.children.clone());
            if !root && let Some(base) = os.based_on.and_then(|b| doc.object_styles.get(&b)) {
                let base_name = if base.builtin {
                    builtin_key(&base.name)
                } else {
                    base.name.clone()
                };
                // The root "[None]" is written as a string.
                let (ty, text) = if base.builtin && base.name == "[None]" {
                    ("string", base_name)
                } else {
                    ("object", format!("ObjectStyle/{}", self_name(&base_name)))
                };
                props.insert(
                    0,
                    Node {
                        tag: "BasedOn".into(),
                        attrs: vec![("type".into(), ty.into())],
                        text: Some(text),
                        children: Vec::new(),
                    },
                );
            }
            if !props.is_empty() {
                x.start("Properties");
                for p in &props {
                    p.write(&mut x);
                }
                x.end();
            }
            for c in node.children.iter().filter(|c| c.tag != "Properties") {
                c.write(&mut x);
            }
            x.end();
        }
        x.end();
        // The trap presets every IDML has (idml-values.md).
        for n in values::list("TrapPreset", self.doc.version.major) {
            n.write(&mut x);
        }
        x.end();
        x.finish()
    }

    /// An object style as a values node: the values every exported IDML
    /// has on such a style (`idml-values.md`), with the values read from
    /// the INDD in their place. See `docs/format/objects.md`.
    pub(super) fn object_style_node(&self, os: &crate::model::ObjectStyle, root: bool) -> Node {
        let major = self.doc.version.major;
        let version = (major, self.doc.version.minor);
        let mut node = if root {
            values::root_style("ObjectStyle", major)
        } else {
            values::object_style(major)
        };
        // The export options of the version (idml-values.md) in place of
        // those of the value files.
        node.children.retain(|c| c.tag != "ObjectExportOption");
        if let Some(e) = export::object_export_option(self.doc.version, None, true) {
            node.children.insert(0, e);
        }
        let mut attrs = self.item_attr_values(&os.attrs);
        for (id, name) in [
            (0x551E, "GradientFillAngle"),
            (0x5524, "GradientStrokeAngle"),
        ] {
            if let Some(v) = os.attrs.get(id).and_then(Value::as_f64) {
                attrs.push((name, num(v)));
            }
        }
        match os.paragraph_style {
            Some(0) => attrs.push(("AppliedParagraphStyle", "n".into())),
            Some(p) if self.doc.styles.contains_key(&p) => {
                attrs.push(("AppliedParagraphStyle", self.style_ref(Some(p), true)))
            }
            _ => {}
        }
        let mut effects = Vec::new();
        // The root `[None]` has no `Enable…` attributes and no effects
        // category settings in IDML (objects.md, object style settings).
        if let Some(on) = os.enabled.as_ref().filter(|_| !root) {
            // Each attribute is true when its category ID is in the list,
            // from the version on which IDML has it. Where several IDs
            // are present or absent together in every sample, the
            // attribute is written only when they agree.
            // See `docs/format/objects.md`, object style settings.
            type Since = (u32, u32);
            const CATEGORIES: &[(&[u32], Since, &[&str])] = &[
                (&[0x1B933], (7, 0), &["EnableFill"]),
                (&[0x1B934], (7, 0), &["EnableStroke"]),
                (
                    &[0x1B935, 0x1B936],
                    (7, 0),
                    &["EnableStrokeAndCornerOptions"],
                ),
                (&[0x1B93E], (7, 0), &["EnableTextFrameGeneralOptions"]),
                (&[0xADC8], (7, 0), &["EnableTextFrameBaselineOptions"]),
                (&[0xADC9], (8, 0), &["EnableTextFrameAutoSizingOptions"]),
                (&[0x1B940], (7, 0), &["EnableStoryOptions"]),
                (&[0x1B960], (7, 0), &["EnableFrameFittingOptions"]),
                (&[0x1B93F], (7, 0), &["EnableParagraphStyle"]),
                (&[0xADCB], (15, 1), &["EnableTextFrameColumnRuleOptions"]),
                (&[0xCA2F], (7, 0), &["EnableAnchoredObjectOptions"]),
                (
                    &[0x1B942, 0x37C8, 0x37C9],
                    (7, 0),
                    &["EnableTextWrapAndOthers"],
                ),
                (&[0xADCA], (12, 0), &["EnableTextFrameFootnoteOptions"]),
                (
                    &[0x6EA1, 0x6EA2, 0x6EA3, 0x6EA4, 0x6EA5, 0x6EA6, 0x6EA7],
                    (13, 0),
                    &["EnableTransformAttributes"],
                ),
                (
                    &[0x1B97A, 0x1B97C, 0x1B97D, 0x1B97E],
                    (9, 0),
                    &[
                        "EnableExportTagging",
                        "EnableObjectExportAltTextOptions",
                        "EnableObjectExportEpubOptions",
                        "EnableObjectExportTaggedPdfOptions",
                    ],
                ),
            ];
            for (ids, since, names) in CATEGORIES {
                let first = on.contains(&ids[0]);
                if version >= *since && ids.iter().all(|i| on.contains(i) == first) {
                    for name in *names {
                        attrs.push((name, first.to_string()));
                    }
                }
            }
            // The effects categories: transparency of the object, and of
            // its fill, stroke and content together.
            effects.push((
                "ObjectStyleObjectEffectsCategorySettings",
                on.contains(&0x1B937),
            ));
            let parts = [0x1B948, 0x1B950, 0x1B958];
            let first = on.contains(&parts[0]);
            if parts.iter().all(|i| on.contains(i) == first) {
                for tag in [
                    "ObjectStyleFillEffectsCategorySettings",
                    "ObjectStyleStrokeEffectsCategorySettings",
                    "ObjectStyleContentEffectsCategorySettings",
                ] {
                    effects.push((tag, first));
                }
            }
        }
        node.set(&[], attrs);
        for (tag, on) in effects {
            node.set(&[tag], vec![("EnableTransparency", on.to_string())]);
        }
        if let Some(fr) = &os.frame {
            let mut tf = Vec::new();
            if let Some(n) = fr.column_count {
                tf.push(("TextColumnCount", n.to_string()));
            }
            if let Some(v) = fr.column_gutter {
                tf.push(("TextColumnGutter", num(v)));
            }
            if let Some(v) = fr.column_fixed_width {
                tf.push(("TextColumnFixedWidth", num(v)));
            }
            // Fields of the text frame settings that use the frame codes
            // (objects.md, object style settings).
            if let Some(v) = fr.vertical_balance_columns {
                tf.push(("VerticalBalanceColumns", v.to_string()));
            }
            if let Some(v) = fr.use_fixed_width {
                tf.push(("UseFixedColumnWidth", v.to_string()));
            }
            if let Some(v) = fr
                .vertical_justification
                .and_then(|c| JUSTIFY.get(c as usize))
            {
                tf.push(("VerticalJustification", v.to_string()));
            }
            if let Some(v) = fr
                .first_baseline_offset
                .and_then(|c| FIRST_BASELINE.get(c as usize))
            {
                tf.push(("FirstBaselineOffset", v.to_string()));
            }
            if major >= 8
                && let Some((ty, point)) = fr.auto_sizing
                && let (Some(ty), Some(point)) =
                    (SIZING.get(ty as usize), POINTS.get(point as usize))
            {
                tf.push(("AutoSizingType", ty.to_string()));
                tf.push(("AutoSizingReferencePoint", point.to_string()));
            }
            if major >= 8
                && let Some(([use_height, use_width], [height, width])) = fr.minimum_sizes
            {
                tf.push(("UseMinimumHeightForAutoSizing", use_height.to_string()));
                tf.push(("MinimumHeightForAutoSizing", num(height)));
                tf.push(("UseMinimumWidthForAutoSizing", use_width.to_string()));
                tf.push(("MinimumWidthForAutoSizing", num(width)));
            }
            let mut footnote = Vec::new();
            if let Some((span, min, between)) = fr.footnotes
                && span <= 1
            {
                let span = (span == 1).to_string();
                // In `TextFramePreference` from 13.1 (objects.md, object
                // style settings).
                if version >= (13, 1) {
                    tf.push(("FootnotesSpanAcrossColumns", span.clone()));
                    tf.push(("FootnotesMinimumSpacing", num(min)));
                    tf.push(("FootnotesSpaceBetween", num(between)));
                }
                footnote = vec![
                    ("SpanFootnotesAcross", span),
                    ("MinimumSpacingOption", num(min)),
                    ("SpaceBetweenFootnotes", num(between)),
                ];
            }
            if let Some((width, color, tint)) = fr.column_rule {
                let color = match color {
                    0 => Some("n".to_string()),
                    c => self.doc.swatches.get(&c).cloned(),
                };
                tf.push(("ColumnRuleStrokeWidth", num(width)));
                if let Some(c) = color {
                    tf.push(("ColumnRuleStrokeColor", c));
                }
                tf.push(("ColumnRuleStrokeTint", num(tint)));
            }
            node.set(&["TextFramePreference"], tf);
            // Only the top inset is shown apart from the others; the list is
            // written when all four are equal.
            if let Some([a, b, c, e]) = fr.insets
                && a == b
                && b == c
                && c == e
            {
                node.set(&["TextFramePreference", "Properties"], Vec::new());
                let props = node
                    .children
                    .iter_mut()
                    .find(|c| c.tag == "TextFramePreference")
                    .and_then(|t| t.children.iter_mut().find(|c| c.tag == "Properties"))
                    .expect("set above");
                props.children.retain(|c| c.tag != "InsetSpacing");
                props.children.push(Node {
                    tag: "InsetSpacing".into(),
                    attrs: vec![("type".into(), "list".into())],
                    text: None,
                    children: (0..4)
                        .map(|_| Node {
                            tag: "ListItem".into(),
                            attrs: vec![("type".into(), "unit".into())],
                            text: Some(num(a)),
                            children: Vec::new(),
                        })
                        .collect(),
                });
            }
            if !footnote.is_empty() && node.child("TextFrameFootnoteOptionsObject").is_some() {
                node.set(&["TextFrameFootnoteOptionsObject"], footnote);
            }
        }
        // The text frame values every object style has from DOM 12 hold
        // from DOM 8 (idml-values.md, object styles other than the root).
        if (8..12).contains(&major)
            && let Some(t) = values::object_style(12).child("TextFramePreference")
        {
            node.merge(&Node {
                tag: "ObjectStyle".into(),
                children: vec![Node {
                    tag: "TextFramePreference".into(),
                    attrs: t.attrs.clone(),
                    ..Node::default()
                }],
                ..Node::default()
            });
        }
        // Every object style has it from 13.1 (idml-values.md, object
        // styles other than the root).
        if version >= (13, 1) {
            node.merge(&Node {
                tag: "ObjectStyle".into(),
                children: vec![Node {
                    tag: "TextFramePreference".into(),
                    attrs: vec![("FootnotesEnableOverrides".into(), "false".into())],
                    ..Node::default()
                }],
                ..Node::default()
            });
        }
        let mut story = Vec::new();
        if let Some(st) = &os.story {
            match st.frame_type {
                0 => story.push(("FrameType", "Unknown".to_string())),
                1 => story.push(("FrameType", "TextFrameType".to_string())),
                2 => story.push(("FrameType", "FrameGridType".to_string())),
                _ => {}
            }
            match st.orientation {
                0 => story.push(("StoryOrientation", "Unknown".to_string())),
                1 => story.push(("StoryOrientation", "Horizontal".to_string())),
                _ => {}
            }
        }
        match os.direction {
            Some(1) => story.push(("StoryDirection", "LeftToRightDirection".into())),
            Some(0) | None => story.push(("StoryDirection", "UnknownDirection".into())),
            _ => {}
        }
        node.set(&["StoryPreference"], story);
        if let Some(mode) = text_wrap_mode(os.text_wrap.as_ref()) {
            node.set(&["TextWrapPreference"], vec![("TextWrapMode", mode.into())]);
            node.set(
                &["TextWrapPreference", "Properties", "TextWrapOffset"],
                text_wrap_offsets(os.text_wrap.as_ref()),
            );
            if os.contour_type == Some(5) {
                node.set(
                    &["TextWrapPreference", "ContourOption"],
                    vec![("ContourType", "SameAsClipping".into())],
                );
            }
        }
        let all: Vec<u32> = FITTING_ATTRS.iter().map(|(id, ..)| *id).collect();
        node.set(
            &["FrameFittingOption"],
            fitting_attrs(self, &os.fitting, &all),
        );
        if let Some(d) = &os.anchor {
            node.set(&["AnchoredObjectSetting"], anchored_settings(d));
        }
        node
    }

    pub(super) fn style_group_children(
        &self,
        x: &mut Xml,
        g: &StyleGroup,
        tag: &str,
        sub_tag: &str,
        written: &mut std::collections::HashSet<u32>,
    ) {
        for &c in &g.children {
            if let Some(sub) = self.doc.style_groups.get(&c) {
                let path = self.group_path.get(&sub.uid).cloned().unwrap_or_default();
                x.start(sub_tag)
                    .attr(
                        "Self",
                        format!("{sub_tag}/$ID/{}", self_name(&path.join(":"))),
                    )
                    .attr("Name", builtin_key(&sub.name));
                self.style_group_children(x, sub, tag, sub_tag, written);
                x.end();
            } else if let Some(s) = self.doc.styles.get(&c) {
                self.style_element(x, s, tag);
                written.insert(c);
            }
        }
    }

    /// Whether the list attribute `id` (nested, line or GREP styles) of a
    /// paragraph style has items: the first style in its based-on chain that
    /// has the attribute decides. An empty list is stored as a 0 count.
    pub(super) fn inherited_list(&self, s: &Style, id: u32) -> bool {
        let mut cur = Some(s);
        for _ in 0..32 {
            let Some(st) = cur else { break };
            if let Some(v) = st.attrs.get(id) {
                return matches!(v, Value::StyleList { count, .. } if *count != 0);
            }
            cur = st.based_on.and_then(|b| self.doc.styles.get(&b));
        }
        false
    }

    pub(super) fn style_element(&self, x: &mut Xml, s: &Style, tag: &str) {
        let doc = self.doc;
        let paragraph = s.paragraph;
        let name = style_name(s);
        let (mut plain, mut props) = self.text_attrs(&s.attrs);
        if paragraph {
            // The schema allows KerningValue on character styles only.
            plain.retain(|(k, _)| *k != "KerningValue");
        }
        let is_root = s.builtin && s.based_on.is_none() && s.name.starts_with("[No ");
        if is_root {
            // The root paragraph style has no `NextStyle` and no
            // `AllNestedStyles`, and an empty `TabList` only from DOM 8
            // (attributes.md, root styles and text defaults).
            props.retain(|(n, ..)| *n != "AllNestedStyles");
            root_lists(&mut props, doc.version.major);
        }
        x.start(tag)
            .attr("Self", self.style_ref(Some(s.uid), paragraph))
            .attr("Name", &name);
        if paragraph && !is_root {
            x.attr("NextStyle", self.style_ref(s.next.or(Some(s.uid)), true));
        }
        for (k, v) in &plain {
            x.attr(k, v);
        }
        x.attr("Imported", s.imported.to_string());
        if let Some(id) = &s.unique_id {
            x.attr("StyleUniqueId", id);
        }
        let v = doc.version;
        if paragraph && (v.major >= 10 || (v.major == 8 && v.minor >= 1)) {
            for (id, name) in [
                (0x1B75, "EmptyNestedStyles"),
                (0x1BBB, "EmptyLineStyles"),
                (0x1BBA, "EmptyGrepStyles"),
            ] {
                x.attr(name, (!self.inherited_list(s, id)).to_string());
            }
        }
        // Root styles also get the values every exported IDML has on them;
        // values read from the INDD take precedence.
        let mut extra = Vec::new();
        if is_root {
            let written: Vec<&str> = plain.iter().map(|(k, _)| *k).collect();
            let (attrs, more, _) = self.root_values(tag, &written, &props);
            for (k, v) in &attrs {
                if !x.has_attr(k) {
                    x.attr(k, v);
                }
            }
            extra = more;
        } else if let Some(n) = values::element(tag, doc.version.major) {
            let (short, extended) = style_shortcut(s);
            if let Some(k) = short {
                x.attr("KeyboardShortcut", k);
            }
            if v.major >= 15
                && let Some(e) = extended
            {
                x.attr("ExtendedKeyboardShortcut", e);
            }
            // Other styles get the values every IDML has on them.
            x.attrs_missing(n.attrs.iter());
            x.attrs_missing(values::when_written(tag, doc.version.major).iter());
            if let Some(p) = n.child("Properties") {
                extra = p.children.clone();
            }
        }
        if let Some(base) = s.based_on.and_then(|b| doc.styles.get(&b)) {
            // The root "[No ... style]" is written as a string.
            let root = base.builtin && base.name.starts_with("[No ");
            if root {
                props.insert(0, ("BasedOn", "string", style_name(base).into()));
            } else {
                props.insert(
                    0,
                    (
                        "BasedOn",
                        "object",
                        self.style_ref(Some(base.uid), paragraph).into(),
                    ),
                );
            }
        }
        Self::properties_with(x, &props, &extra);
        x.end();
    }
}
