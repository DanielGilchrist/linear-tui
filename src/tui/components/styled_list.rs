use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

pub struct StyledList<'a> {
    items: Vec<ListItem<'a>>,
    title: String,
    focused: bool,
    state: Option<&'a mut ListState>,
    placeholder: Option<String>,
}

impl<'a> StyledList<'a> {
    pub fn new(title: &str) -> Self {
        Self {
            items: Vec::new(),
            title: title.to_string(),
            focused: false,
            state: None,
            placeholder: None,
        }
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

    pub fn render(self, frame: &mut Frame, area: Rect) {
        let border_style = if self.focused {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::Gray)
        };

        let block = Block::default()
            .title(self.title)
            .borders(Borders::ALL)
            .border_style(border_style);

        if let Some(placeholder) = self.placeholder {
            let placeholder = Paragraph::new(placeholder).block(block);
            return frame.render_widget(placeholder, area);
        }

        let list = List::new(self.items)
            .block(block)
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED));

        if let Some(state) = self.state {
            frame.render_stateful_widget(list, area, state);
        } else {
            frame.render_widget(list, area);
        }
    }
}
