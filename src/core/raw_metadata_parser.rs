#![allow(dead_code)]

use crate::rawthumb::core::errors::{ImageProcessingError, Result};
use crate::rawthumb::core::exif::{ExifReader, ParsedExif};
use crate::rawthumb::core::image_helper::ImageHelper;
use crate::rawthumb::core::types::RawMetadata;

#[allow(dead_code)]
pub struct RawMetadataParser;

impl RawMetadataParser {
    pub fn parse(exif: &dyn ExifReader, buffer: &[u8]) -> Result<(ParsedExif, RawMetadata)> {
        exif.parse_raw_metadata(buffer).map_err(ImageProcessingError::from)
    }

    pub fn parse_with_cr3_fallback<'a>(
        exif: &dyn ExifReader,
        buffer: &'a [u8],
    ) -> Result<(ParsedExif, RawMetadata, &'a [u8])> {
        match Self::parse(exif, buffer) {
            Ok((info, basic)) => Ok((info, basic, buffer)),
            Err(e) => {
                if let Some(exif_buffer) = ImageHelper::extract_canon_cr3_exif_segment(buffer) {
                    let (info, basic) = Self::parse(exif, exif_buffer)?;
                    Ok((info, basic, exif_buffer))
                } else {
                    Err(e)
                }
            }
        }
    }
}
