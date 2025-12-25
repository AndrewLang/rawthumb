#![allow(dead_code)]

use once_cell::sync::Lazy;

use crate::describe_exif_rule;
use crate::rawthumb::core::errors::{DecodingError, ImageProcessingError, Result};
use crate::rawthumb::core::exif::{
    ExifError, ExifFieldError, ExifNames, ExifParsingRule, ExifReader, ParsedExif,
};
use crate::rawthumb::core::image_helper::ImageHelper;
use crate::rawthumb::core::thumbnail_extractor::ThumbnailExtractor;
use crate::rawthumb::core::types::{Orientation, RawMetadata, ThumbnailResult};
use std::panic::{catch_unwind, AssertUnwindSafe};

static THUMBNAIL_RULE: Lazy<ExifParsingRule> = Lazy::new(|| {
    describe_exif_rule!(tiff {
        0x0112 / orientation
        0x8769 {
            0x927c / maker_notes {
                offset + 12 {
                    0x2020 {
                        offset + maker_notes {
                            0x0101 / preview_image_start
                            0x0102 / preview_image_len
                        }
                    }
                }
            }
        }
    })
});

#[derive(Default)]
struct OlympusDecoder;

impl OlympusDecoder {
    fn get_thumbnail<'a>(
        &self,
        buffer: &'a [u8],
        exif: &ParsedExif,
    ) -> std::result::Result<&'a [u8], DecodingError> {
        let base = exif.usize(ExifNames::MAKER_NOTES)?;
        let offset = exif.usize(ExifNames::PREVIEW_IMAGE_START)? + base;
        let len = exif.usize(ExifNames::PREVIEW_IMAGE_LEN)?;
        let end = offset
            .checked_add(len)
            .filter(|end| *end <= buffer.len())
            .ok_or_else(|| ExifFieldError::field_not_found("preview_image_bounds"))?;
        Ok(&buffer[offset..end])
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

#[allow(dead_code)]
#[derive(Default)]
pub struct OlympusThumbnailExtractor {
    decoder: OlympusDecoder,
}

impl ThumbnailExtractor for OlympusThumbnailExtractor {
    fn supports_make(&self, make: &str) -> bool {
        matches!(
            make,
            "OLYMPUS CORPORATION" | "OLYMPUS IMAGING CORP." | "OM Digital Solutions"
        )
    }

    fn extract<'a>(
        &self,
        buffer: &'a [u8],
        _info: &RawMetadata,
        exif: &dyn ExifReader,
        parsed: ParsedExif,
    ) -> Result<ThumbnailResult<'a>> {
        // quickexif may panic on malformed maker note offsets; silence the panic hook and fall back gracefully.
        let previous_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(|_| {}));
        let parse_result = catch_unwind(AssertUnwindSafe(|| {
            exif.parse_with_prev_info(buffer, &THUMBNAIL_RULE, parsed)
        }));
        std::panic::set_hook(previous_hook);

        let raw_info = match parse_result {
            Ok(Ok(info)) => info,
            Ok(Err(ExifError::Parse(quickexif::parser::Error::TagNotFound(tag)))) => {
                log::debug!(
                    "Olympus maker note tag 0x{tag:04x} missing; falling back to JPEG scan"
                );
                return Self::fallback_thumbnail(buffer, exif);
            }
            Ok(Err(e)) => {
                log::warn!(
                    "Olympus maker note parse failed ({}); falling back to JPEG scan",
                    e
                );
                return Self::fallback_thumbnail(buffer, exif);
            }
            Err(_) => {
                log::warn!(
                    "Olympus maker note parse panicked; falling back to JPEG scan"
                );
                return Self::fallback_thumbnail(buffer, exif);
            }
        };
        log::debug!("Olympus extracted raw_info: {}", raw_info.debug_summary());
        match self.decoder.get_thumbnail(buffer, &raw_info) {
            Ok(thumbnail) => {
                let orientation: Orientation = self.decoder.get_orientation(&raw_info).into();
                Ok(ThumbnailResult {
                    jpeg: thumbnail,
                    orientation,
                })
            }
            Err(e) => {
                log::warn!(
                    "Olympus preview offsets missing or invalid ({}); falling back to JPEG scan",
                    e
                );
                Self::fallback_thumbnail(buffer, exif)
            }
        }
    }
}

impl OlympusThumbnailExtractor {
    fn fallback_thumbnail<'a>(
        buffer: &'a [u8],
        exif: &dyn ExifReader,
    ) -> Result<ThumbnailResult<'a>> {
        if let Some(jpeg) = ImageHelper::extract_largest_jpeg_segment(buffer) {
            let orientation = match exif.get_orientation(buffer) {
                Some(3) => Orientation::Rotate180,
                Some(6) => Orientation::Rotate90,
                Some(8) => Orientation::Rotate270,
                _ => Orientation::Horizontal,
            };
            return Ok(ThumbnailResult { jpeg, orientation });
        }
        Err(ImageProcessingError::Raw(
            "Olympus thumbnail not found in maker notes or JPEG scan".to_string(),
        ))
    }
}
