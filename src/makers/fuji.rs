#![allow(dead_code)]

use once_cell::sync::Lazy;
use std::borrow::Cow;

use crate::describe_exif_rule;
use crate::rawthumb::core::errors::{DecodingError, Result};
use crate::rawthumb::core::exif::{ExifNames, ExifParsingRule, ExifReader, ParsedExif};
use crate::rawthumb::core::image_helper::ImageHelper;
use crate::rawthumb::core::thumbnail_extractor::ThumbnailExtractor;
use crate::rawthumb::core::types::{Orientation, RawMetadata, ThumbnailResult};

static THUMBNAIL_RULE: Lazy<ExifParsingRule> = Lazy::new(|| {
    describe_exif_rule!(tiff {
        0x0112 / orientation
        next {
            0x0201 / thumbnail
            0x0202 / thumbnail_len
        }
    })
});

#[derive(Default)]
struct FujiDecoder;

impl FujiDecoder {
    fn get_orientation(&self, exif: &ParsedExif) -> Option<Orientation> {
        ImageHelper::orientation_from_tag(exif.u16(ExifNames::ORIENTATION).ok())
    }

    fn get_thumbnail<'a>(&self, buffer: &'a [u8], exif: &ParsedExif) -> std::result::Result<&'a [u8], DecodingError> {
        let offset = exif.usize(ExifNames::THUMBNAIL)?;
        let len = exif.usize(ExifNames::THUMBNAIL_LEN)?;
        let jpeg_header_offset = 12;
        let tiny_thumbnail_offset = jpeg_header_offset + offset + len;

        let jpeg_eoi = &buffer[tiny_thumbnail_offset..].windows(2).enumerate().find(|(_, data)| data == &[0xff, 0xd9]);

        match jpeg_eoi {
            None => Ok(&buffer[offset..tiny_thumbnail_offset]),
            &Some((index, _)) => Ok(&buffer[..tiny_thumbnail_offset + index + 2]),
        }
    }
}

#[allow(dead_code)]
#[derive(Default)]
pub struct FujiThumbnailExtractor {
    decoder: FujiDecoder,
}

impl ThumbnailExtractor for FujiThumbnailExtractor {
    fn supports_make(&self, make: &str) -> bool {
        make == "FUJIFILM"
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
        let orientation = self.decoder.get_orientation(&raw_info).unwrap_or(Orientation::Horizontal);

        log::trace!("Fuji thumbnail orientation from EXIF: {:?}", orientation);

        Ok(ThumbnailResult::new(Cow::Borrowed(thumbnail), orientation))
    }
}
