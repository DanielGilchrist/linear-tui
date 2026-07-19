use crossterm::event::{KeyCode, KeyEvent};
use ratatui::widgets::{ListState, ScrollbarState};

use super::action::{Action, ConfirmInput, PickerInput};
use super::app::{
    App, Confirm, Focus, FocusedIssue, Menu, Overlay, Picker, PickerKind, ViewKind, SCROLL_STEP,
};
use super::message::{Command, Message};
use crate::api::IssueUpdate;

pub fn handle_key(app: &mut App, key: KeyEvent) -> Option<Command> {
    if super::action::is_quit(&key) {
        app.should_quit = true;
        return None;
    }

    match std::mem::take(&mut app.overlay) {
        Overlay::Confirm(confirm) => apply_confirm(app, confirm, ConfirmInput::from_key(key)),
        Overlay::Picker(picker) => apply_picker(app, picker, PickerInput::from_key(key)),
        Overlay::Menu(menu) => apply_menu(app, menu, key),
        Overlay::None => Action::from_key(key).and_then(|action| apply_action(app, action)),
    }
}

fn apply_menu(app: &mut App, mut menu: Menu, key: KeyEvent) -> Option<Command> {
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('?') => None,
        KeyCode::Char('j') | KeyCode::Down => {
            menu.move_selection(true);
            app.overlay = Overlay::Menu(menu);
            None
        }
        KeyCode::Char('k') | KeyCode::Up => {
            menu.move_selection(false);
            app.overlay = Overlay::Menu(menu);
            None
        }
        KeyCode::Tab => {
            menu.jump_section(true);
            app.overlay = Overlay::Menu(menu);
            None
        }
        KeyCode::BackTab => {
            menu.jump_section(false);
            app.overlay = Overlay::Menu(menu);
            None
        }
        KeyCode::Enter => match menu.selected_action() {
            Some(action) => apply_action(app, action),
            None => {
                app.overlay = Overlay::Menu(menu);
                None
            }
        },
        _ => {
            app.overlay = Overlay::Menu(menu);
            None
        }
    }
}

fn open_menu(app: &mut App) {
    app.overlay = Overlay::Menu(Menu::for_focus(app.focus));
}

fn apply_action(app: &mut App, action: Action) -> Option<Command> {
    match action {
        Action::Quit => {
            app.should_quit = true;
            None
        }
        Action::NextPanel => {
            cycle_panel(app, true);
            None
        }
        Action::PrevPanel => {
            cycle_panel(app, false);
            None
        }
        Action::Descend => descend(app),
        Action::Ascend => {
            ascend(app);
            None
        }
        Action::SelectNext => {
            move_selection(app, true);
            None
        }
        Action::SelectPrev => {
            move_selection(app, false);
            None
        }
        Action::NextView => cycle_view(app, true),
        Action::PrevView => cycle_view(app, false),
        Action::JumpToPanel(index) => {
            jump_panel(app, index);
            None
        }
        Action::Reload => Some(reload(app)),
        Action::OpenInBrowser => open_in_browser(app),
        Action::YankUrl => yank_url(app),
        Action::SetStatus => open_status_picker(app),
        Action::Assign => open_assign_picker(app),
        Action::Help => {
            open_menu(app);
            None
        }
    }
}

pub fn apply(app: &mut App, msg: Message) -> Option<Command> {
    match msg {
        Message::SessionLoaded(session) => {
            app.session = Some(session);
            None
        }
        Message::IssuesLoaded { view, issues } => {
            if view == app.active_view_index() {
                app.issues = issues;
                app.loading = false;
                app.status = None;
                clamp_selection(&mut app.list_state, app.issues.len());
            }
            None
        }
        Message::InboxLoaded { view, items } => {
            if view == app.active_view_index() {
                app.notifications = items;
                app.loading = false;
                app.status = None;
                clamp_selection(&mut app.list_state, app.notifications.len());
            }
            None
        }
        Message::DetailLoaded(detail) => {
            app.detail = Some(*detail);
            app.detail_loading = false;
            app.status = None;
            app.scroll_position = 0;
            app.scroll_state = ScrollbarState::default();
            None
        }
        Message::PickerLoaded(items) => {
            if let Overlay::Picker(picker) = &mut app.overlay {
                picker.items = items;
                picker.loading = false;
                picker.state.select(Some(0));
            }
            None
        }
        Message::IssueUpdated { id } => {
            app.loading = false;
            app.status = Some("Issue updated".into());
            let reload = load_active_command(app);
            if app.focus == Focus::Detail {
                app.detail_loading = true;
                Some(Command::Batch(vec![reload, Command::LoadDetail(id)]))
            } else {
                Some(reload)
            }
        }
        Message::Status(message) => {
            app.status = Some(message);
            None
        }
        Message::Failed(error) => {
            app.loading = false;
            app.detail_loading = false;
            app.status = Some(error);
            None
        }
    }
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

fn reload(app: &mut App) -> Command {
    if app.focus == Focus::Detail {
        if let Some(detail) = &app.detail {
            app.detail_loading = true;
            return Command::LoadDetail(detail.id.clone());
        }
    }
    app.loading = true;
    load_active_command(app)
}

fn cycle_panel(app: &mut App, forward: bool) {
    let order = focus_cycle(app);
    let current = order.iter().position(|&f| f == app.focus).unwrap_or(0);
    let len = order.len();
    let next = if forward {
        (current + 1) % len
    } else {
        (current + len - 1) % len
    };
    app.focus = order[next];
}

fn focus_cycle(app: &App) -> Vec<Focus> {
    let mut order: Vec<Focus> = (0..app.panel_count()).map(|i| app.focus_at(i)).collect();
    order.push(Focus::Detail);
    order
}

fn jump_panel(app: &mut App, index: usize) {
    if index < app.panel_count() {
        app.focus = app.focus_at(index);
    }
}

fn ascend(app: &mut App) {
    app.focus = Focus::MyWork;
}

fn descend(app: &mut App) -> Option<Command> {
    match app.focus {
        Focus::MyWork => open_detail(app),
        Focus::Stub(_) | Focus::Detail => None,
    }
}

fn open_detail(app: &mut App) -> Option<Command> {
    let id = match app.active_view().kind {
        ViewKind::Issues(_) => app.selected_issue().map(|i| i.id.clone()),
        ViewKind::Inbox => app.selected_notification().and_then(|n| n.issue_id.clone()),
    }?;

    app.focus = Focus::Detail;
    app.scroll_position = 0;
    app.scroll_state = ScrollbarState::default();

    if app.detail.as_ref().is_some_and(|d| d.id == id) {
        return None;
    }

    app.detail = None;
    app.detail_loading = true;
    Some(Command::LoadDetail(id))
}

fn move_selection(app: &mut App, forward: bool) {
    if app.focus == Focus::Detail {
        app.scroll_position = if forward {
            app.scroll_position.saturating_add(SCROLL_STEP)
        } else {
            app.scroll_position.saturating_sub(SCROLL_STEP)
        };
        return;
    }

    let len = app.focused_list_len();
    if let Some(state) = app.focused_list_mut() {
        navigate_list(state, len, forward);
    }
}

fn cycle_view(app: &mut App, forward: bool) -> Option<Command> {
    if app.focus != Focus::MyWork {
        return None;
    }
    let len = app.views.len();
    let current = app.active_view_index();
    let next = if forward {
        (current + 1) % len
    } else {
        (current + len - 1) % len
    };
    Some(select_view(app, next))
}

fn select_view(app: &mut App, index: usize) -> Command {
    app.focus = Focus::MyWork;
    app.view_state.select(Some(index));
    app.list_state.select(Some(0));
    app.detail = None;
    app.loading = true;
    load_active_command(app)
}

fn open_status_picker(app: &mut App) -> Option<Command> {
    let target = require(app, app.action_target(), "Open the issue first (enter)")?;
    Some(open_picker(app, PickerKind::Status, target))
}

fn open_assign_picker(app: &mut App) -> Option<Command> {
    let target = require(app, app.action_target(), "Open the issue first (enter)")?;
    Some(open_picker(app, PickerKind::Assign, target))
}

fn open_picker(app: &mut App, kind: PickerKind, target: FocusedIssue) -> Command {
    let team_id = target.team_id;
    app.overlay = Overlay::Picker(Picker {
        kind,
        target_issue: target.id,
        target_label: target.identifier,
        items: Vec::new(),
        state: ListState::default().with_selected(Some(0)),
        loading: true,
    });
    match kind {
        PickerKind::Status => Command::LoadStates { team_id },
        PickerKind::Assign => Command::LoadMembers { team_id },
    }
}

fn open_in_browser(app: &mut App) -> Option<Command> {
    let target = require(app, app.open_target(), "Highlight an issue first")?;
    Some(Command::OpenUrl(target.url))
}

fn yank_url(app: &mut App) -> Option<Command> {
    let target = require(app, app.open_target(), "Highlight an issue first")?;
    app.status = Some("Copied issue URL to clipboard".into());
    Some(Command::CopyToClipboard(target.url))
}

fn require<T>(app: &mut App, target: Option<T>, message: &str) -> Option<T> {
    if target.is_none() {
        app.status = Some(message.into());
    }
    target
}

fn apply_confirm(app: &mut App, confirm: Confirm, input: Option<ConfirmInput>) -> Option<Command> {
    match input {
        Some(ConfirmInput::Accept) => {
            app.loading = true;
            app.status = Some("Applying…".into());
            Some(confirm.command)
        }
        Some(ConfirmInput::Reject) => {
            app.status = Some("Cancelled".into());
            None
        }
        None => {
            app.overlay = Overlay::Confirm(confirm);
            None
        }
    }
}

fn apply_picker(app: &mut App, mut picker: Picker, input: Option<PickerInput>) -> Option<Command> {
    match input {
        Some(PickerInput::Next) => {
            let len = picker.items.len();
            navigate_list(&mut picker.state, len, true);
            app.overlay = Overlay::Picker(picker);
            None
        }
        Some(PickerInput::Prev) => {
            let len = picker.items.len();
            navigate_list(&mut picker.state, len, false);
            app.overlay = Overlay::Picker(picker);
            None
        }
        Some(PickerInput::Accept) => confirm_picker(app, picker),
        Some(PickerInput::Cancel) => None,
        None => {
            app.overlay = Overlay::Picker(picker);
            None
        }
    }
}

fn confirm_picker(app: &mut App, picker: Picker) -> Option<Command> {
    let Some(item) = picker.selected() else {
        app.overlay = Overlay::Picker(picker);
        return None;
    };

    let (update, message) = match picker.kind {
        PickerKind::Status => (
            IssueUpdate {
                state_id: Some(item.id.clone()),
                assignee_id: None,
            },
            format!("Set {} to \"{}\"?", picker.target_label, item.label),
        ),
        PickerKind::Assign => (
            IssueUpdate {
                state_id: None,
                assignee_id: Some(item.id.clone()),
            },
            format!("Assign {} to {}?", picker.target_label, item.label),
        ),
    };

    app.overlay = Overlay::Confirm(Confirm {
        message,
        command: Command::UpdateIssue {
            id: picker.target_issue.clone(),
            update,
        },
    });
    None
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
