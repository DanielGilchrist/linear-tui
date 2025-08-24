use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, ListItem, ListState, Paragraph},
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
        if self.show_placeholder {
            let placeholder = Paragraph::new("Select a team to view issues")
                .block(Block::default().title("Issues").borders(Borders::ALL));
            frame.render_widget(placeholder, area);
            return;
        }

        let max_desc_width = area.width.saturating_sub(5) as usize;

        let items: Vec<ListItem> = self
            .issues
            .iter()
            .map(|issue| {
                let description = match &issue.description {
                    Some(desc) if !desc.is_empty() => {
                        if desc.len() > max_desc_width {
                            format!("{}...", &desc[..max_desc_width.saturating_sub(3)])
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

        StyledList::new("Issues")
            .items(items)
            .focused(self.focused)
            .state(self.state)
            .render(frame, area);
    }
}
