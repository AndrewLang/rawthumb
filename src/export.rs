#![allow(dead_code)]

use std::fs;

use crate::rawthumb::core::errors::Result;
use crate::rawthumb::core::types::ThumbnailResult;
use crate::rawthumb::decode;

pub fn get_thumbnail(buffer: &[u8]) -> Result<ThumbnailResult<'_>> {
    decode::get_thumbnail(buffer)
}

pub fn export_thumbnail_data(buffer: &[u8]) -> Result<Vec<u8>> {
    let thumb = decode::get_thumbnail(buffer)?;
    Ok(thumb.jpeg.to_vec())
}

pub fn export_thumbnail_to_file(buffer: &[u8], path: &str) -> Result<()> {
    let thumb = decode::get_thumbnail(buffer)?;
    fs::write(path, thumb.jpeg)?;
    Ok(())
}
