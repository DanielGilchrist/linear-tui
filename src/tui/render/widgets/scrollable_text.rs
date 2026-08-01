use ratatui::{
    layout::{Margin, Rect},
    style::Style,
    text::Text,
    widgets::{Block, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap},
    Frame,
};

use crate::tui::focus::Scroll;

pub struct ScrollableText<'a> {
    content: Text<'a>,
    scroll: Scroll,
    title: Option<&'a str>,
    border: Style,
}

impl<'a> ScrollableText<'a> {
    pub fn new(content: Text<'a>, scroll: Scroll) -> Self {
        Self {
            content,
            scroll,
            title: None,
            border: Style::default(),
        }
    }

    pub fn title(mut self, title: &'a str) -> Self {
        self.title = Some(title);
        self
    }

    pub fn border_style(mut self, border: Style) -> Self {
        self.border = border;
        self
    }

    #[must_use = "the content height is scratch the detail session needs to resolve Scroll::Bottom"]
    pub fn render(self, frame: &mut Frame, area: Rect) -> usize {
        let text_height = area.height.saturating_sub(2) as usize;
        let text_width = area.width.saturating_sub(2);
        let paragraph = Paragraph::new(self.content).wrap(Wrap { trim: false });
        let wrapped_line_count = paragraph.line_count(text_width);
        let max_scroll = wrapped_line_count.saturating_sub(text_height);
        let row = self.scroll.resolve(max_scroll);

        let mut block = Block::bordered().border_style(self.border);

        if let Some(title) = self.title {
            block = block.title(title);
        }

        frame.render_widget(paragraph.block(block).scroll((row as u16, 0)), area);

        let mut scroll_state = ScrollbarState::default()
            .content_length(max_scroll)
            .position(row);

        frame.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("↑"))
                .end_symbol(Some("↓")),
            area.inner(Margin {
                vertical: 1,
                horizontal: 0,
            }),
            &mut scroll_state,
        );

        max_scroll
    }
}
