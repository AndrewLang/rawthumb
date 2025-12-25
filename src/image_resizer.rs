#![allow(dead_code)]

use crate::rawthumb::core::errors::Result as CoreResult;

pub trait ImageResizer: Send + Sync {
    fn resize(&self, buffer: &[u8], max_border: Option<u32>) -> CoreResult<Vec<u8>>;
}

/// No-op resizer placeholder to keep the pipeline wired without extra cost.
pub struct DefaultImageResizer;

impl ImageResizer for DefaultImageResizer {
    fn resize(&self, buffer: &[u8], _max_border: Option<u32>) -> CoreResult<Vec<u8>> {
        Ok(buffer.to_vec())
    }
}
