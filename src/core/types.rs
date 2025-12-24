#![allow(dead_code)]

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Orientation {
    Horizontal = 0,
    Rotate90 = 90,
    Rotate180 = 180,
    Rotate270 = 270,
}

#[allow(clippy::upper_case_acronyms)]
#[derive(Debug)]
pub enum CFAPattern {
    RGGB,
    GRBG,
    GBRG,
    BGGR,
    XTrans0, // RBGBRG
    XTrans1, // GGRGGB
}

pub struct Crop {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Debug)]
pub struct BasicInfo {
    pub make: String,
    pub model: String,
    pub dng_version: Option<u16>,
    pub cfa_pattern: Option<[u8; 4]>,
}

pub struct ThumbnailResult<'a> {
    pub jpeg: &'a [u8],
    pub orientation: Orientation,
}

pub struct ParsedBasicInfo;

pub struct ImageMeta {
    pub width: usize,
    pub height: usize,
    pub cfa_pattern: CFAPattern,
    pub crop: Option<Crop>,
    pub orientation: Orientation,
}
