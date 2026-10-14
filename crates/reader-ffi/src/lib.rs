//! API estável para apps nativos via UniFFI.
//!
//! O app de plataforma lê o arquivo (picker / assets / rede) e entrega
//! `(nome, bytes)` — a engine não faz I/O de filesystem.

uniffi::setup_scaffolding!();

use reader_core::{Command, Locator, ReaderSession, Registry, Source, ThemeTokens};
use reader_epub::EpubParser;
use reader_hpub::HpubParser;
use reader_json::JsonParser;
use std::sync::{Arc, Mutex};

/// Erro exposto às linguagens estrangeiras.
#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum ReaderFfiError {
    #[error("{msg}")]
    Engine { msg: String },
}

impl From<reader_core::Error> for ReaderFfiError {
    fn from(value: reader_core::Error) -> Self {
        Self::Engine {
            msg: value.to_string(),
        }
    }
}

/// Item do sumário com índice no spine.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct TocItem {
    pub title: String,
    pub spine_index: u32,
}

/// Reading position that can be persisted and restored.
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct SavedPosition {
    pub spine_index: u32,
    /// 0.0..=1.0 progression within the spine item.
    pub progression: f32,
}

/// Info about a publication resource for resource listing.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct ResourceInfo {
    pub href: String,
    pub media_type: String,
}

/// Snapshot do estado para a UI observar.
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct ReaderSnapshot {
    pub title: String,
    pub authors: Vec<String>,
    pub spine_index: u32,
    pub spine_len: u32,
    pub font_scale: f32,
    pub toc: Vec<TocItem>,
    /// Stable identifier for the book, used as key for persisting reading position.
    pub book_id: String,
}

fn default_registry() -> Registry {
    let mut reg = Registry::new();
    reg.register(Box::new(JsonParser))
        .register(Box::new(EpubParser))
        .register(Box::new(HpubParser));
    reg
}

fn default_theme() -> ThemeTokens {
    ThemeTokens::from_json(include_str!("../../../themes/material3.json"))
        .expect("themes/material3.json válido")
}

fn inject_theme_css(mut html: String, css: &str) -> String {
    let inject = format!("<style>{css}</style>");
    if let Some(pos) = html.find("</head>") {
        html.insert_str(pos, &inject);
    } else if let Some(pos) = html.find("<body") {
        html.insert_str(pos, &format!("<head>{inject}</head>"));
    } else {
        html = format!("<!doctype html><html><head>{inject}</head><body>{html}</body></html>");
    }
    html
}

fn book_id_of(session: &ReaderSession) -> String {
    let meta = &session.publication().metadata;
    if let Some(id) = &meta.identifier {
        if !id.is_empty() {
            return id.clone();
        }
    }
    format!("{}:{}", meta.title, meta.authors.join(","))
}

fn snapshot_of(session: &ReaderSession) -> ReaderSnapshot {
    let pub_ = session.publication();
    let st = session.state();
    let toc = pub_
        .toc
        .iter()
        .filter_map(|entry| {
            let spine_index = pub_.spine.iter().position(|h| h == &entry.href)? as u32;
            Some(TocItem {
                title: entry.title.clone(),
                spine_index,
            })
        })
        .collect();
    ReaderSnapshot {
        title: pub_.metadata.title.clone(),
        authors: pub_.metadata.authors.clone(),
        spine_index: st.locator.spine_index as u32,
        spine_len: st.spine_len as u32,
        font_scale: st.font_scale,
        toc,
        book_id: book_id_of(session),
    }
}

/// Fábrica: abre publicações a partir de bytes fornecidos pelo app.
#[derive(uniffi::Object)]
pub struct ReaderEngine;

#[uniffi::export]
impl ReaderEngine {
    #[uniffi::constructor]
    pub fn new() -> Arc<Self> {
        Arc::new(Self)
    }

    /// Abre EPUB / HPUB / reader-json (ou formato registrado) a partir dos bytes.
    pub fn open(
        &self,
        name: String,
        bytes: Vec<u8>,
    ) -> Result<Arc<FfiReaderSession>, ReaderFfiError> {
        let publication = default_registry().open(Source { name, bytes })?;
        let session = ReaderSession::new(publication, default_theme());
        Ok(Arc::new(FfiReaderSession {
            inner: Mutex::new(session),
        }))
    }
}

/// Sessão de leitura controlada pela UI nativa.
#[derive(uniffi::Object)]
pub struct FfiReaderSession {
    inner: Mutex<ReaderSession>,
}

#[uniffi::export]
impl FfiReaderSession {
    pub fn snapshot(&self) -> ReaderSnapshot {
        snapshot_of(&self.inner.lock().expect("session lock"))
    }

    pub fn next(&self) -> ReaderSnapshot {
        let mut g = self.inner.lock().expect("session lock");
        g.dispatch(Command::Next);
        snapshot_of(&g)
    }

    pub fn prev(&self) -> ReaderSnapshot {
        let mut g = self.inner.lock().expect("session lock");
        g.dispatch(Command::Prev);
        snapshot_of(&g)
    }

    pub fn go_to(&self, spine_index: u32) -> ReaderSnapshot {
        let mut g = self.inner.lock().expect("session lock");
        g.dispatch(Command::GoTo(Locator {
            spine_index: spine_index as usize,
            progression: 0.0,
        }));
        snapshot_of(&g)
    }

    pub fn set_font_scale(&self, scale: f32) -> ReaderSnapshot {
        let mut g = self.inner.lock().expect("session lock");
        g.dispatch(Command::SetFontScale(scale));
        snapshot_of(&g)
    }

    /// JSON de tokens de tema (arquivos em themes/) aplicado ao conteúdo.
    pub fn set_theme_json(&self, json: String) -> Result<ReaderSnapshot, ReaderFfiError> {
        let theme = ThemeTokens::from_json(&json)?;
        let mut g = self.inner.lock().expect("session lock");
        g.dispatch(Command::SetTheme(theme));
        Ok(snapshot_of(&g))
    }

    /// HTML da página atual com CSS de tema injetado (para WebView).
    pub fn current_html(&self) -> Result<String, ReaderFfiError> {
        let g = self.inner.lock().expect("session lock");
        let st = g.state();
        let href = g
            .publication()
            .spine
            .get(st.locator.spine_index)
            .ok_or_else(|| ReaderFfiError::Engine {
                msg: "spine vazio ou índice inválido".into(),
            })?;
        let bytes = g.publication().read(href)?;
        let html =
            String::from_utf8(bytes).map_err(|e| ReaderFfiError::Engine { msg: e.to_string() })?;
        let css = st.theme.to_content_css(st.font_scale);
        Ok(inject_theme_css(html, &css))
    }

    /// Returns the current reading position for persistence.
    pub fn get_position(&self) -> SavedPosition {
        let g = self.inner.lock().expect("session lock");
        let loc = &g.state().locator;
        SavedPosition {
            spine_index: loc.spine_index as u32,
            progression: loc.progression,
        }
    }

    /// Restores a previously saved reading position.
    pub fn restore_position(&self, position: SavedPosition) -> ReaderSnapshot {
        let mut g = self.inner.lock().expect("session lock");
        g.dispatch(Command::GoTo(Locator {
            spine_index: position.spine_index as usize,
            progression: position.progression,
        }));
        snapshot_of(&g)
    }

    /// Read a resource from the publication by its href.
    /// Use this to serve images, stylesheets, fonts, etc. to WebView.
    pub fn read_resource(&self, href: String) -> Result<Vec<u8>, ReaderFfiError> {
        let g = self.inner.lock().expect("session lock");
        Ok(g.publication().read(&href)?)
    }

    /// List all resources in the publication with their media types.
    pub fn list_resources(&self) -> Vec<ResourceInfo> {
        let g = self.inner.lock().expect("session lock");
        g.publication()
            .resources
            .iter()
            .map(|r| ResourceInfo {
                href: r.href.clone(),
                media_type: r.media_type.clone(),
            })
            .collect()
    }

    /// Get the media type for a resource href, if known.
    pub fn resource_media_type(&self, href: String) -> Option<String> {
        let g = self.inner.lock().expect("session lock");
        g.publication()
            .resources
            .iter()
            .find(|r| r.href == href)
            .map(|r| r.media_type.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_json_bytes_and_navigate() {
        let engine = ReaderEngine::new();
        let bytes = include_bytes!("../../reader-demo/samples/noite-clara.json").to_vec();
        let session = engine
            .open("noite-clara.json".into(), bytes)
            .expect("open json");
        let snap = session.snapshot();
        assert_eq!(snap.title, "Noite Clara");
        assert_eq!(snap.spine_len, 3);
        assert_eq!(snap.spine_index, 0);

        let snap = session.next();
        assert_eq!(snap.spine_index, 1);
        let html = session.current_html().expect("html");
        assert!(html.contains("<style>"));
        assert!(
            html.contains("ponte")
                || html.contains("Ponte")
                || html.contains("Capítulo")
                || html.contains("article")
        );
    }

    #[test]
    fn open_epub_bytes() {
        let engine = ReaderEngine::new();
        let bytes = include_bytes!("../../reader-epub/tests/fixtures/sample.epub").to_vec();
        let session = engine.open("sample.epub".into(), bytes).expect("open epub");
        let snap = session.snapshot();
        assert_eq!(snap.title, "EPUB Fixture");
        assert_eq!(snap.spine_len, 2);
        assert!(!session.current_html().unwrap().is_empty());
    }

    #[test]
    fn get_and_restore_position() {
        let engine = ReaderEngine::new();
        let bytes = include_bytes!("../../reader-demo/samples/noite-clara.json").to_vec();
        let session = engine
            .open("noite-clara.json".into(), bytes)
            .expect("open json");

        session.next();
        session.next();
        let pos = session.get_position();
        assert_eq!(pos.spine_index, 2);
        assert!((pos.progression - 0.0).abs() < f32::EPSILON);

        session.prev();
        assert_eq!(session.snapshot().spine_index, 1);

        let snap = session.restore_position(pos);
        assert_eq!(snap.spine_index, 2);
    }

    #[test]
    fn snapshot_includes_book_id() {
        let engine = ReaderEngine::new();
        let bytes = include_bytes!("../../reader-demo/samples/noite-clara.json").to_vec();
        let session = engine
            .open("noite-clara.json".into(), bytes)
            .expect("open json");
        let snap = session.snapshot();
        assert!(!snap.book_id.is_empty());
    }

    #[test]
    fn restore_position_across_sessions() {
        let engine = ReaderEngine::new();
        let bytes = include_bytes!("../../reader-demo/samples/noite-clara.json").to_vec();

        let session1 = engine
            .open("noite-clara.json".into(), bytes.clone())
            .expect("open json");
        session1.next();
        session1.next();
        let saved_pos = session1.get_position();
        let saved_book_id = session1.snapshot().book_id.clone();

        let session2 = engine
            .open("noite-clara.json".into(), bytes)
            .expect("open json");
        assert_eq!(session2.snapshot().spine_index, 0);
        assert_eq!(session2.snapshot().book_id, saved_book_id);

        let snap = session2.restore_position(saved_pos);
        assert_eq!(snap.spine_index, 2);
    }

    #[test]
    fn list_resources_returns_publication_resources() {
        let engine = ReaderEngine::new();
        let bytes = include_bytes!("../../reader-epub/tests/fixtures/sample.epub").to_vec();
        let session = engine.open("sample.epub".into(), bytes).expect("open epub");
        let resources = session.list_resources();
        assert!(!resources.is_empty());
        assert!(resources.iter().any(|r| r.href.ends_with(".xhtml")));
    }

    #[test]
    fn read_resource_returns_bytes() {
        let engine = ReaderEngine::new();
        let bytes = include_bytes!("../../reader-epub/tests/fixtures/sample.epub").to_vec();
        let session = engine.open("sample.epub".into(), bytes).expect("open epub");
        let resources = session.list_resources();
        let first = resources.first().expect("at least one resource");
        let content = session
            .read_resource(first.href.clone())
            .expect("read resource");
        assert!(!content.is_empty());
    }

    #[test]
    fn resource_media_type_returns_correct_type() {
        let engine = ReaderEngine::new();
        let bytes = include_bytes!("../../reader-epub/tests/fixtures/sample.epub").to_vec();
        let session = engine.open("sample.epub".into(), bytes).expect("open epub");
        let resources = session.list_resources();
        let xhtml_resource = resources
            .iter()
            .find(|r| r.href.ends_with(".xhtml"))
            .expect("xhtml resource");
        let media_type = session
            .resource_media_type(xhtml_resource.href.clone())
            .expect("media type for xhtml");
        assert!(media_type.contains("xhtml") || media_type.contains("html"));
    }

    #[test]
    fn read_nonexistent_resource_returns_error() {
        let engine = ReaderEngine::new();
        let bytes = include_bytes!("../../reader-epub/tests/fixtures/sample.epub").to_vec();
        let session = engine.open("sample.epub".into(), bytes).expect("open epub");
        let result = session.read_resource("nonexistent.png".into());
        assert!(result.is_err());
    }
}
