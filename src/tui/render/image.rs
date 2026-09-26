use std::collections::VecDeque;
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

const KEPT_SIZES: usize = 4;

pub struct Loaded {
    source: Arc<DynamicImage>,
    encoded: VecDeque<Encoded>,
    wanted: Vec<Size>,
    pending: Vec<Size>,
    failed: Vec<Size>,
    encodes: usize,
    pub width: u32,
    pub height: u32,
}

pub fn load(source: DynamicImage) -> Loaded {
    let (width, height) = (source.width(), source.height());

    Loaded {
        source: Arc::new(source),
        encoded: VecDeque::new(),
        wanted: Vec::new(),
        pending: Vec::new(),
        failed: Vec::new(),
        encodes: 0,
        width,
        height,
    }
}

impl Loaded {
    pub fn sliced(&mut self, size: Size) -> Option<&SlicedProtocol> {
        let used = self
            .encoded
            .iter()
            .position(|encoded| encoded.size == size)
            .and_then(|index| self.encoded.remove(index));

        let Some(used) = used else {
            if !self.wanted.contains(&size) {
                self.wanted.push(size);
            }

            return None;
        };

        self.encoded.push_back(used);

        self.encoded.back().map(|encoded| &encoded.sliced)
    }

    pub fn take_requests(&mut self) -> Vec<EncodeRequest> {
        let mut requests = Vec::new();

        for size in std::mem::take(&mut self.wanted) {
            if self.pending.contains(&size) || self.failed.contains(&size) {
                continue;
            }

            self.pending.push(size);
            requests.push(EncodeRequest {
                size,
                source: Arc::clone(&self.source),
            });
        }

        requests
    }

    pub fn set_encoded(&mut self, encoded: Encoded) {
        self.pending.retain(|size| *size != encoded.size);
        self.encoded.retain(|kept| kept.size != encoded.size);
        self.encoded.push_back(encoded);
        self.encodes += 1;

        while self.encoded.len() > KEPT_SIZES {
            self.encoded.pop_front();
        }
    }

    pub fn encode_failed(&mut self, size: Size) {
        self.pending.retain(|pending| *pending != size);

        if !self.failed.contains(&size) {
            self.failed.push(size);
        }
    }

    pub fn failed_at(&self, size: Size) -> bool {
        self.failed.contains(&size)
    }

    pub fn is_encoding(&self) -> bool {
        !self.pending.is_empty()
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
    fn the_least_recently_drawn_size_is_evicted_first() -> Result<(), &'static str> {
        let source = DynamicImage::ImageRgb8(image::RgbImage::new(48, 24));
        let mut loaded = load(source.clone());
        let oldest = Size::new(4, 4);
        let second = Size::new(8, 4);
        let newest = Size::new(64, 4);

        for width in 1..=KEPT_SIZES as u16 {
            loaded.set_encoded(encode(&source, Size::new(width * 4, 4)).ok_or("encodes")?);
        }

        assert!(
            loaded.sliced(oldest).is_some(),
            "drawing refreshes the oldest"
        );

        loaded.set_encoded(encode(&source, newest).ok_or("encodes")?);

        assert!(loaded.sliced(oldest).is_some(), "a recent size survives");
        assert!(loaded.sliced(second).is_none(), "the stalest size goes");

        Ok(())
    }

    #[test]
    fn a_small_image_is_left_alone() {
        let small = DynamicImage::ImageRgb8(image::RgbImage::new(48, 24));

        let kept = downscale(small);

        assert_eq!((kept.width(), kept.height()), (48, 24));
    }
}
