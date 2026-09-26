use ratatui::{
    layout::{Margin, Rect},
    style::Style,
    text::{Line, Text},
    widgets::{Block, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap},
    Frame,
};

use crate::tui::focus::Scroll;

pub fn text_area(area: Rect) -> (u16, u16) {
    (area.width.saturating_sub(2), area.height.saturating_sub(2))
}

pub fn saturating_u16(value: usize) -> u16 {
    u16::try_from(value).unwrap_or(u16::MAX)
}

pub fn wrapped_rows(lines: &[Line<'_>], width: u16) -> usize {
    if lines.is_empty() {
        return 0;
    }

    Paragraph::new(Text::from(lines.to_vec()))
        .wrap(Wrap { trim: false })
        .line_count(width)
}

pub fn max_scroll(content: &Text<'_>, area: Rect) -> usize {
    let (width, height) = text_area(area);

    wrapped_rows(&content.lines, width).saturating_sub(usize::from(height))
}

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
        let (text_width, text_height) = text_area(area);
        let paragraph = Paragraph::new(self.content).wrap(Wrap { trim: false });
        let max_scroll = paragraph
            .line_count(text_width)
            .saturating_sub(usize::from(text_height));
        let row = self.scroll.resolve(max_scroll);

        let mut block = Block::bordered().border_style(self.border);

        if let Some(title) = self.title {
            block = block.title(title);
        }

        frame.render_widget(
            paragraph.block(block).scroll((saturating_u16(row), 0)),
            area,
        );

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
