#![allow(dead_code)]

use std::borrow::Cow;

use crate::rawthumb::core::basic_info::BasicInfoParser;
use crate::rawthumb::core::errors::{Error, Result};
use crate::rawthumb::core::types::Orientation;
use crate::rawthumb::core::types::ThumbnailResult;
use crate::rawthumb::formats::cr3::Cr3FastPath;
use crate::rawthumb::formats::fuji::FujiFix;
use crate::rawthumb::selector::select_and_decode_thumbnail;

fn is_tiff_header(bytes: &[u8]) -> bool {
    bytes == [0x49, 0x49, 0x2a, 0x00] || bytes == [0x4d, 0x4d, 0x00, 0x2a]
}

fn canon_cr3_exif_slice(buffer: &[u8]) -> Option<&[u8]> {
    const EXIF_HEADER: &[u8] = b"Exif\0\0";

    if let Some(pos) = buffer
        .windows(EXIF_HEADER.len())
        .position(|window| window == EXIF_HEADER)
    {
        let after_exif = pos + EXIF_HEADER.len();
        if let Some(header) = buffer.get(after_exif..after_exif + 4) {
            if is_tiff_header(header) {
                return buffer.get(after_exif..);
            }
        }
        if let Some(rel) = buffer[after_exif..]
            .windows(4)
            .position(|w| is_tiff_header(w))
        {
            let start = after_exif + rel;
            return buffer.get(start..);
        }
    }

    if let Some(pos) = buffer.windows(4).position(|w| is_tiff_header(w)) {
        return buffer.get(pos..);
    }

    None
}

pub(crate) fn largest_jpeg_slice(buffer: &[u8]) -> Option<&[u8]> {
    let mut start = 0usize;
    let mut best: Option<(usize, usize)> = None;
    while let Some(rel_soi) = buffer[start..]
        .windows(3)
        .position(|w| w == [0xff, 0xd8, 0xff])
    {
        let soi = start + rel_soi;
        if let Some(rel_eoi) = buffer[soi + 3..].windows(2).position(|w| w == [0xff, 0xd9]) {
            let end = soi + 3 + rel_eoi + 2;
            let len = end - soi;
            if best.map(|(_, b_len)| len > b_len).unwrap_or(true) {
                best = Some((soi, len));
            }
            start = end;
        } else {
            break;
        }
    }
    best.map(|(s, l)| &buffer[s..s + l])
}

fn parse_basic_info_with_fallback<'a>(
    buffer: &'a [u8],
) -> std::result::Result<
    (
        quickexif::ParsedInfo,
        crate::rawthumb::core::types::BasicInfo,
        &'a [u8],
    ),
    Error,
> {
    let buffer_cow: Cow<'a, [u8]> = FujiFix::apply(buffer);
    let buffer_ref: &'a [u8] = match buffer_cow {
        Cow::Borrowed(b) => b,
        Cow::Owned(_) => unreachable!("FujiFix does not allocate"),
    };
    match BasicInfoParser::parse(buffer_ref) {
        Ok((info, basic)) => Ok((info, basic, buffer_ref)),
        Err(e) => {
            if let Some(exif_buffer) = canon_cr3_exif_slice(buffer_ref) {
                let (info, basic) = BasicInfoParser::parse(exif_buffer)?;
                Ok((info, basic, exif_buffer))
            } else {
                Err(e)
            }
        }
    }
}

pub fn get_thumbnail(buffer: &[u8]) -> Result<ThumbnailResult<'_>> {
    println!("Decoding thumbnail with raw thumb");
    let buffer_cow: Cow<'_, [u8]> = FujiFix::apply(buffer);
    let buffer = match buffer_cow {
        Cow::Borrowed(b) => b,
        Cow::Owned(_) => unreachable!("FujiFix does not allocate"),
    };

    if let Some(result) = Cr3FastPath::try_extract(buffer) {
        return Ok(result);
    }

    match parse_basic_info_with_fallback(buffer) {
        Ok((parsed, basic, buf)) => {
            let result = select_and_decode_thumbnail(buf, parsed, basic).map_err(Error::from)?;
            Ok(result)
        }
        Err(e) => {
            if let Some(jpeg) = largest_jpeg_slice(buffer) {
                Ok(ThumbnailResult {
                    jpeg,
                    orientation: Orientation::Horizontal,
                })
            } else {
                Err(e)
            }
        }
    }
}
