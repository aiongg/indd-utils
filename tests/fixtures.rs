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
fn little_endian_fixture_objects_read() {
    for rel in [
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
fn big_endian_database_is_reported_unsupported() {
    let bytes = std::fs::read(fixture("xmp-toolkit-bluesquare/BlueSquare.indd")).unwrap();
    let c = indd::Container::parse(&bytes).unwrap();
    assert!(matches!(c.database(), Err(indd::Error::Unsupported(_))));
}
