use crate::model::{Locator, Publication};
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
