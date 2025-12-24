#![allow(dead_code)]

use crate::rawthumb::core::types::ThumbnailResult;

/// Processor that can optionally preprocess the buffer or short-circuit with a fast-path extract.
pub trait FormatPreprocessor: Send + Sync {
    fn preprocess<'a>(&self, buffer: &'a [u8]) -> &'a [u8] {
        buffer
    }

    fn try_extract<'a>(&self, _buffer: &'a [u8]) -> Option<ThumbnailResult<'a>> {
        None
    }
}
