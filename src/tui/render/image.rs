use std::collections::VecDeque;
use std::sync::{Arc, OnceLock};

use std::io::Cursor;
use std::num::NonZeroU16;

use crate::api::ImageUrl;
use crate::tui::cache::{CacheStatus, Remote};
use crate::tui::spinner::Spinner;

use image::imageops::FilterType;
use image::{DynamicImage, ImageReader, Limits};
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

const MAX_DECODED_EDGE: u32 = 12_000;

const MAX_DECODE_ALLOCATION: u64 = 256 * 1024 * 1024;

pub fn decode(bytes: &[u8]) -> Option<DynamicImage> {
    let mut limits = Limits::default();
    limits.max_image_width = Some(MAX_DECODED_EDGE);
    limits.max_image_height = Some(MAX_DECODED_EDGE);
    limits.max_alloc = Some(MAX_DECODE_ALLOCATION);

    let mut reader = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .ok()?;
    reader.limits(limits);

    Some(downscale(reader.decode().ok()?))
}

fn downscale(image: DynamicImage) -> DynamicImage {
    let fitted = if image.width() <= MAX_EDGE && image.height() <= MAX_EDGE {
        image
    } else {
        image.resize(MAX_EDGE, MAX_EDGE, FilterType::Triangle)
    };

    DynamicImage::ImageRgba8(fitted.into_rgba8())
}

pub struct Encoded {
    size: Size,
    sliced: SlicedProtocol,
}

impl Encoded {
    pub fn size(&self) -> Size {
        self.size
    }

    pub fn sliced(&self) -> &SlicedProtocol {
        &self.sliced
    }
}

impl std::fmt::Debug for Encoded {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Encoded").field("size", &self.size).finish()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum EncodeFailure {
    #[error("Could not render this image")]
    Unsupported,
    #[error("The image worker stopped before rendering this image")]
    WorkerStopped,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DrawnImage {
    pub url: ImageUrl,
    pub size: Size,
}

pub fn encode(source: &DynamicImage, size: Size) -> Result<Encoded, EncodeFailure> {
    let font = picker().font_size();
    let box_width = u32::from(size.width).saturating_mul(u32::from(font.width));
    let box_height = u32::from(size.height).saturating_mul(u32::from(font.height));
    let fitted = if source.width() <= box_width && source.height() <= box_height {
        source.clone()
    } else {
        source.resize(box_width, box_height, FilterType::Triangle)
    };
    let sliced = SlicedProtocol::new(picker(), fitted, Some(size))
        .map_err(|_| EncodeFailure::Unsupported)?;

    Ok(Encoded { size, sliced })
}

pub enum Shown<'a> {
    Drawn(&'a Encoded),
    Placeholder(Placeholder),
}

pub enum Placeholder {
    Preparing,
    NotLoaded,
    Unrenderable(EncodeFailure),
    FetchFailed(String),
}

pub fn shown(cell: Option<&Remote<Loaded>>, size: Size) -> Shown<'_> {
    let Some(cell) = cell else {
        return Shown::Placeholder(Placeholder::NotLoaded);
    };

    if let Some(loaded) = cell.value() {
        return match (loaded.drawable(size), loaded.failure_at(size)) {
            (Some(encoded), _) => Shown::Drawn(encoded),
            (None, Some(failure)) => Shown::Placeholder(Placeholder::Unrenderable(failure)),
            (None, None) => Shown::Placeholder(Placeholder::Preparing),
        };
    }

    Shown::Placeholder(match cell.status() {
        CacheStatus::Loading | CacheStatus::Revalidating => Placeholder::Preparing,
        CacheStatus::Failed(error) => Placeholder::FetchFailed(error),
        CacheStatus::Idle | CacheStatus::Ready => Placeholder::NotLoaded,
    })
}

impl Placeholder {
    pub fn message(&self, spinner: Spinner) -> String {
        match self {
            Placeholder::Preparing => format!("{spinner}  Loading image…"),
            Placeholder::NotLoaded => "Image not loaded".to_string(),
            Placeholder::Unrenderable(failure) => failure.to_string(),
            Placeholder::FetchFailed(error) => error.clone(),
        }
    }

    pub fn is_failure(&self) -> bool {
        match self {
            Placeholder::Unrenderable(_) | Placeholder::FetchFailed(_) => true,
            Placeholder::Preparing | Placeholder::NotLoaded => false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct EncodeRequest {
    pub size: Size,
    pub source: Arc<DynamicImage>,
}

const KEPT_SIZES: usize = 4;

pub struct Loaded {
    source: Arc<DynamicImage>,
    encoded: VecDeque<Encoded>,
    pending: Vec<Size>,
    failed: Vec<(Size, EncodeFailure)>,
    pub width: u32,
    pub height: u32,
}

pub fn load(source: DynamicImage) -> Loaded {
    let (width, height) = (source.width(), source.height());

    Loaded {
        source: Arc::new(source),
        encoded: VecDeque::new(),
        pending: Vec::new(),
        failed: Vec::new(),
        width,
        height,
    }
}

fn fits_within(inner: Size, outer: Size) -> bool {
    inner.width <= outer.width && inner.height <= outer.height
}

impl Loaded {
    pub fn drawable(&self, size: Size) -> Option<&Encoded> {
        let exact = self.encoded.iter().find(|encoded| encoded.size == size);
        let stale = || {
            self.encoded
                .iter()
                .filter(|encoded| fits_within(encoded.size, size))
                .max_by_key(|encoded| {
                    u32::from(encoded.size.width) * u32::from(encoded.size.height)
                })
        };

        exact.or_else(stale)
    }

    pub fn claim(&mut self, size: Size) -> Option<EncodeRequest> {
        let exact = self
            .encoded
            .iter()
            .position(|encoded| encoded.size == size)
            .and_then(|index| self.encoded.remove(index));

        if let Some(exact) = exact {
            self.encoded.push_back(exact);

            return None;
        }

        if self.pending.contains(&size) || self.failure_at(size).is_some() {
            return None;
        }

        self.pending.push(size);

        Some(EncodeRequest {
            size,
            source: Arc::clone(&self.source),
        })
    }

    pub fn set_encoded(&mut self, encoded: Encoded) {
        self.pending.retain(|size| *size != encoded.size);
        self.encoded.retain(|kept| kept.size != encoded.size);
        self.encoded.push_back(encoded);

        while self.encoded.len() > KEPT_SIZES {
            self.encoded.pop_front();
        }
    }

    pub fn encode_failed(&mut self, size: Size, failure: EncodeFailure) {
        self.pending.retain(|pending| *pending != size);

        if self.failure_at(size).is_none() {
            self.failed.push((size, failure));
        }
    }

    pub fn failure_at(&self, size: Size) -> Option<EncodeFailure> {
        self.failed
            .iter()
            .find(|(failed, _)| *failed == size)
            .map(|(_, failure)| *failure)
    }

    pub fn is_encoding(&self) -> bool {
        !self.pending.is_empty()
    }

    pub fn rows_for(&self, cells_wide: u16, max_rows: NonZeroU16) -> u16 {
        if self.width == 0 || self.height == 0 || cells_wide == 0 {
            return 1;
        }

        let font = picker().font_size();
        let pixels_wide = u32::from(cells_wide) * u32::from(font.width);
        let pixels_high = pixels_wide.saturating_mul(self.height) / self.width;
        let rows = pixels_high.div_ceil(u32::from(font.height).max(1));

        u16::try_from(rows)
            .unwrap_or(u16::MAX)
            .clamp(1, max_rows.get())
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

        for width in (1u16..).take(KEPT_SIZES) {
            loaded.set_encoded(encode(&source, Size::new(width * 4, 4)).map_err(|_| "encodes")?);
        }

        assert!(
            loaded.claim(oldest).is_none(),
            "drawing refreshes the oldest"
        );

        loaded.set_encoded(encode(&source, newest).map_err(|_| "encodes")?);

        assert!(loaded.claim(oldest).is_none(), "a recent size survives");
        assert!(loaded.claim(second).is_some(), "the stalest size goes");

        Ok(())
    }

    #[test]
    fn a_resized_pane_draws_the_largest_encoding_that_still_fits() -> Result<(), &'static str> {
        let source = DynamicImage::ImageRgb8(image::RgbImage::new(48, 24));
        let mut loaded = load(source.clone());

        loaded.set_encoded(encode(&source, Size::new(10, 4)).map_err(|_| "encodes")?);
        loaded.set_encoded(encode(&source, Size::new(20, 8)).map_err(|_| "encodes")?);
        loaded.set_encoded(encode(&source, Size::new(40, 16)).map_err(|_| "encodes")?);

        let wider = loaded
            .drawable(Size::new(30, 12))
            .ok_or("a fitting encoding")?;

        assert_eq!(wider.size(), Size::new(20, 8));
        assert!(
            loaded.drawable(Size::new(5, 2)).is_none(),
            "nothing smaller exists, so the loader shows"
        );

        Ok(())
    }

    #[test]
    fn a_small_image_is_left_alone() {
        let small = DynamicImage::ImageRgb8(image::RgbImage::new(48, 24));

        let kept = downscale(small);

        assert_eq!((kept.width(), kept.height()), (48, 24));
    }
}
