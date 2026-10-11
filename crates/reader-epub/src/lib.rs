use reader_core::zip_provider::{has_zip_entry, ZipProvider};
use reader_core::{
    Error, FormatParser, Metadata, Publication, Resource, Result, Source, TocEntry,
};
use std::sync::Arc;

pub struct EpubParser;

impl FormatParser for EpubParser {
    fn id(&self) -> &'static str {
        "epub"
    }

    fn can_parse(&self, src: &Source) -> bool {
        src.extension().as_deref() == Some("epub")
            || has_zip_entry(&src.bytes, "META-INF/container.xml")
    }

    fn parse(&self, src: Source) -> Result<Publication> {
        let zip = ZipProvider::new(src.bytes)?;

        let container = zip.read_string("META-INF/container.xml")?;
        let doc = roxmltree::Document::parse(&container).map_err(|e| Error::Invalid(e.to_string()))?;
        let opf_path = doc
            .descendants()
            .find(|n| n.has_tag_name("rootfile"))
            .and_then(|n| n.attribute("full-path"))
            .ok_or_else(|| Error::Invalid("container.xml sem rootfile".into()))?
            .to_string();
        let base = opf_path.rsplit_once('/').map(|(d, _)| format!("{d}/")).unwrap_or_default();

        let opf = zip.read_string(&opf_path)?;
        let opf = roxmltree::Document::parse(&opf).map_err(|e| Error::Invalid(e.to_string()))?;

        let text_of = |tag: &str| {
            opf.descendants()
                .find(|n| n.tag_name().name() == tag)
                .and_then(|n| n.text())
                .map(|s| s.trim().to_string())
        };
        let metadata = Metadata {
            title: text_of("title").unwrap_or_default(),
            authors: opf
                .descendants()
                .filter(|n| n.tag_name().name() == "creator")
                .filter_map(|n| n.text().map(|s| s.trim().to_string()))
                .collect(),
            language: text_of("language"),
            identifier: text_of("identifier"),
        };

        let mut resources = Vec::new();
        let mut id_to_href = std::collections::HashMap::new();
        let mut nav_href = None;
        for item in opf.descendants().filter(|n| n.has_tag_name("item")) {
            let (Some(id), Some(href)) = (item.attribute("id"), item.attribute("href")) else {
                continue;
            };
            let full = format!("{base}{href}");
            if item.attribute("properties").is_some_and(|p| p.split(' ').any(|x| x == "nav")) {
                nav_href = Some(full.clone());
            }
            id_to_href.insert(id.to_string(), full.clone());
            resources.push(Resource {
                href: full,
                media_type: item.attribute("media-type").unwrap_or("").to_string(),
            });
        }

        let spine = opf
            .descendants()
            .filter(|n| n.has_tag_name("itemref"))
            .filter_map(|n| n.attribute("idref"))
            .filter_map(|id| id_to_href.get(id).cloned())
            .collect();

        let toc = nav_href
            .and_then(|href| zip.read_string(&href).ok().map(|x| parse_nav(&x, &href)))
            .unwrap_or_default();

        Ok(Publication { metadata, spine, resources, toc, provider: Arc::new(zip) })
    }
}

/// TOC plano a partir do `<nav epub:type="toc">`; hierarquia fica como evolução.
fn parse_nav(xhtml: &str, nav_path: &str) -> Vec<TocEntry> {
    let Ok(doc) = roxmltree::Document::parse(xhtml) else {
        return Vec::new();
    };
    let dir = nav_path.rsplit_once('/').map(|(d, _)| format!("{d}/")).unwrap_or_default();
    doc.descendants()
        .find(|n| {
            n.has_tag_name("nav")
                && n.attributes().any(|a| a.name() == "type" && a.value().contains("toc"))
        })
        .map(|nav| {
            nav.descendants()
                .filter(|n| n.has_tag_name("a"))
                .filter_map(|a| {
                    let href = a.attribute("href")?;
                    let title: String = a.descendants().filter_map(|t| t.text()).collect();
                    Some(TocEntry {
                        title: title.trim().to_string(),
                        href: format!("{dir}{href}"),
                        children: Vec::new(),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}
