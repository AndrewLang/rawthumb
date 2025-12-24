#![allow(dead_code)]

use std::sync::Arc;

use crate::rawthumb::core::thumbnail_extractor::ThumbnailExtractor;

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
