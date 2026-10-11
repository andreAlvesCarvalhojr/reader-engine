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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Metadata, ResourceProvider};
    use crate::Error;
    use std::sync::Arc;

    struct StubProvider;

    impl ResourceProvider for StubProvider {
        fn read(&self, href: &str) -> Result<Vec<u8>> {
            Err(Error::NotFound(href.into()))
        }
    }

    struct AlwaysJson;

    impl FormatParser for AlwaysJson {
        fn id(&self) -> &'static str {
            "always-json"
        }

        fn can_parse(&self, src: &Source) -> bool {
            src.extension().as_deref() == Some("json")
        }

        fn parse(&self, src: Source) -> Result<Publication> {
            Ok(Publication {
                metadata: Metadata {
                    title: src.name,
                    authors: vec![],
                    language: None,
                    identifier: None,
                },
                spine: vec![],
                resources: vec![],
                toc: vec![],
                provider: Arc::new(StubProvider),
            })
        }
    }

    struct Never;

    impl FormatParser for Never {
        fn id(&self) -> &'static str {
            "never"
        }
        fn can_parse(&self, _: &Source) -> bool {
            false
        }
        fn parse(&self, _: Source) -> Result<Publication> {
            unreachable!()
        }
    }

    #[test]
    fn open_uses_first_matching_parser() {
        let mut reg = Registry::new();
        reg.register(Box::new(AlwaysJson))
            .register(Box::new(Never));
        let pub_ = reg
            .open(Source {
                name: "book.json".into(),
                bytes: b"{}".to_vec(),
            })
            .unwrap();
        assert_eq!(pub_.metadata.title, "book.json");
    }

    #[test]
    fn unsupported_when_no_parser_matches() {
        let mut reg = Registry::new();
        reg.register(Box::new(Never));
        let result = reg.open(Source {
            name: "x.bin".into(),
            bytes: vec![1, 2, 3],
        });
        assert!(matches!(result, Err(Error::UnsupportedFormat)));
    }
}
