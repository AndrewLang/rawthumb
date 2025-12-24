#![allow(dead_code)]

use std::sync::Arc;

use crate::rawthumb::core::errors::{Error, Result};
use crate::rawthumb::core::thumbnail::{ThumbnailExtractor, ThumbnailRegistry};
use crate::rawthumb::core::types::{BasicInfo, ThumbnailResult};
use crate::rawthumb::makers::{
    adobe::AdobeThumbnailExtractor, canon::CanonThumbnailExtractor, fuji::FujiThumbnailExtractor,
    nikon::NikonThumbnailExtractor, olympus::OlympusThumbnailExtractor,
    panasonic::PanasonicThumbnailExtractor, sony::SonyThumbnailExtractor,
};

pub fn select_and_decode_thumbnail<'a>(
    buffer: &'a [u8],
    basic_parsed: quickexif::ParsedInfo,
    basic: BasicInfo,
) -> Result<ThumbnailResult<'a>> {
    let registry = ThumbnailRegistry::new(vec![
        Arc::new(NikonThumbnailExtractor),
        Arc::new(SonyThumbnailExtractor),
        Arc::new(PanasonicThumbnailExtractor),
        Arc::new(OlympusThumbnailExtractor),
        Arc::new(FujiThumbnailExtractor),
        Arc::new(CanonThumbnailExtractor),
    ]);

    if basic.dng_version.is_some() {
        let extractor = AdobeThumbnailExtractor;
        return extractor.extract(buffer, &basic, basic_parsed);
    }

    let make = basic.make.as_str();
    let extractor = registry
        .find(make)
        .ok_or_else(|| Error::Raw(format!("Maker not supported: {make}")))?;
    extractor.extract(buffer, &basic, basic_parsed)
}
