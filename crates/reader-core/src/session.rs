use crate::model::{Bookmark, Locator, Publication};
use crate::search::SearchResult;
use crate::theme::ThemeTokens;
use std::time::{SystemTime, UNIX_EPOCH};

/// Error type for bookmark operations.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum BookmarkError {
    #[error("Bookmark not found: {0}")]
    NotFound(String),
    #[error("Invalid bookmark JSON: {0}")]
    InvalidJson(String),
}

#[derive(Debug, Clone)]
pub enum Command {
    Next,
    Prev,
    GoTo(Locator),
    SetTheme(ThemeTokens),
    SetFontScale(f32),
    /// Update the progression (0.0..=1.0) within the current spine item.
    ReportProgression(f32),
}

/// Estado que a UI nativa observa para se renderizar.
#[derive(Debug, Clone)]
pub struct ReaderState {
    pub locator: Locator,
    pub theme: ThemeTokens,
    pub font_scale: f32,
    pub spine_len: usize,
    pub bookmarks: Vec<Bookmark>,
}

pub struct ReaderSession {
    publication: Publication,
    state: ReaderState,
}

fn generate_bookmark_id() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    format!("bm-{}-{}", now.as_millis(), now.subsec_nanos())
}

fn current_timestamp_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

impl ReaderSession {
    pub fn new(publication: Publication, theme: ThemeTokens) -> Self {
        let state = ReaderState {
            locator: Locator::default(),
            theme,
            font_scale: 1.0,
            spine_len: publication.spine.len(),
            bookmarks: Vec::new(),
        };
        Self { publication, state }
    }

    pub fn state(&self) -> &ReaderState {
        &self.state
    }

    pub fn publication(&self) -> &Publication {
        &self.publication
    }

    /// Searches for `query` across all spine items.
    ///
    /// Returns results with spine index, approximate progression within each chapter,
    /// and a snippet of text around each match. Search is case-insensitive and
    /// accent-insensitive.
    pub fn search(&self, query: &str) -> Vec<SearchResult> {
        self.publication.search(query)
    }

    pub fn dispatch(&mut self, cmd: Command) -> &ReaderState {
        let last = self.state.spine_len.saturating_sub(1);
        match cmd {
            Command::Next => {
                self.state.locator = Locator {
                    spine_index: (self.state.locator.spine_index + 1).min(last),
                    progression: 0.0,
                }
            }
            Command::Prev => {
                self.state.locator = Locator {
                    spine_index: self.state.locator.spine_index.saturating_sub(1),
                    progression: 0.0,
                }
            }
            Command::GoTo(mut l) => {
                l.spine_index = l.spine_index.min(last);
                l.progression = l.progression.clamp(0.0, 1.0);
                self.state.locator = l;
            }
            Command::SetTheme(t) => self.state.theme = t,
            Command::SetFontScale(s) => self.state.font_scale = s.clamp(0.5, 3.0),
            Command::ReportProgression(p) => {
                self.state.locator.progression = p.clamp(0.0, 1.0);
            }
        }
        &self.state
    }

    /// Returns the current chapter progress as a value in 0.0..=1.0.
    pub fn chapter_progress(&self) -> f32 {
        self.state.locator.progression
    }

    /// Returns the overall book progress as a value in 0.0..=1.0.
    /// This is calculated as (completed_chapters + current_chapter_progress) / total_chapters.
    pub fn book_progress(&self) -> f32 {
        if self.state.spine_len == 0 {
            return 0.0;
        }
        let completed = self.state.locator.spine_index as f32;
        let current = self.state.locator.progression;
        (completed + current) / self.state.spine_len as f32
    }

    /// Adds a bookmark at the current position with an optional label.
    /// Returns the newly created bookmark.
    pub fn add_bookmark(&mut self, label: Option<String>) -> Bookmark {
        let bookmark = Bookmark {
            id: generate_bookmark_id(),
            locator: self.state.locator.clone(),
            label,
            created_at: current_timestamp_millis(),
        };
        self.state.bookmarks.push(bookmark.clone());
        bookmark
    }

    /// Removes a bookmark by its ID. Returns true if a bookmark was removed.
    pub fn remove_bookmark(&mut self, id: &str) -> bool {
        let initial_len = self.state.bookmarks.len();
        self.state.bookmarks.retain(|b| b.id != id);
        self.state.bookmarks.len() < initial_len
    }

    /// Returns a reference to all bookmarks.
    pub fn list_bookmarks(&self) -> &[Bookmark] {
        &self.state.bookmarks
    }

    /// Navigates to a bookmark by its ID.
    /// Returns the new state, or an error if the bookmark is not found.
    pub fn go_to_bookmark(&mut self, id: &str) -> Result<&ReaderState, BookmarkError> {
        let bookmark = self
            .state
            .bookmarks
            .iter()
            .find(|b| b.id == id)
            .ok_or_else(|| BookmarkError::NotFound(id.to_string()))?;
        let locator = bookmark.locator.clone();
        self.dispatch(Command::GoTo(locator));
        Ok(&self.state)
    }

    /// Exports all bookmarks as a JSON string for persistence.
    pub fn export_bookmarks_json(&self) -> String {
        serde_json::to_string(&self.state.bookmarks).unwrap_or_else(|_| "[]".to_string())
    }

    /// Imports bookmarks from a JSON string.
    /// Replaces the current bookmarks with the imported ones.
    pub fn import_bookmarks_json(&mut self, json: &str) -> Result<(), BookmarkError> {
        let bookmarks: Vec<Bookmark> =
            serde_json::from_str(json).map_err(|e| BookmarkError::InvalidJson(e.to_string()))?;
        self.state.bookmarks = bookmarks;
        Ok(())
    }

    /// Checks if there is a bookmark at the current position (same spine_index).
    pub fn has_bookmark_at_current(&self) -> Option<&Bookmark> {
        self.state
            .bookmarks
            .iter()
            .find(|b| b.locator.spine_index == self.state.locator.spine_index)
    }

    /// Toggles a bookmark at the current position.
    /// If a bookmark exists at this spine_index, removes it and returns None.
    /// Otherwise, adds a new bookmark with the given label and returns it.
    pub fn toggle_bookmark(&mut self, label: Option<String>) -> Option<Bookmark> {
        let existing_id = self
            .state
            .bookmarks
            .iter()
            .find(|b| b.locator.spine_index == self.state.locator.spine_index)
            .map(|b| b.id.clone());

        if let Some(id) = existing_id {
            self.remove_bookmark(&id);
            None
        } else {
            Some(self.add_bookmark(label))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Metadata, Publication, ResourceProvider};
    use crate::theme::{Colors, DesignSystem, Shape, Typography};
    use crate::Error;
    use std::collections::HashMap;
    use std::sync::Arc;

    struct Mem(HashMap<String, Vec<u8>>);

    impl ResourceProvider for Mem {
        fn read(&self, href: &str) -> crate::Result<Vec<u8>> {
            self.0
                .get(href)
                .cloned()
                .ok_or_else(|| Error::NotFound(href.into()))
        }
    }

    fn theme() -> ThemeTokens {
        ThemeTokens {
            design_system: DesignSystem::Custom,
            colors: Colors {
                background: "#111".into(),
                surface: "#222".into(),
                on_surface: "#eee".into(),
                primary: "#6af".into(),
                on_primary: "#000".into(),
            },
            typography: Typography {
                font_family: "serif".into(),
                base_size_px: 16.0,
                line_height: 1.5,
            },
            shape: Shape {
                corner_radius_px: 8.0,
                spacing_unit_px: 8.0,
            },
        }
    }

    fn pub_with_pages(n: usize) -> Publication {
        let spine: Vec<String> = (0..n).map(|i| format!("p{i}.html")).collect();
        let resources = spine
            .iter()
            .map(|h| crate::model::Resource {
                href: h.clone(),
                media_type: "text/html".into(),
            })
            .collect();
        let mut files = HashMap::new();
        for h in &spine {
            files.insert(h.clone(), b"<p>x</p>".to_vec());
        }
        Publication {
            metadata: Metadata {
                title: "Demo".into(),
                authors: vec![],
                language: Some("pt".into()),
                identifier: None,
            },
            spine,
            resources,
            toc: vec![],
            provider: Arc::new(Mem(files)),
        }
    }

    #[test]
    fn starts_at_first_page() {
        let s = ReaderSession::new(pub_with_pages(3), theme());
        assert_eq!(s.state().locator.spine_index, 0);
        assert_eq!(s.state().spine_len, 3);
        assert_eq!(s.state().font_scale, 1.0);
    }

    #[test]
    fn next_and_prev_navigate_spine() {
        let mut s = ReaderSession::new(pub_with_pages(3), theme());
        s.dispatch(Command::Next);
        assert_eq!(s.state().locator.spine_index, 1);
        s.dispatch(Command::Next);
        assert_eq!(s.state().locator.spine_index, 2);
        s.dispatch(Command::Next);
        assert_eq!(s.state().locator.spine_index, 2, "não passa do fim");
        s.dispatch(Command::Prev);
        assert_eq!(s.state().locator.spine_index, 1);
        s.dispatch(Command::Prev);
        s.dispatch(Command::Prev);
        assert_eq!(s.state().locator.spine_index, 0, "não passa do início");
    }

    #[test]
    fn goto_clamps_index_and_progression() {
        let mut s = ReaderSession::new(pub_with_pages(2), theme());
        s.dispatch(Command::GoTo(Locator {
            spine_index: 99,
            progression: 1.5,
        }));
        assert_eq!(s.state().locator.spine_index, 1);
        assert_eq!(s.state().locator.progression, 1.0);

        s.dispatch(Command::GoTo(Locator {
            spine_index: 0,
            progression: -0.2,
        }));
        assert_eq!(s.state().locator.spine_index, 0);
        assert_eq!(s.state().locator.progression, 0.0);
    }

    #[test]
    fn font_scale_is_clamped() {
        let mut s = ReaderSession::new(pub_with_pages(1), theme());
        s.dispatch(Command::SetFontScale(10.0));
        assert_eq!(s.state().font_scale, 3.0);
        s.dispatch(Command::SetFontScale(0.1));
        assert_eq!(s.state().font_scale, 0.5);
    }

    #[test]
    fn empty_spine_stays_at_zero() {
        let mut s = ReaderSession::new(pub_with_pages(0), theme());
        assert_eq!(s.state().spine_len, 0);
        s.dispatch(Command::Next);
        assert_eq!(s.state().locator.spine_index, 0);
    }

    #[test]
    fn locator_can_be_serialized_and_restored() {
        let mut s = ReaderSession::new(pub_with_pages(5), theme());
        s.dispatch(Command::GoTo(Locator {
            spine_index: 3,
            progression: 0.75,
        }));

        let locator = s.state().locator.clone();
        let json = serde_json::to_string(&locator).expect("serialize locator");
        let restored: Locator = serde_json::from_str(&json).expect("deserialize locator");

        assert_eq!(restored.spine_index, 3);
        assert!((restored.progression - 0.75).abs() < f32::EPSILON);
    }

    #[test]
    fn restore_position_after_session_recreate() {
        let mut s1 = ReaderSession::new(pub_with_pages(5), theme());
        s1.dispatch(Command::GoTo(Locator {
            spine_index: 2,
            progression: 0.5,
        }));
        let saved_locator = s1.state().locator.clone();

        let mut s2 = ReaderSession::new(pub_with_pages(5), theme());
        assert_eq!(s2.state().locator.spine_index, 0);

        s2.dispatch(Command::GoTo(saved_locator));
        assert_eq!(s2.state().locator.spine_index, 2);
        assert!((s2.state().locator.progression - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn restore_position_clamps_to_valid_range() {
        let mut s = ReaderSession::new(pub_with_pages(3), theme());
        s.dispatch(Command::GoTo(Locator {
            spine_index: 100,
            progression: 2.0,
        }));
        assert_eq!(s.state().locator.spine_index, 2);
        assert!((s.state().locator.progression - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn report_progression_updates_locator() {
        let mut s = ReaderSession::new(pub_with_pages(3), theme());
        s.dispatch(Command::ReportProgression(0.5));
        assert!((s.state().locator.progression - 0.5).abs() < f32::EPSILON);
        assert_eq!(s.state().locator.spine_index, 0);
    }

    #[test]
    fn report_progression_clamps_value() {
        let mut s = ReaderSession::new(pub_with_pages(3), theme());
        s.dispatch(Command::ReportProgression(1.5));
        assert!((s.state().locator.progression - 1.0).abs() < f32::EPSILON);

        s.dispatch(Command::ReportProgression(-0.5));
        assert!((s.state().locator.progression - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn chapter_progress_returns_current_progression() {
        let mut s = ReaderSession::new(pub_with_pages(3), theme());
        assert!((s.chapter_progress() - 0.0).abs() < f32::EPSILON);

        s.dispatch(Command::ReportProgression(0.75));
        assert!((s.chapter_progress() - 0.75).abs() < f32::EPSILON);
    }

    #[test]
    fn book_progress_calculation() {
        let mut s = ReaderSession::new(pub_with_pages(4), theme());
        // At start: 0 / 4 = 0.0
        assert!((s.book_progress() - 0.0).abs() < f32::EPSILON);

        // Go to chapter 2 (index 1), no progression: 1 / 4 = 0.25
        s.dispatch(Command::Next);
        assert!((s.book_progress() - 0.25).abs() < f32::EPSILON);

        // At chapter 2 with 50% progression: (1 + 0.5) / 4 = 0.375
        s.dispatch(Command::ReportProgression(0.5));
        assert!((s.book_progress() - 0.375).abs() < f32::EPSILON);

        // Go to last chapter (index 3), no progression: 3 / 4 = 0.75
        s.dispatch(Command::GoTo(Locator {
            spine_index: 3,
            progression: 0.0,
        }));
        assert!((s.book_progress() - 0.75).abs() < f32::EPSILON);

        // At last chapter with 100% progression: (3 + 1.0) / 4 = 1.0
        s.dispatch(Command::ReportProgression(1.0));
        assert!((s.book_progress() - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn book_progress_empty_spine() {
        let s = ReaderSession::new(pub_with_pages(0), theme());
        assert!((s.book_progress() - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn progression_preserved_across_goto() {
        let mut s = ReaderSession::new(pub_with_pages(3), theme());
        s.dispatch(Command::GoTo(Locator {
            spine_index: 1,
            progression: 0.6,
        }));
        assert_eq!(s.state().locator.spine_index, 1);
        assert!((s.state().locator.progression - 0.6).abs() < f32::EPSILON);
    }

    #[test]
    fn next_resets_progression_to_zero() {
        let mut s = ReaderSession::new(pub_with_pages(3), theme());
        s.dispatch(Command::ReportProgression(0.8));
        s.dispatch(Command::Next);
        assert_eq!(s.state().locator.spine_index, 1);
        assert!((s.state().locator.progression - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn prev_resets_progression_to_zero() {
        let mut s = ReaderSession::new(pub_with_pages(3), theme());
        s.dispatch(Command::Next);
        s.dispatch(Command::ReportProgression(0.8));
        s.dispatch(Command::Prev);
        assert_eq!(s.state().locator.spine_index, 0);
        assert!((s.state().locator.progression - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn add_bookmark_at_current_position() {
        let mut s = ReaderSession::new(pub_with_pages(5), theme());
        s.dispatch(Command::GoTo(Locator {
            spine_index: 2,
            progression: 0.5,
        }));

        let bookmark = s.add_bookmark(Some("My bookmark".to_string()));
        assert_eq!(bookmark.locator.spine_index, 2);
        assert!((bookmark.locator.progression - 0.5).abs() < f32::EPSILON);
        assert_eq!(bookmark.label, Some("My bookmark".to_string()));
        assert!(bookmark.created_at > 0);
        assert!(!bookmark.id.is_empty());
    }

    #[test]
    fn add_bookmark_without_label() {
        let mut s = ReaderSession::new(pub_with_pages(3), theme());
        let bookmark = s.add_bookmark(None);
        assert_eq!(bookmark.label, None);
        assert_eq!(bookmark.locator.spine_index, 0);
    }

    #[test]
    fn list_bookmarks_returns_all() {
        let mut s = ReaderSession::new(pub_with_pages(5), theme());
        s.add_bookmark(Some("First".to_string()));
        s.dispatch(Command::Next);
        s.add_bookmark(Some("Second".to_string()));
        s.dispatch(Command::Next);
        s.add_bookmark(None);

        let bookmarks = s.list_bookmarks();
        assert_eq!(bookmarks.len(), 3);
        assert_eq!(bookmarks[0].label, Some("First".to_string()));
        assert_eq!(bookmarks[1].label, Some("Second".to_string()));
        assert_eq!(bookmarks[2].label, None);
    }

    #[test]
    fn remove_bookmark_by_id() {
        let mut s = ReaderSession::new(pub_with_pages(3), theme());
        let bm1 = s.add_bookmark(Some("First".to_string()));
        s.dispatch(Command::Next);
        let _bm2 = s.add_bookmark(Some("Second".to_string()));

        assert_eq!(s.list_bookmarks().len(), 2);
        let removed = s.remove_bookmark(&bm1.id);
        assert!(removed);
        assert_eq!(s.list_bookmarks().len(), 1);
        assert_eq!(s.list_bookmarks()[0].label, Some("Second".to_string()));
    }

    #[test]
    fn remove_nonexistent_bookmark_returns_false() {
        let mut s = ReaderSession::new(pub_with_pages(3), theme());
        s.add_bookmark(Some("First".to_string()));
        let removed = s.remove_bookmark("nonexistent-id");
        assert!(!removed);
        assert_eq!(s.list_bookmarks().len(), 1);
    }

    #[test]
    fn go_to_bookmark_navigates() {
        let mut s = ReaderSession::new(pub_with_pages(5), theme());
        s.dispatch(Command::GoTo(Locator {
            spine_index: 3,
            progression: 0.75,
        }));
        let bookmark = s.add_bookmark(Some("Chapter 3".to_string()));

        s.dispatch(Command::GoTo(Locator {
            spine_index: 0,
            progression: 0.0,
        }));
        assert_eq!(s.state().locator.spine_index, 0);

        let state = s.go_to_bookmark(&bookmark.id).unwrap();
        assert_eq!(state.locator.spine_index, 3);
        assert!((state.locator.progression - 0.75).abs() < f32::EPSILON);
    }

    #[test]
    fn go_to_nonexistent_bookmark_returns_error() {
        let mut s = ReaderSession::new(pub_with_pages(3), theme());
        let result = s.go_to_bookmark("nonexistent-id");
        assert!(result.is_err());
        assert!(matches!(result, Err(BookmarkError::NotFound(_))));
    }

    #[test]
    fn export_and_import_bookmarks_json() {
        let mut s1 = ReaderSession::new(pub_with_pages(5), theme());
        s1.add_bookmark(Some("Chapter 1".to_string()));
        s1.dispatch(Command::GoTo(Locator {
            spine_index: 2,
            progression: 0.5,
        }));
        s1.add_bookmark(None);

        let json = s1.export_bookmarks_json();
        assert!(json.contains("Chapter 1"));

        let mut s2 = ReaderSession::new(pub_with_pages(5), theme());
        assert!(s2.list_bookmarks().is_empty());
        s2.import_bookmarks_json(&json).unwrap();
        assert_eq!(s2.list_bookmarks().len(), 2);
        assert_eq!(s2.list_bookmarks()[0].label, Some("Chapter 1".to_string()));
    }

    #[test]
    fn import_invalid_json_returns_error() {
        let mut s = ReaderSession::new(pub_with_pages(3), theme());
        let result = s.import_bookmarks_json("not valid json");
        assert!(result.is_err());
        assert!(matches!(result, Err(BookmarkError::InvalidJson(_))));
    }

    #[test]
    fn has_bookmark_at_current_returns_bookmark() {
        let mut s = ReaderSession::new(pub_with_pages(5), theme());
        assert!(s.has_bookmark_at_current().is_none());

        let bm = s.add_bookmark(Some("Here".to_string()));
        let found = s.has_bookmark_at_current();
        assert!(found.is_some());
        assert_eq!(found.unwrap().id, bm.id);

        s.dispatch(Command::Next);
        assert!(s.has_bookmark_at_current().is_none());
    }

    #[test]
    fn toggle_bookmark_adds_when_none() {
        let mut s = ReaderSession::new(pub_with_pages(3), theme());
        assert!(s.has_bookmark_at_current().is_none());

        let result = s.toggle_bookmark(Some("Toggled".to_string()));
        assert!(result.is_some());
        assert_eq!(result.unwrap().label, Some("Toggled".to_string()));
        assert!(s.has_bookmark_at_current().is_some());
    }

    #[test]
    fn toggle_bookmark_removes_when_exists() {
        let mut s = ReaderSession::new(pub_with_pages(3), theme());
        s.add_bookmark(Some("Existing".to_string()));
        assert!(s.has_bookmark_at_current().is_some());

        let result = s.toggle_bookmark(None);
        assert!(result.is_none());
        assert!(s.has_bookmark_at_current().is_none());
    }

    #[test]
    fn bookmarks_included_in_state() {
        let mut s = ReaderSession::new(pub_with_pages(3), theme());
        assert!(s.state().bookmarks.is_empty());

        s.add_bookmark(Some("Test".to_string()));
        assert_eq!(s.state().bookmarks.len(), 1);
        assert_eq!(s.state().bookmarks[0].label, Some("Test".to_string()));
    }

    #[test]
    fn bookmark_can_be_serialized() {
        let mut s = ReaderSession::new(pub_with_pages(3), theme());
        let bm = s.add_bookmark(Some("Test".to_string()));

        let json = serde_json::to_string(&bm).expect("serialize bookmark");
        let restored: crate::model::Bookmark =
            serde_json::from_str(&json).expect("deserialize bookmark");

        assert_eq!(restored.id, bm.id);
        assert_eq!(restored.label, bm.label);
        assert_eq!(restored.locator.spine_index, bm.locator.spine_index);
    }
}
