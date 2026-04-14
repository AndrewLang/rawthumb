#![allow(dead_code)]

use std::borrow::Cow;
use std::cell::RefCell;

#[cfg(any(target_arch = "x86", target_arch = "x86_64", target_arch = "aarch64"))]
use fast_image_resize::CpuExtensions;
use fast_image_resize::images::Image;
use fast_image_resize::{PixelType, ResizeAlg, ResizeOptions, Resizer};
use turbojpeg::{Compressor, Decompressor, Image as TjImage, PixelFormat, Subsamp};

use crate::rawthumb::core::errors::{ImageProcessingError, Result as CoreResult};

fn create_resizer() -> Resizer {
    #[cfg(any(target_arch = "x86", target_arch = "x86_64", target_arch = "aarch64"))]
    let mut resizer = Resizer::new();
    #[cfg(not(any(target_arch = "x86", target_arch = "x86_64", target_arch = "aarch64")))]
    let resizer = Resizer::new();
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    unsafe {
        resizer.set_cpu_extensions(CpuExtensions::default());
    }
    #[cfg(target_arch = "aarch64")]
    unsafe {
        if CpuExtensions::Neon.is_supported() {
            resizer.set_cpu_extensions(CpuExtensions::Neon);
        }
    }
    resizer
}

thread_local! {
    static TLS_RESIZER: RefCell<Resizer> = RefCell::new(create_resizer());

    static TLS_DECOMPRESSOR: RefCell<Decompressor> =
        RefCell::new(Decompressor::new().unwrap());

    static TLS_COMPRESSOR: std::cell::RefCell<Compressor> =
        std::cell::RefCell::new({
            let mut c = Compressor::new().unwrap();
            c.set_quality(80).ok();
            c.set_subsamp(Subsamp::Sub2x2).ok();
            c
        });
}

pub trait ImageResizer: Send + Sync {
    fn resize<'a>(&self, buffer: &'a [u8], max_border: Option<u32>) -> CoreResult<Cow<'a, [u8]>>;
}

pub struct DefaultImageResizer {
    resize_opts: ResizeOptions,
}

impl Default for DefaultImageResizer {
    fn default() -> Self {
        let resize_opts = ResizeOptions { algorithm: ResizeAlg::Nearest, ..Default::default() };
        Self { resize_opts }
    }
}

impl DefaultImageResizer {
    fn dimensions(&self, jpeg_bytes: &[u8]) -> CoreResult<(u32, u32)> {
        TLS_DECOMPRESSOR.with(|dec| {
            let header = dec
                .borrow_mut()
                .read_header(jpeg_bytes)
                .map_err(|e| ImageProcessingError::Raw(format!("JPEG header read failed: {e}")))?;
            Ok((header.width as u32, header.height as u32))
        })
    }

    fn decode_to_rgb(&self, jpeg_bytes: &[u8], w: u32, h: u32, dst_max: u32) -> CoreResult<Vec<u8>> {
        let pitch = (w as usize) * 3;
        let mut out = vec![0u8; (h as usize) * pitch];

        let img = TjImage {
            pixels: out.as_mut_slice(),
            width: w as usize,
            pitch,
            height: h as usize,
            format: PixelFormat::RGB,
        };

        TLS_DECOMPRESSOR.with(|dec| {
            dec.borrow_mut()
                .decompress(jpeg_bytes, img)
                .map_err(|e| ImageProcessingError::Raw(format!("JPEG decode failed: {e}")))
        })?;

        Ok(out)
    }

    fn choose_scale(&self, w: u32, h: u32, target: u32) -> turbojpeg::ScalingFactor {
        let max_dim = w.max(h);

        if max_dim / 8 >= target {
            turbojpeg::ScalingFactor::new(1, 8)
        } else if max_dim / 4 >= target {
            turbojpeg::ScalingFactor::new(1, 4)
        } else if max_dim / 2 >= target {
            turbojpeg::ScalingFactor::new(1, 2)
        } else {
            turbojpeg::ScalingFactor::new(1, 1)
        }
    }
}

impl ImageResizer for DefaultImageResizer {
    fn resize<'a>(&self, buffer: &'a [u8], max_border: Option<u32>) -> CoreResult<Cow<'a, [u8]>> {
        let dst_max = match max_border {
            Some(m) if m > 0 => m,
            _ => return Ok(Cow::Borrowed(buffer)),
        };

        let (w, h) = self.dimensions(buffer)?;

        if w <= dst_max && h <= dst_max {
            return Ok(Cow::Borrowed(buffer));
        }
        let mut start = std::time::Instant::now();

        let mut rgb_data = self.decode_to_rgb(buffer, w, h, dst_max)?;

        log::info!("Decoded JPEG in [{:?}] for size {}x{}", start.elapsed(), w, h);
        start = std::time::Instant::now();

        log::info!("Decoded JPEG in [{:?}] for size {}x{}", start.elapsed(), w, h);

        let (dst_w, dst_h) = if w > h { (dst_max, (h * dst_max) / w) } else { ((w * dst_max) / h, dst_max) };

        if dst_w == 0 || dst_h == 0 {
            return Ok(Cow::Borrowed(buffer));
        }

        let src = Image::from_slice_u8(w, h, &mut rgb_data, PixelType::U8x3)
            .map_err(|e| ImageProcessingError::Raw(e.to_string()))?;
        let mut dst_buf = vec![0u8; (dst_w * dst_h * 3) as usize];
        let mut dst = Image::from_slice_u8(dst_w, dst_h, &mut dst_buf, PixelType::U8x3)
            .map_err(|e| ImageProcessingError::Raw(e.to_string()))?;

        TLS_RESIZER.with(|r| {
            r.borrow_mut()
                .resize(&src, &mut dst, Some(&self.resize_opts))
                .map_err(|e| ImageProcessingError::Raw(e.to_string()))
        })?;

        let pitch = (dst_w as usize) * 3;

        let image = turbojpeg::Image {
            pixels: &dst_buf[..],
            width: dst_w as usize,
            height: dst_h as usize,
            pitch,
            format: PixelFormat::RGB,
        };

        let jpeg = TLS_COMPRESSOR
            .with(|c| c.borrow_mut().compress_to_vec(image).map_err(|e| ImageProcessingError::Raw(e.to_string())))?;

        log::info!(
            "Resizing and re-encoding took [{:?}] for original size {}x{} -> {}x{}",
            start.elapsed(),
            w,
            h,
            dst_w,
            dst_h
        );

        Ok(Cow::Owned(jpeg))
    }
}
