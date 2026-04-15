#![allow(dead_code)]

use std::fs;
use std::io::Write;
use std::sync::Arc;

use crate::export_config::ExportConfig;
use crate::rawthumb::core::errors::{ImageProcessingError, Result as CoreResult};
use crate::rawthumb::core::exif::{ExifReader, ParsedExif, QuickExifReader};
use crate::rawthumb::core::image_helper::ImageHelper;
use crate::rawthumb::core::raw_metadata_parser::RawMetadataParser;
use crate::rawthumb::core::types::{Orientation, RawMetadata, ThumbnailResult};
use crate::rawthumb::formats::format_registry::FORMAT_REGISTRY;
use crate::rawthumb::makers::maker_registry::MAKER_REGISTRY;
use memmap2::MmapOptions;
use std::borrow::Cow;

pub struct ThumbnailExporter {
    exif_reader: Arc<dyn ExifReader>,
    config: ExportConfig,
}

impl ThumbnailExporter {
    pub fn new() -> Self {
        Self { exif_reader: Arc::new(QuickExifReader::new()), config: ExportConfig::default() }
    }

    pub fn new_with_exif(exif: Arc<dyn ExifReader>) -> Self {
        Self { exif_reader: exif, config: ExportConfig::default() }
    }

    pub fn new_with_config(config: ExportConfig) -> Self {
        Self { exif_reader: Arc::new(QuickExifReader::new()), config }
    }

    pub fn new_with_exif_and_config(exif: Arc<dyn ExifReader>, config: ExportConfig) -> Self {
        Self { exif_reader: exif, config }
    }

    pub fn get_thumbnail<'a>(&self, buffer: &'a [u8]) -> CoreResult<ThumbnailResult<'a>> {
        let result = self.decode_thumbnail(buffer)?;
        let resized = self.apply_resize(result)?;
        self.apply_auto_rotate(resized)
    }

    pub fn export(&self, input_path: &str) -> CoreResult<ThumbnailResult<'static>> {
        let file = fs::File::open(input_path)?;
        let mmap = unsafe { MmapOptions::new().map(&file)? };
        let thumb = self.get_thumbnail(&mmap)?;

        Ok(ThumbnailResult {
            jpeg: Cow::Owned(thumb.jpeg.into_owned()),
            orientation: thumb.orientation,
            is_rotated: thumb.is_rotated,
            is_resized: thumb.is_resized,
        })
    }

    pub fn export_to_file(&self, input_path: &str, output_path: &str) -> CoreResult<()> {
        // Prefer mmap to avoid an extra heap copy for large RAW files.
        let file = fs::File::open(input_path)?;
        let mmap = unsafe { MmapOptions::new().map(&file)? };
        let thumb = self.get_thumbnail(&mmap)?;

        log::trace!(
            "Exporting thumbnail (rotated={}, resized={}) to {}",
            thumb.is_rotated,
            thumb.is_resized,
            output_path
        );

        let mut out = fs::File::create(output_path)?;
        out.write_all(thumb.jpeg.as_ref())?;
        Ok(())
    }

    fn decode_thumbnail<'a>(&self, buffer: &'a [u8]) -> CoreResult<ThumbnailResult<'a>> {
        let buffer = FORMAT_REGISTRY.apply_preprocessors(buffer);

        if let Some(result) = FORMAT_REGISTRY.try_fast_path(buffer) {
            return Ok(result);
        }

        let parsed = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            RawMetadataParser::parse_with_cr3_fallback(self.exif_reader.as_ref(), buffer)
        }));

        match parsed {
            Ok(Ok((parsed, basic, buf))) => self.select_extractor_and_decode(buf, parsed, basic),
            Ok(Err(e)) => {
                if let Some(jpeg) = Self::fallback_jpeg(buffer) {
                    Ok(ThumbnailResult::new(Cow::Borrowed(jpeg), Orientation::Horizontal))
                } else {
                    Err(e)
                }
            }
            Err(_) => {
                if let Some(jpeg) = Self::fallback_jpeg(buffer) {
                    Ok(ThumbnailResult::new(Cow::Borrowed(jpeg), Orientation::Horizontal))
                } else {
                    Err(ImageProcessingError::Raw("Panic while parsing EXIF; JPEG fallback failed".to_string()))
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
        let extractor = MAKER_REGISTRY
            .find(&metadata)
            .ok_or_else(|| ImageProcessingError::Raw(format!("Maker is not supported: {}", metadata.make)))?;

        match extractor.extract(buffer, &metadata, self.exif_reader.as_ref(), basic_parsed) {
            Ok(res) => Ok(res),
            Err(ImageProcessingError::ExifParse(e)) if e.tag_not_found().is_some() => {
                if let Some(jpeg) = ImageHelper::extract_valid_jpeg_with_cap(buffer, 64 * 1024 * 1024, 16 * 1024, true)
                    .or_else(|| ImageHelper::extract_best_jpeg_capped(buffer, buffer.len()))
                {
                    Ok(ThumbnailResult::new(Cow::Borrowed(jpeg), Orientation::Horizontal))
                } else {
                    Err(ImageProcessingError::Raw("Fallback JPEG scan failed after missing EXIF tag".to_string()))
                }
            }
            Err(e) => Err(ImageProcessingError::from(e)),
        }
    }

    fn fallback_jpeg<'a>(buffer: &'a [u8]) -> Option<&'a [u8]> {
        // Try a small scan first to avoid expensive full-buffer walks on failures.
        ImageHelper::extract_valid_jpeg_with_cap(buffer, 16 * 1024 * 1024, 16 * 1024, true)
            .or_else(|| ImageHelper::extract_valid_jpeg_with_cap(buffer, 128 * 1024 * 1024, 16 * 1024, true))
            .or_else(|| ImageHelper::extract_best_jpeg_capped(buffer, buffer.len()))
            .or_else(|| ImageHelper::extract_largest_jpeg_segment(buffer))
    }

    fn apply_auto_rotate<'a>(&self, result: ThumbnailResult<'a>) -> CoreResult<ThumbnailResult<'a>> {
        if !self.config.auto_rotate || result.orientation == Orientation::Horizontal {
            return Ok(result);
        }

        log::trace!("🔄 Auto-rotating thumbnail with orientation {:?}", result.orientation);

        let ThumbnailResult { jpeg, orientation, is_rotated, is_resized } = result;
        let rotated = match self.config.rotator.rotate(jpeg.as_ref(), orientation) {
            Ok(r) => r,
            Err(e) => {
                log::warn!("Auto-rotate failed for orientation {:?}: {}; returning original thumbnail", orientation, e);
                return Ok(ThumbnailResult { jpeg, orientation: Orientation::Horizontal, is_rotated, is_resized });
            }
        };

        Ok(ThumbnailResult {
            jpeg: match rotated {
                Cow::Borrowed(_) => jpeg,
                Cow::Owned(buf) => Cow::Owned(buf),
            },
            orientation: Orientation::Horizontal,
            is_rotated: true,
            is_resized,
        })
    }

    fn apply_resize<'a>(&self, result: ThumbnailResult<'a>) -> CoreResult<ThumbnailResult<'a>> {
        if !self.config.resize {
            return Ok(result);
        }

        let max_border = self.config.max_border;
        if max_border.is_none() {
            return Ok(result);
        }

        let ThumbnailResult { jpeg, orientation, is_rotated, is_resized } = result;
        let resized = self.config.resizer.resize(jpeg.as_ref(), max_border)?;
        let (jpeg, was_resized) = match resized {
            Cow::Borrowed(_) => (jpeg, false),
            Cow::Owned(buf) => (Cow::Owned(buf), true),
        };

        Ok(ThumbnailResult { jpeg, orientation, is_rotated, is_resized: is_resized || was_resized })
    }
}

pub type Exporter = ThumbnailExporter;
