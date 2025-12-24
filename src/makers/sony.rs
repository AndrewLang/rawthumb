#![allow(dead_code)]

use once_cell::sync::Lazy;

use crate::rawthumb::core::errors::{DecodingError, Result};
use crate::rawthumb::core::thumbnail::ThumbnailExtractor;
use crate::rawthumb::core::types::{BasicInfo, Orientation, ThumbnailResult};

static THUMBNAIL_RULE: Lazy<quickexif::ParsingRule> = Lazy::new(|| {
    quickexif::describe_rule!(tiff {
        0x0112 / orientation
        0x0201 / preview_offset
        0x0202 / preview_len
    })
});

struct SonyDecoder {
    info: quickexif::ParsedInfo,
}

impl SonyDecoder {
    fn new(info: quickexif::ParsedInfo) -> Self {
        Self { info }
    }

    fn get_thumbnail<'a>(&self, buffer: &'a [u8]) -> std::result::Result<&'a [u8], DecodingError> {
        let offset = self.info.usize("preview_offset")?;
        let len = self.info.usize("preview_len")?;
        Ok(&buffer[offset..offset + len])
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
}

#[allow(dead_code)]
pub struct SonyThumbnailExtractor;

impl ThumbnailExtractor for SonyThumbnailExtractor {
    fn supports_make(&self, make: &str) -> bool {
        make == "SONY"
    }

    fn extract<'a>(
        &self,
        buffer: &'a [u8],
        _info: &BasicInfo,
        parsed: quickexif::ParsedInfo,
    ) -> Result<ThumbnailResult<'a>> {
        let raw_info = quickexif::parse_with_prev_info(buffer, &THUMBNAIL_RULE, parsed)?;
        let decoder = SonyDecoder::new(raw_info);
        let thumbnail = decoder.get_thumbnail(buffer)?;
        let orientation: Orientation = decoder.get_orientation().into();
        Ok(ThumbnailResult { jpeg: thumbnail, orientation })
    }
}
