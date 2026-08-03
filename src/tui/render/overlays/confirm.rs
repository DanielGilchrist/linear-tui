use ratatui::{
    layout::Rect,
    text::{Line, Span, Text},
    widgets::{Block, Paragraph, Wrap},
    Frame,
};

use super::super::theme;
use crate::tui::layout;
use crate::tui::overlay::Confirm;

pub fn area(frame_area: Rect) -> Rect {
    layout::centred_rect_fixed(frame_area, 50, 6)
}

pub fn render(confirm: &Confirm, frame: &mut Frame, area: Rect) {
    let block = Block::bordered()
        .title("Confirm")
        .border_style(theme::accent());

    let text = Text::from(vec![
        Line::from(confirm.message.clone()),
        Line::from(""),
        Line::from(Span::styled("[y] yes    [n] no", theme::dim())),
    ]);

    frame.render_widget(
        Paragraph::new(text).block(block).wrap(Wrap { trim: true }),
        area,
    );
}
