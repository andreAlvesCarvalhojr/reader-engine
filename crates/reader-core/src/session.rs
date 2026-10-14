use crate::model::{Locator, Publication};
use crate::search::SearchResult;
use crate::theme::ThemeTokens;

#[derive(Debug, Clone)]
pub enum Command {
    Next,
    Prev,
    GoTo(Locator),
    SetTheme(ThemeTokens),
    SetFontScale(f32),
}

/// Estado que a UI nativa observa para se renderizar.
#[derive(Debug, Clone)]
pub struct ReaderState {
    pub locator: Locator,
    pub theme: ThemeTokens,
    pub font_scale: f32,
    pub spine_len: usize,
}

pub struct ReaderSession {
    publication: Publication,
    state: ReaderState,
}

impl ReaderSession {
    pub fn new(publication: Publication, theme: ThemeTokens) -> Self {
        let state = ReaderState {
            locator: Locator::default(),
            theme,
            font_scale: 1.0,
            spine_len: publication.spine.len(),
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
        }
        &self.state
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
}
