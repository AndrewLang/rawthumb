#![allow(dead_code)]

use crate::rawthumb::core::errors::{ImageProcessingError, Result as CoreResult};
use crate::rawthumb::core::types::Orientation;
use image::codecs::jpeg::{JpegDecoder, JpegEncoder};
use image::{ColorType, ImageBuffer, ImageDecoder, ImageFormat};
use std::borrow::Cow;
use std::io::Cursor;
use turbojpeg::{Transform, TransformOp, transform as tj_transform};

pub trait ImageRotator: Send + Sync {
    fn rotate<'a>(&self, buffer: &'a [u8], orientation: Orientation) -> CoreResult<Cow<'a, [u8]>>;
}

pub struct DefaultImageRotator {
    quality: u8,
}

pub const DEFAULT_ROTATE_JPEG_QUALITY: u8 = 75;

impl DefaultImageRotator {
    pub fn new(quality: u8) -> Self {
        Self { quality }
    }

    #[inline]
    fn rotate_jpeg_lossless(
        jpeg: &[u8],
        orientation: Orientation,
    ) -> Result<Cow<'_, [u8]>, String> {
        let op = match orientation {
            Orientation::Rotate90 => TransformOp::Rot90,
            Orientation::Rotate180 => TransformOp::Rot180,
            Orientation::Rotate270 => TransformOp::Rot270,
            Orientation::Horizontal => return Ok(Cow::Borrowed(jpeg)),
        };

        let mut transform = Transform::op(op);
        transform.copy_none = true;

        log::trace!(
            "Attempting JPEG rotation for orientation {:?}, {:?}",
            orientation,
            transform
        );

        let buf = tj_transform(&transform, jpeg).map_err(|e| e.to_string())?;

        Ok(Cow::Owned(buf.as_ref().to_vec()))
    }

    fn rotate_and_encode<P>(
        &self,
        data: Vec<u8>,
        width: u32,
        height: u32,
        orientation: Orientation,
        color_type: ColorType,
    ) -> CoreResult<Vec<u8>>
    where
        P: image::Pixel<Subpixel = u8> + 'static,
        image::ImageBuffer<P, Vec<u8>>: image::GenericImageView<Pixel = P>,
    {
        let img: ImageBuffer<P, Vec<u8>> =
            ImageBuffer::from_raw(width, height, data).ok_or_else(|| {
                ImageProcessingError::Raw(
                    "Failed to construct image buffer for rotation".to_string(),
                )
            })?;

        let rotated = match orientation {
            Orientation::Horizontal => img,
            Orientation::Rotate90 => image::imageops::rotate90(&img),
            Orientation::Rotate180 => image::imageops::rotate180(&img),
            Orientation::Rotate270 => image::imageops::rotate270(&img),
        };

        let mut out = Vec::new();
        let mut encoder = JpegEncoder::new_with_quality(Cursor::new(&mut out), self.quality);
        let w = rotated.width();
        let h = rotated.height();
        encoder
            .encode(&rotated, w, h, color_type.into())
            .map_err(|e| {
                ImageProcessingError::Raw(format!("Failed to encode rotated JPEG: {e}"))
            })?;
        Ok(out)
    }
}

impl DefaultImageRotator {
    fn rotate_decode_encode(
        &self,
        buffer: &[u8],
        orientation: Orientation,
    ) -> CoreResult<Cow<'static, [u8]>> {
        log::info!(
            "Falling back to decode-rotate-encode for orientation {:?}",
            orientation
        );
        let cursor = Cursor::new(buffer);
        let decoder = JpegDecoder::new(cursor)
            .map_err(|e| ImageProcessingError::Raw(format!("JPEG decode failed: {e}")))?;
        let color_type = decoder.color_type();
        let (width, height) = decoder.dimensions();
        let mut data = vec![0u8; decoder.total_bytes() as usize];
        decoder
            .read_image(&mut data)
            .map_err(|e| ImageProcessingError::Raw(format!("JPEG decode failed: {e}")))?;

        match color_type {
            ColorType::Rgb8 => self.rotate_and_encode::<image::Rgb<u8>>(
                data,
                width,
                height,
                orientation,
                ColorType::Rgb8,
            ),
            ColorType::L8 => self.rotate_and_encode::<image::Luma<u8>>(
                data,
                width,
                height,
                orientation,
                ColorType::L8,
            ),
            ColorType::Rgba8 => self.rotate_and_encode::<image::Rgba<u8>>(
                data,
                width,
                height,
                orientation,
                ColorType::Rgba8,
            ),
            _ => {
                let img = image::load_from_memory_with_format(buffer, ImageFormat::Jpeg).map_err(
                    |e| {
                        ImageProcessingError::Raw(format!(
                            "Failed to decode JPEG for rotation: {e}"
                        ))
                    },
                )?;

                let rotated = match orientation {
                    Orientation::Horizontal => img,
                    Orientation::Rotate90 => img.rotate90(),
                    Orientation::Rotate180 => img.rotate180(),
                    Orientation::Rotate270 => img.rotate270(),
                };

                let mut out = Vec::new();
                rotated
                    .write_to(&mut Cursor::new(&mut out), ImageFormat::Jpeg)
                    .map_err(|e| {
                        ImageProcessingError::Raw(format!("Failed to encode rotated JPEG: {e}"))
                    })?;
                Ok(out)
            }
        }
        .map(Cow::Owned)
    }
}

impl ImageRotator for DefaultImageRotator {
    fn rotate<'a>(&self, buffer: &'a [u8], orientation: Orientation) -> CoreResult<Cow<'a, [u8]>> {
        if orientation == Orientation::Horizontal {
            return Ok(Cow::Borrowed(buffer));
        }

        log::trace!("Attempting rotation for orientation {:?}", orientation);

        if let Ok(out) = Self::rotate_jpeg_lossless(buffer, orientation) {
            return Ok(out);
        }

        self.rotate_decode_encode(buffer, orientation)
    }
}
