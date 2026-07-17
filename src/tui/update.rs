use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::widgets::{ListState, ScrollbarState};

use super::app::{App, Pane, Screen, ViewKind, SCROLL_STEP};
use super::message::{Command, Message};

pub fn handle_key(app: &mut App, key: KeyEvent) -> Vec<Command> {
    match (key.modifiers, key.code) {
        (_, KeyCode::Char('q')) | (KeyModifiers::CONTROL, KeyCode::Char('c')) => {
            app.should_quit = true;
            vec![]
        }
        (_, KeyCode::Char('r')) => reload(app),
        (_, KeyCode::Tab) => {
            toggle_pane(app);
            vec![]
        }
        (_, KeyCode::Char('h')) | (_, KeyCode::Left) => {
            if app.screen == Screen::Home {
                app.pane = Pane::Sidebar;
            }
            vec![]
        }
        (_, KeyCode::Char('l')) | (_, KeyCode::Right) => {
            if app.screen == Screen::Home {
                app.pane = Pane::Main;
            }
            vec![]
        }
        (_, KeyCode::Char('j')) | (_, KeyCode::Down) => down(app),
        (_, KeyCode::Char('k')) | (_, KeyCode::Up) => up(app),
        (_, KeyCode::Enter) => enter(app),
        (_, KeyCode::Esc) | (_, KeyCode::Backspace) => {
            back(app);
            vec![]
        }
        (_, KeyCode::Char(c @ '1'..='9')) => jump_view(app, c as usize - '1' as usize),
        _ => vec![],
    }
}

pub fn apply(app: &mut App, msg: Message) -> Vec<Command> {
    match msg {
        Message::SessionLoaded(session) => app.session = Some(session),
        Message::IssuesLoaded { view, issues } => {
            if view == app.active_view_index() {
                app.issues = issues;
                app.loading = false;
                app.status = None;
                clamp_selection(&mut app.list_state, app.issues.len());
            }
        }
        Message::InboxLoaded { view, items } => {
            if view == app.active_view_index() {
                app.notifications = items;
                app.loading = false;
                app.status = None;
                clamp_selection(&mut app.list_state, app.notifications.len());
            }
        }
        Message::DetailLoaded(detail) => {
            app.detail = Some(*detail);
            app.loading = false;
            app.status = None;
            app.scroll_position = 0;
            app.scroll_state = ScrollbarState::default();
        }
        Message::Failed(error) => {
            app.loading = false;
            app.status = Some(error);
        }
    }
    vec![]
}

pub fn initial_commands(app: &App) -> Vec<Command> {
    vec![Command::LoadSession, load_active_command(app)]
}

fn load_active_command(app: &App) -> Command {
    let view = app.active_view_index();
    match &app.active_view().kind {
        ViewKind::Issues(filter) => Command::LoadIssues {
            view,
            filter: filter.clone(),
        },
        ViewKind::Inbox => Command::LoadInbox { view },
    }
}

fn reload(app: &mut App) -> Vec<Command> {
    if app.screen == Screen::Detail {
        if let Some(detail) = &app.detail {
            app.loading = true;
            return vec![Command::LoadDetail(detail.id.clone())];
        }
    }
    app.loading = true;
    vec![load_active_command(app)]
}

fn toggle_pane(app: &mut App) {
    if app.screen != Screen::Home {
        return;
    }
    app.pane = match app.pane {
        Pane::Sidebar => Pane::Main,
        Pane::Main => Pane::Sidebar,
    };
}

fn down(app: &mut App) -> Vec<Command> {
    match app.screen {
        Screen::Detail => {
            app.scroll_position = app.scroll_position.saturating_add(SCROLL_STEP);
            vec![]
        }
        Screen::Home => match app.pane {
            Pane::Sidebar => move_view(app, true),
            Pane::Main => {
                let len = app.main_len();
                navigate_list(&mut app.list_state, len, true);
                vec![]
            }
        },
    }
}

fn up(app: &mut App) -> Vec<Command> {
    match app.screen {
        Screen::Detail => {
            app.scroll_position = app.scroll_position.saturating_sub(SCROLL_STEP);
            vec![]
        }
        Screen::Home => match app.pane {
            Pane::Sidebar => move_view(app, false),
            Pane::Main => {
                let len = app.main_len();
                navigate_list(&mut app.list_state, len, false);
                vec![]
            }
        },
    }
}

fn enter(app: &mut App) -> Vec<Command> {
    if app.screen != Screen::Home {
        return vec![];
    }
    if app.pane == Pane::Sidebar {
        app.pane = Pane::Main;
        return vec![];
    }
    let issue_id = match app.active_view().kind {
        ViewKind::Issues(_) => app.selected_issue().map(|i| i.id.clone()),
        ViewKind::Inbox => app
            .selected_notification()
            .and_then(|n| n.issue_id.clone()),
    };
    if let Some(id) = issue_id {
        app.screen = Screen::Detail;
        app.detail = None;
        app.loading = true;
        return vec![Command::LoadDetail(id)];
    }
    vec![]
}

fn back(app: &mut App) {
    if app.screen == Screen::Detail {
        app.screen = Screen::Home;
        app.detail = None;
    }
}

fn jump_view(app: &mut App, index: usize) -> Vec<Command> {
    if index >= app.views.len() || index == app.active_view_index() && app.screen == Screen::Home {
        if index < app.views.len() {
            app.screen = Screen::Home;
        }
        return vec![];
    }
    app.screen = Screen::Home;
    app.view_state.select(Some(index));
    app.list_state.select(Some(0));
    app.loading = true;
    vec![load_active_command(app)]
}

fn move_view(app: &mut App, forward: bool) -> Vec<Command> {
    let len = app.views.len();
    if len == 0 {
        return vec![];
    }
    let current = app.active_view_index();
    let next = if forward {
        (current + 1) % len
    } else {
        (current + len - 1) % len
    };
    app.view_state.select(Some(next));
    app.list_state.select(Some(0));
    app.loading = true;
    vec![load_active_command(app)]
}

fn navigate_list(state: &mut ListState, len: usize, forward: bool) {
    if len == 0 {
        return;
    }
    let index = match state.selected() {
        Some(current) if forward => (current + 1) % len,
        Some(current) => (current + len - 1) % len,
        None => 0,
    };
    state.select(Some(index));
}

fn clamp_selection(state: &mut ListState, len: usize) {
    if len == 0 {
        state.select(Some(0));
    } else if state.selected().unwrap_or(0) >= len {
        state.select(Some(len - 1));
    }
}
