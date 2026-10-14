//! Engine headless: modelo unificado, plugins de formato, tema por tokens e estado do leitor.
//! Sem UI e sem I/O de plataforma.

pub mod error;
pub mod model;
pub mod parser;
pub mod registry;
pub mod search;
pub mod session;
pub mod theme;
#[cfg(feature = "zip")]
pub mod zip_provider;

pub use error::{Error, Result};
pub use model::*;
pub use parser::{FormatParser, Source};
pub use registry::Registry;
pub use search::SearchResult;
pub use session::{BookmarkError, Command, ReaderSession, ReaderState};
pub use theme::{DesignSystem, ThemeTokens};
