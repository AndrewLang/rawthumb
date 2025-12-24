#![allow(dead_code)]

use std::sync::Arc;

use crate::rawthumb::core::errors::Result;
use crate::rawthumb::core::types::{BasicInfo, ThumbnailResult};

pub trait ThumbnailExtractor: Send + Sync {
    fn supports_make(&self, make: &str) -> bool;
    fn extract<'a>(
        &self,
        buffer: &'a [u8],
        info: &BasicInfo,
        parsed: quickexif::ParsedInfo,
    ) -> Result<ThumbnailResult<'a>>;
}

pub struct ThumbnailRegistry {
    makers: Vec<Arc<dyn ThumbnailExtractor>>,
}

impl ThumbnailRegistry {
    pub fn new(makers: Vec<Arc<dyn ThumbnailExtractor>>) -> Self {
        Self { makers }
    }

    pub fn find(&self, make: &str) -> Option<&Arc<dyn ThumbnailExtractor>> {
        self.makers.iter().find(|m| m.supports_make(make))
    }
}
