use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::ListItem,
    Frame,
};

use super::action;
use super::app::App;
use super::components::{ScrollableText, StyledList};
use super::focus::{Focus, LeftPanel};
use super::layout;
use super::overlay::{Confirm, Input, Menu, MenuRow, Overlay, Picker, PrefixUnder, Search};
use super::spinner::Spinner;
use super::view::{View, ViewKind};
use crate::api::{IssueDetail, IssueSummary, NotificationItem, Priority, Rgb, StateType};

const LEFT_PCT: u16 = 38;
const COLLAPSED_PEEK: usize = 2;

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

    render_overlay(&mut app.overlay, app.spinner, frame);
}

fn render_overlay(overlay: &mut Overlay, spinner: Spinner, frame: &mut Frame) {
    match overlay {
        Overlay::Picker(picker) => render_picker(picker, spinner, frame),
        Overlay::Confirm(confirm) => render_confirm(confirm, frame),
        Overlay::Menu(menu) => render_menu(menu, frame),
        Overlay::Input(input) => render_input(input, frame),
        Overlay::Search(search) => render_search(search, spinner, frame),
        Overlay::Prefix(prefix) => match &mut prefix.under {
            PrefixUnder::Modal(modal) => render_overlay(modal, spinner, frame),
            PrefixUnder::Browse => render_prefix(prefix.keymap, prefix.title, frame),
        },
        Overlay::Find(_) | Overlay::None => {}
    }
}

fn render_prefix(keymap: &action::Keymap<action::Action>, title: &str, frame: &mut Frame) {
    use ratatui::widgets::Clear;

    let items: Vec<ListItem> = keymap
        .bindings
        .iter()
        .filter_map(|binding| {
            keymap.describe(binding.action).map(|(keys, label)| {
                ListItem::new(Line::from(vec![
                    Span::styled(keys, Style::default().fg(Color::Yellow)),
                    Span::raw("  "),
                    Span::styled(label, Style::default().fg(Color::White)),
                ]))
            })
        })
        .collect();

    let area = layout::centered_rect_fixed(frame.area(), 30, items.len() as u16 + 2);
    frame.render_widget(Clear, area);

    StyledList::new(title)
        .items(items)
        .focused(true)
        .render(frame, area);
}

fn render_input(input: &Input, frame: &mut Frame) {
    use ratatui::widgets::{Block, Borders, Clear, Paragraph};

    let area = layout::centered_rect_fixed(frame.area(), 60, 3);
    frame.render_widget(Clear, area);

    let block = Block::default()
        .title(input.prompt)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));

    frame.render_widget(Paragraph::new(input_line(input)).block(block), area);
}

fn input_line(input: &Input) -> Line<'static> {
    let chars: Vec<char> = input.buffer.chars().collect();
    let cursor = input.cursor.min(chars.len());
    let before: String = chars[..cursor].iter().collect();
    let under = chars.get(cursor).copied().unwrap_or(' ').to_string();
    let after: String = chars
        .get(cursor + 1..)
        .map(|rest| rest.iter().collect())
        .unwrap_or_default();

    Line::from(vec![
        Span::raw(format!(" {before}")),
        Span::styled(under, Style::default().add_modifier(Modifier::REVERSED)),
        Span::raw(after),
    ])
}

fn render_search(search: &mut Search, spinner: Spinner, frame: &mut Frame) {
    use ratatui::widgets::Clear;

    let area = layout::centered_rect(frame.area(), 60, 60);
    frame.render_widget(Clear, area);

    let items: Vec<ListItem> = search
        .results
        .iter()
        .map(|issue| {
            ListItem::new(Line::from(vec![
                Span::styled(
                    issue.identifier.clone(),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::raw("  "),
                Span::styled(
                    issue.title.clone().unwrap_or_else(|| "Untitled".into()),
                    Style::default().fg(Color::White),
                ),
            ]))
        })
        .collect();

    let title = format!("Search  {}", search.query);
    let mut list = StyledList::new(&title).items(items).focused(true);

    if search.results.is_empty() {
        let placeholder = if search.loading {
            format!("{spinner}  Searching…")
        } else {
            "No matches".to_string()
        };

        list = list.placeholder(&placeholder);

        list.render(frame, area);
    } else {
        let selected = search.state.selected();
        let total = search.results.len();

        list.state(&mut search.state)
            .position(selected, total)
            .render(frame, area);
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

fn render_picker(picker: &mut Picker, spinner: Spinner, frame: &mut Frame) {
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
                    Style::default().fg(Color::DarkGray),
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
    let panels = app.panels();

    let expanded = app.focus.left();
    let constraints: Vec<Constraint> = panels
        .iter()
        .map(|&panel| {
            if panel == expanded {
                Constraint::Min(5)
            } else {
                let rows = app.panel_len(panel.focus()).clamp(1, COLLAPSED_PEEK);
                Constraint::Length(rows as u16 + 2)
            }
        })
        .collect();

    let rects = Layout::vertical(constraints).split(area);

    for (rect, panel) in rects.iter().zip(panels) {
        if panel.focus() == app.focus {
            app.viewport = (rect.height as usize).saturating_sub(2);
        }

        match panel {
            LeftPanel::MyWork => render_my_work(app, frame, *rect),
            LeftPanel::Recent => render_recent(app, frame, *rect),
            LeftPanel::Stub(index) => render_stub(app, frame, *rect, index),
        }
    }
}

fn render_stub(app: &mut App, frame: &mut Frame, rect: Rect, index: usize) {
    let focused = app.focus == Focus::Stub(index);
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

fn render_recent(app: &mut App, frame: &mut Frame, area: Rect) {
    let focused = app.focus == Focus::Recent;
    let selected = app.recent_state.selected();
    let total = app.recently_viewed.len();
    let items = issue_items(&app.recently_viewed);

    let mut list = StyledList::new("Recently viewed")
        .items(items)
        .focused(focused)
        .state(&mut app.recent_state)
        .position(selected, total);

    if total == 0 {
        list = list.placeholder("Issues you open land here");
    }

    list.render(frame, area);
}

fn render_my_work(app: &mut App, frame: &mut Frame, area: Rect) {
    let focused = app.focus == Focus::MyWork;
    let selected = app.list_state.selected();
    let max_title = area.width.saturating_sub(2) as usize;
    let title = view_tabs(
        &app.views,
        app.active_view_index(),
        app.loading,
        app.spinner,
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
    spinner: Spinner,
    max_width: usize,
) -> Line<'static> {
    let active_style = Style::default()
        .fg(Color::White)
        .add_modifier(Modifier::BOLD);

    let dim = Style::default().fg(Color::DarkGray);
    let separator = " · ";

    let spinner_width = if loading {
        2 + spinner.glyph().chars().count()
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
    match app.focus {
        Focus::Stub(index) => render_stub_placeholder(app, frame, area, index),
        Focus::Recent => {
            let (title, text) = match app.selected_recent() {
                Some(issue) => (issue.identifier.clone(), preview_text(issue)),
                None => (
                    "Recently viewed".to_string(),
                    Text::from("Open an issue and it shows up here"),
                ),
            };
            render_text_panel(frame, area, &title, text, Color::Gray);
        }
        Focus::MyWork => {
            if app.detail_ready() && render_detail_if_loaded(app, frame, area, Color::Gray) {
                return;
            }
            render_work_preview(app, frame, area, Color::Gray);
        }
        Focus::Detail(_) => {
            app.viewport = (area.height as usize).saturating_sub(2);
            if render_detail_if_loaded(app, frame, area, Color::Yellow) {
                return;
            }
            if app.detail_loading {
                render_text_panel(
                    frame,
                    area,
                    "Issue",
                    Text::from(format!("{}  Loading issue…", app.spinner)),
                    Color::Yellow,
                );
                return;
            }
            render_work_preview(app, frame, area, Color::Yellow);
        }
    }
}

fn render_stub_placeholder(app: &mut App, frame: &mut Frame, area: Rect, index: usize) {
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
}

fn render_detail_if_loaded(app: &mut App, frame: &mut Frame, area: Rect, border: Color) -> bool {
    let Some(detail) = app.detail.as_ref() else {
        return false;
    };
    let content = detail_text(detail);
    let title = detail.identifier.clone();
    let clamped = {
        let mut scrollable =
            ScrollableText::new(content, app.scroll_position, &mut app.scroll_state)
                .title(&title)
                .border_color(border);
        scrollable.render(frame, area);
        scrollable.clamped_scroll_position()
    };
    app.scroll_position = clamped;
    true
}

fn render_work_preview(app: &mut App, frame: &mut Frame, area: Rect, border: Color) {
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
                Style::default().fg(state_type_color(issue.state.state_type)),
            ),
            Span::raw("  "),
            Span::styled(
                issue.priority.label().to_string(),
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
            Style::default().fg(Color::Black).bg(rgb_color(label.color)),
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
        if notification.is_read {
            "read"
        } else {
            "unread"
        },
        Style::default().fg(Color::DarkGray),
    )));

    lines.push(Line::from(""));

    lines.extend(notification.issue_id.as_ref().map(|_| {
        Line::from(Span::styled(
            "Press enter to open the linked issue",
            Style::default().fg(Color::DarkGray),
        ))
    }));

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

fn render_footer(app: &App, frame: &mut Frame, area: Rect) {
    use ratatui::layout::Alignment;
    use ratatui::widgets::Paragraph;

    if let Some(bar) = search_bar(app) {
        frame.render_widget(Paragraph::new(bar), area);
        return;
    }

    let workspace = match &app.session {
        Some(session) => format!("{} · @{} ", session.org_name, session.user.display_name),
        None => "connecting… ".to_string(),
    };

    let [left, right] = layout::split_footer(area, workspace.chars().count() as u16 + 1);

    frame.render_widget(Paragraph::new(footer_left(app, left.width as usize)), left);

    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            workspace,
            Style::default().fg(Color::Cyan),
        )))
        .alignment(Alignment::Right),
        right,
    );
}

fn search_bar(app: &App) -> Option<Line<'static>> {
    match &app.overlay {
        Overlay::Find(find) => Some(find_bar(app, &find.query, true)),
        Overlay::None => app.find_query.as_deref().map(|q| find_bar(app, q, false)),
        _ => None,
    }
}

fn find_bar(app: &App, query: &str, typing: bool) -> Line<'static> {
    let label = Span::styled(
        " Search ",
        Style::default()
            .fg(Color::Black)
            .bg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    );

    let matches = app.focused_matches(query);
    let total = matches.len();

    if typing {
        return Line::from(vec![
            label,
            Span::styled(format!(" {query}"), Style::default().fg(Color::White)),
            Span::styled(" ", Style::default().add_modifier(Modifier::REVERSED)),
            Span::styled(
                format!("   {total} matches   enter select   esc cancel"),
                Style::default().fg(Color::DarkGray),
            ),
        ]);
    }

    if total == 0 {
        return Line::from(vec![
            label,
            Span::styled(
                format!(" no matches for '{query}'"),
                Style::default().fg(Color::Red),
            ),
            Span::styled("   esc exit", Style::default().fg(Color::DarkGray)),
        ]);
    }

    let position = app
        .focused_selection()
        .and_then(|selected| matches.iter().position(|&index| index == selected))
        .map(|index| index + 1)
        .unwrap_or(0);

    Line::from(vec![
        label,
        Span::styled(
            format!(" '{query}'  {position} of {total}"),
            Style::default().fg(Color::White),
        ),
        Span::styled(
            "   n next   N prev   esc exit",
            Style::default().fg(Color::DarkGray),
        ),
    ])
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

fn footer_left(app: &App, width: usize) -> Line<'static> {
    if let Some(status) = &app.status {
        return Line::from(Span::styled(
            fit(&format!(" {status}"), width),
            Style::default().fg(Color::Red),
        ));
    }

    Line::from(Span::styled(
        fit(&format!(" {}", footer_hint(app)), width),
        Style::default().fg(Color::DarkGray),
    ))
}

fn footer_hint(app: &App) -> String {
    match &app.overlay {
        Overlay::Menu(_) => return action::MENU.hint_bar(action::MENU_HINTS),
        Overlay::Confirm(_) => return action::CONFIRM.hint_bar(action::CONFIRM_HINTS),
        Overlay::Picker(_) | Overlay::Search(_) => {
            return action::PICKER.hint_bar(action::PICKER_HINTS)
        }
        Overlay::Prefix(prefix) => return format!("{}   esc cancel", prefix.keymap.summary()),
        Overlay::Input(_) => return action::INPUT.hint_bar(action::INPUT_HINTS),
        Overlay::Find(_) | Overlay::None => {}
    }

    let specs = match app.focus {
        Focus::MyWork => action::MY_WORK_HINTS,
        Focus::Recent => action::RECENT_HINTS,
        Focus::Stub(_) => action::STUB_HINTS,
        Focus::Detail(_) => action::DETAIL_HINTS,
    };
    action::BROWSE.hint_bar(specs)
}

fn issue_items(issues: &[IssueSummary]) -> Vec<ListItem<'static>> {
    issues
        .iter()
        .map(|issue| {
            let (icon, priority_color) = priority_indicator(issue.priority);
            let state_color = state_type_color(issue.state.state_type);

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
                    Style::default().fg(Color::Black).bg(rgb_color(label.color)),
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
            Style::default().fg(state_type_color(detail.state.state_type)),
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
            Style::default().fg(Color::Black).bg(rgb_color(label.color)),
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

fn priority_indicator(priority: Priority) -> (&'static str, Color) {
    match priority {
        Priority::Urgent => ("!!!", Color::Red),
        Priority::High => ("!! ", Color::LightRed),
        Priority::Medium => ("!  ", Color::Yellow),
        Priority::Low => ("-  ", Color::Blue),
        Priority::None => ("   ", Color::DarkGray),
    }
}

fn state_type_color(state_type: StateType) -> Color {
    match state_type {
        StateType::Started => Color::Yellow,
        StateType::Completed => Color::Green,
        StateType::Canceled => Color::Red,
        StateType::Triage => Color::Magenta,
        StateType::Backlog => Color::DarkGray,
        StateType::Unstarted => Color::Gray,
    }
}

fn rgb_color(color: Rgb) -> Color {
    Color::Rgb(color.r, color.g, color.b)
}
