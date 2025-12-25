#![allow(dead_code)]

use once_cell::sync::Lazy;

use crate::describe_exif_rule;
use crate::rawthumb::core::errors::{DecodingError, Result};
use crate::rawthumb::core::exif::{
    ExifFieldError, ExifNames, ExifParsingRule, ExifReader, ParsedExif,
};
use crate::rawthumb::core::image_helper::ImageHelper;
use crate::rawthumb::core::thumbnail_extractor::ThumbnailExtractor;
use crate::rawthumb::core::types::{Orientation, RawMetadata, ThumbnailResult};

static THUMBNAIL_RULE: Lazy<ExifParsingRule> = Lazy::new(|| {
    describe_exif_rule!(tiff {
        0x0112 : u16 / orientation
        0x014a? / sub_ifd(sub_ifd_count)
        0x828e? / cfa_pattern
        if sub_ifd_count ?
        {
            if sub_ifd_count > 2
            {
                0x014a {
                    offset + 8 {
                        offset address {
                            0x0111 ? / thumbnail(thumbnail_len)
                            0x0117 ? / thumbnail_len
                            0x0201 ? / preview_offset
                            0x0202 ? / preview_len
                            0x0110 ? / main_preview_offset
                            0x0111 ? / main_preview_len
                        }
                    }
                }
            }
            else
            {
                if sub_ifd_count > 1
                {
                    0x014a {
                        offset + 4 {
                            offset address {
                                0x0111 ? / thumbnail(thumbnail_len)
                                0x0117 ? / thumbnail_len
                                0x0201 ? / preview_offset
                                0x0202 ? / preview_len
                                0x0110 ? / main_preview_offset
                                0x0111 ? / main_preview_len
                            }
                        }
                    }
                }
            }
        }
        if cfa_pattern ? {

        } else {
            0x0100 : u16 / orientation // use width tag to force Horizontal orientation
            0x0111 ? / thumbnail(thumbnail_len)
            0x0117 ? / thumbnail_len
            0x0201 ? / preview_offset
            0x0202 ? / preview_len
            0x0110 ? / main_preview_offset
            0x0111 ? / main_preview_len
        }
    })
});

#[derive(Default)]
struct AdobeDecoder;

impl AdobeDecoder {
    fn get_thumbnail<'a>(
        &self,
        parsed: &ParsedExif,
        buffer: &'a [u8],
    ) -> std::result::Result<&'a [u8], DecodingError> {
        let exif_preview = self
            .try_slice(
                parsed,
                ExifNames::PREVIEW_OFFSET,
                ExifNames::PREVIEW_LEN,
                buffer,
            )
            .or_else(|| {
                self.try_slice(
                    parsed,
                    ExifNames::THUMBNAIL,
                    ExifNames::THUMBNAIL_LEN,
                    buffer,
                )
            })
            .or_else(|| self.try_slice(parsed, "main_preview_offset", "main_preview_len", buffer))
            .filter(|slice| slice.len() > 100 * 1024);

        if let Some(slice) = exif_preview {
            return Ok(slice);
        }

        if let Some(jpeg) = Self::quick_jpeg_scan(buffer, 64 * 1024 * 1024, 16 * 1024) {
            return Ok(jpeg);
        }

        if let Some(jpeg) = ImageHelper::extract_best_jpeg(buffer) {
            return Ok(jpeg);
        }

        Err(DecodingError::RawInfoError(
            ExifFieldError::field_not_found(ExifNames::THUMBNAIL),
        ))
    }

    fn get_orientation(&self, parsed: &ParsedExif) -> Orientation {
        match parsed.u16(ExifNames::ORIENTATION).ok() {
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

    fn try_slice<'a>(
        &self,
        parsed: &ParsedExif,
        offset_name: &str,
        len_name: &str,
        buffer: &'a [u8],
    ) -> Option<&'a [u8]> {
        let offset = parsed.usize(offset_name).ok()?;
        let len = parsed.usize(len_name).ok()?;
        if len < 1024 {
            return None;
        }
        let end = offset.checked_add(len)?;
        if end > buffer.len() {
            return None;
        }
        let slice = &buffer[offset..end];
        if ImageHelper::is_valid_jpeg(slice) {
            Some(slice)
        } else {
            None
        }
    }

    fn quick_jpeg_scan<'a>(
        buffer: &'a [u8],
        max_scan_bytes: usize,
        min_size: usize,
    ) -> Option<&'a [u8]> {
        let scan_len = buffer.len().min(max_scan_bytes);
        let mut cursor = 0usize;

        while let Some(rel_soi) = buffer[cursor..scan_len]
            .windows(3)
            .position(|w| w == [0xff, 0xd8, 0xff])
        {
            let soi = cursor + rel_soi;
            if let Some(rel_eoi) = buffer[soi + 3..].windows(2).position(|w| w == [0xff, 0xd9]) {
                let end = soi + 3 + rel_eoi + 2;
                if let Some(slice) = buffer.get(soi..end) {
                    if slice.len() >= min_size && ImageHelper::jpeg_has_sof(slice) {
                        return Some(slice);
                    }
                }
                cursor = end;
                continue;
            } else {
                break;
            }
        }

        None
    }
}

#[allow(dead_code)]
#[derive(Default)]
pub struct AdobeThumbnailExtractor {
    decoder: AdobeDecoder,
}

impl ThumbnailExtractor for AdobeThumbnailExtractor {
    fn supports_make(&self, make: &str) -> bool {
        make == "ADOBE"
    }

    fn extract<'a>(
        &self,
        buffer: &'a [u8],
        _info: &RawMetadata,
        exif: &dyn ExifReader,
        parsed: ParsedExif,
    ) -> Result<ThumbnailResult<'a>> {
        let raw_info = exif.parse_with_prev_info(buffer, &THUMBNAIL_RULE, parsed)?;
        let thumbnail = self.decoder.get_thumbnail(&raw_info, buffer)?;
        let orientation: Orientation = self.decoder.get_orientation(&raw_info).into();
        Ok(ThumbnailResult {
            jpeg: thumbnail,
            orientation,
        })
    }
}
