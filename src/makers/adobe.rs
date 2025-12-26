#![allow(dead_code)]

use once_cell::sync::Lazy;
use std::borrow::Cow;
use std::collections::HashSet;

use crate::describe_exif_rule;
use crate::rawthumb::core::errors::{DecodingError, Result};
use crate::rawthumb::core::exif::{
    ExifError, ExifFieldError, ExifNames, ExifParsingRule, ExifReader, ParsedExif,
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
            0x0100 : u16 / image_width // use width tag to force Horizontal orientation
            0x0111 ? / thumbnail(thumbnail_len)
            0x0117 ? / thumbnail_len
            0x0201 ? / preview_offset
            0x0202 ? / preview_len
            0x0110 ? / main_preview_offset
            0x0111 ? / main_preview_len
        }
    })
});

#[derive(Default)]
struct AdobeDecoder;

impl AdobeDecoder {
    fn find_jpeg_ifd_preview<'a>(&self, buffer: &'a [u8]) -> Option<(Option<u16>, &'a [u8])> {
        if buffer.len() < 8 {
            return None;
        }
        let (le, first_ifd) = match &buffer[..4] {
            [0x49, 0x49, 0x2a, 0x00] => (true, Self::read_u32(buffer, 4, true)?),
            [0x4d, 0x4d, 0x00, 0x2a] => (false, Self::read_u32(buffer, 4, false)?),
            _ => return None,
        };

        let mut stack = vec![first_ifd];
        let mut visited = HashSet::new();
        let mut best: Option<(bool, Option<u16>, &'a [u8])> = None;

        while let Some(offset) = stack.pop() {
            let offset = offset as usize;
            if offset == 0 || offset >= buffer.len() || !visited.insert(offset) {
                continue;
            }

            let entry_count = match Self::read_u16(buffer, offset, le) {
                Some(c) => c as usize,
                None => continue,
            };
            let entries_start = offset.checked_add(2)?;
            let entries_bytes = entry_count.checked_mul(12)?;
            let entries_end = entries_start.checked_add(entries_bytes)?;
            if entries_end + 4 > buffer.len() {
                continue;
            }

            let mut compression: Option<u16> = None;
            let mut photometric: Option<u16> = None;
            let mut new_subfile: Option<u32> = None;
            let mut jpeg_offset: Option<u32> = None;
            let mut jpeg_len: Option<u32> = None;
            let mut strip_offset: Option<u32> = None;
            let mut strip_len: Option<u32> = None;
            let mut sub_ifds: Vec<u32> = Vec::new();
            let mut orientation_tag: Option<u16> = None;

            for i in 0..entry_count {
                let entry_offset = entries_start + i * 12;
                let tag = Self::read_u16(buffer, entry_offset, le)?;
                let type_id = Self::read_u16(buffer, entry_offset + 2, le)?;
                let count = Self::read_u32(buffer, entry_offset + 4, le)?;
                let value_offset = Self::read_u32(buffer, entry_offset + 8, le)?;
                let value_bytes = &buffer[entry_offset + 8..entry_offset + 12];

                match tag {
                    0x0103 => {
                        compression = Self::read_first_value(
                            buffer,
                            le,
                            type_id,
                            count,
                            value_offset,
                            value_bytes,
                        )
                        .map(|v| v as u16);
                    }
                    0x0106 => {
                        photometric = Self::read_first_value(
                            buffer,
                            le,
                            type_id,
                            count,
                            value_offset,
                            value_bytes,
                        )
                        .map(|v| v as u16);
                    }
                    0x00fe => {
                        new_subfile = Self::read_first_value(
                            buffer,
                            le,
                            type_id,
                            count,
                            value_offset,
                            value_bytes,
                        );
                    }
                    0x0201 => {
                        jpeg_offset = Self::read_first_value(
                            buffer,
                            le,
                            type_id,
                            count,
                            value_offset,
                            value_bytes,
                        );
                    }
                    0x0202 => {
                        jpeg_len = Self::read_first_value(
                            buffer,
                            le,
                            type_id,
                            count,
                            value_offset,
                            value_bytes,
                        );
                    }
                    0x0111 => {
                        strip_offset = Self::read_first_value(
                            buffer,
                            le,
                            type_id,
                            count,
                            value_offset,
                            value_bytes,
                        );
                    }
                    0x0117 => {
                        strip_len = Self::read_first_value(
                            buffer,
                            le,
                            type_id,
                            count,
                            value_offset,
                            value_bytes,
                        );
                    }
                    0x014a => {
                        let offsets = Self::read_values(
                            buffer,
                            le,
                            type_id,
                            count,
                            value_offset,
                            value_bytes,
                        );
                        sub_ifds.extend(offsets);
                    }
                    0x0112 => {
                        orientation_tag = Self::read_first_value(
                            buffer,
                            le,
                            type_id,
                            count,
                            value_offset,
                            value_bytes,
                        )
                        .map(|v| v as u16);
                    }
                    _ => {}
                }
            }

            if compression == Some(7) && matches!(photometric, Some(2) | Some(6)) {
                let (data_offset, data_len) = if let (Some(o), Some(l)) = (jpeg_offset, jpeg_len) {
                    (o as usize, l as usize)
                } else if let (Some(o), Some(l)) = (strip_offset, strip_len) {
                    (o as usize, l as usize)
                } else {
                    (0usize, 0usize)
                };

                if let Some(slice) = Self::fixup_jpeg_slice(buffer, data_offset, data_len) {
                    let reduced = new_subfile.map(|v| v & 1 == 1).unwrap_or(false);
                    let replace = match &best {
                        None => true,
                        Some((prev_reduced, _, prev_slice)) => {
                            if reduced != *prev_reduced {
                                reduced
                            } else {
                                slice.len() > prev_slice.len()
                            }
                        }
                    };
                    if replace {
                        best = Some((reduced, orientation_tag, slice));
                    }
                }
            }

            for sub in sub_ifds {
                if sub != 0 {
                    stack.push(sub);
                }
            }

            if let Some(next_ifd_offset) = Self::read_u32(buffer, entries_end, le) {
                if next_ifd_offset != 0 {
                    stack.push(next_ifd_offset);
                }
            }
        }

        best.map(|(_, o, s)| (o, s))
    }

    fn fixup_jpeg_slice<'a>(buffer: &'a [u8], offset: usize, len: usize) -> Option<&'a [u8]> {
        if offset >= buffer.len() {
            return None;
        }
        let max_end = buffer
            .len()
            .min(offset.saturating_add(len).saturating_add(32 * 1024 * 1024));
        let window = &buffer[offset..max_end];

        if window.len() < 2 || window[0] != 0xff || window[1] != 0xd8 {
            return None;
        }

        ImageHelper::extract_valid_jpeg_with_cap(window, window.len(), 1024, true).filter(|s| {
            let start_match = std::ptr::eq(s.as_ptr(), window.as_ptr());
            start_match
        })
    }

    fn read_u16(buffer: &[u8], offset: usize, le: bool) -> Option<u16> {
        if offset + 2 > buffer.len() {
            return None;
        }
        let bytes = [buffer[offset], buffer[offset + 1]];
        Some(if le {
            u16::from_le_bytes(bytes)
        } else {
            u16::from_be_bytes(bytes)
        })
    }

    fn read_u32(buffer: &[u8], offset: usize, le: bool) -> Option<u32> {
        if offset + 4 > buffer.len() {
            return None;
        }
        let bytes = [
            buffer[offset],
            buffer[offset + 1],
            buffer[offset + 2],
            buffer[offset + 3],
        ];
        Some(if le {
            u32::from_le_bytes(bytes)
        } else {
            u32::from_be_bytes(bytes)
        })
    }

    fn read_first_value(
        buffer: &[u8],
        le: bool,
        type_id: u16,
        count: u32,
        value_offset: u32,
        value_bytes: &[u8],
    ) -> Option<u32> {
        Self::read_values(buffer, le, type_id, count, value_offset, value_bytes)
            .into_iter()
            .next()
    }

    fn read_values(
        buffer: &[u8],
        le: bool,
        type_id: u16,
        count: u32,
        value_offset: u32,
        value_bytes: &[u8],
    ) -> Vec<u32> {
        let mut out = Vec::new();
        if count == 0 {
            return out;
        }

        let value_size: usize = match type_id {
            1 | 2 | 6 | 7 => 1,
            3 | 8 => 2,
            4 | 9 => 4,
            _ => return out,
        };
        let total_size = value_size.saturating_mul(count as usize);
        if total_size <= 4 {
            for idx in 0..count.min(4) as usize {
                let start = idx * value_size;
                if start + value_size > value_bytes.len() {
                    break;
                }
                let val = match value_size {
                    1 => value_bytes[start] as u32,
                    2 => {
                        let bytes = [value_bytes[start], value_bytes[start + 1]];
                        if le {
                            u16::from_le_bytes(bytes) as u32
                        } else {
                            u16::from_be_bytes(bytes) as u32
                        }
                    }
                    4 => {
                        let bytes = [
                            value_bytes[start],
                            value_bytes[start + 1],
                            value_bytes[start + 2],
                            value_bytes[start + 3],
                        ];
                        if le {
                            u32::from_le_bytes(bytes)
                        } else {
                            u32::from_be_bytes(bytes)
                        }
                    }
                    _ => 0,
                };
                out.push(val);
            }
            return out;
        }

        let base = value_offset as usize;
        if base + total_size > buffer.len() {
            return out;
        }

        for idx in 0..count as usize {
            let start = base + idx * value_size;
            if start + value_size > buffer.len() {
                break;
            }
            let val = match value_size {
                1 => buffer[start] as u32,
                2 => {
                    let bytes = [buffer[start], buffer[start + 1]];
                    if le {
                        u16::from_le_bytes(bytes) as u32
                    } else {
                        u16::from_be_bytes(bytes) as u32
                    }
                }
                4 => {
                    let bytes = [
                        buffer[start],
                        buffer[start + 1],
                        buffer[start + 2],
                        buffer[start + 3],
                    ];
                    if le {
                        u32::from_le_bytes(bytes)
                    } else {
                        u32::from_be_bytes(bytes)
                    }
                }
                _ => 0,
            };
            out.push(val);
        }

        out
    }

    fn get_thumbnail<'a>(
        &self,
        parsed: &ParsedExif,
        buffer: &'a [u8],
    ) -> std::result::Result<(Option<u16>, &'a [u8]), DecodingError> {
        if let Some((orientation_tag, slice)) = self.find_jpeg_ifd_preview(buffer) {
            return Ok((orientation_tag, slice));
        }

        let exif_preview = self
            .try_slice(
                parsed,
                ExifNames::PREVIEW_OFFSET,
                ExifNames::PREVIEW_LEN,
                buffer,
            )
            .or_else(|| {
                self.try_slice(
                    parsed,
                    ExifNames::THUMBNAIL,
                    ExifNames::THUMBNAIL_LEN,
                    buffer,
                )
            })
            .or_else(|| self.try_slice(parsed, "main_preview_offset", "main_preview_len", buffer))
            .filter(|slice| slice.len() > 100 * 1024);

        if let Some(slice) = exif_preview {
            let orientation = parsed.u16(ExifNames::ORIENTATION).ok();
            return Ok((orientation, slice));
        }

        if let Some(jpeg) = Self::quick_jpeg_scan(buffer, 64 * 1024 * 1024, 16 * 1024) {
            return Ok((None, jpeg));
        }

        if let Some(jpeg) = ImageHelper::extract_best_jpeg(buffer) {
            return Ok((None, jpeg));
        }

        Err(DecodingError::RawInfoError(
            ExifFieldError::field_not_found(ExifNames::THUMBNAIL),
        ))
    }

    fn get_orientation(&self, parsed: &ParsedExif) -> Orientation {
        match parsed.u16(ExifNames::ORIENTATION).ok() {
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
        parsed: &ParsedExif,
        offset_name: &str,
        len_name: &str,
        buffer: &'a [u8],
    ) -> Option<&'a [u8]> {
        let offset = parsed.usize(offset_name).ok()?;
        let len = parsed.usize(len_name).ok()?;
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

    fn quick_jpeg_scan<'a>(
        buffer: &'a [u8],
        max_scan_bytes: usize,
        min_size: usize,
    ) -> Option<&'a [u8]> {
        ImageHelper::extract_valid_jpeg_with_cap(buffer, max_scan_bytes, min_size, true)
            .filter(|s| ImageHelper::is_decodable_jpeg(s))
    }
}

#[allow(dead_code)]
#[derive(Default)]
pub struct AdobeThumbnailExtractor {
    decoder: AdobeDecoder,
}

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
        let raw_info = match exif.parse_with_prev_info(buffer, &THUMBNAIL_RULE, parsed) {
            Ok(info) => info,
            Err(ExifError::Parse(quickexif::parser::Error::TagNotFound(_))) => {
                let (orientation_tag, jpeg) = self
                    .decoder
                    .find_jpeg_ifd_preview(buffer)
                    .or_else(|| {
                        AdobeDecoder::quick_jpeg_scan(buffer, 64 * 1024 * 1024, 16 * 1024)
                            .map(|s| (None, s))
                    })
                    .or_else(|| {
                        ImageHelper::extract_best_jpeg_capped(buffer, buffer.len())
                            .filter(|slice| ImageHelper::is_decodable_jpeg(slice))
                            .map(|s| (None, s))
                    })
                    .ok_or_else(|| {
                        DecodingError::RawInfoError(ExifFieldError::field_not_found(
                            ExifNames::THUMBNAIL,
                        ))
                    })?;
                let orientation = ImageHelper::orientation_from_tag(orientation_tag)
                    .unwrap_or(Orientation::Horizontal);
                return Ok(ThumbnailResult {
                    jpeg: Cow::Borrowed(jpeg),
                    orientation,
                });
            }
            Err(e) => return Err(e.into()),
        };
        let (orientation_tag, thumbnail) = self.decoder.get_thumbnail(&raw_info, buffer)?;
        let orientation = ImageHelper::orientation_from_tag(orientation_tag)
            .unwrap_or_else(|| self.decoder.get_orientation(&raw_info));
        Ok(ThumbnailResult {
            jpeg: Cow::Borrowed(thumbnail),
            orientation,
        })
    }
}
