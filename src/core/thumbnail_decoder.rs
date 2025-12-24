#![allow(dead_code)]

use crate::rawthumb::core::errors::{ImageProcessingError, Result as CoreResult};
use crate::rawthumb::core::image_helper::ImageHelper;
use crate::rawthumb::core::raw_metadata_parser::RawMetadataParser;
use crate::rawthumb::core::types::{Orientation, RawMetadata, ThumbnailResult};
use crate::rawthumb::formats::format_registry::FORMAT_REGISTRY;
use crate::rawthumb::makers::maker_registry::MAKER_REGISTRY;

pub struct ThumbnailDecoder;

impl ThumbnailDecoder {
    pub fn new() -> Self {
        Self
    }

    pub fn get_thumbnail<'a>(
        &self,
        buffer: &'a [u8],
        _ext: &str,
    ) -> CoreResult<ThumbnailResult<'a>> {
        self.decode_thumbnail(buffer)
    }

    fn decode_thumbnail<'a>(&self, buffer: &'a [u8]) -> CoreResult<ThumbnailResult<'a>> {
        println!("Decoding thumbnail with raw thumb");
        let buffer = FORMAT_REGISTRY.apply_preprocessors(buffer);

        if let Some(result) = FORMAT_REGISTRY.try_fast_path(buffer) {
            return Ok(result);
        }

        match RawMetadataParser::parse_with_cr3_fallback(buffer) {
            Ok((parsed, basic, buf)) => self.select_extractor_and_decode(buf, parsed, basic),
            Err(e) => {
                if let Some(jpeg) = ImageHelper::extract_largest_jpeg_segment(buffer) {
                    Ok(ThumbnailResult {
                        jpeg,
                        orientation: Orientation::Horizontal,
                    })
                } else {
                    Err(e)
                }
            }
        }
    }

    fn select_extractor_and_decode<'a>(
        &self,
        buffer: &'a [u8],
        basic_parsed: quickexif::ParsedInfo,
        metadata: RawMetadata,
    ) -> CoreResult<ThumbnailResult<'a>> {
        let extractor = MAKER_REGISTRY.find(&metadata).ok_or_else(|| {
            ImageProcessingError::Raw(format!("Maker is not supported: {}", metadata.make))
        })?;

        extractor
            .extract(buffer, &metadata, basic_parsed)
            .map_err(ImageProcessingError::from)
    }
}
