#![allow(dead_code)]

use std::sync::Arc;

use crate::rawthumb::core::errors::{ImageProcessingError, Result as CoreResult};
use crate::rawthumb::core::exif::{ExifError, ExifReader, ParsedExif, QuickExifReader};
use crate::rawthumb::core::image_helper::ImageHelper;
use crate::rawthumb::core::raw_metadata_parser::RawMetadataParser;
use crate::rawthumb::core::types::{Orientation, RawMetadata, ThumbnailResult};
use crate::rawthumb::formats::format_registry::FORMAT_REGISTRY;
use crate::rawthumb::makers::maker_registry::MAKER_REGISTRY;

pub struct ThumbnailDecoder {
    exif: Arc<dyn ExifReader>,
}

impl ThumbnailDecoder {
    pub fn new() -> Self {
        Self {
            exif: Arc::new(QuickExifReader::new()),
        }
    }

    pub fn new_with_exif(exif: Arc<dyn ExifReader>) -> Self {
        Self { exif }
    }

    pub fn get_thumbnail<'a>(
        &self,
        buffer: &'a [u8],
        _ext: &str,
    ) -> CoreResult<ThumbnailResult<'a>> {
        self.decode_thumbnail(buffer)
    }

    fn decode_thumbnail<'a>(&self, buffer: &'a [u8]) -> CoreResult<ThumbnailResult<'a>> {
        let buffer = FORMAT_REGISTRY.apply_preprocessors(buffer);

        if let Some(result) = FORMAT_REGISTRY.try_fast_path(buffer) {
            return Ok(result);
        }

        // quickexif may panic on malformed maker notes; catch and fall back.
        let previous_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(|_| {}));
        let parsed = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            RawMetadataParser::parse_with_cr3_fallback(self.exif.as_ref(), buffer)
        }));
        std::panic::set_hook(previous_hook);

        match parsed {
            Ok(Ok((parsed, basic, buf))) => self.select_extractor_and_decode(buf, parsed, basic),
            Ok(Err(e)) => {
                if let Some(jpeg) =
                    ImageHelper::extract_valid_jpeg_with_cap(buffer, 128 * 1024 * 1024, 16 * 1024, true)
                        .or_else(|| ImageHelper::extract_best_jpeg_capped(buffer, buffer.len()))
                        .or_else(|| ImageHelper::extract_largest_jpeg_segment(buffer))
                {
                    Ok(ThumbnailResult {
                        jpeg,
                        orientation: Orientation::Horizontal,
                    })
                } else {
                    Err(e)
                }
            }
            Err(_) => {
                if let Some(jpeg) =
                    ImageHelper::extract_valid_jpeg_with_cap(buffer, 128 * 1024 * 1024, 16 * 1024, true)
                        .or_else(|| ImageHelper::extract_best_jpeg_capped(buffer, buffer.len()))
                        .or_else(|| ImageHelper::extract_largest_jpeg_segment(buffer))
                {
                    Ok(ThumbnailResult {
                        jpeg,
                        orientation: Orientation::Horizontal,
                    })
                } else {
                    Err(ImageProcessingError::Raw(
                        "Panic while parsing EXIF; JPEG fallback failed".to_string(),
                    ))
                }
            }
        }
    }

    fn select_extractor_and_decode<'a>(
        &self,
        buffer: &'a [u8],
        basic_parsed: ParsedExif,
        metadata: RawMetadata,
    ) -> CoreResult<ThumbnailResult<'a>> {
        let extractor = MAKER_REGISTRY.find(&metadata).ok_or_else(|| {
            ImageProcessingError::Raw(format!("Maker is not supported: {}", metadata.make))
        })?;

        match extractor.extract(buffer, &metadata, self.exif.as_ref(), basic_parsed) {
            Ok(res) => Ok(res),
            Err(ImageProcessingError::ExifParse(ExifError::Parse(
                quickexif::parser::Error::TagNotFound(_),
            ))) => {
                if let Some(jpeg) =
                    ImageHelper::extract_valid_jpeg_with_cap(buffer, 64 * 1024 * 1024, 16 * 1024, true)
                        .or_else(|| ImageHelper::extract_best_jpeg_capped(buffer, buffer.len()))
                {
                    Ok(ThumbnailResult {
                        jpeg,
                        orientation: Orientation::Horizontal,
                    })
                } else {
                    Err(ImageProcessingError::Raw(
                        "Fallback JPEG scan failed after missing EXIF tag".to_string(),
                    ))
                }
            }
            Err(e) => Err(ImageProcessingError::from(e)),
        }
    }
}
