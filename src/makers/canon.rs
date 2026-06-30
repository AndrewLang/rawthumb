#![allow(dead_code)]

use once_cell::sync::Lazy;
use std::borrow::Cow;

use crate::describe_exif_rule;
use crate::rawthumb::core::errors::ExifFieldError;
use crate::rawthumb::core::errors::{DecodingError, Result};
use crate::rawthumb::core::exif::{ExifNames, ExifParsingRule, ExifReader, ParsedExif};
use crate::rawthumb::core::image_helper::ImageHelper;
use crate::rawthumb::core::thumbnail_extractor::ThumbnailExtractor;
use crate::rawthumb::core::types::{Orientation, RawMetadata, ThumbnailResult};

pub static THUMBNAIL_RULE: Lazy<ExifParsingRule> = Lazy::new(|| {
    describe_exif_rule!(tiff {
        0x0112 / orientation
        next {
            0x0201 / thumbnail
            0x0202 / thumbnail_len
        }
    })
});

#[derive(Default)]
struct CanonDecoder;

impl CanonDecoder {
    fn get_thumbnail<'a>(&self, buffer: &'a [u8], exif: &ParsedExif) -> std::result::Result<&'a [u8], DecodingError> {
        let exif_jpeg = ImageHelper::jpeg_from_exif(buffer, exif);
        let scanned_jpeg = ImageHelper::find_display_jpeg_slice(buffer);

        match (exif_jpeg, scanned_jpeg) {
            (Some(exif), Some(scanned)) if scanned.len() > exif.len() => Ok(scanned),
            (Some(exif), _) => Ok(exif),
            (_, Some(scanned)) => Ok(scanned),
            _ => Err(DecodingError::RawInfoError(ExifFieldError::field_not_found(ExifNames::THUMBNAIL))),
        }
    }
}

#[derive(Default)]
pub struct CanonThumbnailExtractor {
    decoder: CanonDecoder,
}

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
        let thumbnail = self.decoder.get_thumbnail(buffer, &raw_info)?;
        let orientation: Orientation = raw_info.orientation();
        Ok(ThumbnailResult::new(Cow::Borrowed(thumbnail), orientation))
    }
}
