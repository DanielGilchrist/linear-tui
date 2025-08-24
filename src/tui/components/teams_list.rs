use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{ListItem, ListState},
    Frame,
};

use super::styled_list::StyledList;
use super::Renderable;
use crate::api::Team;

pub struct TeamsList<'a> {
    teams: &'a [Team],
    state: &'a mut ListState,
    focused: bool,
}

impl<'a> TeamsList<'a> {
    pub fn new(teams: &'a [Team], state: &'a mut ListState) -> Self {
        Self {
            teams,
            state,
            focused: false,
        }
    }

    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }
}

impl<'a> Renderable for TeamsList<'a> {
    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let items: Vec<ListItem> = self
            .teams
            .iter()
            .map(|team| {
                ListItem::new(Line::from(vec![
                    Span::styled(&team.name, Style::default().fg(Color::White)),
                    Span::styled(
                        format!(" ({} issues)", team.issue_count),
                        Style::default().fg(Color::Gray),
                    ),
                ]))
            })
            .collect();

        StyledList::new("Teams")
            .items(items)
            .focused(self.focused)
            .state(self.state)
            .render(frame, area);
    }
}
