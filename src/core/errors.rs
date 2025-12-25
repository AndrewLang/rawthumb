use thiserror::Error;

use crate::rawthumb::core::exif::{ExifError, ExifFieldError};

pub type Result<T> = std::result::Result<T, ImageProcessingError>;

#[derive(Debug, Error)]
pub enum ImageProcessingError {
    #[error("Not implemented: {0}")]
    Unimplemented(&'static str),
    #[error(transparent)]
    ExifParse(#[from] ExifError),
    #[error(transparent)]
    Decode(#[from] DecodingError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Raw(String),
}

#[derive(thiserror::Error, Debug)]
pub enum DecodingError {
    #[error("Decoding error.")]
    RawInfoError(#[from] ExifFieldError),

    #[error("The decoded image size({0}) is invalid due to the width x height = {1}.")]
    InvalidDecodedImageSize(usize, usize),

    #[error("JPEG error.")]
    LJPEGError(String),
}
