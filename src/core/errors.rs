use quickexif::{parsed_info::Error as QuickExifFieldError, parser};
use thiserror::Error;

#[derive(Debug, thiserror::Error)]
pub enum ExifError {
    #[error(transparent)]
    Parse(#[from] parser::Error),
}

impl ExifError {
    pub fn tag_not_found(&self) -> Option<u16> {
        match self {
            ExifError::Parse(parser::Error::TagNotFound(tag)) => Some(*tag),
            _ => None,
        }
    }
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
