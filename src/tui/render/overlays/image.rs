use ratatui::{
    layout::{Rect, Size},
    text::Span,
    widgets::{Block, Paragraph},
    Frame,
};
use ratatui_image::sliced::{SignedPosition, SlicedImage};

use super::super::theme::{self, Emphasis};
use crate::tui::cache::Remote;
use crate::tui::overlay::ImageView;
use crate::tui::render::image::{self, DrawnImage, Shown};
use crate::tui::spinner::Spinner;
use crate::tui::workspace::ImageStore;

pub fn render(
    view: &ImageView,
    images: &ImageStore,
    spinner: Spinner,
    frame: &mut Frame,
    area: Rect,
) -> Option<DrawnImage> {
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

    let size = Size::new(inner.width, inner.height);
    let cell = images.get(view.url());
    let state = image::shown(cell, size);

    match &state {
        Shown::Drawn(encoded) => {
            frame.render_widget(
                SlicedImage::new(encoded.sliced(), SignedPosition::from((0, 0))),
                inner,
            );
        }
        Shown::Placeholder(placeholder) => {
            frame.render_widget(
                Paragraph::new(Span::styled(placeholder.message(spinner), theme::dim())),
                inner,
            );
        }
    }

    cell.and_then(Remote::value).map(|_| DrawnImage {
        url: view.url().clone(),
        size,
    })
}
