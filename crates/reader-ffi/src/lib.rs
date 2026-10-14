//! API estável para apps nativos via UniFFI.
//!
//! O app de plataforma lê o arquivo (picker / assets / rede) e entrega
//! `(nome, bytes)` — a engine não faz I/O de filesystem.

uniffi::setup_scaffolding!();

use reader_core::{
    Bookmark, BookmarkError, Command, Locator, ReaderSession, Registry, Source, ThemeTokens,
};
use reader_epub::EpubParser;
use reader_hpub::HpubParser;
use reader_json::JsonParser;
use std::sync::{Arc, Mutex};

/// Erro exposto às linguagens estrangeiras.
#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum ReaderFfiError {
    #[error("{msg}")]
    Engine { msg: String },
    #[error("Bookmark not found: {id}")]
    BookmarkNotFound { id: String },
    #[error("Invalid bookmark JSON: {msg}")]
    InvalidBookmarkJson { msg: String },
}

impl From<reader_core::Error> for ReaderFfiError {
    fn from(value: reader_core::Error) -> Self {
        Self::Engine {
            msg: value.to_string(),
        }
    }
}

impl From<BookmarkError> for ReaderFfiError {
    fn from(value: BookmarkError) -> Self {
        match value {
            BookmarkError::NotFound(id) => Self::BookmarkNotFound { id },
            BookmarkError::InvalidJson(msg) => Self::InvalidBookmarkJson { msg },
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

/// Information about a bookmark for FFI.
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct BookmarkInfo {
    /// Unique identifier for this bookmark.
    pub id: String,
    /// Spine index of the bookmarked position.
    pub spine_index: u32,
    /// Progression within the spine item (0.0..=1.0).
    pub progression: f32,
    /// Optional user-provided label.
    pub label: Option<String>,
    /// Timestamp when the bookmark was created (Unix millis).
    pub created_at: u64,
}

impl From<&Bookmark> for BookmarkInfo {
    fn from(b: &Bookmark) -> Self {
        BookmarkInfo {
            id: b.id.clone(),
            spine_index: b.locator.spine_index as u32,
            progression: b.locator.progression,
            label: b.label.clone(),
            created_at: b.created_at,
        }
    }
}

impl From<Bookmark> for BookmarkInfo {
    fn from(b: Bookmark) -> Self {
        BookmarkInfo::from(&b)
    }
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
    /// Current chapter progress (0.0..=1.0).
    pub chapter_progress: f32,
    /// Overall book progress (0.0..=1.0).
    pub book_progress: f32,
    /// List of all bookmarks in this publication.
    pub bookmarks: Vec<BookmarkInfo>,
    /// Whether there is a bookmark at the current position (same spine_index).
    pub has_bookmark_at_current: bool,
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

fn inject_theme_css(mut html: String, css: &str, initial_scroll: f32) -> String {
    let scroll_script = format!(
        r#"<script>
(function() {{
    var lastReported = 0;
    var scrollTimeout = null;
    
    function getScrollProgress() {{
        var scrollTop = document.documentElement.scrollTop || document.body.scrollTop || 0;
        var scrollHeight = document.documentElement.scrollHeight || document.body.scrollHeight || 1;
        var clientHeight = document.documentElement.clientHeight || document.body.clientHeight || 0;
        var maxScroll = Math.max(0, scrollHeight - clientHeight);
        if (maxScroll === 0) return 0;
        return Math.min(1, Math.max(0, scrollTop / maxScroll));
    }}
    
    function reportProgress() {{
        var progress = getScrollProgress();
        if (Math.abs(progress - lastReported) > 0.01) {{
            lastReported = progress;
            if (window.AndroidScrollProgress) {{
                window.AndroidScrollProgress.reportProgress(progress);
            }}
        }}
    }}
    
    function restoreScroll() {{
        var initialScroll = {initial_scroll};
        if (initialScroll > 0) {{
            var scrollHeight = document.documentElement.scrollHeight || document.body.scrollHeight || 0;
            var clientHeight = document.documentElement.clientHeight || document.body.clientHeight || 0;
            var maxScroll = Math.max(0, scrollHeight - clientHeight);
            var scrollTop = maxScroll * initialScroll;
            window.scrollTo(0, scrollTop);
            lastReported = initialScroll;
        }}
    }}
    
    document.addEventListener('scroll', function() {{
        if (scrollTimeout) clearTimeout(scrollTimeout);
        scrollTimeout = setTimeout(reportProgress, 100);
    }}, {{ passive: true }});
    
    if (document.readyState === 'complete') {{
        restoreScroll();
    }} else {{
        window.addEventListener('load', restoreScroll);
    }}
}})();
</script>"#
    );
    let inject = format!("<style>{css}</style>{scroll_script}");
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
    let bookmarks: Vec<BookmarkInfo> = st.bookmarks.iter().map(BookmarkInfo::from).collect();
    let has_bookmark_at_current = session.has_bookmark_at_current().is_some();
    ReaderSnapshot {
        title: pub_.metadata.title.clone(),
        authors: pub_.metadata.authors.clone(),
        spine_index: st.locator.spine_index as u32,
        spine_len: st.spine_len as u32,
        font_scale: st.font_scale,
        toc,
        book_id: book_id_of(session),
        chapter_progress: session.chapter_progress(),
        book_progress: session.book_progress(),
        bookmarks,
        has_bookmark_at_current,
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

    /// Go to a specific spine item with a specific progression.
    pub fn go_to_with_progression(&self, spine_index: u32, progression: f32) -> ReaderSnapshot {
        let mut g = self.inner.lock().expect("session lock");
        g.dispatch(Command::GoTo(Locator {
            spine_index: spine_index as usize,
            progression,
        }));
        snapshot_of(&g)
    }

    /// Reports the current scroll position within the chapter (0.0..=1.0).
    /// Call this from the client when the user scrolls.
    pub fn report_progression(&self, progression: f32) -> ReaderSnapshot {
        let mut g = self.inner.lock().expect("session lock");
        g.dispatch(Command::ReportProgression(progression));
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
        let initial_scroll = st.locator.progression;
        Ok(inject_theme_css(html, &css, initial_scroll))
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

    /// Add a bookmark at the current position with an optional label.
    /// Returns the newly created bookmark.
    pub fn add_bookmark(&self, label: Option<String>) -> BookmarkInfo {
        let mut g = self.inner.lock().expect("session lock");
        g.add_bookmark(label).into()
    }

    /// Remove a bookmark by its ID.
    /// Returns true if a bookmark was removed, false if no bookmark with that ID existed.
    pub fn remove_bookmark(&self, id: String) -> bool {
        let mut g = self.inner.lock().expect("session lock");
        g.remove_bookmark(&id)
    }

    /// List all bookmarks in this publication.
    pub fn list_bookmarks(&self) -> Vec<BookmarkInfo> {
        let g = self.inner.lock().expect("session lock");
        g.list_bookmarks().iter().map(BookmarkInfo::from).collect()
    }

    /// Navigate to a bookmark by its ID.
    /// Returns the new state after navigation.
    pub fn go_to_bookmark(&self, id: String) -> Result<ReaderSnapshot, ReaderFfiError> {
        let mut g = self.inner.lock().expect("session lock");
        g.go_to_bookmark(&id)?;
        Ok(snapshot_of(&g))
    }

    /// Toggle a bookmark at the current position.
    /// If a bookmark exists at the current spine_index, removes it and returns None.
    /// Otherwise, adds a new bookmark with the given label and returns it.
    pub fn toggle_bookmark(&self, label: Option<String>) -> Option<BookmarkInfo> {
        let mut g = self.inner.lock().expect("session lock");
        g.toggle_bookmark(label).map(BookmarkInfo::from)
    }

    /// Export all bookmarks as a JSON string for persistence.
    pub fn export_bookmarks_json(&self) -> String {
        let g = self.inner.lock().expect("session lock");
        g.export_bookmarks_json()
    }

    /// Import bookmarks from a JSON string.
    /// Replaces the current bookmarks with the imported ones.
    pub fn import_bookmarks_json(&self, json: String) -> Result<ReaderSnapshot, ReaderFfiError> {
        let mut g = self.inner.lock().expect("session lock");
        g.import_bookmarks_json(&json)?;
        Ok(snapshot_of(&g))
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

    #[test]
    fn snapshot_includes_progress_fields() {
        let engine = ReaderEngine::new();
        let bytes = include_bytes!("../../reader-demo/samples/noite-clara.json").to_vec();
        let session = engine
            .open("noite-clara.json".into(), bytes)
            .expect("open json");

        let snap = session.snapshot();
        assert!((snap.chapter_progress - 0.0).abs() < f32::EPSILON);
        assert!((snap.book_progress - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn add_and_list_bookmarks() {
        let engine = ReaderEngine::new();
        let bytes = include_bytes!("../../reader-demo/samples/noite-clara.json").to_vec();
        let session = engine
            .open("noite-clara.json".into(), bytes)
            .expect("open json");

        let snap = session.snapshot();
        assert!(snap.bookmarks.is_empty());
        assert!(!snap.has_bookmark_at_current);

        let bm = session.add_bookmark(Some("First bookmark".into()));
        assert_eq!(bm.spine_index, 0);
        assert_eq!(bm.label, Some("First bookmark".into()));
        assert!(!bm.id.is_empty());

        session.next();
        let bm2 = session.add_bookmark(None);
        assert_eq!(bm2.spine_index, 1);
        assert_eq!(bm2.label, None);

        let bookmarks = session.list_bookmarks();
        assert_eq!(bookmarks.len(), 2);

        let snap = session.snapshot();
        assert_eq!(snap.bookmarks.len(), 2);
    }

    #[test]
    fn report_progression_updates_snapshot() {
        let engine = ReaderEngine::new();
        let bytes = include_bytes!("../../reader-demo/samples/noite-clara.json").to_vec();
        let session = engine
            .open("noite-clara.json".into(), bytes)
            .expect("open json");

        let snap = session.report_progression(0.5);
        assert!((snap.chapter_progress - 0.5).abs() < f32::EPSILON);
        assert!(snap.book_progress > 0.0);
    }

    #[test]
    fn report_progression_clamps_values() {
        let engine = ReaderEngine::new();
        let bytes = include_bytes!("../../reader-demo/samples/noite-clara.json").to_vec();
        let session = engine
            .open("noite-clara.json".into(), bytes)
            .expect("open json");

        let snap = session.report_progression(1.5);
        assert!((snap.chapter_progress - 1.0).abs() < f32::EPSILON);

        let snap = session.report_progression(-0.5);
        assert!((snap.chapter_progress - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn book_progress_advances_with_navigation() {
        let engine = ReaderEngine::new();
        let bytes = include_bytes!("../../reader-demo/samples/noite-clara.json").to_vec();
        let session = engine
            .open("noite-clara.json".into(), bytes)
            .expect("open json");

        // At start
        let snap = session.snapshot();
        assert!((snap.book_progress - 0.0).abs() < f32::EPSILON);

        // After moving to next chapter
        let snap = session.next();
        assert!(snap.book_progress > 0.0);

        // After adding some progression
        let snap = session.report_progression(0.5);
        let expected = (1.0 + 0.5) / 3.0; // (1 completed + 0.5 progress) / 3 total
        assert!((snap.book_progress - expected).abs() < 0.01);
    }

    #[test]
    fn go_to_with_progression_sets_exact_position() {
        let engine = ReaderEngine::new();
        let bytes = include_bytes!("../../reader-demo/samples/noite-clara.json").to_vec();
        let session = engine
            .open("noite-clara.json".into(), bytes)
            .expect("open json");

        let snap = session.go_to_with_progression(1, 0.75);
        assert_eq!(snap.spine_index, 1);
        assert!((snap.chapter_progress - 0.75).abs() < f32::EPSILON);
    }

    #[test]
    fn restore_position_preserves_progression() {
        let engine = ReaderEngine::new();
        let bytes = include_bytes!("../../reader-demo/samples/noite-clara.json").to_vec();
        let session = engine
            .open("noite-clara.json".into(), bytes)
            .expect("open json");

        // Navigate and add progression
        session.next();
        session.report_progression(0.65);
        let pos = session.get_position();

        // Navigate away
        session.prev();
        assert_eq!(session.snapshot().spine_index, 0);

        // Restore position
        let snap = session.restore_position(pos);
        assert_eq!(snap.spine_index, 1);
        assert!((snap.chapter_progress - 0.65).abs() < f32::EPSILON);
    }

    #[test]
    fn saved_position_includes_progression() {
        let engine = ReaderEngine::new();
        let bytes = include_bytes!("../../reader-demo/samples/noite-clara.json").to_vec();
        let session = engine
            .open("noite-clara.json".into(), bytes)
            .expect("open json");

        session.next();
        session.report_progression(0.42);
        let pos = session.get_position();

        assert_eq!(pos.spine_index, 1);
        assert!((pos.progression - 0.42).abs() < f32::EPSILON);
    }

    #[test]
    fn remove_bookmark() {
        let engine = ReaderEngine::new();
        let bytes = include_bytes!("../../reader-demo/samples/noite-clara.json").to_vec();
        let session = engine
            .open("noite-clara.json".into(), bytes)
            .expect("open json");

        let bm = session.add_bookmark(Some("To remove".into()));
        assert_eq!(session.list_bookmarks().len(), 1);

        let removed = session.remove_bookmark(bm.id);
        assert!(removed);
        assert!(session.list_bookmarks().is_empty());

        let removed_again = session.remove_bookmark("nonexistent".into());
        assert!(!removed_again);
    }

    #[test]
    fn go_to_bookmark_navigates() {
        let engine = ReaderEngine::new();
        let bytes = include_bytes!("../../reader-demo/samples/noite-clara.json").to_vec();
        let session = engine
            .open("noite-clara.json".into(), bytes)
            .expect("open json");

        session.next();
        session.next();
        let bm = session.add_bookmark(Some("Chapter 3".into()));
        assert_eq!(session.snapshot().spine_index, 2);

        session.prev();
        session.prev();
        assert_eq!(session.snapshot().spine_index, 0);

        let snap = session.go_to_bookmark(bm.id).expect("go to bookmark");
        assert_eq!(snap.spine_index, 2);
    }

    #[test]
    fn go_to_nonexistent_bookmark_returns_error() {
        let engine = ReaderEngine::new();
        let bytes = include_bytes!("../../reader-demo/samples/noite-clara.json").to_vec();
        let session = engine
            .open("noite-clara.json".into(), bytes)
            .expect("open json");

        let result = session.go_to_bookmark("nonexistent".into());
        assert!(result.is_err());
    }

    #[test]
    fn toggle_bookmark() {
        let engine = ReaderEngine::new();
        let bytes = include_bytes!("../../reader-demo/samples/noite-clara.json").to_vec();
        let session = engine
            .open("noite-clara.json".into(), bytes)
            .expect("open json");

        assert!(!session.snapshot().has_bookmark_at_current);

        let result = session.toggle_bookmark(Some("Toggled".into()));
        assert!(result.is_some());
        assert!(session.snapshot().has_bookmark_at_current);
        assert_eq!(session.list_bookmarks().len(), 1);

        let result = session.toggle_bookmark(None);
        assert!(result.is_none());
        assert!(!session.snapshot().has_bookmark_at_current);
        assert!(session.list_bookmarks().is_empty());
    }

    #[test]
    fn export_and_import_bookmarks_json() {
        let engine = ReaderEngine::new();
        let bytes = include_bytes!("../../reader-demo/samples/noite-clara.json").to_vec();

        let session1 = engine
            .open("noite-clara.json".into(), bytes.clone())
            .expect("open json");
        session1.add_bookmark(Some("First".into()));
        session1.next();
        session1.add_bookmark(Some("Second".into()));
        let json = session1.export_bookmarks_json();

        let session2 = engine
            .open("noite-clara.json".into(), bytes)
            .expect("open json");
        assert!(session2.list_bookmarks().is_empty());

        let snap = session2
            .import_bookmarks_json(json)
            .expect("import bookmarks");
        assert_eq!(snap.bookmarks.len(), 2);
        assert_eq!(session2.list_bookmarks().len(), 2);
    }

    #[test]
    fn import_invalid_json_returns_error() {
        let engine = ReaderEngine::new();
        let bytes = include_bytes!("../../reader-demo/samples/noite-clara.json").to_vec();
        let session = engine
            .open("noite-clara.json".into(), bytes)
            .expect("open json");

        let result = session.import_bookmarks_json("not valid json".into());
        assert!(result.is_err());
    }
}
