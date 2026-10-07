//! Smoke tests on the open-licensed sample files listed in
//! `tests/fixtures/manifest.json`. `tools/fetch_fixtures.py` downloads them
//! into the git-ignored `tests/fixtures/files/`. Each test passes without
//! checking anything when that directory is absent.

use std::path::{Path, PathBuf};

use indd::{ByteOrder, Version, read_header};

/// The fixture directory, or `None` (with a note) if it has not been fetched.
fn fixtures() -> Option<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/files");
    if root.is_dir() {
        Some(root)
    } else {
        eprintln!(
            "tests/fixtures/files/ not present; skipping \
             (fetch with `python3 -I tools/fetch_fixtures.py`)"
        );
        None
    }
}

#[test]
fn fixture_headers() {
    let Some(root) = fixtures() else {
        return;
    };
    let cases = [
        (
            "opf-neddy-flyer/Neddy_Flyer_HeatherRyan.indd",
            ByteOrder::Big,
            3,
            0,
        ),
        ("scml-template/scml.indt", ByteOrder::Little, 7, 5),
        (
            "bootstrap3-template/bootstrap3-indesign-template.indd",
            ByteOrder::Little,
            9,
            2,
        ),
        (
            "lizdenys-minizine/indesign-minizine-template.indd",
            ByteOrder::Little,
            20,
            3,
        ),
        (
            "xmp-toolkit-bluesquare/BlueSquare.indd",
            ByteOrder::Big,
            4,
            0,
        ),
    ];
    for (rel, order, major, minor) in cases {
        let h = read_header(root.join(rel)).unwrap_or_else(|e| panic!("{rel}: {e}"));
        assert_eq!(&h.kind, b"DOCUMENT", "{rel}");
        assert_eq!(h.byte_order, order, "{rel}");
        assert_eq!(h.version, Version { major, minor }, "{rel}");
    }
}

#[test]
fn rejects_non_indd_file() {
    let Some(root) = fixtures() else {
        return;
    };
    let err = read_header(root.join("opf-neddy-flyer/Neddy_Flyer_HeatherRyan.pdf")).unwrap_err();
    assert!(matches!(err, indd::Error::NotIndd));
}

#[test]
fn fixture_containers() {
    let Some(root) = fixtures() else {
        return;
    };
    for rel in [
        "opf-neddy-flyer/Neddy_Flyer_HeatherRyan.indd",
        "scml-template/scml.indt",
        "bootstrap3-template/bootstrap3-indesign-template.indd",
        "lizdenys-minizine/indesign-minizine-template.indd",
        "xmp-toolkit-bluesquare/BlueSquare.indd",
    ] {
        let bytes = std::fs::read(root.join(rel)).unwrap_or_else(|e| panic!("{rel}: {e}"));
        let c = indd::Container::parse(&bytes).unwrap_or_else(|e| panic!("{rel}: {e}"));
        assert!(c.contig_start() < bytes.len(), "{rel}");
        let xmp = c.xmp().unwrap().unwrap_or_else(|| panic!("{rel}: no XMP"));
        let xmp = String::from_utf8_lossy(xmp);
        assert!(
            xmp.contains("Adobe InDesign"),
            "{rel}: XMP has no InDesign creator"
        );
    }
}

#[test]
fn fixture_objects_read() {
    let Some(root) = fixtures() else {
        return;
    };
    for rel in [
        "opf-neddy-flyer/Neddy_Flyer_HeatherRyan.indd",
        "xmp-toolkit-bluesquare/BlueSquare.indd",
        "scml-template/scml.indt",
        "bootstrap3-template/bootstrap3-indesign-template.indd",
        "lizdenys-minizine/indesign-minizine-template.indd",
    ] {
        let bytes = std::fs::read(root.join(rel)).unwrap_or_else(|e| panic!("{rel}: {e}"));
        let c = indd::Container::parse(&bytes).unwrap();
        let db = c.database().unwrap_or_else(|e| panic!("{rel}: {e}"));
        let mut total = 0;
        for uid in db.uids() {
            total += db.object(uid).unwrap().unwrap().len();
        }
        assert!(total > 0, "{rel}: no object data");
    }
}

#[test]
fn big_endian_fixtures_convert() {
    let Some(root) = fixtures() else {
        return;
    };
    // InDesign 3.0 and 4.0. Neither has an IDML, so check values that can
    // be seen otherwise: the flyer's print PDF shows a letter-size page and
    // these headings; BlueSquare is a letter-size page with one blue square.
    // Fonts, the XML root element and colour names exercise big-endian
    // strings, and the font records of both versions.
    for (rel, expected) in [
        (
            "opf-neddy-flyer/Neddy_Flyer_HeatherRyan.indd",
            &[
                "DOMVersion=\"3.0\"",
                "GeometricBounds=\"0 0 792 612\"",
                "<Content>Ned the Narcoleptic</Content>",
                "<Content>The Dream Job!</Content>",
                // A forced line break, as in the PDF.
                "<Content>Neddy Buys\u{2028}a New Pillow</Content>",
                "PostScriptName=\"Times-Roman\"",
                "MarkupTag=\"XMLTag/Root\"",
            ][..],
        ),
        (
            "xmp-toolkit-bluesquare/BlueSquare.indd",
            &[
                "DOMVersion=\"4.0\"",
                "GeometricBounds=\"0 0 792 612\"",
                "<Rectangle Self=\"uad\"",
                "PostScriptName=\"Times-Roman\"",
                "Version=\"5.0d10e1\"",
                "Name=\"C=100 M=90 Y=10 K=0\"",
                "MarkupTag=\"XMLTag/Root\"",
            ][..],
        ),
    ] {
        let bytes = std::fs::read(root.join(rel)).unwrap_or_else(|e| panic!("{rel}: {e}"));
        let mut out = Vec::new();
        indd::convert(&bytes, "test.indd", &mut out).unwrap_or_else(|e| panic!("{rel}: {e}"));
        assert_eq!(&out[..4], b"PK\x03\x04", "{rel}");
        let text = String::from_utf8_lossy(&out);
        for e in expected {
            assert!(text.contains(e), "{rel}: no {e}");
        }
    }
}

#[test]
fn little_endian_fixtures_convert() {
    let Some(root) = fixtures() else {
        return;
    };
    for rel in [
        "scml-template/scml.indt",
        "bootstrap3-template/bootstrap3-indesign-template.indd",
        "lizdenys-minizine/indesign-minizine-template.indd",
    ] {
        let bytes = std::fs::read(root.join(rel)).unwrap_or_else(|e| panic!("{rel}: {e}"));
        let mut out = Vec::new();
        indd::convert(&bytes, "test.indd", &mut out).unwrap_or_else(|e| panic!("{rel}: {e}"));
        assert_eq!(&out[..4], b"PK\x03\x04", "{rel}");
        assert_eq!(&out[30..38], b"mimetype", "{rel}: mimetype must be first");
        let text = String::from_utf8_lossy(&out);
        assert!(text.contains("designmap.xml"), "{rel}");
    }
}

/// The InDesign 7.5 template has character styles whose kind field is
/// followed by a non-zero u16 (`docs/format/objects.md`, Styles).
#[test]
fn style_kind_is_a_u16() {
    let Some(root) = fixtures() else {
        return;
    };
    let bytes = std::fs::read(root.join("scml-template/scml.indt")).unwrap();
    let mut out = Vec::new();
    indd::convert(&bytes, "test.indd", &mut out).unwrap();
    let text = String::from_utf8_lossy(&out);
    assert!(text.contains(r#"<CharacterStyle Self="CharacterStyle/abbr" Name="abbr">"#));
    assert!(!text.contains(r#"<CharacterStyle Self="ParagraphStyle/"#));
}

#[test]
fn audit_accounts_for_every_object() {
    let Some(root) = fixtures() else {
        return;
    };
    for rel in [
        "scml-template/scml.indt",
        "lizdenys-minizine/indesign-minizine-template.indd",
        "opf-neddy-flyer/Neddy_Flyer_HeatherRyan.indd",
    ] {
        let bytes = std::fs::read(root.join(rel)).unwrap();
        let a = indd::audit::audit(&bytes, rel).unwrap_or_else(|e| panic!("{rel}: {e}"));
        assert_eq!(a.error, None, "{rel}");
        let c = indd::Container::parse(&bytes).unwrap();
        let _order = indd::object::use_byte_order(c.header.byte_order);
        let db = c.database().unwrap();
        let with_data = db
            .uids()
            .filter(|&u| db.object(u).unwrap().is_some())
            .count();
        let audited: usize = a.classes.values().map(|c| c.objects).sum();
        assert_eq!(audited, with_data, "{rel}");
        // The document object is always read.
        assert!(a.classes[&Some(0xE01)].read > 0, "{rel}");
        // Auditing reports the same warnings as converting.
        let warnings = indd::convert(&bytes, rel, std::io::sink()).unwrap();
        assert_eq!(a.warnings, warnings, "{rel}");
    }
}
