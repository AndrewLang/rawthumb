use std::path::Path;

pub struct ImageFormt;

impl ImageFormt {
    const SUPPORTED_EXTS: &'static [&'static str] = &["cr2", "cr3", "nef", "raf", "arw", "orf", "rw2", "dng", "raw"];

    pub fn supported_extensions() -> &'static [&'static str] {
        Self::SUPPORTED_EXTS
    }

    pub fn is_supported_extension(ext: &str) -> bool {
        Self::SUPPORTED_EXTS.iter().any(|supported| supported.eq_ignore_ascii_case(ext))
    }

    pub fn is_supported_path(path: &Path) -> bool {
        Self::extension_from_path(path).map(|ext| Self::is_supported_extension(&ext)).unwrap_or(false)
    }

    pub fn extension_from_path(path: &Path) -> Option<String> {
        path.extension().and_then(|ext| ext.to_str()).map(|ext| ext.to_ascii_lowercase())
    }
}
