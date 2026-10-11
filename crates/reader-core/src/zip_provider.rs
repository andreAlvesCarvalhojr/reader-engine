use crate::error::{Error, Result};
use crate::model::ResourceProvider;
use std::io::{Cursor, Read};

/// Provider sobre bytes de um arquivo ZIP (EPUB, HPUB...).
pub struct ZipProvider {
    bytes: Vec<u8>,
}

impl ZipProvider {
    pub fn new(bytes: Vec<u8>) -> Result<Self> {
        zip::ZipArchive::new(Cursor::new(&bytes[..])).map_err(|e| Error::Invalid(e.to_string()))?;
        Ok(Self { bytes })
    }

    pub fn read_string(&self, path: &str) -> Result<String> {
        String::from_utf8(self.read(path)?).map_err(|e| Error::Invalid(e.to_string()))
    }
}

impl ResourceProvider for ZipProvider {
    fn read(&self, href: &str) -> Result<Vec<u8>> {
        let mut zip = zip::ZipArchive::new(Cursor::new(&self.bytes[..]))
            .map_err(|e| Error::Invalid(e.to_string()))?;
        let mut file = zip
            .by_name(href)
            .map_err(|_| Error::NotFound(href.to_string()))?;
        let mut buf = Vec::new();
        file.read_to_end(&mut buf)
            .map_err(|e| Error::Invalid(e.to_string()))?;
        Ok(buf)
    }
}

pub fn has_zip_entry(bytes: &[u8], path: &str) -> bool {
    zip::ZipArchive::new(Cursor::new(bytes))
        .map(|mut z| z.by_name(path).is_ok())
        .unwrap_or(false)
}
