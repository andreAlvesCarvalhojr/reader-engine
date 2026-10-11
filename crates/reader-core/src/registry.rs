use crate::error::{Error, Result};
use crate::model::Publication;
use crate::parser::{FormatParser, Source};

#[derive(Default)]
pub struct Registry {
    parsers: Vec<Box<dyn FormatParser>>,
}

impl Registry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Parsers registrados primeiro têm prioridade na detecção.
    pub fn register(&mut self, parser: Box<dyn FormatParser>) -> &mut Self {
        self.parsers.push(parser);
        self
    }

    pub fn open(&self, src: Source) -> Result<Publication> {
        let parser = self
            .parsers
            .iter()
            .find(|p| p.can_parse(&src))
            .ok_or(Error::UnsupportedFormat)?;
        parser.parse(src)
    }
}
