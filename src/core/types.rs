#![allow(dead_code)]

use std::borrow::Cow;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Orientation {
    Horizontal = 0,
    Rotate90 = 90,
    Rotate180 = 180,
    Rotate270 = 270,
}

#[derive(Clone, Debug)]
pub struct RawMetadata {
    pub make: String,
    pub model: String,
    pub dng_version: Option<u16>,
    pub cfa_pattern: Option<[u8; 4]>,
}

pub struct ThumbnailResult<'a> {
    pub jpeg: Cow<'a, [u8]>,
    pub orientation: Orientation,
}
