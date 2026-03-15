use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{ListItem, ListState},
    Frame,
};

use super::styled_list::StyledList;
use crate::api::notifications::Notification;

pub struct Inbox<'a> {
    notifications: &'a [Notification],
    state: &'a mut ListState,
    focused: bool,
    panel_number: Option<usize>,
}

impl<'a> Inbox<'a> {
    pub fn new(notifications: &'a [Notification], state: &'a mut ListState) -> Self {
        Self {
            notifications,
            state,
            focused: false,
            panel_number: None,
        }
    }

    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }

    pub fn panel_number(mut self, number: usize) -> Self {
        self.panel_number = Some(number);
        self
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect) {
        let items: Vec<ListItem> = self
            .notifications
            .iter()
            .map(|notification| {
                let is_read = notification.is_read();

                let indicator = if is_read {
                    Span::styled("  ", Style::default())
                } else {
                    Span::styled("● ", Style::default().fg(Color::Blue))
                };

                let title_style = if is_read {
                    Style::default().fg(Color::DarkGray)
                } else {
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD)
                };

                ListItem::new(Line::from(vec![
                    indicator,
                    Span::styled(notification.title(), title_style),
                ]))
            })
            .collect();

        let selected = self.state.selected();
        let total = self.notifications.len();

        let mut list = StyledList::new("Inbox")
            .items(items)
            .focused(self.focused)
            .state(self.state)
            .position(selected, total);

        if let Some(num) = self.panel_number {
            list = list.panel_number(num);
        }

        list.render(frame, area);
    }
}
