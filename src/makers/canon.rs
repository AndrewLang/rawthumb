#![allow(dead_code)]

use once_cell::sync::Lazy;

use crate::describe_exif_rule;
use crate::rawthumb::core::errors::{DecodingError, Result};
use crate::rawthumb::core::exif::{ExifFieldError, ExifParsingRule, ExifReader, ParsedExif};
use crate::rawthumb::core::thumbnail_extractor::ThumbnailExtractor;
use crate::rawthumb::core::types::{Orientation, RawMetadata, ThumbnailResult};

static THUMBNAIL_RULE: Lazy<ExifParsingRule> = Lazy::new(|| {
    describe_exif_rule!(tiff {
        0x0112 / orientation
        next {
            0x0201 / thumbnail
            0x0202 / thumbnail_len
        }
    })
});

struct CanonDecoder {
    info: ParsedExif,
}

impl CanonDecoder {
    fn new(info: ParsedExif) -> Self {
        Self { info }
    }

    fn get_orientation(&self) -> Orientation {
        match self.info.u16("orientation").ok() {
            None => Orientation::Horizontal,
            Some(o) => match o {
                1 => Orientation::Horizontal,
                3 => Orientation::Rotate180,
                6 => Orientation::Rotate90,
                8 => Orientation::Rotate270,
                _ => Orientation::Horizontal,
            },
        }
    }

    fn get_thumbnail<'a>(&self, buffer: &'a [u8]) -> std::result::Result<&'a [u8], DecodingError> {
        // Prefer the Exif-provided preview when it looks like a displayable JPEG (APP0/APP1).
        if let Some(exif_jpeg) = jpeg_from_exif(buffer, &self.info) {
            return Ok(exif_jpeg);
        }

        // Fallback: scan for the largest displayable JPEG slice (skip raw lossless JPEG data).
        if let Some(scanned) = find_display_jpeg_slice(buffer) {
            return Ok(scanned);
        }

        Err(DecodingError::RawInfoError(
            ExifFieldError::field_not_found("thumbnail"),
        ))
    }
}

pub struct CanonThumbnailExtractor;

impl ThumbnailExtractor for CanonThumbnailExtractor {
    fn supports_make(&self, make: &str) -> bool {
        matches!(make, "Canon" | "CANON" | "Canon Inc.")
    }

    fn extract<'a>(
        &self,
        buffer: &'a [u8],
        _info: &RawMetadata,
        exif: &dyn ExifReader,
        parsed: ParsedExif,
    ) -> Result<ThumbnailResult<'a>> {
        let raw_info = exif.parse_with_prev_info(buffer, &THUMBNAIL_RULE, parsed)?;
        let decoder = CanonDecoder::new(raw_info);
        let thumbnail = decoder.get_thumbnail(buffer)?;
        let orientation: Orientation = decoder.get_orientation().into();
        Ok(ThumbnailResult {
            jpeg: thumbnail,
            orientation,
        })
    }
}

fn find_largest_jpeg_slice<'a>(buffer: &'a [u8]) -> Option<&'a [u8]> {
    let mut start = 0usize;
    let mut best: Option<(usize, usize)> = None;
    while let Some(rel_soi) = buffer[start..]
        .windows(3)
        .position(|w| w == [0xff, 0xd8, 0xff])
    {
        let soi = start + rel_soi;
        if let Some(rel_eoi) = buffer[soi + 3..].windows(2).position(|w| w == [0xff, 0xd9]) {
            let end = soi + 3 + rel_eoi + 2;
            let len = end - soi;
            if best.map(|(_, b_len)| len > b_len).unwrap_or(true) {
                best = Some((soi, len));
            }
            start = end;
        } else {
            break;
        }
    }
    best.map(|(s, l)| &buffer[s..s + l])
}

fn jpeg_from_exif<'a>(buffer: &'a [u8], info: &ParsedExif) -> Option<&'a [u8]> {
    let offset = info.usize("thumbnail").ok()?;
    let len = info.usize("thumbnail_len").ok()?;
    if offset + len > buffer.len() || len < 4 {
        return None;
    }
    let slice = &buffer[offset..offset + len];
    if is_display_jpeg(slice) {
        Some(slice)
    } else {
        None
    }
}

fn is_valid_jpeg(slice: &[u8]) -> bool {
    if slice.len() < 4 || !slice.starts_with(&[0xff, 0xd8]) {
        return false;
    }
    slice.windows(2).rev().any(|w| w == [0xff, 0xd9])
}

fn is_display_jpeg(slice: &[u8]) -> bool {
    if !is_valid_jpeg(slice) {
        return false;
    }
    // Look for JFIF/EXIF APP markers near the start; avoid lossless RAW JPEG data that lacks them.
    slice
        .windows(4)
        .take(40)
        .any(|w| w == [0xff, 0xe0, b'J', b'F'] || w == [0xff, 0xe1, b'E', b'x'])
}

fn find_display_jpeg_slice<'a>(buffer: &'a [u8]) -> Option<&'a [u8]> {
    find_largest_jpeg_slice(buffer)
        .filter(|s| is_display_jpeg(s))
        .or_else(|| {
            // As a fallback, return the first valid JPEG slice, even without APP markers.
            let mut start = 0usize;
            while let Some(rel_soi) = buffer[start..]
                .windows(3)
                .position(|w| w == [0xff, 0xd8, 0xff])
            {
                let soi = start + rel_soi;
                if let Some(rel_eoi) = buffer[soi + 3..].windows(2).position(|w| w == [0xff, 0xd9])
                {
                    let end = soi + 3 + rel_eoi + 2;
                    let slice = &buffer[soi..end];
                    if is_valid_jpeg(slice) {
                        return Some(slice);
                    }
                    start = end;
                    continue;
                }
                break;
            }
            None
        })
}
