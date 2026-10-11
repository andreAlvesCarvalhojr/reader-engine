use reader_core::{
    Error, FormatParser, Metadata, Publication, Resource, Result, ResourceProvider, Source,
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
        self.0.get(href).cloned().ok_or_else(|| Error::NotFound(href.into()))
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
                .and_then(|v| v.get("format").and_then(|f| f.as_str().map(|s| s == "reader-json")))
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
            resources.push(Resource { href: href.clone(), media_type: "text/html".into() });
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
