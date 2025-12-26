#![allow(dead_code)]

use once_cell::sync::Lazy;
use std::borrow::Cow;

use crate::describe_exif_rule;
use crate::rawthumb::core::errors::{DecodingError, Result};
use crate::rawthumb::core::exif::{ExifNames, ExifParsingRule, ExifReader, ParsedExif};
use crate::rawthumb::core::thumbnail_extractor::ThumbnailExtractor;
use crate::rawthumb::core::types::{Orientation, RawMetadata, ThumbnailResult};

static THUMBNAIL_RULE: Lazy<ExifParsingRule> = Lazy::new(|| {
    describe_exif_rule!(tiff {
        0x0112 : u16 / orientation
        0x014a {
            offset address {
                0x0201 / thumbnail
                0x0202 / thumbnail_len
            }
        }
    })
});

static NIKON_FAST_RULE: Lazy<ExifParsingRule> = Lazy::new(|| {
    describe_exif_rule!(tiff {
        0x0112 / orientation
        0x0201 / thumbnail
        0x0202 / thumbnail_len
    })
});

#[derive(Default)]
struct NikonDecoder;

impl NikonDecoder {
    fn get_thumbnail<'a>(
        &self,
        buffer: &'a [u8],
        exif: &ParsedExif,
    ) -> std::result::Result<&'a [u8], DecodingError> {
        let offset = exif.usize(ExifNames::THUMBNAIL)?;
        let len = exif.usize(ExifNames::THUMBNAIL_LEN)?;
        Ok(&buffer[offset..offset + len])
    }

    fn get_orientation(&self, exif: &ParsedExif) -> Orientation {
        match exif.u16(ExifNames::ORIENTATION).ok() {
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
}

#[derive(Default)]
pub struct NikonThumbnailExtractor {
    decoder: NikonDecoder,
}

impl ThumbnailExtractor for NikonThumbnailExtractor {
    fn supports_make(&self, make: &str) -> bool {
        matches!(make, "NIKON" | "NIKON CORPORATION")
    }

    fn extract<'a>(
        &self,
        buffer: &'a [u8],
        _info: &RawMetadata,
        exif: &dyn ExifReader,
        parsed: ParsedExif,
    ) -> Result<ThumbnailResult<'a>> {
        if let Some(fast) = Self::try_fast_path(buffer, exif) {
            return Ok(fast);
        }

        if let Some(from_parsed) = Self::try_from_parsed(buffer, &parsed) {
            return Ok(from_parsed);
        }

        let raw_info = exif.parse_with_prev_info(buffer, &THUMBNAIL_RULE, parsed)?;
        let thumbnail = self.decoder.get_thumbnail(buffer, &raw_info)?;
        let orientation: Orientation = self.decoder.get_orientation(&raw_info).into();
        Ok(ThumbnailResult {
            jpeg: Cow::Borrowed(thumbnail),
            orientation,
        })
    }
}

impl NikonThumbnailExtractor {
    fn try_fast_path<'a>(
        buffer: &'a [u8],
        exif: &dyn ExifReader,
    ) -> Option<ThumbnailResult<'a>> {
        let parsed = exif.parse_with_rule(buffer, &NIKON_FAST_RULE).ok()?;

        let offset = parsed.u32(ExifNames::THUMBNAIL).ok()? as usize;
        let len = parsed.u32(ExifNames::THUMBNAIL_LEN).ok()? as usize;

        let end = offset.checked_add(len)?;
        if end == 0 || end > buffer.len() {
            return None;
        }
        let thumb = buffer.get(offset..end)?;
        
        let orientation = match parsed.u16(ExifNames::ORIENTATION).ok() {
            Some(3) => Orientation::Rotate180,
            Some(6) => Orientation::Rotate90,
            Some(8) => Orientation::Rotate270,
            _ => Orientation::Horizontal,
        };

        Some(ThumbnailResult {
            jpeg: Cow::Borrowed(thumb),
            orientation,
        })
    }

    fn try_from_parsed<'a>(
        buffer: &'a [u8],
        parsed: &ParsedExif,
    ) -> Option<ThumbnailResult<'a>> {
        let offset = parsed.u32(ExifNames::THUMBNAIL).ok()? as usize;
        let len = parsed.u32(ExifNames::THUMBNAIL_LEN).ok()? as usize;
        let end = offset.checked_add(len)?;
        let thumb = buffer.get(offset..end)?;

        let orientation = match parsed.u16(ExifNames::ORIENTATION).ok() {
            Some(3) => Orientation::Rotate180,
            Some(6) => Orientation::Rotate90,
            Some(8) => Orientation::Rotate270,
            _ => Orientation::Horizontal,
        };

        Some(ThumbnailResult {
            jpeg: Cow::Borrowed(thumb),
            orientation,
        })
    }
}
