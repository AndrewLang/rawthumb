#![allow(dead_code)]

use crate::rawthumb::core::image_helper::ImageHelper;
use crate::rawthumb::core::types::{Orientation, ThumbnailResult};
use crate::rawthumb::formats::format_processor::FormatPreprocessor;

pub struct Cr3Processor;

impl FormatPreprocessor for Cr3Processor {
    fn try_extract<'a>(&self, buffer: &'a [u8]) -> Option<ThumbnailResult<'a>> {
        let is_cr3 = buffer.get(4..12).map(|b| b == b"ftypcrx ").unwrap_or(false);
        if !is_cr3 {
            return None;
        }

        let jpeg = ImageHelper::extract_largest_jpeg_segment(buffer)?;
        Some(ThumbnailResult {
            jpeg,
            orientation: Orientation::Horizontal,
        })
    }
}
