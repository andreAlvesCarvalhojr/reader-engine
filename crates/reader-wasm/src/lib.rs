//! API wasm-bindgen para o leitor web estático.
//!
//! O host JS entrega `(nome, bytes)`; a engine não faz I/O de filesystem.

use reader_core::{
    Bookmark, BookmarkError, Command, Locator, ReaderSession, Registry, ThemeTokens,
};
use reader_epub::EpubParser;
use reader_hpub::HpubParser;
use reader_json::JsonParser;
use serde::Serialize;
use std::sync::Mutex;
use wasm_bindgen::prelude::*;

#[derive(Debug, thiserror::Error)]
pub enum WasmError {
    #[error("{0}")]
    Engine(String),
    #[error("Bookmark not found: {0}")]
    BookmarkNotFound(String),
    #[error("Invalid bookmark JSON: {0}")]
    InvalidBookmarkJson(String),
}

impl From<reader_core::Error> for WasmError {
    fn from(value: reader_core::Error) -> Self {
        Self::Engine(value.to_string())
    }
}

impl From<BookmarkError> for WasmError {
    fn from(value: BookmarkError) -> Self {
        match value {
            BookmarkError::NotFound(id) => Self::BookmarkNotFound(id),
            BookmarkError::InvalidJson(msg) => Self::InvalidBookmarkJson(msg),
        }
    }
}

impl From<WasmError> for JsValue {
    fn from(value: WasmError) -> Self {
        JsValue::from_str(&value.to_string())
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct TocItem {
    pub title: String,
    pub spine_index: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct BookmarkInfo {
    pub id: String,
    pub spine_index: u32,
    pub progression: f32,
    pub label: Option<String>,
    pub created_at: u64,
}

impl From<&Bookmark> for BookmarkInfo {
    fn from(b: &Bookmark) -> Self {
        Self {
            id: b.id.clone(),
            spine_index: b.locator.spine_index as u32,
            progression: b.locator.progression,
            label: b.label.clone(),
            created_at: b.created_at,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct SearchResultItem {
    pub spine_index: u32,
    pub progression: f32,
    pub snippet: String,
    pub href: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReaderSnapshot {
    pub title: String,
    pub authors: Vec<String>,
    pub spine_index: u32,
    pub spine_len: u32,
    pub font_scale: f32,
    pub toc: Vec<TocItem>,
    pub book_id: String,
    pub chapter_progress: f32,
    pub book_progress: f32,
    pub bookmarks: Vec<BookmarkInfo>,
    pub has_bookmark_at_current: bool,
    pub theme: String,
}

fn default_registry() -> Registry {
    let mut reg = Registry::new();
    reg.register(Box::new(JsonParser))
        .register(Box::new(EpubParser))
        .register(Box::new(HpubParser));
    reg
}

fn theme_by_name(name: &str) -> Result<ThemeTokens, WasmError> {
    let json = match name {
        "hig" => include_str!("../../../themes/hig.json"),
        "fluent" => include_str!("../../../themes/fluent.json"),
        _ => include_str!("../../../themes/material3.json"),
    };
    ThemeTokens::from_json(json).map_err(WasmError::from)
}

fn theme_name(tokens: &ThemeTokens) -> String {
    match tokens.design_system {
        reader_core::DesignSystem::Hig => "hig".into(),
        reader_core::DesignSystem::Fluent => "fluent".into(),
        reader_core::DesignSystem::Material3 => "material3".into(),
        reader_core::DesignSystem::Custom => "custom".into(),
    }
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
            window.parent.postMessage({{ type: 'reader-progress', progression: progress }}, '*');
        }}
    }}
    function restoreScroll() {{
        var initialScroll = {initial_scroll};
        if (initialScroll > 0) {{
            var scrollHeight = document.documentElement.scrollHeight || document.body.scrollHeight || 0;
            var clientHeight = document.documentElement.clientHeight || document.body.clientHeight || 0;
            var maxScroll = Math.max(0, scrollHeight - clientHeight);
            window.scrollTo(0, maxScroll * initialScroll);
            lastReported = initialScroll;
        }}
    }}
    document.addEventListener('scroll', function() {{
        if (scrollTimeout) clearTimeout(scrollTimeout);
        scrollTimeout = setTimeout(reportProgress, 100);
    }}, {{ passive: true }});
    if (document.readyState === 'complete') restoreScroll();
    else window.addEventListener('load', restoreScroll);
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
        bookmarks: st.bookmarks.iter().map(BookmarkInfo::from).collect(),
        has_bookmark_at_current: session.has_bookmark_at_current().is_some(),
        theme: theme_name(&st.theme),
    }
}

fn to_js<T: Serialize>(value: &T) -> Result<JsValue, JsValue> {
    serde_wasm_bindgen::to_value(value).map_err(|e| JsValue::from_str(&e.to_string()))
}

fn open_session(name: String, bytes: Vec<u8>) -> Result<ReaderSession, WasmError> {
    let publication = default_registry().open(reader_core::Source { name, bytes })?;
    Ok(ReaderSession::new(publication, theme_by_name("material3")?))
}

/// Fábrica WASM: abre publicações a partir de bytes do browser.
#[wasm_bindgen]
pub struct WasmReaderEngine;

#[wasm_bindgen]
impl WasmReaderEngine {
    #[wasm_bindgen(constructor)]
    pub fn new() -> WasmReaderEngine {
        WasmReaderEngine
    }

    /// Abre EPUB / HPUB / reader-json a partir dos bytes (Uint8Array no JS).
    #[wasm_bindgen]
    pub fn open(&self, name: &str, bytes: &[u8]) -> Result<WasmReaderSession, JsValue> {
        let session = open_session(name.to_string(), bytes.to_vec())?;
        Ok(WasmReaderSession {
            inner: Mutex::new(session),
        })
    }
}

impl Default for WasmReaderEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Sessão de leitura controlada pela UI web.
#[wasm_bindgen]
pub struct WasmReaderSession {
    inner: Mutex<ReaderSession>,
}

#[wasm_bindgen]
impl WasmReaderSession {
    #[wasm_bindgen]
    pub fn snapshot(&self) -> Result<JsValue, JsValue> {
        let g = self.inner.lock().expect("session lock");
        to_js(&snapshot_of(&g))
    }

    #[wasm_bindgen]
    pub fn next(&self) -> Result<JsValue, JsValue> {
        let mut g = self.inner.lock().expect("session lock");
        g.dispatch(Command::Next);
        to_js(&snapshot_of(&g))
    }

    #[wasm_bindgen]
    pub fn prev(&self) -> Result<JsValue, JsValue> {
        let mut g = self.inner.lock().expect("session lock");
        g.dispatch(Command::Prev);
        to_js(&snapshot_of(&g))
    }

    #[wasm_bindgen(js_name = goTo)]
    pub fn go_to(&self, spine_index: u32) -> Result<JsValue, JsValue> {
        let mut g = self.inner.lock().expect("session lock");
        g.dispatch(Command::GoTo(Locator {
            spine_index: spine_index as usize,
            progression: 0.0,
        }));
        to_js(&snapshot_of(&g))
    }

    #[wasm_bindgen(js_name = goToWithProgression)]
    pub fn go_to_with_progression(
        &self,
        spine_index: u32,
        progression: f32,
    ) -> Result<JsValue, JsValue> {
        let mut g = self.inner.lock().expect("session lock");
        g.dispatch(Command::GoTo(Locator {
            spine_index: spine_index as usize,
            progression,
        }));
        to_js(&snapshot_of(&g))
    }

    #[wasm_bindgen(js_name = reportProgression)]
    pub fn report_progression(&self, progression: f32) -> Result<JsValue, JsValue> {
        let mut g = self.inner.lock().expect("session lock");
        g.dispatch(Command::ReportProgression(progression));
        to_js(&snapshot_of(&g))
    }

    #[wasm_bindgen(js_name = setFontScale)]
    pub fn set_font_scale(&self, scale: f32) -> Result<JsValue, JsValue> {
        let mut g = self.inner.lock().expect("session lock");
        g.dispatch(Command::SetFontScale(scale));
        to_js(&snapshot_of(&g))
    }

    /// Tema por nome: `material3` | `hig` | `fluent`.
    #[wasm_bindgen(js_name = setTheme)]
    pub fn set_theme(&self, name: &str) -> Result<JsValue, JsValue> {
        let theme = theme_by_name(name)?;
        let mut g = self.inner.lock().expect("session lock");
        g.dispatch(Command::SetTheme(theme));
        to_js(&snapshot_of(&g))
    }

    /// HTML da página atual com CSS de tema (para iframe/`srcdoc`).
    #[wasm_bindgen(js_name = currentHtml)]
    pub fn current_html(&self) -> Result<String, JsValue> {
        let g = self.inner.lock().expect("session lock");
        let st = g.state();
        let href = g
            .publication()
            .spine
            .get(st.locator.spine_index)
            .ok_or_else(|| WasmError::Engine("spine vazio ou índice inválido".into()))?;
        let bytes = g.publication().read(href).map_err(WasmError::from)?;
        let html = String::from_utf8(bytes).map_err(|e| WasmError::Engine(e.to_string()))?;
        let css = st.theme.to_content_css(st.font_scale);
        Ok(inject_theme_css(html, &css, st.locator.progression))
    }

    #[wasm_bindgen]
    pub fn search(&self, query: &str) -> Result<JsValue, JsValue> {
        let g = self.inner.lock().expect("session lock");
        let results: Vec<SearchResultItem> = g
            .search(query)
            .into_iter()
            .map(|r| SearchResultItem {
                spine_index: r.spine_index as u32,
                progression: r.progression,
                snippet: r.snippet,
                href: r.href,
            })
            .collect();
        to_js(&results)
    }

    #[wasm_bindgen(js_name = toggleBookmark)]
    pub fn toggle_bookmark(&self, label: Option<String>) -> Result<JsValue, JsValue> {
        let mut g = self.inner.lock().expect("session lock");
        let info = g.toggle_bookmark(label).map(|b| BookmarkInfo::from(&b));
        to_js(&info)
    }

    #[wasm_bindgen(js_name = removeBookmark)]
    pub fn remove_bookmark(&self, id: &str) -> bool {
        let mut g = self.inner.lock().expect("session lock");
        g.remove_bookmark(id)
    }

    #[wasm_bindgen(js_name = goToBookmark)]
    pub fn go_to_bookmark(&self, id: &str) -> Result<JsValue, JsValue> {
        let mut g = self.inner.lock().expect("session lock");
        g.go_to_bookmark(id).map_err(WasmError::from)?;
        to_js(&snapshot_of(&g))
    }

    #[wasm_bindgen(js_name = exportBookmarksJson)]
    pub fn export_bookmarks_json(&self) -> String {
        let g = self.inner.lock().expect("session lock");
        g.export_bookmarks_json()
    }

    #[wasm_bindgen(js_name = importBookmarksJson)]
    pub fn import_bookmarks_json(&self, json: &str) -> Result<JsValue, JsValue> {
        let mut g = self.inner.lock().expect("session lock");
        g.import_bookmarks_json(json).map_err(WasmError::from)?;
        to_js(&snapshot_of(&g))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_json_and_navigate() {
        let bytes = include_bytes!("../../reader-demo/samples/noite-clara.json").to_vec();
        let session = open_session("noite-clara.json".into(), bytes).expect("open");
        let snap = snapshot_of(&session);
        assert_eq!(snap.title, "Noite Clara");
        assert_eq!(snap.spine_len, 3);

        let mut session = session;
        session.dispatch(Command::Next);
        assert_eq!(session.state().locator.spine_index, 1);

        let html = {
            let st = session.state();
            let href = &session.publication().spine[st.locator.spine_index];
            let bytes = session.publication().read(href).unwrap();
            let raw = String::from_utf8(bytes).unwrap();
            inject_theme_css(raw, &st.theme.to_content_css(st.font_scale), 0.0)
        };
        assert!(html.contains("<style>"));
        assert!(html.contains("reader-progress"));
    }

    #[test]
    fn open_epub_bytes() {
        let bytes = include_bytes!("../../reader-epub/tests/fixtures/sample.epub").to_vec();
        let session = open_session("sample.epub".into(), bytes).expect("open epub");
        assert!(session.state().spine_len >= 1);
    }
}
