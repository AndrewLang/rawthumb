pub mod core;
pub mod export;
pub mod formats;
pub mod makers;

// Provide the crate-level rawthumb namespace expected by the module references.
pub mod rawthumb {
    pub use super::{core, export, formats, makers};

    pub use crate::export::{Exporter, ThumbnailExporter};
}

pub use export::{Exporter, ThumbnailExporter};
