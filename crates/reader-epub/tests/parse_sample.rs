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

#[test]
fn parses_epub_with_image_and_stylesheet() {
    let bytes = include_bytes!("fixtures/sample-with-image.epub");
    let src = Source {
        name: "sample-with-image.epub".into(),
        bytes: bytes.to_vec(),
    };
    assert!(EpubParser.can_parse(&src));

    let pub_ = EpubParser.parse(src).unwrap();
    assert_eq!(pub_.metadata.title, "EPUB With Image");
    assert_eq!(pub_.metadata.authors, vec!["Test Author".to_string()]);
    assert_eq!(pub_.spine, vec!["OEBPS/chapter1.xhtml".to_string()]);
    assert_eq!(pub_.toc.len(), 1);

    let html = String::from_utf8(pub_.read("OEBPS/chapter1.xhtml").unwrap()).unwrap();
    assert!(html.contains("<img"));
    assert!(html.contains("images/cover.png"));

    let css = pub_.read("OEBPS/styles/main.css").unwrap();
    assert!(!css.is_empty());
    assert!(String::from_utf8_lossy(&css).contains("font-family"));

    let png = pub_.read("OEBPS/images/cover.png").unwrap();
    assert!(!png.is_empty());
    assert_eq!(&png[0..4], b"\x89PNG");
}

#[test]
fn lists_all_resources_including_images_and_css() {
    let bytes = include_bytes!("fixtures/sample-with-image.epub");
    let src = Source {
        name: "sample-with-image.epub".into(),
        bytes: bytes.to_vec(),
    };
    let pub_ = EpubParser.parse(src).unwrap();

    assert!(
        pub_.resources.iter().any(|r| r.href.ends_with(".png")),
        "should have PNG resource"
    );
    assert!(
        pub_.resources.iter().any(|r| r.href.ends_with(".css")),
        "should have CSS resource"
    );
    assert!(
        pub_.resources.iter().any(|r| r.media_type == "image/png"),
        "should have image/png media type"
    );
    assert!(
        pub_.resources.iter().any(|r| r.media_type == "text/css"),
        "should have text/css media type"
    );
}
