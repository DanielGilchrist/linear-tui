use ratatui::text::{Line, Span, Text};

use super::super::theme;
use crate::api::{IssueSummary, NotificationItem};

pub fn preview_text(issue: &IssueSummary) -> Text<'static> {
    let mut lines = vec![status_line(issue), title_line(issue)];

    if let Some(meta) = meta_line(issue) {
        lines.push(meta);
    }

    lines.push(Line::from(Span::styled(issue.url.clone(), theme::dim())));
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "Press enter to load the description and comments",
        theme::dim(),
    )));

    Text::from(lines)
}

fn status_line(issue: &IssueSummary) -> Line<'static> {
    Line::from(vec![
        Span::styled(issue.identifier.clone(), theme::dim()),
        Span::raw("  "),
        Span::styled(
            issue.state.name.clone(),
            theme::state(issue.state.state_type),
        ),
        Span::raw("  "),
        Span::styled(
            issue.priority.label().to_string(),
            theme::priority_style(issue.priority),
        ),
    ])
}

fn title_line(issue: &IssueSummary) -> Line<'static> {
    Line::from(Span::styled(
        issue.title.clone().unwrap_or_else(|| "Untitled".into()),
        theme::TITLE,
    ))
}

fn meta_line(issue: &IssueSummary) -> Option<Line<'static>> {
    let mut meta: Vec<Span> = Vec::new();

    if let Some(assignee) = &issue.assignee {
        meta.push(Span::styled(
            format!("@{}", assignee.display_name),
            theme::person(),
        ));
    }

    for label in &issue.labels {
        meta.push(Span::raw(" "));
        meta.push(Span::styled(
            format!(" {} ", label.name),
            theme::label_chip(label.colour),
        ));
    }

    (!meta.is_empty()).then_some(Line::from(meta))
}

pub fn notification_preview_text(notification: &NotificationItem) -> Text<'static> {
    let mut lines = vec![Line::from(Span::styled(
        notification.title.clone(),
        theme::TITLE,
    ))];

    lines.push(Line::from(Span::styled(
        if notification.is_read {
            "read"
        } else {
            "unread"
        },
        theme::dim(),
    )));

    lines.push(Line::from(""));

    lines.extend(notification.issue_id.as_ref().map(|_| {
        Line::from(Span::styled(
            "Press enter to open the linked issue",
            theme::dim(),
        ))
    }));

    Text::from(lines)
}
