#![allow(dead_code)]

use once_cell::sync::Lazy;

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

struct PanasonicDecoder {
    exif: ParsedExif,
}

impl PanasonicDecoder {
    fn new(exif: ParsedExif) -> Self {
        Self { exif }
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

    fn get_thumbnail<'a>(&self, buffer: &'a [u8]) -> std::result::Result<&'a [u8], DecodingError> {
        let offset = self.exif.usize(ExifNames::THUMBNAIL)?;
        let len = self.exif.usize(ExifNames::THUMBNAIL_LEN)?;
        Ok(&buffer[offset..offset + len])
    }
}

#[allow(dead_code)]
pub struct PanasonicThumbnailExtractor;

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
        let decoder = PanasonicDecoder::new(raw_info);
        let thumbnail = decoder.get_thumbnail(buffer)?;
        let orientation: Orientation = decoder.get_orientation().into();
        Ok(ThumbnailResult {
            jpeg: thumbnail,
            orientation,
        })
    }
}
