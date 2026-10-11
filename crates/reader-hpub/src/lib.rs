use reader_core::zip_provider::{has_zip_entry, ZipProvider};
use reader_core::{Error, FormatParser, Metadata, Publication, Resource, Result, Source, TocEntry};
use serde::Deserialize;
use std::sync::Arc;

/// HPUB (Baker Framework): ZIP com `book.json` listando as páginas HTML em `contents`.
#[derive(Deserialize)]
struct BookJson {
    #[serde(default)]
    title: String,
    #[serde(default)]
    author: Vec<String>,
    #[serde(default)]
    language: Option<String>,
    #[serde(default)]
    contents: Vec<Content>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum Content {
    Path(String),
    Entry { url: String, title: Option<String> },
}

pub struct HpubParser;

impl FormatParser for HpubParser {
    fn id(&self) -> &'static str {
        "hpub"
    }

    fn can_parse(&self, src: &Source) -> bool {
        src.extension().as_deref() == Some("hpub") || has_zip_entry(&src.bytes, "book.json")
    }

    fn parse(&self, src: Source) -> Result<Publication> {
        let zip = ZipProvider::new(src.bytes)?;
        let book: BookJson = serde_json::from_str(&zip.read_string("book.json")?)
            .map_err(|e| Error::Invalid(e.to_string()))?;

        let mut spine = Vec::new();
        let mut toc = Vec::new();
        for c in book.contents {
            let (url, title) = match c {
                Content::Path(p) => (p.clone(), p),
                Content::Entry { url, title } => (url.clone(), title.unwrap_or(url)),
            };
            toc.push(TocEntry {
                title,
                href: url.clone(),
                children: Vec::new(),
            });
            spine.push(url);
        }
        let resources = spine
            .iter()
            .map(|h| Resource {
                href: h.clone(),
                media_type: "text/html".into(),
            })
            .collect();

        Ok(Publication {
            metadata: Metadata {
                title: book.title,
                authors: book.author,
                language: book.language,
                identifier: None,
            },
            spine,
            resources,
            toc,
            provider: Arc::new(zip),
        })
    }
}
