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
        0x0112 / orientation
        0x002e / thumbnail(thumbnail_len)
    })
});

#[derive(Default)]
struct PanasonicDecoder;

impl PanasonicDecoder {
    fn get_thumbnail<'a>(
        &self,
        buffer: &'a [u8],
        exif: &ParsedExif,
    ) -> std::result::Result<&'a [u8], DecodingError> {
        let offset = exif.usize(ExifNames::THUMBNAIL)?;
        let len = exif.usize(ExifNames::THUMBNAIL_LEN)?;
        Ok(&buffer[offset..offset + len])
    }
}

#[allow(dead_code)]
#[derive(Default)]
pub struct PanasonicThumbnailExtractor {
    decoder: PanasonicDecoder,
}

impl ThumbnailExtractor for PanasonicThumbnailExtractor {
    fn supports_make(&self, make: &str) -> bool {
        make == "Panasonic"
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
        let orientation: Orientation = raw_info.orientation();
        Ok(ThumbnailResult::new(Cow::Borrowed(thumbnail), orientation))
    }
}
