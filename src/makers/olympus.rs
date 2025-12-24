#![allow(dead_code)]

use once_cell::sync::Lazy;

use crate::describe_exif_rule;
use crate::rawthumb::core::errors::{DecodingError, Result};
use crate::rawthumb::core::exif::{ExifParsingRule, ExifReader, ParsedExif};
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
    info: ParsedExif,
}

impl OlympusDecoder {
    fn new(info: ParsedExif) -> Self {
        Self { info }
    }

    fn get_thumbnail<'a>(&self, buffer: &'a [u8]) -> std::result::Result<&'a [u8], DecodingError> {
        let base = self.info.usize("maker_notes")?;
        let offset = self.info.usize("preview_image_start")? + base;
        let len = self.info.usize("preview_image_len")?;
        Ok(&buffer[offset..offset + len])
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
}

#[allow(dead_code)]
pub struct OlympusThumbnailExtractor;

impl ThumbnailExtractor for OlympusThumbnailExtractor {
    fn supports_make(&self, make: &str) -> bool {
        matches!(make, "OLYMPUS CORPORATION" | "OLYMPUS IMAGING CORP.")
    }

    fn extract<'a>(
        &self,
        buffer: &'a [u8],
        _info: &RawMetadata,
        exif: &dyn ExifReader,
        parsed: ParsedExif,
    ) -> Result<ThumbnailResult<'a>> {
        let raw_info = exif.parse_with_prev_info(buffer, &THUMBNAIL_RULE, parsed)?;
        let decoder = OlympusDecoder::new(raw_info);
        let thumbnail = decoder.get_thumbnail(buffer)?;
        let orientation: Orientation = decoder.get_orientation().into();
        Ok(ThumbnailResult {
            jpeg: thumbnail,
            orientation,
        })
    }
}
