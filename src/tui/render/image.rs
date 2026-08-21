use std::sync::OnceLock;

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

pub struct Loaded {
    source: DynamicImage,
    sliced: Option<(Size, SlicedProtocol)>,
    encodes: usize,
    pub width: u32,
    pub height: u32,
}

pub fn load(source: DynamicImage) -> Loaded {
    let (width, height) = (source.width(), source.height());

    Loaded {
        source,
        sliced: None,
        encodes: 0,
        width,
        height,
    }
}

impl Loaded {
    pub fn sliced(&mut self, size: Size) -> Option<&SlicedProtocol> {
        let stale = self
            .sliced
            .as_ref()
            .is_none_or(|(encoded, _)| encoded != &size);

        if stale {
            let sliced = SlicedProtocol::new(picker(), self.source.clone(), Some(size)).ok()?;

            self.sliced = Some((size, sliced));
            self.encodes += 1;
        }

        self.sliced.as_ref().map(|(_, sliced)| sliced)
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
