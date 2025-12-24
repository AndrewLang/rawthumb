#![allow(dead_code)]

use once_cell::sync::Lazy;

use crate::rawthumb::core::errors::{DecodingError, Result};
use crate::rawthumb::core::thumbnail::ThumbnailExtractor;
use crate::rawthumb::core::types::{BasicInfo, Orientation, ThumbnailResult};

static THUMBNAIL_RULE: Lazy<quickexif::ParsingRule> = Lazy::new(|| {
    quickexif::describe_rule!(tiff {
        0x0112 / orientation
        next {
            0x0201 / thumbnail
            0x0202 / thumbnail_len
        }
    })
});

struct FujiDecoder {
    info: quickexif::ParsedInfo,
}

impl FujiDecoder {
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
        let jpeg_header_offset = 12;
        let tiny_thumbnail_offset = jpeg_header_offset + offset + len;

        let jpeg_eoi = &buffer[tiny_thumbnail_offset..]
            .windows(2)
            .enumerate()
            .find(|(_, data)| data == &[0xff, 0xd9]);

        match jpeg_eoi {
            None => Ok(&buffer[offset..tiny_thumbnail_offset]),
            &Some((index, _)) => Ok(&buffer[..tiny_thumbnail_offset + index + 2]),
        }
    }
}

#[allow(dead_code)]
pub struct FujiThumbnailExtractor;

impl ThumbnailExtractor for FujiThumbnailExtractor {
    fn supports_make(&self, make: &str) -> bool {
        make == "FUJIFILM"
    }

    fn extract<'a>(
        &self,
        buffer: &'a [u8],
        _info: &BasicInfo,
        parsed: quickexif::ParsedInfo,
    ) -> Result<ThumbnailResult<'a>> {
        let raw_info = quickexif::parse_with_prev_info(buffer, &THUMBNAIL_RULE, parsed)?;
        let decoder = FujiDecoder::new(raw_info);
        let thumbnail = decoder.get_thumbnail(buffer)?;
        let orientation: Orientation = decoder.get_orientation().into();
        Ok(ThumbnailResult { jpeg: thumbnail, orientation })
    }
}
