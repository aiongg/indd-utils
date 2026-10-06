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
