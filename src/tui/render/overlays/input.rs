use ratatui::{
    layout::Rect,
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use super::super::theme;
use super::super::widgets::cursor_line;
use crate::tui::layout;
use crate::tui::overlay::Input;

pub fn area(frame_area: Rect) -> Rect {
    layout::centred_rect_fixed(frame_area, 60, 3)
}

pub fn render(input: &Input, frame: &mut Frame, area: Rect) {
    let block = Block::default()
        .title(input.prompt)
        .borders(Borders::ALL)
        .border_style(theme::ACCENT);

    frame.render_widget(
        Paragraph::new(cursor_line(&input.buffer, input.cursor)).block(block),
        area,
    );
}
