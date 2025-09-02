use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{ListItem, ListState},
    Frame,
};

use super::styled_list::StyledList;
use super::Renderable;
use crate::api::TeamIssue;

pub struct IssuesList<'a> {
    issues: &'a [TeamIssue],
    state: &'a mut ListState,
    focused: bool,
    show_placeholder: bool,
}

impl<'a> IssuesList<'a> {
    pub fn new(issues: &'a [TeamIssue], state: &'a mut ListState) -> Self {
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
}

impl<'a> Renderable for IssuesList<'a> {
    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let max_desc_width = area.width.saturating_sub(5) as usize;

        let items: Vec<ListItem> = self
            .issues
            .iter()
            .map(|issue| {
                let description = match &issue.description {
                    Some(desc) if !desc.is_empty() => {
                        if desc.chars().count() > max_desc_width {
                            let truncate_at = max_desc_width.saturating_sub(3);
                            let truncated: String = desc.chars().take(truncate_at).collect();
                            format!("{}...", truncated)
                        } else {
                            desc.clone()
                        }
                    }
                    _ => "No description".to_string(),
                };

                ListItem::new(vec![
                    Line::from(Span::styled(
                        &issue.title,
                        Style::default().fg(Color::White),
                    )),
                    Line::from(Span::styled(description, Style::default().fg(Color::Gray))),
                ])
            })
            .collect();

        let mut list = StyledList::new("Issues")
            .items(items)
            .focused(self.focused)
            .state(self.state);

        if self.show_placeholder {
            list = list.placeholder("Select a team to view issues");
        }

        list.render(frame, area);
    }
}
