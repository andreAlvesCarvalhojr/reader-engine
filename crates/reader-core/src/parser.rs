use crate::error::Result;
use crate::model::Publication;

/// Entrada já carregada; a camada de plataforma lê o arquivo/URL e entrega os bytes.
pub struct Source {
    pub name: String,
    pub bytes: Vec<u8>,
}

impl Source {
    pub fn extension(&self) -> Option<String> {
        self.name
            .rsplit_once('.')
            .map(|(_, e)| e.to_ascii_lowercase())
    }
}

/// Plugin de formato. Formatos customizados implementam esta trait.
pub trait FormatParser: Send + Sync {
    fn id(&self) -> &'static str;
    fn can_parse(&self, src: &Source) -> bool;
    fn parse(&self, src: Source) -> Result<Publication>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extension_is_lowercase_suffix() {
        let src = Source {
            name: "Livro.EPUB".into(),
            bytes: vec![],
        };
        assert_eq!(src.extension().as_deref(), Some("epub"));
    }

    #[test]
    fn extension_none_without_dot() {
        let src = Source {
            name: "semext".into(),
            bytes: vec![],
        };
        assert_eq!(src.extension(), None);
    }
}
