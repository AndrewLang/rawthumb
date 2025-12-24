#![allow(dead_code)]

use once_cell::sync::Lazy;

use crate::rawthumb::core::errors::{DecodingError, Result};
use crate::rawthumb::core::thumbnail::ThumbnailExtractor;
use crate::rawthumb::core::types::{BasicInfo, Orientation, ThumbnailResult};

static THUMBNAIL_RULE: Lazy<quickexif::ParsingRule> = Lazy::new(|| {
    quickexif::describe_rule!(tiff {
        0x0112 / orientation
        0x002e / thumbnail(thumbnail_len)
    })
});

struct PanasonicDecoder {
    info: quickexif::ParsedInfo,
}

impl PanasonicDecoder {
    fn new(info: quickexif::ParsedInfo) -> Self {
        Self { info }
    }

    fn get_orientation(&self) -> Orientation {
        match self.info.u16("orientation").ok() {
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
        let offset = self.info.usize("thumbnail")?;
        let len = self.info.usize("thumbnail_len")?;
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
        _info: &BasicInfo,
        parsed: quickexif::ParsedInfo,
    ) -> Result<ThumbnailResult<'a>> {
        let raw_info = quickexif::parse_with_prev_info(buffer, &THUMBNAIL_RULE, parsed)?;
        let decoder = PanasonicDecoder::new(raw_info);
        let thumbnail = decoder.get_thumbnail(buffer)?;
        let orientation: Orientation = decoder.get_orientation().into();
        Ok(ThumbnailResult { jpeg: thumbnail, orientation })
    }
}
