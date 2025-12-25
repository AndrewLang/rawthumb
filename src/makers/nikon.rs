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
        let raw_info = exif.parse_with_prev_info(buffer, &THUMBNAIL_RULE, parsed)?;
        let thumbnail = self.decoder.get_thumbnail(buffer, &raw_info)?;
        let orientation: Orientation = self.decoder.get_orientation(&raw_info).into();
        Ok(ThumbnailResult {
            jpeg: Cow::Borrowed(thumbnail),
            orientation,
        })
    }
}
