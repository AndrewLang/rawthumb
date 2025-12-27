#![allow(dead_code)]

use crate::image_resizer::{DefaultImageResizer, ImageResizer};
use crate::image_rotator::{DEFAULT_ROTATE_JPEG_QUALITY, DefaultImageRotator, ImageRotator};
use std::fmt;
use std::sync::Arc;

#[derive(Clone)]
pub struct ExportConfig {
    pub auto_rotate: bool,
    pub resize: bool,
    pub max_border: Option<u32>,
    pub rotator: Arc<dyn ImageRotator>,
    pub resizer: Arc<dyn ImageResizer>,
}

impl Default for ExportConfig {
    fn default() -> Self {
        Self {
            auto_rotate: true,
            resize: false,
            max_border: None,
            rotator: Arc::new(DefaultImageRotator::new(DEFAULT_ROTATE_JPEG_QUALITY)),
            resizer: Arc::new(DefaultImageResizer::default()),
        }
    }
}

impl ExportConfig {
    pub fn with_auto_rotate(mut self, auto_rotate: bool) -> Self {
        self.auto_rotate = auto_rotate;
        self
    }

    pub fn with_max_border(mut self, max_border: Option<u32>) -> Self {
        self.max_border = max_border;
        self.resize = max_border.is_some();
        self
    }

    pub fn with_rotator(mut self, rotator: Arc<dyn ImageRotator>) -> Self {
        self.rotator = rotator;
        self
    }

    pub fn with_resizer(mut self, resizer: Arc<dyn ImageResizer>) -> Self {
        self.resizer = resizer;
        self
    }
}

impl fmt::Debug for ExportConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ExportConfig")
            .field("auto_rotate", &self.auto_rotate)
            .field("max_border", &self.max_border)
            .field("rotator", &"<ImageRotator>")
            .field("resizer", &"<ImageResizer>")
            .finish()
    }
}
