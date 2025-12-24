#![allow(dead_code)]

use crate::rawthumb::core::errors::Result;
use crate::rawthumb::core::types::{RawMetadata, ThumbnailResult};

pub trait ThumbnailExtractor: Send + Sync {
    fn supports_make(&self, make: &str) -> bool;
    fn extract<'a>(
        &self,
        buffer: &'a [u8],
        info: &RawMetadata,
        parsed: quickexif::ParsedInfo,
    ) -> Result<ThumbnailResult<'a>>;
}
