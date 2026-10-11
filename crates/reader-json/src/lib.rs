use reader_core::{
    Error, FormatParser, Metadata, Publication, Resource, ResourceProvider, Result, Source,
    TocEntry,
};
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;

/// Formato JSON próprio:
/// `{ "format": "reader-json", "title": "...", "authors": [], "pages": [{ "id": "p1", "title": "...", "html": "<p>..</p>" }] }`
#[derive(Deserialize)]
struct Doc {
    format: String,
    title: String,
    #[serde(default)]
    authors: Vec<String>,
    language: Option<String>,
    pages: Vec<Page>,
}

#[derive(Deserialize)]
struct Page {
    id: String,
    title: Option<String>,
    html: String,
}

struct MemoryProvider(HashMap<String, Vec<u8>>);

impl ResourceProvider for MemoryProvider {
    fn read(&self, href: &str) -> Result<Vec<u8>> {
        self.0
            .get(href)
            .cloned()
            .ok_or_else(|| Error::NotFound(href.into()))
    }
}

pub struct JsonParser;

impl FormatParser for JsonParser {
    fn id(&self) -> &'static str {
        "reader-json"
    }

    fn can_parse(&self, src: &Source) -> bool {
        src.extension().as_deref() == Some("json")
            && serde_json::from_slice::<serde_json::Value>(&src.bytes)
                .ok()
                .and_then(|v| {
                    v.get("format")
                        .and_then(|f| f.as_str().map(|s| s == "reader-json"))
                })
                .unwrap_or(false)
    }

    fn parse(&self, src: Source) -> Result<Publication> {
        let doc: Doc =
            serde_json::from_slice(&src.bytes).map_err(|e| Error::Invalid(e.to_string()))?;
        if doc.format != "reader-json" {
            return Err(Error::UnsupportedFormat);
        }

        let mut files = HashMap::new();
        let (mut spine, mut resources, mut toc) = (Vec::new(), Vec::new(), Vec::new());
        for p in doc.pages {
            let href = format!("{}.html", p.id);
            toc.push(TocEntry {
                title: p.title.unwrap_or_else(|| p.id.clone()),
                href: href.clone(),
                children: Vec::new(),
            });
            resources.push(Resource {
                href: href.clone(),
                media_type: "text/html".into(),
            });
            files.insert(href.clone(), p.html.into_bytes());
            spine.push(href);
        }

        Ok(Publication {
            metadata: Metadata {
                title: doc.title,
                authors: doc.authors,
                language: doc.language,
                identifier: None,
            },
            spine,
            resources,
            toc,
            provider: Arc::new(MemoryProvider(files)),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_bytes() -> Vec<u8> {
        r#"{
          "format": "reader-json",
          "title": "Noite Clara",
          "authors": ["Ana"],
          "language": "pt-BR",
          "pages": [
            { "id": "p1", "title": "Abertura", "html": "<p>Ola</p>" },
            { "id": "p2", "html": "<p>Capitulo</p>" }
          ]
        }"#
        .as_bytes()
        .to_vec()
    }

    #[test]
    fn can_parse_only_reader_json_files() {
        let ok = Source {
            name: "livro.json".into(),
            bytes: sample_bytes(),
        };
        assert!(JsonParser.can_parse(&ok));

        let wrong_ext = Source {
            name: "livro.txt".into(),
            bytes: sample_bytes(),
        };
        assert!(!JsonParser.can_parse(&wrong_ext));

        let other_json = Source {
            name: "data.json".into(),
            bytes: br#"{"format":"other"}"#.to_vec(),
        };
        assert!(!JsonParser.can_parse(&other_json));
    }

    #[test]
    fn parse_builds_spine_toc_and_resources() {
        let pub_ = JsonParser
            .parse(Source {
                name: "livro.json".into(),
                bytes: sample_bytes(),
            })
            .unwrap();

        assert_eq!(pub_.metadata.title, "Noite Clara");
        assert_eq!(pub_.metadata.authors, vec!["Ana".to_string()]);
        assert_eq!(
            pub_.spine,
            vec!["p1.html".to_string(), "p2.html".to_string()]
        );
        assert_eq!(pub_.toc[0].title, "Abertura");
        assert_eq!(pub_.toc[1].title, "p2");
        assert_eq!(
            String::from_utf8(pub_.read("p1.html").unwrap()).unwrap(),
            "<p>Ola</p>"
        );
    }

    #[test]
    fn invalid_json_returns_error() {
        let result = JsonParser.parse(Source {
            name: "x.json".into(),
            bytes: b"{not json".to_vec(),
        });
        assert!(matches!(result, Err(Error::Invalid(_))));
    }
}
