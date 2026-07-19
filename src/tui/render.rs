use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::ListItem,
    Frame,
};

use super::action;
use super::app::{App, Confirm, Focus, Menu, MenuRow, Overlay, Picker, View, ViewKind};
use super::components::{ScrollableText, StyledList};
use super::layout;
use crate::api::{IssueDetail, IssueSummary, NotificationItem};

const SPINNER: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
const LEFT_PCT: u16 = 38;
const COLLAPSED_PEEK: usize = 2;
const MENU_HINT: &str = "j/k move   tab section   enter run   esc close";

pub fn render(app: &mut App, frame: &mut Frame) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(frame.area());

    let body = chunks[0];
    let footer = chunks[1];

    let [left, right] = layout::split_horizontal(body, LEFT_PCT);
    render_left(app, frame, left);
    render_right(app, frame, right);

    render_footer(app, frame, footer);

    let spinner = SPINNER[app.spinner_frame % SPINNER.len()];
    match &mut app.overlay {
        Overlay::Picker(picker) => render_picker(picker, spinner, frame),
        Overlay::Confirm(confirm) => render_confirm(confirm, frame),
        Overlay::Menu(menu) => render_menu(menu, frame),
        Overlay::None => {}
    }
}

fn render_menu(menu: &mut Menu, frame: &mut Frame) {
    use ratatui::widgets::Clear;

    let area = layout::centered_rect(frame.area(), 44, 70);
    let items = menu_items(menu);

    frame.render_widget(Clear, area);
    StyledList::new("Keybindings")
        .items(items)
        .focused(true)
        .state(&mut menu.state)
        .render(frame, area);
}

fn menu_items(menu: &Menu) -> Vec<ListItem<'static>> {
    let key_width = menu
        .rows
        .iter()
        .filter_map(|row| match row {
            MenuRow::Item { keys, .. } => Some(keys.chars().count()),
            MenuRow::Header(_) => None,
        })
        .max()
        .unwrap_or(0);

    menu.rows
        .iter()
        .map(|row| match row {
            MenuRow::Header(title) => ListItem::new(Line::from(Span::styled(
                *title,
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ))),
            MenuRow::Item { keys, label, .. } => ListItem::new(Line::from(vec![
                Span::styled(
                    format!("{keys:>key_width$}"),
                    Style::default().fg(Color::Yellow),
                ),
                Span::raw("  "),
                Span::styled(*label, Style::default().fg(Color::White)),
            ])),
        })
        .collect()
}

fn render_confirm(confirm: &Confirm, frame: &mut Frame) {
    use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};

    let area = layout::centered_rect_fixed(frame.area(), 50, 6);
    frame.render_widget(Clear, area);

    let block = Block::default()
        .title("Confirm")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));

    let text = Text::from(vec![
        Line::from(confirm.message.clone()),
        Line::from(""),
        Line::from(Span::styled(
            "[y] yes    [n] no",
            Style::default().fg(Color::DarkGray),
        )),
    ]);

    frame.render_widget(
        Paragraph::new(text).block(block).wrap(Wrap { trim: true }),
        area,
    );
}

fn render_picker(picker: &mut Picker, spinner: &str, frame: &mut Frame) {
    use ratatui::widgets::Clear;

    let area = layout::centered_rect(frame.area(), 44, 55);
    frame.render_widget(Clear, area);

    let items: Vec<ListItem> = picker
        .items
        .iter()
        .map(|item| {
            let mut spans = vec![Span::styled(
                item.label.clone(),
                Style::default().fg(Color::White),
            )];
            if !item.hint.is_empty() {
                spans.push(Span::raw("  "));
                spans.push(Span::styled(
                    item.hint.clone(),
                    Style::default().fg(state_type_color(&item.hint)),
                ));
            }
            ListItem::new(Line::from(spans))
        })
        .collect();

    let title = format!("{}  {}", picker.verb(), picker.target_label);
    let mut list = StyledList::new(&title).items(items).focused(true);

    if picker.items.is_empty() {
        let placeholder = if picker.loading {
            format!("{spinner}  Loading…")
        } else {
            "Nothing to choose".to_string()
        };
        list = list.placeholder(&placeholder);
        list.render(frame, area);
    } else {
        let selected = picker.state.selected();
        let total = picker.items.len();
        list.state(&mut picker.state)
            .position(selected, total)
            .render(frame, area);
    }
}

fn render_left(app: &mut App, frame: &mut Frame, area: Rect) {
    let expanded = app.expanded_panel();

    let constraints: Vec<Constraint> = (0..app.panel_count())
        .map(|panel| {
            if panel == expanded {
                Constraint::Min(5)
            } else {
                Constraint::Length(app.panel_len(panel).min(COLLAPSED_PEEK) as u16 + 2)
            }
        })
        .collect();
    let rects = Layout::vertical(constraints).split(area);

    render_my_work(app, frame, rects[0]);

    for index in 0..app.stubs.len() {
        let focused = app.focus == Focus::Stub(index);
        let rect = rects[index + 1];
        let stub = &mut app.stubs[index];
        let items: Vec<ListItem> = stub
            .items
            .iter()
            .map(|item| ListItem::new(Line::from(item.clone())))
            .collect();
        let selected = stub.state.selected();
        let total = stub.items.len();
        StyledList::new(&stub.title)
            .items(items)
            .focused(focused)
            .state(&mut stub.state)
            .position(selected, total)
            .render(frame, rect);
    }
}

fn render_my_work(app: &mut App, frame: &mut Frame, area: Rect) {
    let spinner = SPINNER[app.spinner_frame % SPINNER.len()];
    let focused = app.focus == Focus::MyWork;
    let selected = app.list_state.selected();
    let max_title = area.width.saturating_sub(2) as usize;
    let title = view_tabs(
        &app.views,
        app.active_view_index(),
        app.loading,
        spinner,
        max_title,
    );
    let is_inbox = matches!(app.active_view().kind, ViewKind::Inbox);

    let (items, total, empty) = if is_inbox {
        (
            notification_items(&app.notifications),
            app.notifications.len(),
            "Inbox empty",
        )
    } else {
        (
            issue_items(&app.issues),
            app.issues.len(),
            "No issues in this view",
        )
    };

    let mut list = StyledList::new("My Work")
        .title_line(title)
        .items(items)
        .focused(focused)
        .state(&mut app.list_state)
        .position(selected, total);
    if total == 0 {
        list = list.placeholder(if app.loading { "Loading…" } else { empty });
    }
    list.render(frame, area);
}

fn view_tabs(
    views: &[View],
    active: usize,
    loading: bool,
    spinner: &str,
    max_width: usize,
) -> Line<'static> {
    let active_style = Style::default()
        .fg(Color::White)
        .add_modifier(Modifier::BOLD);

    let dim = Style::default().fg(Color::DarkGray);
    let separator = " · ";

    let spinner_width = if loading {
        2 + spinner.chars().count()
    } else {
        0
    };

    let strip_width: usize = views.iter().map(|v| v.name.chars().count()).sum::<usize>()
        + separator.chars().count() * views.len().saturating_sub(1);

    let mut spans: Vec<Span> = Vec::new();
    if strip_width + spinner_width <= max_width {
        for (index, view) in views.iter().enumerate() {
            if index > 0 {
                spans.push(Span::styled(separator.to_string(), dim));
            }
            let style = if index == active { active_style } else { dim };
            spans.push(Span::styled(view.name.clone(), style));
        }
    } else {
        let indicator = format!(" {}/{}", active + 1, views.len());
        let name_budget = max_width.saturating_sub(indicator.chars().count() + spinner_width);

        spans.push(Span::styled(
            fit(&views[active].name, name_budget),
            active_style,
        ));
        spans.push(Span::styled(indicator, dim));
    }

    if loading {
        spans.push(Span::styled(
            format!("  {spinner}"),
            Style::default().fg(Color::Yellow),
        ));
    }
    Line::from(spans)
}

fn render_right(app: &mut App, frame: &mut Frame, area: Rect) {
    if let Focus::Stub(index) = app.focus {
        let stub = &app.stubs[index];
        let selected = stub
            .state
            .selected()
            .and_then(|i| stub.items.get(i))
            .cloned()
            .unwrap_or_default();
        let text = Text::from(vec![
            Line::from(Span::styled(
                "Not implemented yet",
                Style::default().fg(Color::DarkGray),
            )),
            Line::from(""),
            Line::from(selected),
        ]);
        render_text_panel(frame, area, &stub.title, text, Color::Yellow);
        return;
    }

    let focused = app.focus == Focus::Detail;
    let border = if focused { Color::Yellow } else { Color::Gray };
    let spinner = SPINNER[app.spinner_frame % SPINNER.len()];

    if app.detail.is_some() && (focused || app.detail_ready()) {
        let detail = app.detail.as_ref().unwrap();
        let content = detail_text(detail);
        let title = detail.identifier.clone();
        ScrollableText::new(content, app.scroll_position, &mut app.scroll_state)
            .title(&title)
            .border_color(border)
            .render(frame, area);
        return;
    }

    if focused && app.detail_loading {
        render_text_panel(
            frame,
            area,
            "Issue",
            Text::from(format!("{spinner}  Loading issue…")),
            border,
        );
        return;
    }

    let (title, text) = match app.active_view().kind {
        ViewKind::Issues(_) => match app.selected_issue() {
            Some(issue) => (issue.identifier.clone(), preview_text(issue)),
            None => ("Preview".to_string(), Text::from("No issue selected")),
        },
        ViewKind::Inbox => match app.selected_notification() {
            Some(notification) => (
                "Notification".to_string(),
                notification_preview_text(notification),
            ),
            None => ("Notification".to_string(), Text::from("Nothing selected")),
        },
    };
    render_text_panel(frame, area, &title, text, border);
}

fn render_text_panel(frame: &mut Frame, area: Rect, title: &str, text: Text, border: Color) {
    use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

    let block = Block::default()
        .title(title.to_string())
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border));
    frame.render_widget(
        Paragraph::new(text).block(block).wrap(Wrap { trim: false }),
        area,
    );
}

fn preview_text(issue: &IssueSummary) -> Text<'static> {
    let mut lines: Vec<Line> = vec![
        Line::from(vec![
            Span::styled(
                issue.identifier.clone(),
                Style::default().fg(Color::DarkGray),
            ),
            Span::raw("  "),
            Span::styled(
                issue.state.name.clone(),
                Style::default().fg(state_type_color(&issue.state.state_type)),
            ),
            Span::raw("  "),
            Span::styled(
                priority_label(issue.priority).to_string(),
                Style::default().fg(priority_indicator(issue.priority).1),
            ),
        ]),
        Line::from(Span::styled(
            issue.title.clone().unwrap_or_else(|| "Untitled".into()),
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )),
    ];

    let mut meta: Vec<Span> = Vec::new();
    if let Some(assignee) = &issue.assignee {
        meta.push(Span::styled(
            format!("@{}", assignee.display_name),
            Style::default().fg(Color::Blue),
        ));
    }
    for label in &issue.labels {
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
        issue.url.clone(),
        Style::default().fg(Color::DarkGray),
    )));
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "Press enter to load the description and comments",
        Style::default().fg(Color::DarkGray),
    )));

    Text::from(lines)
}

fn notification_preview_text(notification: &NotificationItem) -> Text<'static> {
    let mut lines = vec![Line::from(Span::styled(
        notification.title.clone(),
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD),
    ))];
    lines.push(Line::from(Span::styled(
        if notification.is_read { "read" } else { "unread" },
        Style::default().fg(Color::DarkGray),
    )));
    lines.push(Line::from(""));
    if notification.issue_id.is_some() {
        lines.push(Line::from(Span::styled(
            "Press enter to open the linked issue",
            Style::default().fg(Color::DarkGray),
        )));
    }
    Text::from(lines)
}

fn fit(text: &str, width: usize) -> String {
    if text.chars().count() <= width {
        return text.to_string();
    }

    if width == 0 {
        return String::new();
    }

    let mut output: String = text.chars().take(width - 1).collect();
    output.push('…');

    output
}

fn priority_label(priority: u8) -> &'static str {
    match priority {
        1 => "Urgent",
        2 => "High",
        3 => "Medium",
        4 => "Low",
        _ => "No priority",
    }
}

fn render_footer(app: &App, frame: &mut Frame, area: Rect) {
    use ratatui::layout::Alignment;
    use ratatui::widgets::Paragraph;

    let workspace = match &app.session {
        Some(session) => format!("{} · @{} ", session.org_name, session.user.display_name),
        None => "connecting… ".to_string(),
    };

    let [left, right] = layout::split_footer(area, workspace.chars().count() as u16 + 1);

    let (text, color) = match &app.status {
        Some(status) => (format!(" {status}"), Color::Red),
        None => (format!(" {}", footer_hint(app)), Color::DarkGray),
    };
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            fit(&text, left.width as usize),
            Style::default().fg(color),
        ))),
        left,
    );

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

fn footer_hint(app: &App) -> String {
    match &app.overlay {
        Overlay::Menu(_) => return MENU_HINT.to_string(),
        Overlay::Confirm(_) => return action::CONFIRM.hint_bar(action::CONFIRM_HINTS),
        Overlay::Picker(_) => return action::PICKER.hint_bar(action::PICKER_HINTS),
        Overlay::None => {}
    }

    let specs = match app.focus {
        Focus::MyWork => action::MY_WORK_HINTS,
        Focus::Stub(_) => action::STUB_HINTS,
        Focus::Detail => action::DETAIL_HINTS,
    };
    action::BROWSE.hint_bar(specs)
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
                Span::styled(
                    issue.identifier.clone(),
                    Style::default().fg(Color::DarkGray),
                ),
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
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD)
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
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD),
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
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
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
