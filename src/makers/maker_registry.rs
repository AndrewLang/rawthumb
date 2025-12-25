#![allow(dead_code)]

use std::sync::Arc;

use once_cell::sync::Lazy;

use crate::rawthumb::core::thumbnail_extractor::ThumbnailExtractor;
use crate::rawthumb::core::thumbnail_registry::ThumbnailRegistry;
use crate::rawthumb::core::types::RawMetadata;
use crate::rawthumb::makers::{
    adobe::AdobeThumbnailExtractor, canon::CanonThumbnailExtractor, fuji::FujiThumbnailExtractor,
    nikon::NikonThumbnailExtractor, olympus::OlympusThumbnailExtractor,
    panasonic::PanasonicThumbnailExtractor, sony::SonyThumbnailExtractor,
};

pub struct MakerRegistry {
    registry: ThumbnailRegistry,
}

impl MakerRegistry {
    pub fn new(registry: ThumbnailRegistry) -> Self {
        Self { registry }
    }

    pub fn effective_make<'a>(&self, basic: &'a RawMetadata) -> &'a str {
        if basic.dng_version.is_some() {
            "ADOBE"
        } else {
            basic.make.as_str()
        }
    }

    pub fn find<'a>(&self, basic: &'a RawMetadata) -> Option<&Arc<dyn ThumbnailExtractor>> {
        let make = self.effective_make(basic);
        self.registry.find(make)
    }
}

// Explicit order for maker selection: Nikon, Sony, Panasonic, Olympus, Fuji, Canon, Adobe (DNG).
pub static MAKER_REGISTRY: Lazy<MakerRegistry> = Lazy::new(|| {
    MakerRegistry::new(ThumbnailRegistry::new(vec![
        Arc::new(NikonThumbnailExtractor::default()),
        Arc::new(SonyThumbnailExtractor::default()),
        Arc::new(PanasonicThumbnailExtractor::default()),
        Arc::new(OlympusThumbnailExtractor::default()),
        Arc::new(FujiThumbnailExtractor::default()),
        Arc::new(CanonThumbnailExtractor::default()),
        Arc::new(AdobeThumbnailExtractor::default()),
    ]))
});
