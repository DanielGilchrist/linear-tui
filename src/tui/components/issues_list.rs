use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{ListItem, ListState},
    Frame,
};

use super::styled_list::StyledList;
use crate::api::team_issues;

pub struct IssuesList<'a> {
    issues: &'a [team_issues::Issue],
    state: &'a mut ListState,
    focused: bool,
    show_placeholder: bool,
}

impl<'a> IssuesList<'a> {
    pub fn new(issues: &'a [team_issues::Issue], state: &'a mut ListState) -> Self {
        Self {
            issues,
            state,
            focused: false,
            show_placeholder: issues.is_empty(),
        }
    }

    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }

    pub fn show_placeholder(mut self, show: bool) -> Self {
        self.show_placeholder = show;
        self
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect) {
        let items: Vec<ListItem> = self
            .issues
            .iter()
            .map(|issue| {
                ListItem::new(Line::from(Span::styled(
                    issue.title.as_deref().unwrap_or("Untitled"),
                    Style::default().fg(Color::White),
                )))
            })
            .collect();

        let selected = self.state.selected();
        let total = self.issues.len();

        let mut list = StyledList::new("Issues")
            .items(items)
            .focused(self.focused)
            .state(self.state)
            .position(selected, total);

        if self.show_placeholder {
            list = list.placeholder("Select a team to view issues");
        }

        list.render(frame, area);
    }
}
