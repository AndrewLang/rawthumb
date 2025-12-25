#![allow(dead_code)]

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
        let len = buffer.len();
        let mut i = 0;

        let mut best_start = 0;
        let mut best_end = 0;
        let mut best_size = 0;

        while i + 1 < len {
            if buffer[i] == 0xFF && buffer[i + 1] == 0xD8 {
                let start = i;
                let mut j = i + 2;
                let mut has_sof = false;

                while j + 1 < len {
                    if buffer[j] != 0xFF {
                        j += 1;
                        continue;
                    }

                    let marker = buffer[j + 1];

                    if matches!(marker, 0xC0 | 0xC1 | 0xC2) {
                        has_sof = true;
                    }

                    if marker == 0xD9 {
                        let end = j + 2;
                        let size = end - start;

                        if has_sof && size > best_size {
                            best_start = start;
                            best_end = end;
                            best_size = size;
                        }

                        i = end;
                        break;
                    }

                    if (0xD0..=0xD7).contains(&marker) || marker == 0x01 {
                        j += 2;
                        continue;
                    }

                    if j + 3 >= len {
                        break;
                    }

                    let seg_len = u16::from_be_bytes([buffer[j + 2], buffer[j + 3]]) as usize;

                    if seg_len < 2 {
                        break;
                    }

                    j += 2 + seg_len;
                }

                i += 2;
            } else {
                i += 1;
            }
        }

        if best_size > 0 {
            Some(&buffer[best_start..best_end])
        } else {
            None
        }
    }

    pub fn is_valid_jpeg(slice: &[u8]) -> bool {
        if slice.len() < 1024 {
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
