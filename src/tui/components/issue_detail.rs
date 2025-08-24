use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

use super::Renderable;
use crate::api::Issue;

pub struct IssueDetail<'a> {
    issue: &'a Issue,
}

impl<'a> IssueDetail<'a> {
    pub fn new(issue: &'a Issue) -> Self {
        Self { issue }
    }
}

impl<'a> Renderable for IssueDetail<'a> {
    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let mut content = vec![
            Line::from(vec![
                Span::styled("URL: ", Style::default().fg(Color::Yellow)),
                Span::raw(&self.issue.url),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Title: ", Style::default().fg(Color::Yellow)),
                Span::raw(&self.issue.title),
            ]),
            Line::from(""),
        ];

        if !self.issue.description.is_empty() {
            content.push(Line::from(Span::styled(
                "Description:",
                Style::default().fg(Color::Yellow),
            )));
            content.push(Line::from(""));

            for line in self.issue.description.lines() {
                content.push(Line::from(line));
            }
            content.push(Line::from(""));
        }

        if !self.issue.comments.nodes.is_empty() {
            content.push(Line::from(Span::styled(
                "Comments:",
                Style::default().fg(Color::Yellow),
            )));
            content.push(Line::from(""));

            for comment in &self.issue.comments.nodes {
                content.push(Line::from("---"));
                for line in comment.body.lines() {
                    content.push(Line::from(line));
                }
                content.push(Line::from(""));
            }
        }

        let text = Text::from(content);
        let paragraph = Paragraph::new(text)
            .block(
                Block::default()
                    .title("Issue Detail")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Yellow)),
            )
            .wrap(Wrap { trim: true });

        frame.render_widget(paragraph, area);
    }
}
