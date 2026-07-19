use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::widgets::{ListState, ScrollbarState};

use super::action::{self, Action, ConfirmInput, EditorInput, InputInput, MenuInput, PickerInput};
use super::app::{App, FocusedIssue, RECENT_CAP, SCROLL_STEP};
use super::focus::{DetailView, Direction, Edge, Focus, LeftPanel, Nav, Reveal};
use super::issue_ref::parse_issue_ref;
use super::message::{Command, Message};
use super::overlay::{
    Confirm, Editor, Find, Input, InputPurpose, Menu, Overlay, Picker, PickerKind, Prefix,
    PrefixUnder, Search,
};
use super::status::Status;
use super::view::ViewKind;
use crate::api::{IssueSummary, IssueUpdate};

pub fn handle_key(app: &mut App, key: KeyEvent) -> Option<Command> {
    if super::action::is_quit(&key) {
        app.should_quit = true;
        return None;
    }

    app.status = None;

    match std::mem::take(&mut app.overlay) {
        Overlay::Confirm(confirm) => apply_confirm(app, confirm, ConfirmInput::from_key(key)),
        Overlay::Picker(picker) => apply_picker(app, picker, key),
        Overlay::Menu(menu) => apply_menu(app, menu, key),
        Overlay::Prefix(prefix) => apply_prefix(app, prefix, key),
        Overlay::Input(input) => apply_input(app, input, key),
        Overlay::Editor(editor) => apply_editor(app, editor, key),
        Overlay::Search(search) => apply_search(app, search, key),
        Overlay::Find(find) => apply_find(app, find, key),
        Overlay::None => resolve_browse(app, key).and_then(|action| apply_action(app, action)),
    }
}

fn resolve_browse(app: &App, key: KeyEvent) -> Option<Action> {
    context_keymap(app.focus)
        .and_then(|keymap| keymap.resolve(key))
        .or_else(|| Action::from_key(key))
}

fn context_keymap(focus: Focus) -> Option<&'static action::Keymap<Action>> {
    match focus {
        Focus::Detail(_, DetailView::Reading) => Some(&action::DETAIL_KEYS),
        Focus::Detail(_, DetailView::Comments) => Some(&action::COMMENTS_KEYS),
        Focus::MyWork | Focus::Recent | Focus::Stub(_) => None,
    }
}

fn open_prefix(under: Overlay) -> Overlay {
    let (keymap, under) = match under {
        Overlay::None => (&action::GO_GROUP, PrefixUnder::Browse),
        modal => (&action::GO_MODAL, PrefixUnder::Modal(Box::new(modal))),
    };

    Overlay::Prefix(Prefix {
        title: "Go to",
        keymap,
        under,
    })
}

fn apply_prefix(app: &mut App, prefix: Prefix, key: KeyEvent) -> Option<Command> {
    let action = prefix.keymap.resolve(key);

    app.overlay = match prefix.under {
        PrefixUnder::Browse => Overlay::None,
        PrefixUnder::Modal(modal) => *modal,
    };
    action.and_then(|action| apply_action(app, action))
}

fn apply_find(app: &mut App, mut find: Find, key: KeyEvent) -> Option<Command> {
    match key.code {
        KeyCode::Esc => {
            if let Some(state) = app.focused_list_mut() {
                state.select(find.origin);
            }
            None
        }
        KeyCode::Enter => {
            app.find_query = (!find.query.is_empty()).then(|| find.query.clone());
            None
        }
        KeyCode::Backspace => {
            find.query.pop();
            refresh_find(app, &find.query);
            app.overlay = Overlay::Find(find);
            None
        }
        KeyCode::Char(c)
            if !key
                .modifiers
                .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
        {
            find.query.push(c);
            refresh_find(app, &find.query);
            app.overlay = Overlay::Find(find);
            None
        }
        _ => {
            app.overlay = Overlay::Find(find);
            None
        }
    }
}

fn refresh_find(app: &mut App, query: &str) {
    if query.is_empty() {
        return;
    }

    if let Some(&first) = app.focused_matches(query).first() {
        if let Some(state) = app.focused_list_mut() {
            state.select(Some(first));
        }
    }
}

fn find_step(app: &mut App, direction: Direction) {
    let Some(query) = app.find_query.clone() else {
        app.status = Some(Status::NoActiveSearch);
        return;
    };

    let matches = app.focused_matches(&query);

    if matches.is_empty() {
        return;
    }

    let current = app.focused_selection().unwrap_or(0);

    let target = match direction {
        Direction::Next => matches
            .iter()
            .find(|&&i| i > current)
            .copied()
            .unwrap_or(matches[0]),
        Direction::Prev => matches
            .iter()
            .rev()
            .find(|&&i| i < current)
            .copied()
            .unwrap_or(matches[matches.len() - 1]),
    };

    if let Some(state) = app.focused_list_mut() {
        state.select(Some(target));
    }
}

fn open_find(app: &mut App) -> Option<Command> {
    match app.focus {
        Focus::Detail(..) => {
            app.status = Some(Status::FindInList);
            return None;
        }
        Focus::MyWork | Focus::Recent | Focus::Stub(_) => {}
    }

    if app.focused_list_len() == 0 {
        app.status = Some(Status::NothingToSearch);
        return None;
    }

    app.overlay = Overlay::Find(Find {
        query: String::new(),
        origin: app.focused_selection(),
    });

    None
}

fn apply_input(app: &mut App, mut input: Input, key: KeyEvent) -> Option<Command> {
    match InputInput::from_key(key) {
        Some(InputInput::Cancel) => {
            app.status = Some(Status::Cancelled);
            None
        }
        Some(InputInput::Submit) => submit_input(app, input),
        Some(InputInput::Erase) => {
            input.backspace();
            restore_input(app, input)
        }
        Some(InputInput::MoveLeft) => {
            input.move_left();
            restore_input(app, input)
        }
        Some(InputInput::MoveRight) => {
            input.move_right();
            restore_input(app, input)
        }
        None => match key.code {
            KeyCode::Char(c)
                if !key
                    .modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
            {
                input.insert(c);
                restore_input(app, input)
            }
            _ => restore_input(app, input),
        },
    }
}

fn restore_input(app: &mut App, input: Input) -> Option<Command> {
    app.overlay = Overlay::Input(input);
    None
}

fn submit_input(app: &mut App, input: Input) -> Option<Command> {
    let query = input.buffer.trim().to_string();

    if query.is_empty() {
        return None;
    }

    match input.purpose {
        InputPurpose::Jump => open_issue(app, parse_issue_ref(&query)),
        InputPurpose::Search => {
            app.overlay = Overlay::Search(Search::new(query.clone()));
            Some(Command::Search(query))
        }
    }
}

fn apply_editor(app: &mut App, mut editor: Editor, key: KeyEvent) -> Option<Command> {
    if action::is_editor_submit(key) {
        return submit_editor(app, editor);
    }

    match EditorInput::from_key(key) {
        Some(EditorInput::Cancel) => {
            app.status = Some(Status::Cancelled);
            None
        }
        Some(EditorInput::Newline) => {
            editor.newline();
            restore_editor(app, editor)
        }
        Some(EditorInput::Erase) => {
            editor.backspace();
            restore_editor(app, editor)
        }
        Some(EditorInput::MoveLeft) => {
            editor.move_left();
            restore_editor(app, editor)
        }
        Some(EditorInput::MoveRight) => {
            editor.move_right();
            restore_editor(app, editor)
        }
        Some(EditorInput::MoveUp) => {
            editor.move_up();
            restore_editor(app, editor)
        }
        Some(EditorInput::MoveDown) => {
            editor.move_down();
            restore_editor(app, editor)
        }
        None => match key.code {
            KeyCode::Char(c)
                if !key
                    .modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
            {
                editor.insert(c);
                restore_editor(app, editor)
            }
            _ => restore_editor(app, editor),
        },
    }
}

fn restore_editor(app: &mut App, editor: Editor) -> Option<Command> {
    app.overlay = Overlay::Editor(editor);
    None
}

fn submit_editor(app: &mut App, editor: Editor) -> Option<Command> {
    if editor.is_empty() {
        return None;
    }

    let issue_id = app.detail.as_ref().map(|detail| detail.id.clone())?;
    app.status = Some(Status::PostingComment);

    Some(Command::CreateComment {
        issue_id,
        body: editor.text(),
        parent_id: editor.parent_id,
    })
}

fn apply_search(app: &mut App, mut search: Search, key: KeyEvent) -> Option<Command> {
    match key.code {
        KeyCode::Char('g') => {
            app.overlay = open_prefix(Overlay::Search(search));
            return None;
        }
        KeyCode::Char('G') => {
            select_edge(&mut search.state, search.results.len(), Edge::Bottom);
            app.overlay = Overlay::Search(search);
            return None;
        }
        _ => {}
    }

    match PickerInput::from_key(key) {
        Some(PickerInput::Next) => {
            let len = search.results.len();
            navigate_list(&mut search.state, len, Direction::Next);
            app.overlay = Overlay::Search(search);

            None
        }
        Some(PickerInput::Prev) => {
            let len = search.results.len();
            navigate_list(&mut search.state, len, Direction::Prev);
            app.overlay = Overlay::Search(search);

            None
        }
        Some(PickerInput::Accept) => match search.selected().map(|issue| issue.id.clone()) {
            Some(id) => {
                let command = open_issue(app, id);
                app.search_return = Some(search);
                command
            }
            None => {
                app.overlay = Overlay::Search(search);
                None
            }
        },
        Some(PickerInput::Cancel) => {
            app.search_return = None;
            None
        }
        None => {
            app.overlay = Overlay::Search(search);
            None
        }
    }
}

fn apply_menu(app: &mut App, mut menu: Menu, key: KeyEvent) -> Option<Command> {
    match key.code {
        KeyCode::Char('g') => {
            app.overlay = open_prefix(Overlay::Menu(menu));
            return None;
        }
        KeyCode::Char('G') => {
            menu.jump_edge(Edge::Bottom);
            app.overlay = Overlay::Menu(menu);
            return None;
        }
        _ => {}
    }

    match MenuInput::from_key(key) {
        Some(MenuInput::Next) => {
            menu.move_selection(Direction::Next);
            app.overlay = Overlay::Menu(menu);
            None
        }
        Some(MenuInput::Prev) => {
            menu.move_selection(Direction::Prev);
            app.overlay = Overlay::Menu(menu);
            None
        }
        Some(MenuInput::SectionNext) => {
            menu.jump_section(Direction::Next);
            app.overlay = Overlay::Menu(menu);
            None
        }
        Some(MenuInput::SectionPrev) => {
            menu.jump_section(Direction::Prev);
            app.overlay = Overlay::Menu(menu);
            None
        }
        Some(MenuInput::Run) => match menu.selected_action() {
            Some(action) => apply_action(app, action),
            None => {
                app.overlay = Overlay::Menu(menu);
                None
            }
        },
        Some(MenuInput::Close) => None,
        None => {
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
            cycle_panel(app, Direction::Next);
            None
        }
        Action::PrevPanel => {
            cycle_panel(app, Direction::Prev);
            None
        }
        Action::Descend => descend(app),
        Action::Ascend => ascend(app),
        Action::SelectNext => {
            move_selection(app, Direction::Next);
            None
        }
        Action::SelectPrev => {
            move_selection(app, Direction::Prev);
            None
        }
        Action::NextView => cycle_view(app, Direction::Next),
        Action::PrevView => cycle_view(app, Direction::Prev),
        Action::JumpToPanel(index) => {
            jump_panel(app, index);
            None
        }
        Action::Reload => Some(reload(app)),
        Action::OpenInBrowser => open_in_browser(app),
        Action::YankUrl => yank_url(app),
        Action::SetStatus => open_status_picker(app),
        Action::Assign => open_assign_picker(app),
        Action::Comment => open_comment_input(app),
        Action::EnterComments => enter_comments(app),
        Action::Reply => open_reply_editor(app),
        Action::ClearRecent => {
            clear_recent(app);
            None
        }
        Action::GoPrefix => {
            app.overlay = open_prefix(Overlay::None);
            None
        }
        Action::GoToIssue => {
            app.overlay = Overlay::Input(Input::new(InputPurpose::Jump, "Issue id or URL"));
            None
        }
        Action::Search => {
            app.search_return = None;
            app.overlay = Overlay::Input(Input::new(InputPurpose::Search, "Search issues"));
            None
        }
        Action::Find => open_find(app),
        Action::FindNext => {
            find_step(app, Direction::Next);
            None
        }
        Action::FindPrev => {
            find_step(app, Direction::Prev);
            None
        }
        Action::HalfPageDown => {
            scroll_half(app, Direction::Next);
            None
        }
        Action::HalfPageUp => {
            scroll_half(app, Direction::Prev);
            None
        }
        Action::HistoryBack => history_step(app, Direction::Prev),
        Action::HistoryForward => history_step(app, Direction::Next),
        Action::JumpToTop => {
            jump_edge(app, Edge::Top);
            None
        }
        Action::JumpToBottom => {
            jump_edge(app, Edge::Bottom);
            None
        }
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
        Message::DetailLoaded { detail, reveal } => {
            app.detail = Some(*detail);
            app.detail_loading = false;
            app.status = None;

            app.scroll_position = match reveal {
                Reveal::Top => 0,
                Reveal::Bottom => usize::MAX,
            };

            app.scroll_state = ScrollbarState::default();

            let Some(detail) = &app.detail else {
                return None;
            };

            app.record_recent(IssueSummary::from_detail(detail));

            Some(Command::SaveRecent(app.recently_viewed.clone()))
        }
        Message::RecentLoaded(mut issues) => {
            if app.recently_viewed.is_empty() {
                issues.truncate(RECENT_CAP);
                app.recently_viewed = issues;
                app.recent_state.select(Some(0));
            }

            None
        }
        Message::RecentCleared => {
            app.recently_viewed.clear();
            app.recent_state.select(Some(0));
            app.status = Some(Status::RecentCleared);

            if app.focus.left() == LeftPanel::Recent {
                app.focus = Focus::MyWork;
            }

            None
        }
        Message::SearchResults(results) => {
            if let Overlay::Search(search) = &mut app.overlay {
                search.results = results;
                search.loading = false;
                search.state.select(Some(0));
            }

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
            app.status = Some(Status::IssueUpdated);
            let reload = load_active_command(app);
            match app.focus {
                Focus::Detail(..) => {
                    app.detail_loading = true;
                    Some(Command::Batch(vec![
                        reload,
                        Command::LoadDetail {
                            id,
                            reveal: Reveal::Top,
                        },
                    ]))
                }
                Focus::MyWork | Focus::Recent | Focus::Stub(_) => Some(reload),
            }
        }
        Message::CommentPosted { id } => {
            app.status = Some(Status::CommentPosted);
            app.detail_loading = true;

            if let Focus::Detail(panel, DetailView::Comments) = app.focus {
                app.focus = Focus::Detail(panel, DetailView::Reading);
            }

            Some(Command::LoadDetail {
                id,
                reveal: Reveal::Bottom,
            })
        }
        Message::Failed(error) => {
            app.loading = false;
            app.detail_loading = false;
            app.status = Some(Status::Error(error));
            None
        }
    }
}

pub fn initial_commands(app: &App) -> Vec<Command> {
    vec![
        Command::LoadSession,
        Command::LoadRecent,
        load_active_command(app),
    ]
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
    match app.focus {
        Focus::Detail(..) => match &app.detail {
            Some(detail) => {
                let id = detail.id.clone();
                app.detail_loading = true;
                Command::LoadDetail {
                    id,
                    reveal: Reveal::Top,
                }
            }
            None => reload_list(app),
        },
        Focus::MyWork | Focus::Recent | Focus::Stub(_) => reload_list(app),
    }
}

fn reload_list(app: &mut App) -> Command {
    app.loading = true;
    load_active_command(app)
}

fn cycle_panel(app: &mut App, direction: Direction) {
    let panels = app.panels();
    let count = panels.len();
    let current = panels
        .iter()
        .position(|&p| p == app.focus.left())
        .unwrap_or(0);

    let next = direction.wrap(current, count + 1);

    app.focus = if next == count {
        Focus::Detail(app.focus.left(), DetailView::Reading)
    } else {
        panels[next].focus()
    };
}

fn jump_panel(app: &mut App, index: usize) {
    if index < app.panel_count() {
        app.focus = app.panel_at(index).focus();
    }
}

fn ascend(app: &mut App) -> Option<Command> {
    if app.find_query.take().is_some() {
        return None;
    }

    match app.focus {
        Focus::Detail(panel, DetailView::Comments) => {
            app.focus = Focus::Detail(panel, DetailView::Reading);
        }
        Focus::Detail(_, DetailView::Reading) => match app.search_return.take() {
            Some(search) => app.overlay = Overlay::Search(search),
            None => app.focus = Focus::MyWork,
        },
        Focus::MyWork | Focus::Recent | Focus::Stub(_) => app.focus = Focus::MyWork,
    }

    None
}

fn enter_comments(app: &mut App) -> Option<Command> {
    if !app.has_comments() {
        app.status = Some(Status::NoComments);
        return None;
    }

    app.focus = Focus::Detail(app.focus.left(), DetailView::Comments);
    app.comment_state.select(Some(0));

    None
}

fn open_reply_editor(app: &mut App) -> Option<Command> {
    let detail = app.detail.as_ref()?;
    let selected = app.comment_state.selected()?;
    let comment = detail.threaded_comments().get(selected)?.comment;

    app.overlay = Overlay::Editor(Editor::new("Reply", Some(comment.reply_parent())));
    None
}

fn descend(app: &mut App) -> Option<Command> {
    let id = match app.focus {
        Focus::MyWork => match app.active_view().kind {
            ViewKind::Issues(_) => app.selected_issue().map(|i| i.id.clone()),
            ViewKind::Inbox => app.selected_notification().and_then(|n| n.issue_id.clone()),
        },
        Focus::Recent => app.selected_recent().map(|i| i.id.clone()),
        Focus::Stub(_) | Focus::Detail(..) => None,
    }?;

    open_issue(app, id)
}

fn open_issue(app: &mut App, id: String) -> Option<Command> {
    app.search_return = None;
    app.focus = Focus::Detail(app.focus.left(), DetailView::Reading);
    app.scroll_position = 0;
    app.scroll_state = ScrollbarState::default();

    if app.detail.as_ref().is_some_and(|d| d.id == id) {
        return None;
    }

    app.detail = None;
    app.detail_loading = true;
    Some(Command::LoadDetail {
        id,
        reveal: Reveal::Top,
    })
}

fn history_step(app: &mut App, direction: Direction) -> Option<Command> {
    if app.recently_viewed.is_empty() {
        return None;
    }

    let target = match (app.open_recent_pos(), direction) {
        (Some(pos), Direction::Next) => pos.checked_sub(1),
        (Some(pos), Direction::Prev) => Some(pos + 1),
        (None, _) => Some(0),
    };

    let issue = target.and_then(|index| app.recently_viewed.get(index))?;
    let id = issue.id.clone();
    open_issue(app, id)
}

fn move_selection(app: &mut App, direction: Direction) {
    match app.nav() {
        Nav::List { state, len, .. } => navigate_list(state, len, direction),
        Nav::Scroll { position, .. } => *position = scrolled(*position, SCROLL_STEP, direction),
    }
}

fn scroll_half(app: &mut App, direction: Direction) {
    match app.nav() {
        Nav::List {
            state,
            len,
            viewport,
        } => {
            if len == 0 {
                return;
            }

            let step = (viewport / 2).max(1);
            let current = state.selected().unwrap_or(0);
            let next = match direction {
                Direction::Next => (current + step).min(len - 1),
                Direction::Prev => current.saturating_sub(step),
            };

            state.select(Some(next));
        }
        Nav::Scroll { position, viewport } => {
            *position = scrolled(*position, (viewport / 2).max(1), direction)
        }
    }
}

fn jump_edge(app: &mut App, edge: Edge) {
    match &mut app.overlay {
        Overlay::Menu(menu) => {
            menu.jump_edge(edge);
            return;
        }
        Overlay::Picker(picker) => {
            select_edge(&mut picker.state, picker.items.len(), edge);
            return;
        }
        Overlay::Search(search) => {
            select_edge(&mut search.state, search.results.len(), edge);
            return;
        }
        _ => {}
    }

    match app.nav() {
        Nav::List { state, len, .. } => select_edge(state, len, edge),
        Nav::Scroll { position, .. } => {
            *position = match edge {
                Edge::Bottom => usize::MAX,
                Edge::Top => 0,
            }
        }
    }
}

fn scrolled(position: usize, step: usize, direction: Direction) -> usize {
    match direction {
        Direction::Next => position.saturating_add(step),
        Direction::Prev => position.saturating_sub(step),
    }
}

fn select_edge(state: &mut ListState, len: usize, edge: Edge) {
    if len == 0 {
        return;
    }

    state.select(Some(match edge {
        Edge::Bottom => len - 1,
        Edge::Top => 0,
    }));
}

fn cycle_view(app: &mut App, direction: Direction) -> Option<Command> {
    match app.focus {
        Focus::MyWork => {
            let next = direction.wrap(app.active_view_index(), app.views.len());
            Some(select_view(app, next))
        }
        Focus::Recent | Focus::Stub(_) | Focus::Detail(..) => None,
    }
}

fn select_view(app: &mut App, index: usize) -> Command {
    app.focus = Focus::MyWork;
    app.view_state.select(Some(index));
    app.list_state.select(Some(0));
    app.detail = None;
    app.find_query = None;
    app.loading = true;

    load_active_command(app)
}

fn clear_recent(app: &mut App) {
    match app.focus {
        Focus::Recent if !app.recently_viewed.is_empty() => {
            app.overlay = Overlay::Confirm(Confirm {
                message: "Clear recently viewed?".into(),
                command: Command::ClearRecent,
            });
        }
        Focus::MyWork | Focus::Recent | Focus::Stub(_) | Focus::Detail(..) => {}
    }
}

fn open_status_picker(app: &mut App) -> Option<Command> {
    let target = require(app, app.action_target(), Status::NeedOpenIssue)?;
    Some(open_picker(app, PickerKind::Status, target))
}

fn open_assign_picker(app: &mut App) -> Option<Command> {
    let target = require(app, app.action_target(), Status::NeedOpenIssue)?;
    Some(open_picker(app, PickerKind::Assign, target))
}

fn open_comment_input(app: &mut App) -> Option<Command> {
    require(app, app.action_target(), Status::NeedOpenIssue)?;
    app.overlay = Overlay::Editor(Editor::new("Comment", None));
    None
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
    let target = require(app, app.open_target(), Status::NeedHighlightedIssue)?;
    Some(Command::OpenUrl(target.url))
}

fn yank_url(app: &mut App) -> Option<Command> {
    let target = require(app, app.open_target(), Status::NeedHighlightedIssue)?;
    app.status = Some(Status::CopiedUrl);
    Some(Command::CopyToClipboard(target.url))
}

fn require<T>(app: &mut App, target: Option<T>, status: Status) -> Option<T> {
    match target {
        some @ Some(_) => some,
        None => {
            app.status = Some(status);
            None
        }
    }
}

fn apply_confirm(app: &mut App, confirm: Confirm, input: Option<ConfirmInput>) -> Option<Command> {
    match input {
        Some(ConfirmInput::Accept) => {
            app.status = Some(Status::Applying);
            Some(confirm.command)
        }
        Some(ConfirmInput::Reject) => {
            app.status = Some(Status::Cancelled);
            None
        }
        None => {
            app.overlay = Overlay::Confirm(confirm);
            None
        }
    }
}

fn apply_picker(app: &mut App, mut picker: Picker, key: KeyEvent) -> Option<Command> {
    match key.code {
        KeyCode::Char('g') => {
            app.overlay = open_prefix(Overlay::Picker(picker));
            return None;
        }
        KeyCode::Char('G') => {
            select_edge(&mut picker.state, picker.items.len(), Edge::Bottom);
            app.overlay = Overlay::Picker(picker);
            return None;
        }
        _ => {}
    }

    match PickerInput::from_key(key) {
        Some(PickerInput::Next) => {
            let len = picker.items.len();
            navigate_list(&mut picker.state, len, Direction::Next);
            app.overlay = Overlay::Picker(picker);
            None
        }
        Some(PickerInput::Prev) => {
            let len = picker.items.len();
            navigate_list(&mut picker.state, len, Direction::Prev);
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

fn navigate_list(state: &mut ListState, len: usize, direction: Direction) {
    if len == 0 {
        return;
    }
    let index = match state.selected() {
        Some(current) => direction.wrap(current, len),
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
