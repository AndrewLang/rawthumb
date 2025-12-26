#![allow(dead_code)]

use once_cell::sync::Lazy;
use std::borrow::Cow;
use std::panic::{AssertUnwindSafe, catch_unwind};

use crate::describe_exif_rule;
use crate::rawthumb::core::errors::ExifFieldError;
use crate::rawthumb::core::errors::{DecodingError, ImageProcessingError, Result};
use crate::rawthumb::core::exif::{ExifNames, ExifParsingRule, ExifReader, ParsedExif};
use crate::rawthumb::core::image_helper::ImageHelper;
use crate::rawthumb::core::thumbnail_extractor::ThumbnailExtractor;
use crate::rawthumb::core::types::{Orientation, RawMetadata, ThumbnailResult};

static THUMBNAIL_RULE: Lazy<ExifParsingRule> = Lazy::new(|| {
    describe_exif_rule!(tiff {
        0x0112 / orientation
        0x8769 {
            0x927c / maker_notes {
                offset + 12 {
                    0x2020 {
                        offset + maker_notes {
                            0x0101? / preview_image_start
                            0x0102? / preview_image_len
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
        let offset = exif.usize(ExifNames::PREVIEW_IMAGE_START)?;
        let len = exif.usize(ExifNames::PREVIEW_IMAGE_LEN)?;
        // Maker note offsets are relative to base.
        let offset = offset + base;
        let end = offset
            .checked_add(len)
            .filter(|end| *end <= buffer.len())
            .ok_or_else(|| ExifFieldError::field_not_found("preview_image_bounds"))?;
        Ok(&buffer[offset..end])
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
        let previous_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(|_| {}));
        let parse_result = catch_unwind(AssertUnwindSafe(|| {
            exif.parse_with_prev_info(buffer, &THUMBNAIL_RULE, parsed)
        }));
        std::panic::set_hook(previous_hook);

        let raw_info = match parse_result {
            Ok(Ok(info)) => info,
            Ok(Err(e)) if matches!(e.tag_not_found(), Some(_)) => {
                let tag = e.tag_not_found().unwrap_or(0);
                log::trace!(
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
                log::warn!("Olympus maker note parse panicked; falling back to JPEG scan");
                return Self::fallback_thumbnail(buffer, exif);
            }
        };

        log::trace!(
            " 🪓 Olympus extracted raw_info: {}",
            raw_info.debug_summary()
        );
        match self.decoder.get_thumbnail(buffer, &raw_info) {
            Ok(thumbnail) => {
                if thumbnail.len() >= 8 * 1024
                    && ImageHelper::is_valid_jpeg(thumbnail)
                    && ImageHelper::jpeg_has_sof(thumbnail)
                {
                    let orientation: Orientation = raw_info.orientation();
                    Ok(ThumbnailResult {
                        jpeg: Cow::Borrowed(thumbnail),
                        orientation,
                    })
                } else {
                    log::warn!(
                        "Olympus maker-note thumbnail failed validation; falling back to JPEG scan"
                    );
                    Self::fallback_thumbnail(buffer, exif)
                }
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
        let valid = |jpeg: &[u8]| {
            jpeg.len() >= 8 * 1024
                && ImageHelper::is_valid_jpeg(jpeg)
                && ImageHelper::jpeg_has_sof(jpeg)
        };

        // Try a capped fast scan first to avoid scanning huge buffers.
        if let Some(jpeg) =
            ImageHelper::extract_valid_jpeg_with_cap(buffer, 64 * 1024 * 1024, 32 * 1024, true)
        {
            if valid(jpeg) {
                let orientation = match exif.get_orientation(buffer) {
                    Some(3) => Orientation::Rotate180,
                    Some(6) => Orientation::Rotate90,
                    Some(8) => Orientation::Rotate270,
                    _ => Orientation::Horizontal,
                };
                return Ok(ThumbnailResult {
                    jpeg: Cow::Borrowed(jpeg),
                    orientation,
                });
            }
        }
        if let Some(jpeg) = ImageHelper::extract_best_jpeg_capped(buffer, 128 * 1024 * 1024)
            .filter(|j| valid(j))
            .or_else(|| {
                ImageHelper::extract_largest_jpeg_segment_capped(buffer, 128 * 1024 * 1024)
                    .filter(|j| valid(j))
            })
            .or_else(|| ImageHelper::extract_best_jpeg(buffer).filter(|j| valid(j)))
            .or_else(|| ImageHelper::extract_largest_jpeg_segment(buffer).filter(|j| valid(j)))
        {
            let orientation = match exif.get_orientation(buffer) {
                Some(3) => Orientation::Rotate180,
                Some(6) => Orientation::Rotate90,
                Some(8) => Orientation::Rotate270,
                _ => Orientation::Horizontal,
            };
            return Ok(ThumbnailResult {
                jpeg: Cow::Borrowed(jpeg),
                orientation,
            });
        }
        Err(ImageProcessingError::Raw(
            "Olympus thumbnail not found in maker notes or JPEG scan".to_string(),
        ))
    }
}
