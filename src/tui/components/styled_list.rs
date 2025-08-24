use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, ListState},
    Frame,
};

pub struct StyledList<'a> {
    items: Vec<ListItem<'a>>,
    title: String,
    focused: bool,
    state: Option<&'a mut ListState>,
}

impl<'a> StyledList<'a> {
    pub fn new(title: &str) -> Self {
        Self {
            items: Vec::new(),
            title: title.to_string(),
            focused: false,
            state: None,
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

    pub fn render(self, frame: &mut Frame, area: Rect) {
        let border_style = if self.focused {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::Gray)
        };

        let list = List::new(self.items)
            .block(
                Block::default()
                    .title(self.title)
                    .borders(Borders::ALL)
                    .border_style(border_style),
            )
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED));

        if let Some(state) = self.state {
            frame.render_stateful_widget(list, area, state);
        } else {
            frame.render_widget(list, area);
        }
    }
}
