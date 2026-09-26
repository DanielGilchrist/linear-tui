use ratatui::{
    layout::{Rect, Size},
    text::Span,
    widgets::{Block, Paragraph},
    Frame,
};
use ratatui_image::sliced::{SignedPosition, SlicedImage};

use super::super::theme::{self, Emphasis};
use crate::tui::cache::CacheStatus;
use crate::tui::overlay::ImageView;
use crate::tui::spinner::Spinner;
use crate::tui::workspace::ImageStore;

pub fn render(
    view: &ImageView,
    images: &mut ImageStore,
    spinner: Spinner,
    frame: &mut Frame,
    area: Rect,
) {
    let (position, total) = view.position();
    let title = if total > 1 {
        format!(" {} · {position} of {total} ", view.url())
    } else {
        format!(" {} ", view.url())
    };

    let block = Block::bordered()
        .title(Span::styled(title, theme::dim()))
        .border_style(Emphasis::Focused.border());
    let inner = block.inner(area);

    frame.render_widget(block, area);

    let cell = images.get_or_default(&view.url().to_string());
    let size = Size::new(inner.width, inner.height);
    let status = match cell.value() {
        Some(loaded) if loaded.failed_at(size) => {
            CacheStatus::Failed("Could not render this image".to_string())
        }
        _ => cell.status(),
    };

    match cell.value_mut().and_then(|loaded| loaded.sliced(size)) {
        Some(sliced) => {
            frame.render_widget(
                SlicedImage::new(sliced, SignedPosition::from((0, 0))),
                inner,
            );
        }
        None => {
            let text = match status {
                CacheStatus::Failed(error) => error,
                _ => format!("{spinner}  Loading image…"),
            };

            frame.render_widget(Paragraph::new(Span::styled(text, theme::dim())), inner);
        }
    }
}
