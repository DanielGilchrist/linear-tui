use ratatui::{
    buffer::Buffer,
    layout::Rect,
    text::{Line, Span},
    widgets::{Block, List, ListItem, ListState, Paragraph, StatefulWidget, Widget},
};

use super::super::theme::{self, Emphasis};

pub struct StyledList<'a> {
    items: Vec<ListItem<'a>>,
    title: String,
    title_line: Option<Line<'a>>,
    emphasis: Emphasis,
    state: Option<&'a mut ListState>,
    placeholder: Option<Line<'a>>,
    position: Option<(Option<usize>, usize)>,
}

impl<'a> StyledList<'a> {
    pub fn new(title: &str) -> Self {
        Self {
            items: Vec::new(),
            title: title.to_string(),
            title_line: None,
            emphasis: Emphasis::Blurred,
            state: None,
            placeholder: None,
            position: None,
        }
    }

    pub fn title_line(mut self, line: Line<'a>) -> Self {
        self.title_line = Some(line);
        self
    }

    pub fn emphasis(mut self, emphasis: Emphasis) -> Self {
        self.emphasis = emphasis;
        self
    }

    pub fn items(mut self, items: Vec<ListItem<'a>>) -> Self {
        self.items = items;
        self
    }

    pub fn state(mut self, state: &'a mut ListState) -> Self {
        self.state = Some(state);
        self
    }

    pub fn placeholder(mut self, placeholder: Line<'a>) -> Self {
        self.placeholder = Some(placeholder);
        self
    }

    pub fn position(mut self, selected: Option<usize>, total: usize) -> Self {
        self.position = Some((selected, total));
        self
    }
}

impl Widget for StyledList<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let title = match self.title_line {
            Some(line) => self.emphasis.blur_title(line),
            None => Line::from(Span::styled(self.title.clone(), self.emphasis.title())),
        };

        let position_text = self.position.and_then(|(selected, total)| {
            if total == 0 {
                return None;
            }

            let current = selected.map(|s| s + 1).unwrap_or(0);
            Some(format!(" {} of {} ", current, total))
        });

        let mut block = Block::bordered()
            .title(title)
            .border_style(self.emphasis.border());

        if let Some(pos) = position_text {
            block = block.title_bottom(Span::styled(pos, theme::dim()));
        }

        if let (true, Some(placeholder)) = (self.items.is_empty(), self.placeholder) {
            Paragraph::new(placeholder).block(block).render(area, buf);
            return;
        }

        let list = List::new(self.items)
            .block(block)
            .highlight_style(self.emphasis.highlight())
            .scroll_padding(1);

        match self.state {
            Some(state) => StatefulWidget::render(list, area, buf, state),
            None => Widget::render(list, area, buf),
        }
    }
}
