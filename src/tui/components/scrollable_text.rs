use ratatui::{
    layout::{Margin, Rect},
    style::{Color, Style},
    text::Text,
    widgets::{Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap},
    Frame,
};

pub struct ScrollableText<'a> {
    content: Text<'a>,
    scroll_position: usize,
    scroll_state: &'a mut ScrollbarState,
    title: Option<&'a str>,
    border_colour: Color,
}

impl<'a> ScrollableText<'a> {
    pub fn new(
        content: Text<'a>,
        scroll_position: usize,
        scroll_state: &'a mut ScrollbarState,
    ) -> Self {
        Self {
            content,
            scroll_position,
            scroll_state,
            title: None,
            border_colour: Color::Yellow,
        }
    }

    pub fn title(mut self, title: &'a str) -> Self {
        self.title = Some(title);
        self
    }

    pub fn border_colour(mut self, colour: Color) -> Self {
        self.border_colour = colour;
        self
    }

    pub fn clamped_scroll_position(&self) -> usize {
        self.scroll_position
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect) {
        let text_height = area.height.saturating_sub(2) as usize;
        let max_scroll = self.content.lines.len().saturating_sub(text_height);
        self.scroll_position = self.scroll_position.min(max_scroll);

        let mut block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(self.border_colour));

        if let Some(title) = self.title {
            block = block.title(title);
        }

        let paragraph = Paragraph::new(self.content.clone())
            .block(block)
            .wrap(Wrap { trim: false })
            .scroll((self.scroll_position as u16, 0));

        frame.render_widget(paragraph, area);

        *self.scroll_state = self
            .scroll_state
            .content_length(self.content.lines.len())
            .position(self.scroll_position);

        frame.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("↑"))
                .end_symbol(Some("↓")),
            area.inner(Margin {
                vertical: 1,
                horizontal: 0,
            }),
            self.scroll_state,
        );
    }
}
