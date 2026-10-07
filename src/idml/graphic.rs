//! Placed graphics (images, PDF, EPS, SVG) with their settings, clipping
//! path, import options, layers and links.
//!
//! Evidence: `docs/format/objects.md` (placed graphics, links, clipping
//! path settings, graphic layers) and `idml-values.md`.

use super::*;
use crate::model::{GraphicLayer, Name};

/// `Space` of an image by its colour space code (and whether it has a
/// colour table).
fn color_space(code: u32, indexed: bool) -> Option<&'static str> {
    match (code, indexed) {
        (1, _) => Some("$ID/#Links_Grayscale"),
        (2, false) => Some("$ID/#Links_RGB"),
        (2, true) => Some("$ID/#Links_Indexed RGB"),
        (4, _) => Some("$ID/#Links_CMYK"),
        _ => None,
    }
}

/// `Properties/Profile` of an image by the code of chunk 0x7C0F.
fn profile(code: Option<u32>) -> Option<&'static str> {
    match code {
        None | Some(0) => Some("$ID/None"),
        Some(1) => Some("$ID/Use Document Default"),
        Some(3) => Some("$ID/Embedded"),
        _ => None,
    }
}

/// A vector colour policy code of chunk 0x7C42.
fn vector_policy(code: u32) -> Option<&'static str> {
    match code {
        1 => Some("IgnoreAll"),
        3 => Some("HonorAllProfiles"),
        _ => None,
    }
}

/// `PDFCrop` by the crop code of chunk 0x251B.
fn pdf_crop(code: u32) -> Option<&'static str> {
    match code {
        0 => Some("CropContentVisibleLayers"),
        2 => Some("CropArt"),
        3 => Some("CropPDF"),
        4 => Some("CropTrim"),
        6 => Some("CropMedia"),
        7 => Some("CropContentAllLayers"),
        _ => None,
    }
}

/// A value rounded half up, as IDML rounds resolutions.
fn round_half_up(v: f64) -> f64 {
    (v + 0.5).floor()
}

/// The horizontal and vertical scale of a transform: √(a² + b²), and
/// |ad − bc| divided by the horizontal scale.
fn scales(m: &Matrix) -> (f64, f64) {
    let [a, b, c, d, ..] = m.0;
    let h = a.hypot(b);
    (h, (a * d - b * c).abs() / h)
}

impl Writer<'_> {
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
    }

    /// `ImageIOPreference` of an image, from chunk 0x1714; without the
    /// chunk, the values every image without it has.
    fn image_io(x: &mut Xml, g: &Graphic) {
        let (clipping, alpha) = match &g.import {
            Some((clipping, alpha)) => (*clipping, alpha.idml()),
            None => (true, "$ID/".to_string()),
        };
        x.empty(
            "ImageIOPreference",
            &[
                ("ApplyPhotoshopClippingPath", clipping.to_string()),
                ("AllowAutoEmbedding", "true".into()),
                ("AlphaChannelName", alpha),
            ],
        );
    }

    /// A `Link` element. Values without an INDD field are those of every
    /// link in the corpus IDML files (`idml-values.md`, links). See
    /// `docs/format/objects.md`, links.
    pub(super) fn link(&self, x: &mut Xml, link: &Link) {
        let major = self.doc.version.major;
        x.start("Link")
            .attr("Self", uref(Some(link.uid)))
            .attr("AssetURL", "$ID/")
            .attr("AssetID", "$ID/")
            .attr("LinkResourceURI", link.uri.clone());
        if let Some(f) = &link.format {
            x.attr("LinkResourceFormat", builtin_key(f));
        }
        x.attr(
            "StoredState",
            if link.embedded { "Embedded" } else { "Normal" },
        )
        .attr("LinkClassID", "35906")
        .attr("LinkClientID", "257");
        if let Some(m) = link.modified {
            x.attr("LinkResourceModified", m.to_string());
        }
        x.attr("LinkObjectModified", "false");
        if let Some(s) = link.shown {
            x.attr("ShowInUI", s.to_string());
        }
        x.attr("CanEmbed", "true")
            .attr("CanUnembed", "true")
            .attr("CanPackage", "true")
            .attr("ImportPolicy", "NoAutoImport")
            .attr("ExportPolicy", "NoAutoExport");
        if let Some(stamp) = &link.stamp {
            x.attr("LinkImportStamp", stamp.clone());
        }
        if let Some([modified, imported]) = link.times {
            if link.stamp.is_some() {
                x.attr(
                    "LinkImportModificationTime",
                    link_time(modified, &self.doc.xmp_dates),
                );
            }
            x.attr("LinkImportTime", link_time(imported, &self.doc.xmp_dates));
        }
        if let Some([high, low]) = link.size {
            x.attr("LinkResourceSize", format!("{high:x}~{low:x}"));
        }
        if major >= 13 {
            x.attr("RenditionData", "Actual");
        }
        if major >= 21 {
            x.attr(
                "PDFIdentifier",
                link.pdf_identifier.unwrap_or(0).to_string(),
            );
        }
        x.end();
    }

    /// The vector colour policies of a PDF or EPS: from chunk 0x7C42, or
    /// without it from the document's colour policies (`objects.md`,
    /// vector colour policies).
    fn vector_policies(&self, g: &Graphic) -> [Option<&'static str>; 2] {
        let document = |name: &str| {
            self.doc
                .prefs
                .values
                .iter()
                .find(|v| v.element == "Document" && v.name == name)
                .map(|v| v.value.as_str())
        };
        let rgb = match document("RGBPolicy") {
            Some("ColorPolicyOff") => Some("IgnoreAll"),
            Some(_) => Some("HonorAllProfiles"),
            None => None,
        };
        let cmyk = match document("CMYKPolicy") {
            Some("ColorPolicyOff" | "CombinationOfPreserveAndSafeCmyk") => Some("IgnoreAll"),
            Some("PreserveEmbeddedProfiles" | "ConvertToWorkingSpace") => Some("HonorAllProfiles"),
            _ => None,
        };
        match g.vector_policies {
            Some([_, r, _, c]) => [vector_policy(r).or(rgb), vector_policy(c).or(cmyk)],
            None => [rgb, cmyk],
        }
    }

    /// `outer` is the transform from the graphic's parent to the spread.
    pub(super) fn placed_graphic(&self, x: &mut Xml, g: &Graphic, outer: &Matrix) {
        let major = self.doc.version.major;
        let tag = match g.kind {
            GraphicKind::Image => "Image",
            GraphicKind::Pdf => "PDF",
            GraphicKind::Eps => "EPS",
            GraphicKind::Svg => "SVG",
        };
        let [left, top, right, bottom] = g.bounds;
        x.start(tag).attr("Self", uref(Some(g.uid)));
        match g.kind {
            GraphicKind::Image => {
                if g.link.is_some()
                    && let Some(p) = &g.image
                {
                    if let Some(space) = p.color_space.and_then(|c| color_space(c, p.indexed)) {
                        x.attr("Space", space);
                    }
                    if let Some([h, v]) = p.resolution {
                        let actual = [round_half_up(h), round_half_up(v)];
                        x.attr("ActualPpi", nums(&actual));
                        let (sh, sv) = scales(&g.transform.then(outer));
                        if sh > 0.0 && sv > 0.0 {
                            let effective =
                                [round_half_up(actual[0] / sh), round_half_up(actual[1] / sv)];
                            if effective.iter().all(|e| e.is_finite()) {
                                x.attr("EffectivePpi", nums(&effective));
                            }
                        }
                    }
                }
                x.attr("ImageRenderingIntent", "UseColorSettings");
            }
            GraphicKind::Pdf | GraphicKind::Eps => {
                x.attr("GrayVectorPolicy", "IgnoreAll");
                let [rgb, cmyk] = self.vector_policies(g);
                if let Some(v) = rgb {
                    x.attr("RGBVectorPolicy", v);
                }
                if let Some(v) = cmyk {
                    x.attr("CMYKVectorPolicy", v);
                }
            }
            GraphicKind::Svg => {
                x.attr("UseSVGAs", "EmbedCode");
            }
        }
        x.attr("LocalDisplaySetting", "Default");
        if let Some(f) = g.link.as_ref().and_then(|l| l.format.as_ref()) {
            x.attr("ImageTypeName", builtin_key(f));
        }
        if let Some(style) = g.object_style.and_then(|u| self.object_style_ref(u)) {
            x.attr("AppliedObjectStyle", style);
        }
        x.attr("ItemTransform", matrix(&g.transform));
        self.settings(x, &g.props, None);
        if g.kind == GraphicKind::Image {
            for (name, v) in self.item_attr_values(&g.attrs) {
                if name == "FillColor" {
                    x.attr(name, v);
                }
            }
            for (name, v) in Self::gradient_values(&g.attrs) {
                if name.starts_with("GradientFill") {
                    x.attr(name, v);
                }
            }
        }
        if major >= 21 && g.kind != GraphicKind::Eps {
            x.attr("FlexItemWidthMode", "FlexFixed")
                .attr("FlexItemHeightMode", "FlexFixed");
        }
        if g.kind == GraphicKind::Svg && (major, self.doc.version.minor) >= (21, 2) {
            x.attr("IsMathMLObject", "false");
        }
        x.start("Properties");
        if g.kind == GraphicKind::Image
            && let Some(p) = profile(g.profile)
        {
            x.start("Profile").attr("type", "string").text(p).end();
        }
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
        if g.kind == GraphicKind::Image {
            Self::image_io(x, g);
        }
        if g.kind == GraphicKind::Pdf
            || g.kind == GraphicKind::Image && g.layers.as_ref().is_some_and(|l| l.option)
        {
            self.graphic_layers(x, g);
        }
        Self::text_wrap_preference(x, g.text_wrap.as_ref(), g.contour_type);
        if let Some(link) = &g.link {
            self.link(x, link);
        }
        if g.kind == GraphicKind::Pdf {
            let (page, transparent, crop) = match g.pdf {
                Some((page, transparent, crop)) => (
                    page.to_string(),
                    match transparent {
                        0 => Some("false"),
                        1 => Some("true"),
                        _ => None,
                    },
                    pdf_crop(crop),
                ),
                None => (
                    "1".to_string(),
                    Some("true"),
                    Some("CropContentVisibleLayers"),
                ),
            };
            x.start("PDFAttribute").attr("PageNumber", page);
            if let Some(c) = crop {
                x.attr("PDFCrop", c);
            }
            if let Some(t) = transparent {
                x.attr("TransparentBackground", t);
            }
            x.end();
        }
        x.end();
    }

    /// `GraphicLayerOption` with the graphic's layers, and for an image
    /// `LayerCompOption`. See `docs/format/objects.md`, graphic layers.
    fn graphic_layers(&self, x: &mut Xml, g: &Graphic) {
        let layers = g.layers.as_ref().map_or(&[][..], |l| &l.layers[..]);
        x.start("GraphicLayerOption")
            .attr("UpdateLinkOption", "KeepOverrides");
        let base = format!("{}GraphicLayerOption1", uref(Some(g.uid)));
        // Top-level layers, then each layer's children, in record order. A
        // layer whose parent is not listed is left out.
        let mut path = Vec::new();
        Self::layer_children(x, layers, -1, &base, &mut path);
        x.end();
        if g.kind == GraphicKind::Image
            && let Some(comp) = g.layer_comp
        {
            x.empty("LayerCompOption", &[("AppliedLayerComp", comp.to_string())]);
        }
    }

    fn layer_children(
        x: &mut Xml,
        layers: &[GraphicLayer],
        parent: i32,
        base: &str,
        path: &mut Vec<u32>,
    ) {
        for l in layers.iter().filter(|l| l.parent == parent) {
            // A layer that contains itself would never end.
            if path.contains(&l.id) || i32::try_from(l.id).is_err() {
                continue;
            }
            path.push(l.id);
            let id: String = path.iter().map(|i| format!("i{i:x}")).collect();
            x.start("GraphicLayer")
                .attr("Self", format!("{base}{id}"))
                .attr("Name", layer_name(&l.name))
                .attr("OriginalVisibility", l.original_visibility.to_string())
                .attr("CurrentVisibility", l.current_visibility.to_string())
                .attr("SeparatorLayer", (l.flags & 0x01 != 0).to_string())
                .attr("AdjustmentLayer", "false")
                .attr("FXLayer", (l.flags & 0x04 != 0).to_string())
                .attr("Locked", (l.flags & 0x08 != 0).to_string());
            for name in [
                "HasViewState",
                "ViewState",
                "HasExportState",
                "ExportState",
                "HasPrintState",
                "PrintState",
            ] {
                x.attr(name, "false");
            }
            x.attr("Id", l.id.to_string());
            Self::layer_children(x, layers, l.id as i32, base, path);
            x.end();
            path.pop();
        }
    }
}

/// A layer name: a key (flag 1) is written with `$ID/`.
fn layer_name(n: &Name) -> String {
    n.idml()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn measures_scales_of_skewed_transforms() {
        let (h, v) = scales(&Matrix([2.0, 0.0, 0.0, 3.0, 5.0, 5.0]));
        assert_eq!((h, v), (2.0, 3.0));
        // A skew keeps the area: the vertical scale is the determinant
        // over the horizontal scale.
        let (h, v) = scales(&Matrix([1.0, 0.0, 0.5, 1.0, 0.0, 0.0]));
        assert_eq!((h, v), (1.0, 1.0));
    }

    #[test]
    fn rounds_resolutions_half_up() {
        assert_eq!(round_half_up(76.5), 77.0);
        assert_eq!(round_half_up(72.009), 72.0);
    }

    #[test]
    fn names_colour_spaces() {
        assert_eq!(color_space(2, true), Some("$ID/#Links_Indexed RGB"));
        assert_eq!(color_space(4, false), Some("$ID/#Links_CMYK"));
        assert_eq!(color_space(3, false), None);
        assert_eq!(profile(None), Some("$ID/None"));
        assert_eq!(profile(Some(2)), None);
    }
}
