#![allow(dead_code)]

use once_cell::sync::Lazy;

use crate::rawthumb::core::errors::{Error, Result};
use crate::rawthumb::core::types::{BasicInfo, ParsedBasicInfo};

static BASIC_INFO_RULE: Lazy<quickexif::ParsingRule> = Lazy::new(|| {
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

pub fn parse_basic_info(_buffer: &[u8]) -> Option<ParsedBasicInfo> {
    None
}

#[allow(dead_code)]
pub struct BasicInfoParser;

impl BasicInfoParser {
    pub fn parse(buffer: &[u8]) -> Result<(quickexif::ParsedInfo, BasicInfo)> {
        let parsed = quickexif::parse(buffer, &BASIC_INFO_RULE).map_err(Error::from)?;
        let basic = BasicInfo {
            make: parsed
                .str("make")
                .unwrap_or_default()
                .to_string(),
            model: parsed
                .str("model")
                .unwrap_or_default()
                .to_string(),
            dng_version: parsed.u16("dng_version").ok(),
            cfa_pattern: parsed.u8a4("cfa_pattern").ok(),
        };
        Ok((parsed, basic))
    }
}
