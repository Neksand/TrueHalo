use anyhow::{Context, Result};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use image::{codecs::jpeg::JpegEncoder, ExtendedColorType, ImageFormat};
use serde::{Deserialize, Serialize};

pub mod permission;

#[cfg(target_os = "macos")]
#[path = "macos.rs"]
mod platform;

#[cfg(windows)]
#[path = "windows.rs"]
mod platform;

/// Options for a single screen capture request coming from the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct CaptureOptions {
    /// Interactive marquee / window selection instead of the whole main display.
    pub interactive: bool,
    /// Include the mouse cursor in the picture (non-interactive captures only).
    pub show_cursor: bool,
    /// Downscale + re-encode so the payload stays small (token/cost friendly).
    pub compress: bool,
    /// Longest edge in pixels after downscaling.
    pub max_dimension: u32,
    /// JPEG quality (1-100) used when re-encoding.
    pub quality: u8,
    /// Hide the TrueHalo window before taking the picture so it never shows up
    /// inside its own screenshot and does not steal focus.
    pub hide_window: bool,
    /// Run local OCR on the captured image and include the text in the response.
    #[serde(default)]
    pub ocr: bool,
}

impl Default for CaptureOptions {
    fn default() -> Self {
        Self {
            interactive: false,
            show_cursor: false,
            compress: true,
            max_dimension: 1568,
            quality: 80,
            hide_window: true,
            ocr: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum CaptureResponse {
    Success {
        base64_data: String,
        media_type: String,
        file_size_bytes: usize,
        original_size_bytes: usize,
        width: u32,
        height: u32,
        compressed: bool,
        /// Text recognized on-device when `CaptureOptions::ocr` was set.
        ocr_text: Option<String>,
    },
    /// The user pressed Escape in the interactive selection overlay.
    Cancelled,
    /// Screen Recording permission is missing; the UI must guide the user.
    PermissionRequired,
}

pub struct ScreenCaptureService;

impl ScreenCaptureService {
    pub fn capture(options: CaptureOptions) -> Result<CaptureResponse> {
        if !permission::has_permission() {
            return Ok(CaptureResponse::PermissionRequired);
        }

        let temp_path =
            std::env::temp_dir().join(format!("truehalo_cap_{}.png", uuid::Uuid::new_v4()));

        if !platform::grab(&options, &temp_path)? {
            return Ok(CaptureResponse::Cancelled);
        }

        let metadata = std::fs::metadata(&temp_path)?;
        if metadata.len() == 0 {
            let _ = std::fs::remove_file(&temp_path);
            return Ok(CaptureResponse::Cancelled);
        }

        let original_bytes =
            std::fs::read(&temp_path).context("Failed to read captured screenshot")?;

        // Delete the temporary file immediately per privacy guidelines
        let _ = std::fs::remove_file(&temp_path);

        let original_size = original_bytes.len();
        let (original_width, original_height) = png_dimensions(&original_bytes).unwrap_or((0, 0));

        let mut final_bytes = original_bytes;
        let mut media_type = "image/png".to_string();
        let mut width = original_width;
        let mut height = original_height;
        let mut compressed = false;

        if options.compress {
            match compress_png(&final_bytes, options.max_dimension, options.quality) {
                // Never ship a "compressed" payload that ended up bigger than the PNG.
                Ok((bytes, w, h)) if bytes.len() < final_bytes.len() => {
                    final_bytes = bytes;
                    media_type = "image/jpeg".to_string();
                    width = w;
                    height = h;
                    compressed = true;
                }
                Ok(_) => {}
                Err(err) => {
                    tracing::warn!("Screenshot compression failed, sending raw PNG: {}", err)
                }
            }
        }

        let base64_data = STANDARD.encode(&final_bytes);

        Ok(CaptureResponse::Success {
            base64_data,
            media_type,
            file_size_bytes: final_bytes.len(),
            original_size_bytes: original_size,
            width,
            height,
            compressed,
            ocr_text: None,
        })
    }
}

/// Read width/height from a PNG header without decoding the whole image.
fn png_dimensions(bytes: &[u8]) -> Option<(u32, u32)> {
    if bytes.len() < 24 || &bytes[0..8] != b"\x89PNG\r\n\x1a\n" {
        return None;
    }
    let width = u32::from_be_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]);
    let height = u32::from_be_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]);
    Some((width, height))
}

/// Downscale to `max_dimension` (longest edge) and re-encode as JPEG.
fn compress_png(bytes: &[u8], max_dimension: u32, quality: u8) -> Result<(Vec<u8>, u32, u32)> {
    let image = image::load_from_memory_with_format(bytes, ImageFormat::Png)
        .context("Failed to decode captured PNG")?;

    let (orig_w, orig_h) = (image.width(), image.height());
    let max_dimension = max_dimension.clamp(512, 4096);
    let longest = orig_w.max(orig_h).max(1);
    let scale = max_dimension as f32 / longest as f32;

    let image = if scale < 1.0 {
        let new_w = ((orig_w as f32) * scale).round().max(1.0) as u32;
        let new_h = ((orig_h as f32) * scale).round().max(1.0) as u32;
        image.resize_exact(new_w, new_h, image::imageops::FilterType::Triangle)
    } else {
        image
    };

    let (width, height) = (image.width(), image.height());
    let rgb = image.to_rgb8();

    let mut out: Vec<u8> = Vec::new();
    {
        let mut encoder = JpegEncoder::new_with_quality(&mut out, quality.clamp(1, 100));
        encoder
            .encode(rgb.as_raw(), width, height, ExtendedColorType::Rgb8)
            .context("Failed to encode JPEG")?;
    }

    Ok((out, width, height))
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgb, RgbImage};
    use std::io::Cursor;

    fn encode_test_png(width: u32, height: u32) -> Vec<u8> {
        let image = RgbImage::from_fn(width, height, |x, y| {
            Rgb([(x % 256) as u8, (y % 256) as u8, 128])
        });
        let mut bytes = Vec::new();
        image
            .write_to(&mut Cursor::new(&mut bytes), ImageFormat::Png)
            .unwrap();
        bytes
    }

    /// Deterministic noise: behaves much closer to a real screenshot, whose PNG
    /// is far bigger than a downscaled JPEG.
    fn encode_noise_png(width: u32, height: u32) -> Vec<u8> {
        let mut state: u32 = 0x1234_5678;
        let mut next = || {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            (state & 0xFF) as u8
        };
        let image = RgbImage::from_fn(width, height, |_, _| Rgb([next(), next(), next()]));
        let mut bytes = Vec::new();
        image
            .write_to(&mut Cursor::new(&mut bytes), ImageFormat::Png)
            .unwrap();
        bytes
    }

    #[test]
    fn reads_png_dimensions_from_header() {
        let png = encode_test_png(40, 30);
        assert_eq!(png_dimensions(&png), Some((40, 30)));
        assert_eq!(png_dimensions(b"not a png"), None);
    }

    #[test]
    fn compresses_and_downscales_to_max_dimension() {
        let png = encode_noise_png(3000, 2000);
        let (jpeg, width, height) = compress_png(&png, 1568, 80).unwrap();

        assert_eq!(width, 1568);
        assert_eq!(height, 1045);
        // JPEG SOI marker
        assert_eq!(&jpeg[0..2], &[0xFF, 0xD8]);
        // A real screenshot must shrink dramatically.
        assert!(jpeg.len() < png.len());
    }

    #[test]
    fn small_images_are_not_upscaled() {
        let png = encode_test_png(800, 600);
        let (_, width, height) = compress_png(&png, 1568, 80).unwrap();
        assert_eq!((width, height), (800, 600));
    }
}
