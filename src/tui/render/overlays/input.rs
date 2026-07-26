use ratatui::{
    layout::Rect,
    widgets::{Block, Paragraph},
    Frame,
};

use super::super::format;
use super::super::theme;
use super::super::widgets::cursor_line;
use crate::tui::layout;
use crate::tui::overlay::Input;

pub fn area(frame_area: Rect) -> Rect {
    layout::centred_rect_fixed(frame_area, 60, 3)
}

pub fn render(input: &Input, frame: &mut Frame, area: Rect) {
    let block = Block::bordered()
        .title(input.prompt)
        .border_style(theme::ACCENT);

    let before: String = input.buffer.chars().take(input.cursor).collect();
    let cursor_column = 1 + format::width(&before) as u16;
    let inner_width = area.width.saturating_sub(2);
    let scroll_x = cursor_column.saturating_sub(inner_width.saturating_sub(1));
    let paragraph = Paragraph::new(cursor_line(&input.buffer, input.cursor))
        .block(block)
        .scroll((0, scroll_x));

    frame.render_widget(paragraph, area);
}
