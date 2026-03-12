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
            Line::from(vec![
                Span::styled(&self.issue.identifier, Style::default().fg(Color::DarkGray)),
                Span::raw(" "),
                Span::styled(
                    self.issue.title.as_deref().unwrap_or("Untitled"),
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Status: ", Style::default().fg(Color::Yellow)),
                Span::styled(&self.issue.state.name, Style::default().fg(Color::White)),
                Span::styled("  Priority: ", Style::default().fg(Color::Yellow)),
                Span::styled(
                    &self.issue.priority_label,
                    Style::default().fg(Color::White),
                ),
            ]),
        ];

        if let Some(assignee) = &self.issue.assignee {
            content.push(Line::from(vec![
                Span::styled("Assignee: ", Style::default().fg(Color::Yellow)),
                Span::styled(&assignee.display_name, Style::default().fg(Color::White)),
            ]));
        }

        let labels = &self.issue.labels.nodes;
        if !labels.is_empty() {
            let label_names: Vec<&str> = labels.iter().map(|l| l.name.as_str()).collect();
            content.push(Line::from(vec![
                Span::styled("Labels: ", Style::default().fg(Color::Yellow)),
                Span::styled(label_names.join(", "), Style::default().fg(Color::White)),
            ]));
        }

        content.push(Line::from(""));

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
            "Enter: load full details  ·  Esc: close preview",
            Style::default().fg(Color::DarkGray),
        )));

        let paragraph = Paragraph::new(Text::from(content))
            .block(block)
            .wrap(Wrap { trim: true });

        frame.render_widget(paragraph, area);
    }
}
