#![allow(dead_code)]

use crate::rawthumb::core::exif::{ExifReader, QuickExifReader};
use crate::rawthumb::core::image_helper::ImageHelper;
use crate::rawthumb::core::types::{Orientation, ThumbnailResult};
use crate::rawthumb::formats::format_processor::FormatPreprocessor;
use std::borrow::Cow;

pub struct Cr3Processor {
    exif: QuickExifReader,
}

impl Cr3Processor {
    pub fn new() -> Self {
        Self { exif: QuickExifReader::new() }
    }

    fn read_orientation(&self, raw: &[u8], jpeg: &[u8]) -> Orientation {
        // Prefer the embedded JPEG EXIF first (smallest parse), then fall back to Canon EXIF segment, then full raw.
        let orientation = self
            .exif
            .get_orientation(jpeg)
            .or_else(|| ImageHelper::extract_canon_cr3_exif_segment(raw).and_then(|seg| self.exif.get_orientation(seg)))
            .or_else(|| self.exif.get_orientation(raw));

        match orientation {
            Some(3) => Orientation::Rotate180,
            Some(6) => Orientation::Rotate90,
            Some(8) => Orientation::Rotate270,
            _ => Orientation::Horizontal,
        }
    }

    fn fast_first_valid_jpeg<'a>(buffer: &'a [u8], max_scan_bytes: usize, min_size: usize) -> Option<&'a [u8]> {
        let scan_len = buffer.len().min(max_scan_bytes);
        let mut cursor = 0usize;

        while let Some(rel_soi) = buffer[cursor..scan_len].windows(3).position(|w| w == [0xff, 0xd8, 0xff]) {
            let soi = cursor + rel_soi;
            if let Some(rel_eoi) = buffer[soi + 3..].windows(2).position(|w| w == [0xff, 0xd9]) {
                let end = soi + 3 + rel_eoi + 2;
                if let Some(slice) = buffer.get(soi..end) {
                    if slice.len() >= min_size && ImageHelper::jpeg_has_sof(slice) {
                        return Some(slice);
                    }
                }
                cursor = end;
                continue;
            } else {
                break;
            }
        }

        None
    }
}

impl FormatPreprocessor for Cr3Processor {
    fn try_extract<'a>(&self, buffer: &'a [u8]) -> Option<ThumbnailResult<'a>> {
        let is_cr3 = buffer.get(4..12).map(|b| b == b"ftypcrx ").unwrap_or(false);
        if !is_cr3 {
            return None;
        }

        // Fast-path: grab the first reasonably sized JPEG with a SOF marker within 32 MiB of the start.
        let jpeg = Self::fast_first_valid_jpeg(buffer, 32 * 1024 * 1024, 64 * 1024)
            .or_else(|| ImageHelper::extract_largest_jpeg_segment(buffer))?;
        let orientation = self.read_orientation(buffer, jpeg);

        Some(ThumbnailResult::new(Cow::Borrowed(jpeg), orientation))
    }
}
