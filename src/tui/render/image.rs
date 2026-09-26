use std::sync::{Arc, OnceLock};

use image::imageops::FilterType;
use image::DynamicImage;
use ratatui::layout::Size;
use ratatui_image::picker::Picker;
use ratatui_image::sliced::SlicedProtocol;

static PICKER: OnceLock<Picker> = OnceLock::new();

pub fn init(picker: Picker) {
    let _ = PICKER.set(picker);
}

fn picker() -> &'static Picker {
    PICKER.get_or_init(Picker::halfblocks)
}

const MAX_EDGE: u32 = 1600;

pub fn decode(bytes: &[u8]) -> Option<DynamicImage> {
    let image = image::load_from_memory(bytes).ok()?;

    Some(downscale(image))
}

fn downscale(image: DynamicImage) -> DynamicImage {
    if image.width() <= MAX_EDGE && image.height() <= MAX_EDGE {
        return image;
    }

    image.resize(MAX_EDGE, MAX_EDGE, FilterType::Triangle)
}

pub struct Encoded {
    size: Size,
    sliced: SlicedProtocol,
}

impl std::fmt::Debug for Encoded {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Encoded").field("size", &self.size).finish()
    }
}

pub fn encode(source: &DynamicImage, size: Size) -> Option<Encoded> {
    let sliced = SlicedProtocol::new(picker(), source.clone(), Some(size)).ok()?;

    Some(Encoded { size, sliced })
}

pub struct EncodeRequest {
    pub size: Size,
    pub source: Arc<DynamicImage>,
}

pub struct Loaded {
    source: Arc<DynamicImage>,
    encoded: Option<Encoded>,
    wanted: Option<Size>,
    pending: Option<Size>,
    failed: Option<Size>,
    encodes: usize,
    pub width: u32,
    pub height: u32,
}

pub fn load(source: DynamicImage) -> Loaded {
    let (width, height) = (source.width(), source.height());

    Loaded {
        source: Arc::new(source),
        encoded: None,
        wanted: None,
        pending: None,
        failed: None,
        encodes: 0,
        width,
        height,
    }
}

impl Loaded {
    pub fn sliced(&mut self, size: Size) -> Option<&SlicedProtocol> {
        match &self.encoded {
            Some(encoded) if encoded.size == size => Some(&encoded.sliced),
            _ => {
                self.wanted = Some(size);

                None
            }
        }
    }

    pub fn take_request(&mut self) -> Option<EncodeRequest> {
        let size = self.wanted.take()?;

        if self.pending == Some(size) || self.failed == Some(size) {
            return None;
        }

        self.pending = Some(size);

        Some(EncodeRequest {
            size,
            source: Arc::clone(&self.source),
        })
    }

    pub fn set_encoded(&mut self, encoded: Encoded) {
        if self.pending == Some(encoded.size) {
            self.pending = None;
        }

        self.encoded = Some(encoded);
        self.encodes += 1;
    }

    pub fn encode_failed(&mut self, size: Size) {
        if self.pending == Some(size) {
            self.pending = None;
        }

        self.failed = Some(size);
    }

    pub fn failed_at(&self, size: Size) -> bool {
        self.failed == Some(size)
    }

    pub fn is_encoding(&self) -> bool {
        self.pending.is_some()
    }

    pub fn encodes(&self) -> usize {
        self.encodes
    }

    pub fn rows_for(&self, cells_wide: u16, max_rows: u16) -> u16 {
        if self.width == 0 || self.height == 0 || cells_wide == 0 {
            return 1;
        }

        let font = picker().font_size();
        let pixels_wide = u32::from(cells_wide) * u32::from(font.width);
        let pixels_high = pixels_wide.saturating_mul(self.height) / self.width;
        let rows = pixels_high.div_ceil(u32::from(font.height).max(1));

        (rows as u16).clamp(1, max_rows)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn garbage_bytes_decode_to_nothing() {
        assert!(decode(b"not an image").is_none());
    }

    #[test]
    fn a_large_photo_is_downscaled_keeping_its_aspect() {
        let huge = DynamicImage::ImageRgb8(image::RgbImage::new(2400, 1800));

        let scaled = downscale(huge);

        assert_eq!(scaled.width(), MAX_EDGE);
        assert_eq!(scaled.height(), MAX_EDGE * 3 / 4);
    }

    #[test]
    fn a_small_image_is_left_alone() {
        let small = DynamicImage::ImageRgb8(image::RgbImage::new(48, 24));

        let kept = downscale(small);

        assert_eq!((kept.width(), kept.height()), (48, 24));
    }
}
