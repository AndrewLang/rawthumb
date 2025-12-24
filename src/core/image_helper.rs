#![allow(dead_code)]

pub struct ImageHelper;

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
}
