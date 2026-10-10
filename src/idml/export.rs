//! `ObjectExportOption` of page items and object styles.
//!
//! The attributes IDML writes depend on the INDD version, major and minor
//! (`docs/format/idml-values.md`, export options of page items); the
//! alternative text and tagging values come from chunk 0x1E206 of a page
//! item (`docs/format/objects.md`, object export options).

use super::values::Node;
use crate::header::Version;
use crate::model::{ExportOptions, Name};

/// The values every `ObjectExportOption` has, in IDML order, after the
/// alternative text and tagging values.
const COMMON: [(&str, &str); 15] = [
    ("ImageConversionType", "JPEG"),
    ("ImageExportResolution", "Ppi300"),
    ("GIFOptionsPalette", "AdaptivePalette"),
    ("GIFOptionsInterlaced", "true"),
    ("JPEGOptionsQuality", "High"),
    ("JPEGOptionsFormat", "BaselineEncoding"),
    ("ImageAlignment", "AlignLeft"),
    ("ImageSpaceBefore", "0"),
    ("ImageSpaceAfter", "0"),
    ("UseImagePageBreak", "false"),
    ("ImagePageBreak", "PageBreakBefore"),
    ("CustomImageAlignment", "false"),
    ("SpaceUnit", "CssPixel"),
    ("CustomLayout", "false"),
    ("CustomLayoutType", "AlignmentAndSpacing"),
];

fn alt_source(code: u32) -> Option<&'static str> {
    match code {
        0 => Some("SourceCustom"),
        5 => Some("SourceXMLStructure"),
        8 => Some("SourceDecorativeImage"),
        _ => None,
    }
}

fn actual_source(code: u32) -> Option<&'static str> {
    match code {
        0 => Some("SourceCustom"),
        5 => Some("SourceXMLStructure"),
        6 => Some("SourceXMPAltText"),
        _ => None,
    }
}

fn tag_type(code: u32) -> Option<&'static str> {
    match code {
        0 => Some("TagFromStructure"),
        1 => Some("TagArtifact"),
        _ => None,
    }
}

/// A flagged string as IDML text: an empty key is `$ID/`.
fn text(n: &Name) -> String {
    n.idml()
}

/// Whose `ObjectExportOption` is written.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Owner {
    Item,
    Group,
    Style,
}

/// The `ObjectExportOption` of a page item, a group or an object style, or `None` for versions whose IDML has none: page items before
/// 8, object styles before 9. `options` are the item's values from chunk
/// 0x1E206; without them the values of an item without the chunk apply.
/// Codes not identified leave their attribute out.
pub(super) fn object_export_option(
    version: Version,
    options: Option<&ExportOptions>,
    size_type: Option<u32>,
    owner: Owner,
) -> Option<Node> {
    let style = owner == Owner::Style;
    let v = (version.major, version.minor);
    if v < (8, 0) || style && v < (9, 0) {
        return None;
    }
    let default = ExportOptions {
        alt_source: 5,
        actual_source: 5,
        alt_text: Name {
            builtin: true,
            name: String::new(),
        },
        actual_text: Name {
            builtin: true,
            name: String::new(),
        },
        ..ExportOptions::default()
    };
    let o = options.unwrap_or(&default);
    let mut attrs: Vec<(String, String)> = Vec::new();
    let mut add = |name: &str, value: String| attrs.push((name.into(), value));
    if let Some(s) = alt_source(o.alt_source) {
        add("AltTextSourceType", s.into());
    }
    if let Some(s) = actual_source(o.actual_source) {
        add("ActualTextSourceType", s.into());
    }
    add("CustomAltText", text(&o.alt_text));
    add("CustomActualText", text(&o.actual_text));
    if let Some(s) = tag_type(o.tag_type) {
        add("ApplyTagType", s.into());
    }
    // 8.0 to 10.0 have CustomImageConversion; 8 and 9 also have
    // CustomImageSizeOption, which the schema the output is checked
    // against does not allow (idml-values.md), so it is left out.
    let old = v <= (10, 0);
    if old {
        add("CustomImageConversion", "false".into());
    }
    for (name, value) in COMMON {
        add(name, value.into());
    }
    if v == (10, 0) {
        for (name, value) in [
            ("EpubType", "$ID/"),
            ("UseExistingImage", "false"),
            ("CustomHeightType", "DefaultHeight"),
            ("CustomHeight", "$ID/"),
            ("CustomWidthType", "DefaultWidth"),
            ("CustomWidth", "$ID/"),
        ] {
            add(name, value.into());
        }
    } else if v >= (10, 1) {
        for (name, value) in [
            ("EpubType", "$ID/"),
            (
                "SizeType",
                // Chunk 0x1E22B; without it the default (objects.md,
                // export options).
                match size_type {
                    Some(2) => "FixedSize",
                    Some(3) => "RelativeToTextFlow",
                    Some(4) => "RelativeToTextSize",
                    _ => "DefaultSize",
                },
            ),
            ("CustomSize", "$ID/"),
            ("PreserveAppearanceFromLayout", "PreserveAppearanceDefault"),
        ] {
            add(name, value.into());
        }
    }
    if v >= (21, 0) {
        add("EpubAriaRole", "$ID/".into());
    }
    if v >= (21, 1) {
        add("EpubAriaLabel", "$ID/".into());
        add("EpubAriaLabelSourceType", "AutomaticARIALabel".into());
    }
    if v >= (21, 4) {
        // Groups and styles have no generated alternative text
        // (idml-values.md, export options).
        if owner == Owner::Item {
            add("AIGeneratedAltText", "false".into());
            add("AltTextGenerationError", "false".into());
        }
        add("AltTextCropSyncRect", String::new());
    }
    if v >= (9, 2) && old {
        add("UseOriginalImage", "false".into());
    }
    let metadata = |tag: &str, m: &[Name; 2]| Node {
        tag: tag.into(),
        attrs: vec![
            ("NamespacePrefix".into(), text(&m[0])),
            ("PropertyPath".into(), text(&m[1])),
        ],
        text: None,
        children: Vec::new(),
    };
    let key = Name {
        builtin: true,
        name: String::new(),
    };
    let empty = [key.clone(), key];
    let (alt, actual) = match options {
        Some(o) => (&o.alt_metadata, &o.actual_metadata),
        None => (&empty, &empty),
    };
    Some(Node {
        tag: "ObjectExportOption".into(),
        attrs,
        text: None,
        children: vec![Node {
            tag: "Properties".into(),
            attrs: Vec::new(),
            text: None,
            children: vec![
                metadata("AltMetadataProperty", alt),
                metadata("ActualMetadataProperty", actual),
            ],
        }],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn version(major: u32, minor: u32) -> Version {
        Version { major, minor }
    }

    fn names(n: &Node) -> Vec<&str> {
        n.attrs.iter().map(|(k, _)| k.as_str()).collect()
    }

    #[test]
    fn writes_attributes_by_version() {
        assert!(object_export_option(version(7, 5), None, None, Owner::Item).is_none());
        assert!(object_export_option(version(8, 0), None, None, Owner::Style).is_none());
        let n = object_export_option(version(9, 2), None, None, Owner::Style).unwrap();
        assert!(names(&n).contains(&"CustomImageConversion"));
        assert_eq!(names(&n).last(), Some(&"UseOriginalImage"));
        let n = object_export_option(version(10, 0), None, None, Owner::Item).unwrap();
        assert!(names(&n).contains(&"CustomWidthType"));
        assert!(!names(&n).contains(&"SizeType"));
        let n = object_export_option(version(10, 1), None, None, Owner::Item).unwrap();
        assert!(names(&n).contains(&"SizeType"));
        assert!(!names(&n).contains(&"CustomImageConversion"));
        assert!(!names(&n).contains(&"EpubAriaRole"));
        let n = object_export_option(version(21, 4), None, None, Owner::Style).unwrap();
        assert!(names(&n).contains(&"AltTextCropSyncRect"));
        assert!(!names(&n).contains(&"AIGeneratedAltText"));
        let n = object_export_option(version(21, 4), None, None, Owner::Item).unwrap();
        assert!(names(&n).contains(&"AIGeneratedAltText"));
        assert_eq!(n.attr("CustomAltText"), Some("$ID/"));
        assert_eq!(n.attr("AltTextSourceType"), Some("SourceXMLStructure"));
    }

    #[test]
    fn writes_the_values_of_the_chunk() {
        let o = ExportOptions {
            alt_source: 8,
            alt_text: Name {
                builtin: false,
                name: "A tree".into(),
            },
            actual_source: 6,
            tag_type: 1,
            ..ExportOptions::default()
        };
        let n = object_export_option(version(20, 5), Some(&o), None, Owner::Item).unwrap();
        assert_eq!(n.attr("AltTextSourceType"), Some("SourceDecorativeImage"));
        assert_eq!(n.attr("ActualTextSourceType"), Some("SourceXMPAltText"));
        assert_eq!(n.attr("CustomAltText"), Some("A tree"));
        assert_eq!(n.attr("ApplyTagType"), Some("TagArtifact"));
        let o = ExportOptions { alt_source: 3, ..o };
        let n = object_export_option(version(20, 5), Some(&o), None, Owner::Item).unwrap();
        assert_eq!(n.attr("AltTextSourceType"), None);
    }
}
