use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::ListItem,
    Frame,
};

use super::app::{App, Pane, Screen, ViewKind};
use super::components::{ScrollableText, StyledList};
use super::layout;
use crate::api::{IssueDetail, IssueSummary, NotificationItem};

const SPINNER: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
const SIDEBAR_PCT: u16 = 26;

pub fn render(app: &mut App, frame: &mut Frame) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(frame.area());
    let body = chunks[0];
    let footer = chunks[1];

    match app.screen {
        Screen::Home => render_home(app, frame, body),
        Screen::Detail => render_detail(app, frame, body),
    }

    render_footer(app, frame, footer);
}

fn render_home(app: &mut App, frame: &mut Frame, area: Rect) {
    let [sidebar_area, main_area] = layout::split_horizontal(area, SIDEBAR_PCT);

    let view_items: Vec<ListItem> = app
        .views
        .iter()
        .map(|view| ListItem::new(Line::from(view.name.clone())))
        .collect();

    StyledList::new("Views")
        .items(view_items)
        .focused(app.pane == Pane::Sidebar)
        .state(&mut app.view_state)
        .render(frame, sidebar_area);

    let spinner = SPINNER[app.spinner_frame % SPINNER.len()];
    let focused = app.pane == Pane::Main;
    let selected = app.list_state.selected();

    match app.active_view().kind {
        ViewKind::Issues(_) => {
            let title = title_with_spinner(&app.active_view().name, app.loading, spinner);
            let items = issue_items(&app.issues);
            let total = app.issues.len();
            let mut list = StyledList::new(&title)
                .items(items)
                .focused(focused)
                .state(&mut app.list_state)
                .position(selected, total);
            if total == 0 {
                list = list.placeholder(if app.loading {
                    "Loading…"
                } else {
                    "No issues in this view"
                });
            }
            list.render(frame, main_area);
        }
        ViewKind::Inbox => {
            let title = title_with_spinner("Inbox", app.loading, spinner);
            let items = notification_items(&app.notifications);
            let total = app.notifications.len();
            let mut list = StyledList::new(&title)
                .items(items)
                .focused(focused)
                .state(&mut app.list_state)
                .position(selected, total);
            if total == 0 {
                list = list.placeholder(if app.loading {
                    "Loading…"
                } else {
                    "Inbox empty"
                });
            }
            list.render(frame, main_area);
        }
    }
}

fn render_detail(app: &mut App, frame: &mut Frame, area: Rect) {
    let Some(detail) = &app.detail else {
        let spinner = SPINNER[app.spinner_frame % SPINNER.len()];
        let placeholder = if app.loading {
            format!("{spinner}  Loading…")
        } else {
            "No issue".to_string()
        };
        StyledList::new("Issue")
            .placeholder(&placeholder)
            .render(frame, area);
        return;
    };

    let content = detail_text(detail);
    let title = detail.identifier.clone();
    ScrollableText::new(content, app.scroll_position, &mut app.scroll_state)
        .title(&title)
        .border_color(Color::Yellow)
        .render(frame, area);
}

fn render_footer(app: &App, frame: &mut Frame, area: Rect) {
    use ratatui::layout::Alignment;
    use ratatui::widgets::Paragraph;

    let workspace = match &app.session {
        Some(session) => format!("{} · @{} ", session.org_name, session.user.display_name),
        None => "connecting… ".to_string(),
    };

    let [left, right] = layout::split_footer(area, workspace.chars().count() as u16);

    if let Some(status) = &app.status {
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                format!(" {status}"),
                Style::default().fg(Color::Red),
            ))),
            left,
        );
    } else {
        let hint = match app.screen {
            Screen::Home => {
                " j/k move   tab switch pane   enter open   1-3 views   r refresh   q quit"
            }
            Screen::Detail => " j/k scroll   esc back   r refresh   q quit",
        };
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                hint,
                Style::default().fg(Color::DarkGray),
            ))),
            left,
        );
    }

    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            workspace,
            Style::default().fg(Color::Cyan),
        )))
        .alignment(Alignment::Right),
        right,
    );
}

pub fn render_to_string(app: &mut App, width: u16, height: u16) -> String {
    use ratatui::{backend::TestBackend, Terminal};

    let mut terminal = Terminal::new(TestBackend::new(width, height)).expect("test backend");
    terminal
        .draw(|frame| render(app, frame))
        .expect("draw to test backend");
    buffer_to_string(terminal.backend().buffer())
}

fn buffer_to_string(buffer: &ratatui::buffer::Buffer) -> String {
    let area = buffer.area;
    let mut out = String::new();
    for y in 0..area.height {
        let mut line = String::new();
        for x in 0..area.width {
            line.push_str(buffer[(x, y)].symbol());
        }
        out.push_str(line.trim_end());
        out.push('\n');
    }
    out
}

fn title_with_spinner(name: &str, loading: bool, spinner: &str) -> String {
    if loading {
        format!("{name}  {spinner}")
    } else {
        name.to_string()
    }
}

fn issue_items(issues: &[IssueSummary]) -> Vec<ListItem<'static>> {
    issues
        .iter()
        .map(|issue| {
            let (icon, priority_color) = priority_indicator(issue.priority);
            let state_color = state_type_color(&issue.state.state_type);

            let mut spans = vec![
                Span::styled(icon.to_string(), Style::default().fg(priority_color)),
                Span::raw(" "),
                Span::styled(issue.identifier.clone(), Style::default().fg(Color::DarkGray)),
                Span::raw(" "),
                Span::styled(issue.state.name.clone(), Style::default().fg(state_color)),
                Span::raw(" "),
                Span::styled(
                    issue.title.clone().unwrap_or_else(|| "Untitled".into()),
                    Style::default().fg(Color::White),
                ),
            ];

            if let Some(assignee) = &issue.assignee {
                spans.push(Span::raw(" "));
                spans.push(Span::styled(
                    assignee.display_name.clone(),
                    Style::default().fg(Color::Blue),
                ));
            }

            for label in &issue.labels {
                spans.push(Span::raw(" "));
                spans.push(Span::styled(
                    format!(" {} ", label.name),
                    Style::default()
                        .fg(Color::Black)
                        .bg(parse_hex_color(&label.color)),
                ));
            }

            ListItem::new(Line::from(spans))
        })
        .collect()
}

fn notification_items(notifications: &[NotificationItem]) -> Vec<ListItem<'static>> {
    notifications
        .iter()
        .map(|notification| {
            let indicator = if notification.is_read {
                Span::raw("  ")
            } else {
                Span::styled("● ", Style::default().fg(Color::Blue))
            };
            let title_style = if notification.is_read {
                Style::default().fg(Color::DarkGray)
            } else {
                Style::default().fg(Color::White).add_modifier(Modifier::BOLD)
            };
            ListItem::new(Line::from(vec![
                indicator,
                Span::styled(notification.title.clone(), title_style),
            ]))
        })
        .collect()
}

fn detail_text(detail: &IssueDetail) -> Text<'static> {
    let mut lines: Vec<Line> = Vec::new();

    lines.push(Line::from(vec![
        Span::styled(
            detail.identifier.clone(),
            Style::default().fg(Color::DarkGray),
        ),
        Span::raw("  "),
        Span::styled(
            detail.state.name.clone(),
            Style::default().fg(state_type_color(&detail.state.state_type)),
        ),
    ]));
    lines.push(Line::from(Span::styled(
        detail.title.clone().unwrap_or_else(|| "Untitled".into()),
        Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
    )));

    let mut meta: Vec<Span> = Vec::new();
    if let Some(assignee) = &detail.assignee {
        meta.push(Span::styled(
            format!("@{}", assignee.display_name),
            Style::default().fg(Color::Blue),
        ));
    }
    for label in &detail.labels {
        meta.push(Span::raw(" "));
        meta.push(Span::styled(
            format!(" {} ", label.name),
            Style::default()
                .fg(Color::Black)
                .bg(parse_hex_color(&label.color)),
        ));
    }
    if !meta.is_empty() {
        lines.push(Line::from(meta));
    }
    lines.push(Line::from(Span::styled(
        detail.url.clone(),
        Style::default().fg(Color::DarkGray),
    )));
    lines.push(Line::from(""));

    if let Some(description) = &detail.description {
        if !description.is_empty() {
            for line in description.lines() {
                lines.push(Line::from(line.to_string()));
            }
            lines.push(Line::from(""));
        }
    }

    if !detail.comments.is_empty() {
        lines.push(Line::from(Span::styled(
            format!("Comments ({})", detail.comments.len()),
            Style::default().fg(Color::Yellow),
        )));
        lines.push(Line::from(""));
        for comment in &detail.comments {
            let author = comment.author.clone().unwrap_or_else(|| "unknown".into());
            lines.push(Line::from(Span::styled(
                author,
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            )));
            for line in comment.body.lines() {
                lines.push(Line::from(line.to_string()));
            }
            lines.push(Line::from(""));
        }
    }

    Text::from(lines)
}

fn priority_indicator(priority: u8) -> (&'static str, Color) {
    match priority {
        1 => ("!!!", Color::Red),
        2 => ("!! ", Color::LightRed),
        3 => ("!  ", Color::Yellow),
        4 => ("-  ", Color::Blue),
        _ => ("   ", Color::DarkGray),
    }
}

fn state_type_color(state_type: &str) -> Color {
    match state_type {
        "started" => Color::Yellow,
        "completed" => Color::Green,
        "canceled" => Color::Red,
        "triage" => Color::Magenta,
        "backlog" => Color::DarkGray,
        "unstarted" => Color::Gray,
        _ => Color::Gray,
    }
}

fn parse_hex_color(hex: &str) -> Color {
    let hex = hex.trim_start_matches('#');
    if hex.len() != 6 {
        return Color::Gray;
    }
    let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(128);
    let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(128);
    let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(128);
    Color::Rgb(r, g, b)
}
