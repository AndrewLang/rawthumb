#![allow(dead_code)]

use once_cell::sync::Lazy;

use crate::describe_exif_rule;
use crate::rawthumb::core::errors::{DecodingError, ImageProcessingError, Result};
use crate::rawthumb::core::exif::{ExifError, ExifNames, ExifParsingRule, ExifReader, ParsedExif};
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
                            0x0101 / preview_image_start
                            0x0102 / preview_image_len
                        }
                    }
                }
            }
        }
    })
});

struct OlympusDecoder {
    exif: ParsedExif,
}

impl OlympusDecoder {
    fn new(exif: ParsedExif) -> Self {
        Self { exif }
    }

    fn get_thumbnail<'a>(&self, buffer: &'a [u8]) -> std::result::Result<&'a [u8], DecodingError> {
        let base = self.exif.usize(ExifNames::MAKER_NOTES)?;
        let offset = self.exif.usize(ExifNames::PREVIEW_IMAGE_START)? + base;
        let len = self.exif.usize(ExifNames::PREVIEW_IMAGE_LEN)?;
        Ok(&buffer[offset..offset + len])
    }

    fn get_orientation(&self) -> Orientation {
        match self.exif.u16(ExifNames::ORIENTATION).ok() {
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
pub struct OlympusThumbnailExtractor;

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
        let raw_info = match exif.parse_with_prev_info(buffer, &THUMBNAIL_RULE, parsed) {
            Ok(info) => info,
            Err(ExifError::Parse(quickexif::parser::Error::TagNotFound(tag))) => {
                log::debug!(
                    "Olympus maker note tag 0x{tag:04x} missing; falling back to JPEG scan"
                );
                return Self::fallback_thumbnail(buffer, exif);
            }
            Err(e) => {
                log::warn!(
                    "Olympus maker note parse failed ({}); falling back to JPEG scan",
                    e
                );
                return Self::fallback_thumbnail(buffer, exif);
            }
        };
        log::debug!("Olympus extracted raw_info: {}", raw_info.debug_summary());
        let decoder = OlympusDecoder::new(raw_info);
        match decoder.get_thumbnail(buffer) {
            Ok(thumbnail) => {
                let orientation: Orientation = decoder.get_orientation().into();
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
