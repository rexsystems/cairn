//! Custom cape validation. Every upload is decoded with hard limits and re-encoded, so
//! only plain RGBA pixels we produced ourselves are ever stored or served.
use image::{ImageFormat, ImageReader, Limits};
use sha2::{Digest, Sha256};
use std::io::Cursor;

pub const MAX_UPLOAD_BYTES: usize = 1024 * 1024;
pub const MAX_FRAMES: u32 = 32;
pub const MAX_WIDTH: u32 = 1024;
/// 16 MiB of RGBA per cape, so a strip cannot blow up texture memory on every client.
pub const MAX_PIXELS: u64 = 4 * 1024 * 1024;
pub const FRAME_MS: std::ops::RangeInclusive<u32> = 50..=1000;

#[derive(Debug)]
pub struct Cape {
    pub png: Vec<u8>,
    pub hash: String,
    pub width: u32,
    pub height: u32,
    pub frames: u32,
    pub frame_ms: u32,
}

pub fn validate(bytes: &[u8], frame_ms: Option<u32>) -> Result<Cape, String> {
    if bytes.len() > MAX_UPLOAD_BYTES {
        return Err("The cape file is larger than 1 MiB.".into());
    }
    if !bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Err("Capes must be PNG images.".into());
    }
    let mut reader = ImageReader::new(Cursor::new(bytes));
    reader.set_format(ImageFormat::Png);
    let mut limits = Limits::default();
    limits.max_image_width = Some(MAX_WIDTH);
    limits.max_image_height = Some(MAX_WIDTH / 2 * MAX_FRAMES);
    limits.max_alloc = Some(MAX_PIXELS * 4 * 2);
    reader.limits(limits.clone());
    let (width, height) = reader.into_dimensions().map_err(|_| "The PNG could not be read.".to_string())?;
    if width < 64 || width % 64 != 0 {
        return Err("Cape width must be 64, 128, 256, 512 or 1024 pixels (any multiple of 64).".into());
    }
    let frame_height = width / 2;
    if height == 0 || height % frame_height != 0 {
        return Err(format!("Each frame must be {width}x{frame_height}; animated capes stack frames vertically."));
    }
    let frames = height / frame_height;
    if frames > MAX_FRAMES {
        return Err(format!("Animated capes can have at most {MAX_FRAMES} frames."));
    }
    if u64::from(width) * u64::from(height) > MAX_PIXELS {
        return Err("The cape has too many pixels; use a smaller width or fewer frames.".into());
    }
    let frame_ms = if frames == 1 {
        0
    } else {
        let ms = frame_ms.unwrap_or(100);
        if !FRAME_MS.contains(&ms) {
            return Err("Frame time must be between 50 and 1000 ms.".into());
        }
        ms
    };
    let mut reader = ImageReader::new(Cursor::new(bytes));
    reader.set_format(ImageFormat::Png);
    reader.limits(limits);
    let image = reader.decode().map_err(|_| "The PNG could not be decoded.".to_string())?.into_rgba8();
    let mut png = Vec::new();
    image.write_to(&mut Cursor::new(&mut png), ImageFormat::Png).map_err(|e| format!("Could not encode the cape: {e}"))?;
    let hash = hex::encode(Sha256::digest(&png));
    Ok(Cape { png, hash, width, height, frames, frame_ms })
}

pub fn valid_hash(hash: &str) -> bool {
    hash.len() == 64 && hash.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
