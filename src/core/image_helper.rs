#![allow(dead_code)]

use memchr::memchr;

use crate::rawthumb::core::exif::{ExifNames, ParsedExif};

pub struct ImageHelper;

pub struct JpegSegment<'a> {
    pub data: &'a [u8],
    pub size: usize,
    pub is_valid_jpeg: bool,
    pub has_sof: bool,
    pub start: usize,
    pub end: usize,
    pub index: usize,
}

impl ImageHelper {
    pub fn is_tiff_header(bytes: &[u8]) -> bool {
        matches!(bytes, [0x49, 0x49, 0x2a, 0x00] | [0x4d, 0x4d, 0x00, 0x2a])
    }

    pub fn extract_canon_cr3_exif_segment(buffer: &[u8]) -> Option<&[u8]> {
        const EXIF_HEADER: &[u8] = b"Exif\0\0";

        if let Some(pos) = buffer
            .windows(EXIF_HEADER.len())
            .position(|window| window == EXIF_HEADER)
        {
            let after_exif = pos + EXIF_HEADER.len();
            if let Some(header) = buffer.get(after_exif..after_exif + 4) {
                if Self::is_tiff_header(header) {
                    return buffer.get(after_exif..);
                }
            }
            if let Some(rel) = buffer[after_exif..]
                .windows(4)
                .position(|w| Self::is_tiff_header(w))
            {
                let start = after_exif + rel;
                return buffer.get(start..);
            }
        }

        if let Some(pos) = buffer.windows(4).position(|w| Self::is_tiff_header(w)) {
            return buffer.get(pos..);
        }

        None
    }

    pub fn extract_largest_jpeg_segment(buffer: &[u8]) -> Option<&[u8]> {
        Self::extract_largest_jpeg_segment_capped(buffer, buffer.len())
    }

    pub fn extract_largest_jpeg_segment_capped<'a>(
        buffer: &'a [u8],
        max_scan_bytes: usize,
    ) -> Option<&'a [u8]> {
        let scan_end = buffer.len().min(max_scan_bytes);
        let mut cursor = 0usize;
        let mut best: Option<(usize, usize)> = None;

        while cursor < scan_end {
            let rel_ff = memchr(0xff, &buffer[cursor..scan_end])?;
            let soi = cursor + rel_ff;
            if soi + 1 >= scan_end || buffer[soi + 1] != 0xd8 {
                cursor = soi + 1;
                continue;
            }

            // find EOI
            if let Some(rel_ff2) = memchr(0xff, &buffer[soi + 2..]) {
                let mut idx = soi + 2 + rel_ff2;
                loop {
                    if idx + 1 >= buffer.len() {
                        break;
                    }
                    if buffer[idx + 1] == 0xd9 {
                        let end = idx + 2;
                        let len = end - soi;
                        if best.map(|(_, b_len)| len > b_len).unwrap_or(true) {
                            best = Some((soi, len));
                        }
                        cursor = end;
                        break;
                    }
                    if let Some(next_ff) = memchr(0xff, &buffer[idx + 2..]) {
                        idx = idx + 2 + next_ff;
                    } else {
                        cursor = scan_end;
                        break;
                    }
                }
            } else {
                break;
            }
        }

        best.map(|(s, l)| &buffer[s..s + l])
    }

    pub fn extract_all_jpeg_segments(buffer: &[u8]) -> Vec<JpegSegment<'_>> {
        let mut results = Vec::with_capacity(16);

        let len = buffer.len();
        let mut i = 0;
        let mut index = 0;

        while i + 1 < len {
            // SOI
            if buffer[i] == 0xFF && buffer[i + 1] == 0xD8 {
                let start = i;
                let mut j = i + 2;

                while j + 1 < len {
                    // Fast skip non-marker bytes
                    if buffer[j] != 0xFF {
                        j += 1;
                        continue;
                    }

                    let marker = buffer[j + 1];

                    // EOI
                    if marker == 0xD9 {
                        let end = j + 2;
                        let slice = &buffer[start..end];

                        // 🔴 CHANGED: DO NOT validate here (defer!)
                        results.push(JpegSegment {
                            // 🔴 CHANGED: no Vec allocation
                            data: slice,
                            size: end - start,
                            is_valid_jpeg: false, // filled later
                            has_sof: false,       // filled later
                            start,
                            end,
                            index,
                        });

                        index += 1;
                        i = end;
                        break;
                    }

                    // Restart markers or TEM
                    if (0xD0..=0xD7).contains(&marker) || marker == 0x01 {
                        j += 2;
                        continue;
                    }

                    // Need length bytes
                    if j + 3 >= len {
                        break;
                    }

                    let seg_len = u16::from_be_bytes([buffer[j + 2], buffer[j + 3]]) as usize;

                    if seg_len < 2 {
                        break;
                    }

                    // Jump to next marker
                    j += 2 + seg_len;
                }

                i += 2;
            } else {
                i += 1;
            }
        }

        for seg in &mut results {
            let slice = &buffer[seg.start..seg.end];
            seg.has_sof = Self::jpeg_has_sof(slice);
            seg.is_valid_jpeg = seg.has_sof && Self::is_valid_jpeg(slice);
        }

        results
    }

    pub fn extract_best_jpeg(buffer: &[u8]) -> Option<&[u8]> {
        Self::extract_best_jpeg_capped(buffer, buffer.len())
    }

    pub fn extract_best_jpeg_capped<'a>(
        buffer: &'a [u8],
        max_scan_bytes: usize,
    ) -> Option<&'a [u8]> {
        let scan_end = buffer.len().min(max_scan_bytes);
        let mut cursor = 0usize;
        let mut best: Option<(usize, usize)> = None;

        while cursor < scan_end {
            let rel_ff = memchr(0xff, &buffer[cursor..scan_end])?;
            let soi = cursor + rel_ff;
            if soi + 1 >= scan_end || buffer[soi + 1] != 0xd8 {
                cursor = soi + 1;
                continue;
            }

            let mut has_sof = false;
            let mut idx = soi + 2;
            loop {
                if idx + 1 >= buffer.len() {
                    break;
                }
                if buffer[idx] != 0xff {
                    idx += 1;
                    continue;
                }
                let marker = buffer[idx + 1];
                if matches!(marker, 0xC0 | 0xC1 | 0xC2) {
                    has_sof = true;
                }
                if marker == 0xd9 {
                    let end = idx + 2;
                    let len = end - soi;
                    if has_sof && best.map(|(_, b_len)| len > b_len).unwrap_or(true) {
                        best = Some((soi, len));
                    }
                    cursor = end;
                    break;
                }
                if (0xD0..=0xD7).contains(&marker) || marker == 0x01 {
                    idx += 2;
                    continue;
                }
                if idx + 3 >= buffer.len() {
                    break;
                }
                let seg_len = u16::from_be_bytes([buffer[idx + 2], buffer[idx + 3]]) as usize;
                if seg_len < 2 {
                    break;
                }
                idx += 2 + seg_len;
            }
        }

        best.map(|(s, l)| &buffer[s..s + l])
    }

    pub fn find_largest_jpeg_slice<'a>(buffer: &'a [u8]) -> Option<&'a [u8]> {
        Self::extract_largest_jpeg_segment(buffer)
    }

    pub fn jpeg_from_exif<'a>(buffer: &'a [u8], info: &ParsedExif) -> Option<&'a [u8]> {
        let offset = info.usize(ExifNames::THUMBNAIL).ok()?;
        let len = info.usize(ExifNames::THUMBNAIL_LEN).ok()?;
        let end = offset.checked_add(len)?;
        if end > buffer.len() || len < 4 {
            return None;
        }
        let slice = &buffer[offset..end];
        if Self::is_display_jpeg(slice) {
            Some(slice)
        } else {
            None
        }
    }

    pub fn is_display_jpeg(slice: &[u8]) -> bool {
        if !Self::is_valid_jpeg(slice) {
            return false;
        }
        // Look for JFIF/EXIF APP markers near the start; avoid lossless RAW JPEG data that lacks them.
        slice
            .windows(4)
            .take(40)
            .any(|w| w == [0xff, 0xe0, b'J', b'F'] || w == [0xff, 0xe1, b'E', b'x'])
    }

    pub fn find_display_jpeg_slice<'a>(buffer: &'a [u8]) -> Option<&'a [u8]> {
        Self::find_largest_jpeg_slice(buffer)
            .filter(|s| Self::is_display_jpeg(s))
            .or_else(|| {
                // As a fallback, return the first valid JPEG slice, even without APP markers.
                let mut start = 0usize;
                while let Some(rel_soi) = buffer[start..]
                    .windows(3)
                    .position(|w| w == [0xff, 0xd8, 0xff])
                {
                    let soi = start + rel_soi;
                    if let Some(rel_eoi) = buffer[soi + 3..].windows(2).position(|w| w == [0xff, 0xd9])
                    {
                        let end = soi + 3 + rel_eoi + 2;
                        let slice = &buffer[soi..end];
                        if Self::is_valid_jpeg(slice) {
                            return Some(slice);
                        }
                        start = end;
                        continue;
                    }
                    break;
                }
                None
            })
    }

    pub fn extract_valid_jpeg_with_cap<'a>(
        buffer: &'a [u8],
        max_scan_bytes: usize,
        min_size: usize,
        require_sof: bool,
    ) -> Option<&'a [u8]> {
        let scan_end = buffer.len().min(max_scan_bytes);
        let mut cursor = 0usize;

        while cursor < scan_end {
            // Find SOI (0xFF 0xD8)
            let rel_ff = memchr(0xff, &buffer[cursor..scan_end])?;
            let soi = cursor + rel_ff;
            if soi + 1 >= buffer.len() {
                break;
            }
            if buffer[soi + 1] != 0xd8 {
                cursor = soi + 1;
                continue;
            }

            // Find EOI (0xFF 0xD9) after SOI.
            let mut search = soi + 2;
            loop {
                if search >= buffer.len().saturating_sub(1) {
                    return None;
                }
                if let Some(rel_ff2) = memchr(0xff, &buffer[search..]) {
                    let idx = search + rel_ff2;
                    if idx + 1 < buffer.len() && buffer[idx + 1] == 0xd9 {
                        let end = idx + 2;
                        let slice = &buffer[soi..end];
                        if slice.len() >= min_size
                            && Self::is_valid_jpeg(slice)
                            && (!require_sof || Self::jpeg_has_sof(slice))
                        {
                            return Some(slice);
                        }
                        cursor = end;
                        break;
                    } else {
                        search = idx + 1;
                        continue;
                    }
                } else {
                    return None;
                }
            }
        }

        None
    }

    pub fn is_valid_jpeg(slice: &[u8]) -> bool {
        if slice.len() < 256 {
            return false;
        }

        if slice[0] != 0xFF || slice[1] != 0xD8 {
            return false;
        }

        if slice[slice.len() - 2] != 0xFF || slice[slice.len() - 1] != 0xD9 {
            return false;
        }

        true
    }

    pub fn jpeg_has_sof(slice: &[u8]) -> bool {
        slice.windows(2).any(|w| {
            matches!(
                w,
                [0xFF, 0xC0] | // Baseline
                [0xFF, 0xC1] | // Extended
                [0xFF, 0xC2] // Progressive
            )
        })
    }
}
