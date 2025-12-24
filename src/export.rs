#![allow(dead_code)]

use std::fs;

use crate::rawthumb::core::errors::Result;
use crate::rawthumb::core::thumbnail_decoder::ThumbnailDecoder;
use crate::rawthumb::core::types::ThumbnailResult;

pub struct Exporter {
    decoder: ThumbnailDecoder,
}

impl Exporter {
    pub fn new() -> Self {
        Self {
            decoder: ThumbnailDecoder::new(),
        }
    }

    pub fn get_thumbnail<'a>(&self, buffer: &'a [u8]) -> Result<ThumbnailResult<'a>> {
        self.decoder.get_thumbnail(buffer, "")
    }

    pub fn export_thumbnail_data(&self, buffer: &[u8]) -> Result<Vec<u8>> {
        let thumb = self.get_thumbnail(buffer)?;
        Ok(thumb.jpeg.to_vec())
    }

    pub fn export_thumbnail_to_file(&self, buffer: &[u8], path: &str) -> Result<()> {
        let thumb = self.get_thumbnail(buffer)?;
        fs::write(path, thumb.jpeg)?;
        Ok(())
    }
}
