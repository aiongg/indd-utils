//! Smoke tests on the redistributable sample files in `tests/fixtures/`.

use indd::{ByteOrder, Version, read_header};

fn fixture(rel: &str) -> String {
    format!("{}/tests/fixtures/{rel}", env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn fixture_headers() {
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
        let h = read_header(fixture(rel)).unwrap_or_else(|e| panic!("{rel}: {e}"));
        assert_eq!(&h.kind, b"DOCUMENT", "{rel}");
        assert_eq!(h.byte_order, order, "{rel}");
        assert_eq!(h.version, Version { major, minor }, "{rel}");
    }
}

#[test]
fn rejects_non_indd_file() {
    let err = read_header(fixture("opf-neddy-flyer/Neddy_Flyer_HeatherRyan.pdf")).unwrap_err();
    assert!(matches!(err, indd::Error::NotIndd));
}

#[test]
fn fixture_containers() {
    for rel in [
        "opf-neddy-flyer/Neddy_Flyer_HeatherRyan.indd",
        "scml-template/scml.indt",
        "bootstrap3-template/bootstrap3-indesign-template.indd",
        "lizdenys-minizine/indesign-minizine-template.indd",
        "xmp-toolkit-bluesquare/BlueSquare.indd",
    ] {
        let bytes = std::fs::read(fixture(rel)).unwrap();
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
    for rel in [
        "opf-neddy-flyer/Neddy_Flyer_HeatherRyan.indd",
        "xmp-toolkit-bluesquare/BlueSquare.indd",
        "scml-template/scml.indt",
        "bootstrap3-template/bootstrap3-indesign-template.indd",
        "lizdenys-minizine/indesign-minizine-template.indd",
    ] {
        let bytes = std::fs::read(fixture(rel)).unwrap();
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
        let bytes = std::fs::read(fixture(rel)).unwrap();
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
    for rel in [
        "scml-template/scml.indt",
        "bootstrap3-template/bootstrap3-indesign-template.indd",
        "lizdenys-minizine/indesign-minizine-template.indd",
    ] {
        let bytes = std::fs::read(fixture(rel)).unwrap();
        let mut out = Vec::new();
        indd::convert(&bytes, "test.indd", &mut out).unwrap_or_else(|e| panic!("{rel}: {e}"));
        assert_eq!(&out[..4], b"PK\x03\x04", "{rel}");
        assert_eq!(&out[30..38], b"mimetype", "{rel}: mimetype must be first");
        let text = String::from_utf8_lossy(&out);
        assert!(text.contains("designmap.xml"), "{rel}");
    }
}
