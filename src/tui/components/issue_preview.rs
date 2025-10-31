use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

use crate::api::team_issues;

pub struct IssuePreview<'a> {
    issue: &'a team_issues::Issue,
}

impl<'a> IssuePreview<'a> {
    pub fn new(issue: &'a team_issues::Issue) -> Self {
        Self { issue }
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .title("Preview")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow));

        let mut content = vec![
            Line::from(Span::styled(
                self.issue.title.as_deref().unwrap_or("Untitled"),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
        ];

        match &self.issue.description {
            Some(desc) if !desc.is_empty() => {
                for line in desc.lines() {
                    content.push(Line::from(Span::styled(
                        line,
                        Style::default().fg(Color::Gray),
                    )));
                }
            }
            _ => {
                content.push(Line::from(Span::styled(
                    "No description",
                    Style::default().fg(Color::DarkGray),
                )));
            }
        }

        content.push(Line::from(""));
        content.push(Line::from(Span::styled(
            "Press Enter to load full details  ·  Esc to close",
            Style::default().fg(Color::DarkGray),
        )));

        let paragraph = Paragraph::new(Text::from(content))
            .block(block)
            .wrap(Wrap { trim: true });

        frame.render_widget(paragraph, area);
    }
}
