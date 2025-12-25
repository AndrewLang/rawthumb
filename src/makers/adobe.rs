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

struct AdobeDecoder {
    exif: ParsedExif,
}

impl AdobeDecoder {
    fn new(exif: ParsedExif) -> Self {
        Self { exif }
    }

    fn get_thumbnail<'a>(&self, buffer: &'a [u8]) -> std::result::Result<&'a [u8], DecodingError> {
        let exif_preview = self
            .try_slice(ExifNames::PREVIEW_OFFSET, ExifNames::PREVIEW_LEN, buffer)
            .or_else(|| self.try_slice(ExifNames::THUMBNAIL, ExifNames::THUMBNAIL_LEN, buffer))
            .or_else(|| self.try_slice("main_preview_offset", "main_preview_len", buffer))
            .filter(|slice| slice.len() > 100 * 1024);

        if let Some(slice) = exif_preview {
            return Ok(slice);
        }

        if let Some(jpeg) = ImageHelper::extract_best_jpeg(buffer) {
            return Ok(jpeg);
        }

        if let Some(jpeg) = ImageHelper::extract_largest_jpeg_segment(buffer) {
            return Ok(jpeg);
        }

        Err(DecodingError::RawInfoError(
            ExifFieldError::field_not_found(ExifNames::THUMBNAIL),
        ))
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

    fn try_slice<'a>(
        &self,
        offset_name: &str,
        len_name: &str,
        buffer: &'a [u8],
    ) -> Option<&'a [u8]> {
        let offset = self.exif.usize(offset_name).ok()?;
        let len = self.exif.usize(len_name).ok()?;
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
}

#[allow(dead_code)]
pub struct AdobeThumbnailExtractor;

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
        let decoder = AdobeDecoder::new(raw_info);
        let thumbnail = decoder.get_thumbnail(buffer)?;
        let orientation: Orientation = decoder.get_orientation().into();
        Ok(ThumbnailResult {
            jpeg: thumbnail,
            orientation,
        })
    }
}
