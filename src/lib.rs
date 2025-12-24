pub mod core;
pub mod decode;
pub mod export;
pub mod formats;
pub mod makers;
pub mod selector;

// Alias the makers module so `crate::maker` paths resolve for existing references.
pub use makers as maker;

// Provide the crate-level rawthumb namespace expected by the module references.
pub mod rawthumb {
    pub use super::{core, decode, export, formats, maker, makers, selector};
}

#[derive(Debug)]
pub struct RawFileReadingError(pub String);

impl std::fmt::Display for RawFileReadingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Raw file reading error: {}", self.0)
    }
}

impl std::error::Error for RawFileReadingError {}

pub use export::{export_thumbnail_data, export_thumbnail_to_file};
