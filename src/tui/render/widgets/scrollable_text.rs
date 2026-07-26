use ratatui::{
    buffer::Buffer,
    layout::{Margin, Rect},
    style::Style,
    text::Text,
    widgets::{
        Block, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, StatefulWidget, Widget,
        Wrap,
    },
};

pub struct ScrollableText<'a> {
    content: Text<'a>,
    scroll_position: &'a mut usize,
    scroll_state: &'a mut ScrollbarState,
    title: Option<&'a str>,
    border: Style,
}

impl<'a> ScrollableText<'a> {
    pub fn new(
        content: Text<'a>,
        scroll_position: &'a mut usize,
        scroll_state: &'a mut ScrollbarState,
    ) -> Self {
        Self {
            content,
            scroll_position,
            scroll_state,
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
}

impl Widget for ScrollableText<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let text_height = area.height.saturating_sub(2) as usize;
        let text_width = area.width.saturating_sub(2);
        let paragraph = Paragraph::new(self.content).wrap(Wrap { trim: false });
        let wrapped_line_count = paragraph.line_count(text_width);
        let max_scroll = wrapped_line_count.saturating_sub(text_height);

        *self.scroll_position = (*self.scroll_position).min(max_scroll);

        let mut block = Block::bordered().border_style(self.border);

        if let Some(title) = self.title {
            block = block.title(title);
        }

        paragraph
            .block(block)
            .scroll((*self.scroll_position as u16, 0))
            .render(area, buf);

        *self.scroll_state = self
            .scroll_state
            .content_length(max_scroll)
            .position(*self.scroll_position);

        StatefulWidget::render(
            Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("↑"))
                .end_symbol(Some("↓")),
            area.inner(Margin {
                vertical: 1,
                horizontal: 0,
            }),
            buf,
            self.scroll_state,
        );
    }
}
