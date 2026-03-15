use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::Span,
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

pub struct StyledList<'a> {
    items: Vec<ListItem<'a>>,
    title: String,
    focused: bool,
    state: Option<&'a mut ListState>,
    placeholder: Option<String>,
    panel_number: Option<usize>,
    position: Option<(Option<usize>, usize)>,
}

impl<'a> StyledList<'a> {
    pub fn new(title: &str) -> Self {
        Self {
            items: Vec::new(),
            title: title.to_string(),
            focused: false,
            state: None,
            placeholder: None,
            panel_number: None,
            position: None,
        }
    }

    pub fn panel_number(mut self, number: usize) -> Self {
        self.panel_number = Some(number);
        self
    }

    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
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

    pub fn placeholder(mut self, placeholder: &str) -> Self {
        self.placeholder = Some(placeholder.to_string());
        self
    }

    pub fn position(mut self, selected: Option<usize>, total: usize) -> Self {
        self.position = Some((selected, total));
        self
    }

    pub fn render(self, frame: &mut Frame, area: Rect) {
        let border_style = if self.focused {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::Gray)
        };

        let title = if let Some(num) = self.panel_number {
            format!("[{}] {}", num, self.title)
        } else {
            self.title.clone()
        };

        let position_text = self.position.and_then(|(selected, total)| {
            if total == 0 {
                return None;
            }
            let current = selected.map(|s| s + 1).unwrap_or(0);
            Some(format!(" {} of {} ", current, total))
        });

        let mut block = Block::default()
            .title(Span::from(title))
            .borders(Borders::ALL)
            .border_style(border_style);

        if let Some(pos) = position_text {
            block = block.title_bottom(Span::styled(pos, Style::default().fg(Color::DarkGray)));
        }

        if let Some(placeholder) = self.placeholder {
            let placeholder = Paragraph::new(placeholder).block(block);
            return frame.render_widget(placeholder, area);
        }

        let highlight_style = if self.focused {
            Style::default().bg(Color::DarkGray).fg(Color::White)
        } else {
            Style::default()
        };

        let list = List::new(self.items)
            .block(block)
            .highlight_style(highlight_style);

        if let Some(state) = self.state {
            frame.render_stateful_widget(list, area, state);
        } else {
            frame.render_widget(list, area);
        }
    }
}
