use ratatui::{
    layout::Rect,
    text::Text,
    widgets::{Block, Paragraph, Wrap},
    Frame,
};

use super::super::theme::Emphasis;

pub fn text_panel(frame: &mut Frame, area: Rect, title: &str, text: Text, emphasis: Emphasis) {
    let block = Block::bordered()
        .title(title.to_string())
        .border_style(emphasis.border());

    frame.render_widget(
        Paragraph::new(text).block(block).wrap(Wrap { trim: false }),
        area,
    );
}
