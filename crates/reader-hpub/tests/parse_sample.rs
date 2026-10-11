use reader_core::{FormatParser, Source};
use reader_hpub::HpubParser;

#[test]
fn parses_fixture_metadata_spine_and_toc() {
    let bytes = include_bytes!("fixtures/sample.hpub");
    let src = Source {
        name: "sample.hpub".into(),
        bytes: bytes.to_vec(),
    };
    assert!(HpubParser.can_parse(&src));

    let pub_ = HpubParser.parse(src).unwrap();
    assert_eq!(pub_.metadata.title, "HPUB Fixture");
    assert_eq!(pub_.metadata.authors, vec!["Baker".to_string()]);
    assert_eq!(
        pub_.spine,
        vec![
            "pages/one.html".to_string(),
            "pages/two.html".to_string()
        ]
    );
    assert_eq!(pub_.toc[0].title, "pages/one.html");
    assert_eq!(pub_.toc[1].title, "Segundo");
    let html = String::from_utf8(pub_.read("pages/one.html").unwrap()).unwrap();
    assert!(html.contains("Primeira"));
}

#[test]
fn detects_by_book_json_entry() {
    let bytes = include_bytes!("fixtures/sample.hpub");
    let src = Source {
        name: "blob.bin".into(),
        bytes: bytes.to_vec(),
    };
    assert!(HpubParser.can_parse(&src));
}
