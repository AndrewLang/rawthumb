#![allow(dead_code)]

pub struct ExifNames;

impl ExifNames {
    pub const ORIENTATION: &'static str = "orientation";
    pub const THUMBNAIL: &'static str = "thumbnail";
    pub const THUMBNAIL_LEN: &'static str = "thumbnail_len";
    pub const PREVIEW_OFFSET: &'static str = "preview_offset";
    pub const PREVIEW_LEN: &'static str = "preview_len";
    pub const MAKER_NOTES: &'static str = "maker_notes";
    pub const PREVIEW_IMAGE_START: &'static str = "preview_image_start";
    pub const PREVIEW_IMAGE_LEN: &'static str = "preview_image_len";
    pub const MAKE: &'static str = "make";
    pub const MODEL: &'static str = "model";
    pub const DNG_VERSION: &'static str = "dng_version";
    pub const CFA_PATTERN: &'static str = "cfa_pattern";
    pub const TAG_VALUE: &'static str = "tag_value";
    pub const TAG_LEN: &'static str = "tag_len";
}
