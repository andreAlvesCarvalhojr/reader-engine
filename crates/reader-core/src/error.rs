use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("nenhum parser reconhece este formato")]
    UnsupportedFormat,
    #[error("arquivo inválido: {0}")]
    Invalid(String),
    #[error("recurso não encontrado: {0}")]
    NotFound(String),
}

pub type Result<T> = std::result::Result<T, Error>;
