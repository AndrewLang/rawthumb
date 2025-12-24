#![allow(dead_code)]

use crate::rawthumb::core::types::Orientation;
use crate::rawthumb::core::types::ThumbnailResult;

pub struct Cr3Format;

pub struct Cr3FastPath;

impl Cr3FastPath {
    pub fn try_extract(buffer: &[u8]) -> Option<ThumbnailResult<'_>> {
        let is_cr3 = buffer.get(4..12).map(|b| b == b"ftypcrx ").unwrap_or(false);
        if !is_cr3 {
            return None;
        }
        let jpeg = crate::rawthumb::decode::largest_jpeg_slice(buffer)?;
        Some(ThumbnailResult {
            jpeg,
            orientation: Orientation::Horizontal,
        })
    }
}
