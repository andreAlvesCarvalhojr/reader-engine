use reader_core::{FormatParser, Source};
use reader_epub::EpubParser;

#[test]
fn parses_fixture_metadata_spine_and_toc() {
    let bytes = include_bytes!("fixtures/sample.epub");
    let src = Source {
        name: "sample.epub".into(),
        bytes: bytes.to_vec(),
    };
    assert!(EpubParser.can_parse(&src));

    let pub_ = EpubParser.parse(src).unwrap();
    assert_eq!(pub_.metadata.title, "EPUB Fixture");
    assert_eq!(pub_.metadata.authors, vec!["Autor Teste".to_string()]);
    assert_eq!(
        pub_.spine,
        vec!["OEBPS/c1.xhtml".to_string(), "OEBPS/c2.xhtml".to_string()]
    );
    assert_eq!(pub_.toc.len(), 2);
    assert_eq!(pub_.toc[0].title, "Um");
    let html = String::from_utf8(pub_.read("OEBPS/c1.xhtml").unwrap()).unwrap();
    assert!(html.contains("Capítulo um"));
}

#[test]
fn detects_by_container_even_without_epub_extension() {
    let bytes = include_bytes!("fixtures/sample.epub");
    let src = Source {
        name: "semext".into(),
        bytes: bytes.to_vec(),
    };
    assert!(EpubParser.can_parse(&src));
}
