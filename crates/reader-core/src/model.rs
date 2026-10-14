use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Metadata {
    pub title: String,
    pub authors: Vec<String>,
    pub language: Option<String>,
    pub identifier: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resource {
    /// Caminho relativo à raiz da publicação.
    pub href: String,
    pub media_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TocEntry {
    pub title: String,
    pub href: String,
    pub children: Vec<TocEntry>,
}

/// Posição no livro, independente de formato.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Locator {
    pub spine_index: usize,
    /// 0.0..=1.0 dentro do item.
    pub progression: f32,
}

/// A bookmark marking a specific position in the publication.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Bookmark {
    /// Unique identifier for this bookmark.
    pub id: String,
    /// The position this bookmark refers to.
    pub locator: Locator,
    /// Optional user-provided label/name for the bookmark.
    pub label: Option<String>,
    /// Timestamp when the bookmark was created (Unix millis).
    pub created_at: u64,
}

/// Acesso lazy aos bytes dos recursos (zip, disco, memória...).
pub trait ResourceProvider: Send + Sync {
    fn read(&self, href: &str) -> Result<Vec<u8>>;
}

#[derive(Clone)]
pub struct Publication {
    pub metadata: Metadata,
    /// Ordem de leitura (hrefs em `resources`).
    pub spine: Vec<String>,
    pub resources: Vec<Resource>,
    pub toc: Vec<TocEntry>,
    pub provider: Arc<dyn ResourceProvider>,
}

impl Publication {
    pub fn read(&self, href: &str) -> Result<Vec<u8>> {
        self.provider.read(href)
    }
}
