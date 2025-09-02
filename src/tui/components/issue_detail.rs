use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::ScrollbarState,
    Frame,
};

use super::{Renderable, ScrollableText};
use crate::api::Issue;

pub struct IssueDetail<'a> {
    issue: &'a Issue,
    scroll_position: usize,
    scroll_state: &'a mut ScrollbarState,
}

impl<'a> IssueDetail<'a> {
    pub fn new(
        issue: &'a Issue,
        scroll_position: usize,
        scroll_state: &'a mut ScrollbarState,
    ) -> Self {
        Self {
            issue,
            scroll_position,
            scroll_state,
        }
    }

    pub fn get_clamped_scroll_position(&self) -> usize {
        self.scroll_position
    }

    fn build_issue_content(issue: &Issue) -> Text<'_> {
        let mut content = vec![
            Line::from(vec![
                Span::styled("URL: ", Style::default().fg(Color::Yellow)),
                Span::raw(&issue.url),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Title: ", Style::default().fg(Color::Yellow)),
                Span::raw(&issue.title),
            ]),
            Line::from(""),
        ];

        if !issue.description.is_empty() {
            content.push(Line::from(Span::styled(
                "Description:",
                Style::default().fg(Color::Yellow),
            )));
            content.push(Line::from(""));

            for line in issue.description.lines() {
                content.push(Line::from(line));
            }
            content.push(Line::from(""));
        }

        if !issue.comments.nodes.is_empty() {
            content.push(Line::from(Span::styled(
                "Comments:",
                Style::default().fg(Color::Yellow),
            )));
            content.push(Line::from(""));

            for comment in &issue.comments.nodes {
                content.push(Line::from("---"));
                for line in comment.body.lines() {
                    content.push(Line::from(line));
                }
                content.push(Line::from(""));
            }
        }

        Text::from(content)
    }
}

impl<'a> Renderable for IssueDetail<'a> {
    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let content = Self::build_issue_content(self.issue);

        let mut scrollable = ScrollableText::new(content, self.scroll_position, self.scroll_state)
            .title("Issue Detail")
            .border_color(Color::Yellow);

        scrollable.render(frame, area);

        self.scroll_position = scrollable.get_clamped_scroll_position();
    }
}
