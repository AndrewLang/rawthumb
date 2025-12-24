#![allow(dead_code)]

use once_cell::sync::Lazy;

use crate::rawthumb::core::errors::{ImageProcessingError, Result};
use crate::rawthumb::core::image_helper::ImageHelper;
use crate::rawthumb::core::types::RawMetadata;

static RAW_METADATA_RULE: Lazy<quickexif::ParsingRule> = Lazy::new(|| {
    quickexif::describe_rule!(tiff {
        0x010f {
            str + 0 / make
        }
        0x0110 {
            str + 0 / model
        }
        0x828e? / cfa_pattern
        0xc612? / dng_version
        if dng_version ? {
            0xc614 {
                str + 0 / make_model
            }
            if cfa_pattern ? {
                0xc622 { // for normal dng
                    r64 + 0 / c0
                    r64 + 1 / c1
                    r64 + 2 / c2
                    r64 + 3 / c3
                    r64 + 4 / c4
                    r64 + 5 / c5
                    r64 + 6 / c6
                    r64 + 7 / c7
                    r64 + 8 / c8
                }
            } else {
                0xc621 { // for Apple ProRaw
                    r64 + 0 / c0
                    r64 + 1 / c1
                    r64 + 2 / c2
                    r64 + 3 / c3
                    r64 + 4 / c4
                    r64 + 5 / c5
                    r64 + 6 / c6
                    r64 + 7 / c7
                    r64 + 8 / c8
                }
            }
        }
    })
});

#[allow(dead_code)]
pub struct RawMetadataParser;

impl RawMetadataParser {
    pub fn parse(buffer: &[u8]) -> Result<(quickexif::ParsedInfo, RawMetadata)> {
        let parsed =
            quickexif::parse(buffer, &RAW_METADATA_RULE).map_err(ImageProcessingError::from)?;
        let basic = RawMetadata {
            make: parsed.str("make").unwrap_or_default().to_string(),
            model: parsed.str("model").unwrap_or_default().to_string(),
            dng_version: parsed.u16("dng_version").ok(),
            cfa_pattern: parsed.u8a4("cfa_pattern").ok(),
        };
        Ok((parsed, basic))
    }

    pub fn parse_with_cr3_fallback(
        buffer: &[u8],
    ) -> Result<(quickexif::ParsedInfo, RawMetadata, &[u8])> {
        match Self::parse(buffer) {
            Ok((info, basic)) => Ok((info, basic, buffer)),
            Err(e) => {
                if let Some(exif_buffer) = ImageHelper::extract_canon_cr3_exif_segment(buffer) {
                    let (info, basic) = Self::parse(exif_buffer)?;
                    Ok((info, basic, exif_buffer))
                } else {
                    Err(e)
                }
            }
        }
    }
}
