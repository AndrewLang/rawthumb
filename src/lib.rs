pub mod core;
pub mod export_config;
pub mod exporter;
pub mod image_resizer;
pub mod image_rotator;
pub mod formats;
pub mod makers;

// Provide the crate-level rawthumb namespace expected by the module references.
pub mod rawthumb {
    pub use super::{core, export_config, exporter, formats, image_resizer, image_rotator, makers};

    pub use crate::exporter::Exporter;
    pub use crate::export_config::ExportConfig;
    pub use crate::image_resizer::{DefaultImageResizer, ImageResizer};
    pub use crate::image_rotator::{DefaultImageRotator, ImageRotator};
    pub use crate::exporter::ThumbnailExporter;
}

pub use exporter::Exporter;
pub use export_config::ExportConfig;
pub use image_resizer::{DefaultImageResizer, ImageResizer};
pub use image_rotator::{DefaultImageRotator, ImageRotator};
pub use exporter::ThumbnailExporter;
