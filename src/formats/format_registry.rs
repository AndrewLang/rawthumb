#![allow(dead_code)]

use std::sync::Arc;

use once_cell::sync::Lazy;

use crate::rawthumb::core::types::ThumbnailResult;
use crate::rawthumb::formats::cr3_processor::Cr3Processor;
use crate::rawthumb::formats::format_processor::FormatPreprocessor;
use crate::rawthumb::formats::fuji_processor::FujiPreprocessor;

pub struct FormatRegistry {
    processors: Vec<Arc<dyn FormatPreprocessor>>,
}

impl FormatRegistry {
    pub fn new(processors: Vec<Arc<dyn FormatPreprocessor>>) -> Self {
        Self { processors }
    }

    pub fn apply_preprocessors<'a>(&self, buffer: &'a [u8]) -> &'a [u8] {
        let mut current = buffer;
        for processor in &self.processors {
            current = processor.preprocess(current);
        }
        current
    }

    pub fn try_fast_path<'a>(&self, buffer: &'a [u8]) -> Option<ThumbnailResult<'a>> {
        self.processors.iter().find_map(|processor| processor.try_extract(buffer))
    }
}

pub static FORMAT_REGISTRY: Lazy<FormatRegistry> =
    Lazy::new(|| FormatRegistry::new(vec![Arc::new(FujiPreprocessor), Arc::new(Cr3Processor::new())]));
