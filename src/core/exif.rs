#![allow(dead_code)]

use once_cell::sync::Lazy;

use crate::rawthumb::core::errors::{ExifError, ExifFieldError};
pub use crate::rawthumb::core::exif_names::ExifNames;
use crate::rawthumb::core::types::{Orientation, RawMetadata};

pub type ExifResult<T> = Result<T, ExifError>;
pub type ExifFieldResult<T> = Result<T, ExifFieldError>;

#[macro_export]
macro_rules! describe_exif_rule {
    ($($rule:tt)*) => {
        $crate::rawthumb::core::exif::ExifParsingRule::new(quickexif::describe_rule!($($rule)*))
    };
}

#[derive(Clone, Debug)]
pub struct ExifParsingRule {
    inner: quickexif::ParsingRule,
}

impl ExifParsingRule {
    pub fn new(inner: quickexif::ParsingRule) -> Self {
        Self { inner }
    }

    pub fn inner(&self) -> &quickexif::ParsingRule {
        &self.inner
    }
}

pub struct ParsedExif {
    inner: quickexif::ParsedInfo,
}

impl ParsedExif {
    pub fn debug_summary(&self) -> String {
        format!(
            "ParsedExif(fields={})",
            self.inner
                .stringify_all()
                .unwrap_or_default()
                .lines()
                .count()
        )
    }

    pub fn u16(&self, name: &str) -> ExifFieldResult<u16> {
        self.inner.u16(name).map_err(ExifFieldError::from)
    }

    pub fn u32(&self, name: &str) -> ExifFieldResult<u32> {
        self.inner.u32(name).map_err(ExifFieldError::from)
    }

    pub fn usize(&self, name: &str) -> ExifFieldResult<usize> {
        self.inner.usize(name).map_err(ExifFieldError::from)
    }

    pub fn str(&self, name: &str) -> ExifFieldResult<&str> {
        self.inner.str(name).map_err(ExifFieldError::from)
    }

    pub fn u8a4(&self, name: &str) -> ExifFieldResult<[u8; 4]> {
        self.inner.u8a4(name).map_err(ExifFieldError::from)
    }

    pub fn orientation(&self) -> Orientation {
        match self.u16(ExifNames::ORIENTATION).ok() {
            Some(3) => Orientation::Rotate180,
            Some(6) => Orientation::Rotate90,
            Some(8) => Orientation::Rotate270,
            _ => Orientation::Horizontal,
        }
    }

    pub fn into_inner(self) -> quickexif::ParsedInfo {
        self.inner
    }
}

impl From<quickexif::ParsedInfo> for ParsedExif {
    fn from(value: quickexif::ParsedInfo) -> Self {
        Self { inner: value }
    }
}

pub trait ExifReader: Send + Sync {
    fn parse_raw_metadata(&self, buffer: &[u8]) -> ExifResult<(ParsedExif, RawMetadata)>;

    fn parse_with_rule(&self, buffer: &[u8], rule: &ExifParsingRule) -> ExifResult<ParsedExif>;

    fn parse_with_prev_info(
        &self,
        buffer: &[u8],
        rule: &ExifParsingRule,
        prev_info: ParsedExif,
    ) -> ExifResult<ParsedExif>;

    fn get_orientation(&self, buffer: &[u8]) -> Option<u16>;

    fn get_tag_u32(&self, buffer: &[u8], tag: u16) -> Option<u32>;

    fn get_tag_bytes<'a>(&self, buffer: &'a [u8], tag: u16) -> Option<&'a [u8]>;
}

static BASIC_INFO_RULE: Lazy<ExifParsingRule> = Lazy::new(|| {
    describe_exif_rule!(tiff {
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
            0xc621? { // for Apple ProRaw (optional)
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

static ORIENTATION_RULE: Lazy<ExifParsingRule> = Lazy::new(|| {
    describe_exif_rule!(tiff {
        0x0112 / tag_value
    })
});

#[derive(Clone, Debug)]
pub struct QuickExifReader;

impl QuickExifReader {
    pub fn new() -> Self {
        Self
    }

    #[inline]
    fn parse_rule(&self, buffer: &[u8], rule: &ExifParsingRule) -> ExifResult<ParsedExif> {
        quickexif::parse(buffer, rule.inner())
            .map(ParsedExif::from)
            .map_err(ExifError::from)
    }

    #[inline]
    fn parse_rule_with_prev(
        &self,
        buffer: &[u8],
        rule: &ExifParsingRule,
        prev_info: ParsedExif,
    ) -> ExifResult<ParsedExif> {
        quickexif::parse_with_prev_info(buffer, rule.inner(), prev_info.into_inner())
            .map(ParsedExif::from)
            .map_err(ExifError::from)
    }

    fn parse_tag<'a>(
        &self,
        buffer: &'a [u8],
        tag: u16,
        is_value_u16: bool,
        len_name: Option<&'static str>,
    ) -> Option<ParsedExif> {
        let rule = ExifParsingRule::new(quickexif::ParsingRule::Tiff(vec![
            quickexif::ParsingRule::TagItem {
                tag,
                name: ExifNames::TAG_VALUE,
                len: len_name,
                is_optional: true,
                is_value_u16,
            },
        ]));
        self.parse_rule(buffer, &rule).ok()
    }
}

impl ExifReader for QuickExifReader {
    fn parse_raw_metadata(&self, buffer: &[u8]) -> ExifResult<(ParsedExif, RawMetadata)> {
        let parsed = self.parse_rule(buffer, &BASIC_INFO_RULE)?;
        let basic = RawMetadata {
            make: parsed.str(ExifNames::MAKE).unwrap_or_default().to_string(),
            model: parsed.str(ExifNames::MODEL).unwrap_or_default().to_string(),
            dng_version: parsed.u16(ExifNames::DNG_VERSION).ok(),
            cfa_pattern: parsed.u8a4(ExifNames::CFA_PATTERN).ok(),
        };
        Ok((parsed, basic))
    }

    fn parse_with_rule(&self, buffer: &[u8], rule: &ExifParsingRule) -> ExifResult<ParsedExif> {
        self.parse_rule(buffer, rule)
    }

    fn parse_with_prev_info(
        &self,
        buffer: &[u8],
        rule: &ExifParsingRule,
        prev_info: ParsedExif,
    ) -> ExifResult<ParsedExif> {
        self.parse_rule_with_prev(buffer, rule, prev_info)
    }

    fn get_orientation(&self, buffer: &[u8]) -> Option<u16> {
        quickexif::parse(buffer, ORIENTATION_RULE.inner())
            .ok()
            .map(ParsedExif::from)
            .and_then(|p| p.u16(ExifNames::TAG_VALUE).ok())
    }

    fn get_tag_u32(&self, buffer: &[u8], tag: u16) -> Option<u32> {
        self.parse_tag(buffer, tag, false, None)
            .and_then(|p| p.u32(ExifNames::TAG_VALUE).ok())
    }

    fn get_tag_bytes<'a>(&self, buffer: &'a [u8], tag: u16) -> Option<&'a [u8]> {
        let parsed = self.parse_tag(buffer, tag, false, Some(ExifNames::TAG_LEN))?;
        let offset = parsed.u32(ExifNames::TAG_VALUE).ok()? as usize;
        let len = parsed.u32(ExifNames::TAG_LEN).ok()? as usize;
        if offset + len <= buffer.len() {
            Some(&buffer[offset..offset + len])
        } else {
            None
        }
    }
}
