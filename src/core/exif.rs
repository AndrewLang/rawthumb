#![allow(dead_code)]

use once_cell::sync::Lazy;
use quickexif::parsed_info::Error as QuickExifFieldError;

use crate::rawthumb::core::types::RawMetadata;

pub type ExifResult<T> = Result<T, ExifError>;
pub type ExifFieldResult<T> = Result<T, ExifFieldError>;

#[derive(Debug, thiserror::Error)]
pub enum ExifError {
    #[error(transparent)]
    Parse(#[from] quickexif::parser::Error),
}

#[derive(Debug, thiserror::Error)]
pub enum ExifFieldError {
    #[error(transparent)]
    Field(#[from] QuickExifFieldError),
}

impl ExifFieldError {
    pub fn field_not_found(name: &str) -> Self {
        ExifFieldError::Field(QuickExifFieldError::FieldNotFound(name.to_owned()))
    }
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

#[macro_export]
macro_rules! describe_exif_rule {
    ($($rule:tt)*) => {
        $crate::rawthumb::core::exif::ExifParsingRule::new(quickexif::describe_rule!($($rule)*))
    };
}

pub struct ParsedExif {
    inner: quickexif::ParsedInfo,
}

impl ParsedExif {
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

#[derive(Clone, Debug)]
pub struct QuickExifReader;

impl QuickExifReader {
    pub fn new() -> Self {
        Self
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
                name: "tag_value",
                len: len_name,
                is_optional: true,
                is_value_u16,
            },
        ]));
        quickexif::parse(buffer, rule.inner())
            .ok()
            .map(ParsedExif::from)
    }
}

impl ExifReader for QuickExifReader {
    fn parse_raw_metadata(&self, buffer: &[u8]) -> ExifResult<(ParsedExif, RawMetadata)> {
        let parsed = quickexif::parse(buffer, BASIC_INFO_RULE.inner())?;
        let parsed = ParsedExif::from(parsed);
        let basic = RawMetadata {
            make: parsed.str("make").unwrap_or_default().to_string(),
            model: parsed.str("model").unwrap_or_default().to_string(),
            dng_version: parsed.u16("dng_version").ok(),
            cfa_pattern: parsed.u8a4("cfa_pattern").ok(),
        };
        Ok((parsed, basic))
    }

    fn parse_with_rule(&self, buffer: &[u8], rule: &ExifParsingRule) -> ExifResult<ParsedExif> {
        quickexif::parse(buffer, rule.inner())
            .map(ParsedExif::from)
            .map_err(ExifError::from)
    }

    fn parse_with_prev_info(
        &self,
        buffer: &[u8],
        rule: &ExifParsingRule,
        prev_info: ParsedExif,
    ) -> ExifResult<ParsedExif> {
        quickexif::parse_with_prev_info(buffer, rule.inner(), prev_info.into_inner())
            .map(ParsedExif::from)
            .map_err(ExifError::from)
    }

    fn get_orientation(&self, buffer: &[u8]) -> Option<u16> {
        self.parse_tag(buffer, 0x0112, true, None)
            .and_then(|p| p.u16("tag_value").ok())
    }

    fn get_tag_u32(&self, buffer: &[u8], tag: u16) -> Option<u32> {
        self.parse_tag(buffer, tag, false, None)
            .and_then(|p| p.u32("tag_value").ok())
    }

    fn get_tag_bytes<'a>(&self, buffer: &'a [u8], tag: u16) -> Option<&'a [u8]> {
        let parsed = self.parse_tag(buffer, tag, false, Some("tag_len"))?;
        let offset = parsed.u32("tag_value").ok()? as usize;
        let len = parsed.u32("tag_len").ok()? as usize;
        if offset + len <= buffer.len() {
            Some(&buffer[offset..offset + len])
        } else {
            None
        }
    }
}
