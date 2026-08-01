use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::widgets::ListState;

use super::feed::{force_feed, load_more, reload};
use super::issue::{
    clear_recent, enter_comments, open_assign_picker, open_comment_input, open_delete_comment,
    open_edit_editor, open_in_browser, open_issue, open_labels, open_priority_picker,
    open_reactions, open_reply_editor, open_status_picker, toggle_reaction, yank_url,
};
use super::nav::{
    ascend, cycle_panel, cycle_view, cycle_view_group, cycle_view_sort, descend, history_step,
    jump_edge, jump_panel, move_selection, scroll_half,
};
use crate::api::Credential;
use crate::api::IssueRef;
use crate::api::IssueUpdate;
use crate::tui::action::{
    self, Action, ConfirmInput, EditorInput, InputInput, LabelsInput, MenuInput, PickerInput,
    ReactionInput, WorkspacesInput,
};
use crate::tui::app::App;
use crate::tui::feed::FeedKey;
use crate::tui::focus::{navigate_list, select_edge, DetailView, Direction, Edge, Focus, Origin};
use crate::tui::message::{ApiCommand, Commands, Effect, Effects, RuntimeCommand};
use crate::tui::overlay::{
    AssignOptions, Compose, Confirm, Editor, Find, Input, InputPurpose, LabelResults, Labels, Menu,
    ModalOverlay, Overlay, Picker, PickerAction, PickerKind, Prefix, PrefixUnder, Reactions,
    Search, SearchPhase, WorkspaceRow, Workspaces,
};
use crate::tui::status::Status;

pub(super) enum StatusEdit {
    Keep,
    Set(Status),
}

pub(super) enum Outcome {
    Set {
        overlay: Overlay,
        commands: Commands,
        status: StatusEdit,
    },
    Act {
        under: Overlay,
        action: Action,
    },
}

impl Outcome {
    fn set(overlay: Overlay) -> Self {
        Outcome::Set {
            overlay,
            commands: Commands::default(),
            status: StatusEdit::Keep,
        }
    }

    fn close() -> Self {
        Self::set(Overlay::None)
    }

    fn with(overlay: Overlay, commands: impl Into<Commands>) -> Self {
        Outcome::Set {
            overlay,
            commands: commands.into(),
            status: StatusEdit::Keep,
        }
    }

    fn dismiss(commands: impl Into<Commands>) -> Self {
        Self::with(Overlay::None, commands)
    }

    fn act(under: Overlay, action: Action) -> Self {
        Outcome::Act { under, action }
    }

    fn set_reporting(overlay: Overlay, status: Status) -> Self {
        Outcome::Set {
            overlay,
            commands: Commands::default(),
            status: StatusEdit::Set(status),
        }
    }

    fn dismiss_reporting(commands: impl Into<Commands>, status: Status) -> Self {
        Outcome::Set {
            overlay: Overlay::None,
            commands: commands.into(),
            status: StatusEdit::Set(status),
        }
    }
}

pub(super) fn apply_outcome(app: &mut App, outcome: Outcome) -> Commands {
    match outcome {
        Outcome::Set {
            overlay,
            commands,
            status,
        } => {
            app.set_overlay(overlay);

            if let StatusEdit::Set(status) = status {
                app.ui.status = Some(status);
            }

            commands
        }
        Outcome::Act { under, action } => {
            app.set_overlay(under);
            apply_action(app, action).into()
        }
    }
}

pub(super) struct Report {
    effects: Effects,
    status: Option<Status>,
}

impl Report {
    pub(super) fn status(status: Status) -> Self {
        Self {
            effects: Effects::default(),
            status: Some(status),
        }
    }

    pub(super) fn with_status(effects: Effects, status: Status) -> Self {
        Self {
            effects,
            status: Some(status),
        }
    }

    fn into_dismiss(self) -> Outcome {
        match self.status {
            Some(status) => Outcome::dismiss_reporting(self.effects, status),
            None => Outcome::dismiss(self.effects),
        }
    }

    fn write(self, app: &mut App) -> Effects {
        app.ui.status = self.status;

        self.effects
    }
}

impl From<Effects> for Report {
    fn from(effects: Effects) -> Self {
        Self {
            effects,
            status: None,
        }
    }
}

pub(super) fn resolve_browse(app: &App, key: KeyEvent) -> Option<Action> {
    if is_plain(key) {
        if let Some(action) = context_keymap(app.focus()).and_then(|keymap| keymap.resolve(key)) {
            return Some(action);
        }
    }

    Action::from_key(key)
}

pub(super) fn context_keymap(focus: &Focus) -> Option<&'static action::Keymap<Action>> {
    match focus {
        Focus::Detail(detail) => match detail.view {
            DetailView::Reading { .. } => Some(&action::DETAIL_KEYS),
            DetailView::Comments { .. } => Some(&action::COMMENTS_KEYS),
        },
        Focus::View(_) => Some(&action::VIEW_KEYS),
        Focus::MyWork | Focus::Recent | Focus::SavedViews | Focus::Teams => None,
    }
}

pub(super) fn open_prefix(under: Overlay) -> Overlay {
    let (keymap, under) = match ModalOverlay::try_from_overlay(under) {
        Ok(modal) => (&action::GO_MODAL, PrefixUnder::Modal(modal)),
        Err(_) => (&action::GO_GROUP, PrefixUnder::Browse),
    };

    Overlay::Prefix(Prefix {
        title: "Go to",
        keymap,
        under,
    })
}

pub(super) fn open_display_prefix() -> Overlay {
    Overlay::Prefix(Prefix {
        title: "Display",
        keymap: &action::VIEW_GROUP,
        under: PrefixUnder::Browse,
    })
}

pub(super) fn open_edit_prefix() -> Overlay {
    Overlay::Prefix(Prefix {
        title: "Edit",
        keymap: &action::EDIT_GROUP,
        under: PrefixUnder::Browse,
    })
}

pub(super) fn apply_prefix(prefix: Prefix, key: KeyEvent) -> Outcome {
    let action = prefix.keymap.resolve(key);

    let under = match prefix.under {
        PrefixUnder::Browse => Overlay::None,
        PrefixUnder::Modal(modal) => modal.into_overlay(),
    };

    match action {
        Some(action) => Outcome::act(under, action),
        None => Outcome::set(under),
    }
}

pub(super) fn apply_find(app: &mut App, mut find: Find, key: KeyEvent) -> Outcome {
    match key.code {
        KeyCode::Esc => {
            app.reveal_focused(find.origin);
            Outcome::close()
        }
        KeyCode::Enter => {
            app.ui.find_query = (!find.query.is_empty()).then(|| find.query.clone());
            Outcome::close()
        }
        KeyCode::Backspace => {
            find.query.pop();
            refresh_find(app, &find.query);
            Outcome::set(Overlay::Find(find))
        }
        KeyCode::Char(c)
            if !key
                .modifiers
                .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
        {
            find.query.push(c);
            refresh_find(app, &find.query);
            Outcome::set(Overlay::Find(find))
        }
        _ => Outcome::set(Overlay::Find(find)),
    }
}

pub(super) fn refresh_find(app: &mut App, query: &str) {
    if query.is_empty() {
        return;
    }

    if let Some(&first) = app.focused_matches(query).first() {
        app.reveal_focused(Some(first));
    }
}

pub(super) fn find_step(app: &mut App, direction: Direction) -> Report {
    let Some(query) = app.ui.find_query.clone() else {
        return Report::status(Status::NoActiveSearch);
    };

    let matches = app.focused_matches(&query);

    if matches.is_empty() {
        return Effects::default().into();
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

    app.reveal_focused(Some(target));

    Effects::default().into()
}

pub(super) fn open_find(app: &mut App) -> Report {
    if app.focused_row_texts().is_empty() {
        return Report::status(Status::NothingToSearch);
    }

    app.set_overlay(Overlay::Find(Find {
        query: String::new(),
        origin: app.focused_selection(),
    }));

    Effects::default().into()
}

pub(super) fn apply_input(app: &mut App, mut input: Input, key: KeyEvent) -> Outcome {
    match InputInput::from_key(key) {
        Some(InputInput::Cancel) => Outcome::set_reporting(Overlay::None, Status::Cancelled),
        Some(InputInput::Submit) => submit_input(app, input),
        Some(InputInput::Erase) => {
            input.backspace();
            Outcome::set(Overlay::Input(input))
        }
        Some(InputInput::MoveLeft) => {
            input.move_left();
            Outcome::set(Overlay::Input(input))
        }
        Some(InputInput::MoveRight) => {
            input.move_right();
            Outcome::set(Overlay::Input(input))
        }
        None => match key.code {
            KeyCode::Char(c)
                if !key
                    .modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
            {
                input.insert(c);
                Outcome::set(Overlay::Input(input))
            }
            _ => Outcome::set(Overlay::Input(input)),
        },
    }
}

fn submit_input(app: &mut App, input: Input) -> Outcome {
    let query = input.buffer.trim().to_string();

    if query.is_empty() {
        return Outcome::close();
    }

    match input.purpose {
        InputPurpose::Jump => {
            let origin = app.take_origin();
            Outcome::dismiss(open_issue(app, IssueRef::parse(&query), None, origin))
        }
        InputPurpose::Search => {
            let key = FeedKey::Search(query.clone());

            app.workspace
                .feeds
                .retain(|existing, _| !existing.is_search() || *existing == key);

            let command = force_feed(app, key);

            Outcome::with(Overlay::Search(Search::new(query)), command)
        }
        InputPurpose::CustomReaction { issue_id, target } => {
            toggle_reaction(app, &issue_id, target, &query).into_dismiss()
        }
        InputPurpose::AssignSearch { issue, label, team } => {
            let picker = Picker {
                kind: PickerKind::Assign(AssignOptions::Matching {
                    query: query.clone(),
                    phase: SearchPhase::InFlight,
                }),
                target_issue: issue,
                target_label: label,
                target_team: team,
                items: Vec::new(),
                state: ListState::default().with_selected(Some(0)),
            };

            Outcome::with(
                Overlay::Picker(picker),
                Effect::Api(ApiCommand::SearchUsers { query }),
            )
        }
        InputPurpose::AddWorkspaceKey => Outcome::dismiss_reporting(
            Commands::runtime(RuntimeCommand::AddAccount {
                credential: Credential::PersonalKey(query),
            }),
            Status::ConnectingWorkspace,
        ),
        InputPurpose::AddWorkspaceEnvVar => Outcome::dismiss_reporting(
            Commands::runtime(RuntimeCommand::AddAccount {
                credential: Credential::EnvVar(query),
            }),
            Status::ConnectingWorkspace,
        ),
    }
}

pub(super) fn apply_editor(editor: Editor, key: KeyEvent) -> Outcome {
    if action::is_editor_submit(key) {
        return submit_editor(editor);
    }

    if editor.mention().is_some() {
        apply_mention(editor, key)
    } else {
        edit(editor, key)
    }
}

fn edit(mut editor: Editor, key: KeyEvent) -> Outcome {
    match EditorInput::from_key(key) {
        Some(EditorInput::Cancel) => Outcome::set_reporting(Overlay::None, Status::Cancelled),
        Some(EditorInput::Newline) => {
            editor.newline();
            Outcome::set(Overlay::Editor(editor))
        }
        Some(EditorInput::Erase) => {
            editor.backspace();
            Outcome::set(Overlay::Editor(editor))
        }
        Some(EditorInput::MoveLeft) => {
            editor.move_left();
            Outcome::set(Overlay::Editor(editor))
        }
        Some(EditorInput::MoveRight) => {
            editor.move_right();
            Outcome::set(Overlay::Editor(editor))
        }
        Some(EditorInput::MoveUp) => {
            editor.move_up();
            Outcome::set(Overlay::Editor(editor))
        }
        Some(EditorInput::MoveDown) => {
            editor.move_down();
            Outcome::set(Overlay::Editor(editor))
        }
        None => match key.code {
            KeyCode::Char('@') if is_plain(key) && editor.at_word_boundary() => {
                editor.open_mention();
                Outcome::set(Overlay::Editor(editor))
            }
            KeyCode::Char(c) if is_plain(key) => {
                editor.insert_char(c);
                Outcome::set(Overlay::Editor(editor))
            }
            _ => Outcome::set(Overlay::Editor(editor)),
        },
    }
}

pub(super) fn is_plain(key: KeyEvent) -> bool {
    !key.modifiers
        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
}

fn apply_mention(mut editor: Editor, key: KeyEvent) -> Outcome {
    match key.code {
        KeyCode::Up => editor.mention_move(Direction::Prev),
        KeyCode::Down => editor.mention_move(Direction::Next),
        KeyCode::Enter => editor.accept_mention(),
        KeyCode::Esc | KeyCode::Left | KeyCode::Right => editor.close_mention(),
        KeyCode::Backspace => editor.mention_backspace(),
        KeyCode::Char(c) if is_plain(key) => editor.mention_type(c),
        _ => {}
    }

    Outcome::set(Overlay::Editor(editor))
}

fn submit_editor(editor: Editor) -> Outcome {
    if editor.is_empty() {
        return Outcome::close();
    }

    let body = editor.text();
    let issue_id = editor.issue_id;
    let team_id = editor.target_team;

    let (command, status) = match editor.compose {
        Compose::Comment => (
            Effect::Api(ApiCommand::CreateComment {
                issue_id,
                team_id,
                body,
                parent_id: None,
            }),
            Status::PostingComment,
        ),
        Compose::Reply { parent_id } => (
            Effect::Api(ApiCommand::CreateComment {
                issue_id,
                team_id,
                body,
                parent_id: Some(parent_id),
            }),
            Status::PostingComment,
        ),
        Compose::Edit { comment_id } => (
            Effect::Api(ApiCommand::UpdateComment {
                issue_id,
                team_id,
                comment_id,
                body,
            }),
            Status::SavingComment,
        ),
    };

    Outcome::dismiss_reporting(command, status)
}

pub(super) fn apply_search(app: &mut App, mut search: Search, key: KeyEvent) -> Outcome {
    let feed_key = FeedKey::Search(search.query.clone());
    let len = app.search_results(&search.query).len();

    let input = PickerInput::from_key(key);

    if key.code == KeyCode::Char('g') {
        return Outcome::set(open_prefix(Overlay::Search(search)));
    }

    match input {
        Some(PickerInput::Accept) => return accept_search(app, search),
        Some(PickerInput::Cancel) => {
            return Outcome::close();
        }
        _ => {}
    }

    let command = match key.code {
        KeyCode::Char('G') => {
            select_edge(&mut search.state, len, Edge::Bottom);
            load_more(app, &feed_key, search.state.selected(), len)
        }
        _ => match input {
            Some(PickerInput::Next) => {
                navigate_list(&mut search.state, len, Direction::Next);
                load_more(app, &feed_key, search.state.selected(), len)
            }
            Some(PickerInput::Prev) => {
                navigate_list(&mut search.state, len, Direction::Prev);
                Effects::default()
            }
            _ => Effects::default(),
        },
    };

    Outcome::with(Overlay::Search(search), command)
}

fn accept_search(app: &mut App, search: Search) -> Outcome {
    let selected = search
        .state
        .selected()
        .and_then(|i| app.search_results(&search.query).get(i))
        .cloned();

    match selected {
        Some(issue) => {
            let command = open_issue(
                app,
                issue.id.clone().into(),
                Some(issue),
                Origin::Search(Box::new(search)),
            );
            Outcome::dismiss(command)
        }
        None => Outcome::set(Overlay::Search(search)),
    }
}

pub(super) fn apply_menu(mut menu: Menu, key: KeyEvent) -> Outcome {
    match key.code {
        KeyCode::Char('g') => {
            return Outcome::set(open_prefix(Overlay::Menu(menu)));
        }
        KeyCode::Char('G') => {
            menu.jump_edge(Edge::Bottom);
            return Outcome::set(Overlay::Menu(menu));
        }
        _ => {}
    }

    match MenuInput::from_key(key) {
        Some(MenuInput::Next) => {
            menu.move_selection(Direction::Next);
            Outcome::set(Overlay::Menu(menu))
        }
        Some(MenuInput::Prev) => {
            menu.move_selection(Direction::Prev);
            Outcome::set(Overlay::Menu(menu))
        }
        Some(MenuInput::SectionNext) => {
            menu.jump_section(Direction::Next);
            Outcome::set(Overlay::Menu(menu))
        }
        Some(MenuInput::SectionPrev) => {
            menu.jump_section(Direction::Prev);
            Outcome::set(Overlay::Menu(menu))
        }
        Some(MenuInput::Run) => match menu.selected_action() {
            Some(action) => Outcome::act(Overlay::None, action),
            None => Outcome::set(Overlay::Menu(menu)),
        },
        Some(MenuInput::Close) => Outcome::close(),
        None => Outcome::set(Overlay::Menu(menu)),
    }
}

pub(super) fn open_menu(app: &mut App) {
    app.set_overlay(Overlay::Menu(Menu::for_focus(app.focus())));
}

pub(super) fn apply_reactions(app: &mut App, mut reactions: Reactions, key: KeyEvent) -> Outcome {
    let Some(input) = ReactionInput::from_key(key) else {
        return Outcome::set(Overlay::Reactions(reactions));
    };

    match input {
        ReactionInput::Left => {
            reactions.move_horizontal(Direction::Prev);
            Outcome::set(Overlay::Reactions(reactions))
        }
        ReactionInput::Right => {
            reactions.move_horizontal(Direction::Next);
            Outcome::set(Overlay::Reactions(reactions))
        }
        ReactionInput::Up => {
            reactions.move_vertical(Direction::Prev);
            Outcome::set(Overlay::Reactions(reactions))
        }
        ReactionInput::Down => {
            reactions.move_vertical(Direction::Next);
            Outcome::set(Overlay::Reactions(reactions))
        }
        ReactionInput::Toggle => match reactions.selected_name().map(str::to_string) {
            Some(name) => {
                toggle_reaction(app, &reactions.issue_id, reactions.target, &name).into_dismiss()
            }
            None => Outcome::close(),
        },
        ReactionInput::Custom => Outcome::set(Overlay::Input(Input::new(
            InputPurpose::CustomReaction {
                issue_id: reactions.issue_id,
                target: reactions.target,
            },
            "React with an emoji",
        ))),
        ReactionInput::Cancel => Outcome::set_reporting(Overlay::None, Status::Cancelled),
    }
}

pub(super) fn apply_labels(mut labels: Labels, key: KeyEvent) -> Outcome {
    match LabelsInput::from_key(key) {
        Some(LabelsInput::Cancel) => Outcome::set_reporting(Overlay::None, Status::Cancelled),
        Some(LabelsInput::Submit) => Outcome::dismiss(Effect::Api(ApiCommand::UpdateIssue {
            id: labels.target_issue.clone(),
            update: IssueUpdate::Labels(labels.selected_ids()),
        })),
        Some(LabelsInput::Toggle) => {
            labels.toggle_highlighted();
            Outcome::set(Overlay::Labels(labels))
        }
        Some(LabelsInput::Next) => {
            let len = labels.results().len();
            navigate_list(&mut labels.state, len, Direction::Next);
            Outcome::set(Overlay::Labels(labels))
        }
        Some(LabelsInput::Prev) => {
            let len = labels.results().len();
            navigate_list(&mut labels.state, len, Direction::Prev);
            Outcome::set(Overlay::Labels(labels))
        }
        Some(LabelsInput::Erase) => {
            labels.query.pop();
            search_labels(labels)
        }
        None => match key.code {
            KeyCode::Char(c) if is_plain(key) => {
                labels.query.push(c);
                search_labels(labels)
            }
            _ => Outcome::set(Overlay::Labels(labels)),
        },
    }
}

fn search_labels(mut labels: Labels) -> Outcome {
    let query = labels.query.clone();
    labels.results = LabelResults::Loading;
    labels.state.select(Some(0));
    Outcome::with(
        Overlay::Labels(labels),
        Effect::Api(ApiCommand::SearchLabels { query }),
    )
}

pub(super) fn apply_workspaces(
    app: &mut App,
    mut workspaces: Workspaces,
    key: KeyEvent,
) -> Outcome {
    let Some(input) = WorkspacesInput::from_key(key) else {
        return Outcome::set(Overlay::Workspaces(workspaces));
    };

    let len = workspaces.rows.len();

    match input {
        WorkspacesInput::Next => {
            navigate_list(&mut workspaces.state, len, Direction::Next);
            Outcome::set(Overlay::Workspaces(workspaces))
        }
        WorkspacesInput::Prev => {
            navigate_list(&mut workspaces.state, len, Direction::Prev);
            Outcome::set(Overlay::Workspaces(workspaces))
        }
        WorkspacesInput::Cancel => Outcome::set_reporting(Overlay::None, Status::Cancelled),
        WorkspacesInput::Accept => match workspaces.selected() {
            Some(WorkspaceRow::Account { key, .. }) => {
                let key = key.clone();
                let account = app
                    .session
                    .accounts()
                    .iter()
                    .find(|a| a.workspace_key == key)
                    .cloned();

                match account {
                    Some(account) => Outcome::dismiss(Commands::runtime(
                        RuntimeCommand::SwitchWorkspace(Box::new(account)),
                    )),
                    None => Outcome::close(),
                }
            }
            Some(WorkspaceRow::AddBrowser) => Outcome::dismiss_reporting(
                Commands::runtime(RuntimeCommand::BeginLogin),
                Status::AwaitingBrowser,
            ),
            Some(WorkspaceRow::AddKey) => Outcome::set(Overlay::Input(Input::new(
                InputPurpose::AddWorkspaceKey,
                "Paste a Linear API key",
            ))),
            Some(WorkspaceRow::AddEnvVar) => Outcome::set(Overlay::Input(Input::new(
                InputPurpose::AddWorkspaceEnvVar,
                "Environment variable name",
            ))),
            None => Outcome::close(),
        },
    }
}

pub(super) fn apply_action(app: &mut App, action: Action) -> Effects {
    app.ui.status = None;

    match action {
        Action::Quit => {
            app.should_quit = true;
            Effects::default()
        }
        Action::NextPanel => cycle_panel(app, Direction::Next),
        Action::PrevPanel => cycle_panel(app, Direction::Prev),
        Action::Descend => descend(app),
        Action::Ascend => ascend(app),
        Action::SelectNext => move_selection(app, Direction::Next),
        Action::SelectPrev => move_selection(app, Direction::Prev),
        Action::NextView => cycle_view(app, Direction::Next),
        Action::PrevView => cycle_view(app, Direction::Prev),
        Action::JumpToPanel(index) => jump_panel(app, index),
        Action::Reload => reload(app),
        Action::OpenInBrowser => open_in_browser(app).write(app),
        Action::YankUrl => yank_url(app).write(app),
        Action::Edit => {
            app.set_overlay(open_edit_prefix());
            Effects::default()
        }
        Action::SetStatus => open_status_picker(app).write(app),
        Action::Assign => open_assign_picker(app).write(app),
        Action::SetPriority => open_priority_picker(app).write(app),
        Action::SetLabels => open_labels(app).write(app),
        Action::Comment => open_comment_input(app).write(app),
        Action::EnterComments => enter_comments(app).write(app),
        Action::Reply => open_reply_editor(app),
        Action::EditComment => open_edit_editor(app).write(app),
        Action::DeleteComment => open_delete_comment(app).write(app),
        Action::React => open_reactions(app),
        Action::CycleGroup => {
            cycle_view_group(app);
            Effects::default()
        }
        Action::CycleSort => {
            cycle_view_sort(app);
            Effects::default()
        }
        Action::ToggleZoom => {
            app.ui.zoom = app.ui.zoom.toggle();
            Effects::default()
        }
        Action::ViewDisplay => {
            app.set_overlay(open_display_prefix());
            Effects::default()
        }
        Action::ClearRecent => {
            clear_recent(app);
            Effects::default()
        }
        Action::GoPrefix => {
            app.set_overlay(open_prefix(Overlay::None));
            Effects::default()
        }
        Action::GoToIssue => {
            app.set_overlay(Overlay::Input(Input::new(
                InputPurpose::Jump,
                "Issue id or URL",
            )));
            Effects::default()
        }
        Action::Search => {
            app.set_overlay(Overlay::Input(Input::new(
                InputPurpose::Search,
                "Search issues",
            )));
            Effects::default()
        }
        Action::Find => open_find(app).write(app),
        Action::FindNext => find_step(app, Direction::Next).write(app),
        Action::FindPrev => find_step(app, Direction::Prev).write(app),
        Action::HalfPageDown => scroll_half(app, Direction::Next),
        Action::HalfPageUp => scroll_half(app, Direction::Prev),
        Action::HistoryBack => history_step(app, Direction::Prev),
        Action::HistoryForward => history_step(app, Direction::Next),
        Action::JumpToTop => jump_edge(app, Edge::Top),
        Action::JumpToBottom => jump_edge(app, Edge::Bottom),
        Action::Help => {
            open_menu(app);
            Effects::default()
        }
        Action::Workspaces => {
            super::open_workspaces(app);
            Effects::default()
        }
    }
}

pub(super) fn apply_confirm(confirm: Confirm, input: Option<ConfirmInput>) -> Outcome {
    match input {
        Some(ConfirmInput::Accept) => Outcome::dismiss_reporting(confirm.command, Status::Applying),
        Some(ConfirmInput::Reject) => Outcome::set_reporting(Overlay::None, Status::Cancelled),
        None => Outcome::set(Overlay::Confirm(confirm)),
    }
}

pub(super) fn apply_picker(mut picker: Picker, key: KeyEvent) -> Outcome {
    match key.code {
        KeyCode::Char('/') if picker.searchable() => {
            let purpose = InputPurpose::AssignSearch {
                issue: picker.target_issue.clone(),
                label: picker.target_label.clone(),
                team: picker.target_team.clone(),
            };

            return Outcome::set(Overlay::Input(Input::new(purpose, "Search people")));
        }
        KeyCode::Char('g') => {
            return Outcome::set(open_prefix(Overlay::Picker(picker)));
        }
        KeyCode::Char('G') => {
            select_edge(&mut picker.state, picker.items.len(), Edge::Bottom);
            return Outcome::set(Overlay::Picker(picker));
        }
        _ => {}
    }

    match PickerInput::from_key(key) {
        Some(PickerInput::Next) => {
            let len = picker.items.len();
            navigate_list(&mut picker.state, len, Direction::Next);
            Outcome::set(Overlay::Picker(picker))
        }
        Some(PickerInput::Prev) => {
            let len = picker.items.len();
            navigate_list(&mut picker.state, len, Direction::Prev);
            Outcome::set(Overlay::Picker(picker))
        }
        Some(PickerInput::Accept) => confirm_picker(picker),
        Some(PickerInput::Cancel) => Outcome::set_reporting(Overlay::None, Status::Cancelled),
        None => Outcome::set(Overlay::Picker(picker)),
    }
}

fn confirm_picker(picker: Picker) -> Outcome {
    let Some(item) = picker.selected() else {
        return Outcome::set(Overlay::Picker(picker));
    };

    let (update, message) = match &item.action {
        PickerAction::SetStatus(state_id) => (
            IssueUpdate::Status(state_id.clone()),
            format!("Set {} to \"{}\"?", picker.target_label, item.label),
        ),
        PickerAction::SetAssignee(Some(assignee_id)) => (
            IssueUpdate::Assignee(Some(assignee_id.clone())),
            format!("Assign {} to {}?", picker.target_label, item.label),
        ),
        PickerAction::SetAssignee(None) => (
            IssueUpdate::Assignee(None),
            format!("Unassign {}?", picker.target_label),
        ),
        PickerAction::SetPriority(priority) => (
            IssueUpdate::Priority(*priority),
            format!(
                "Set {} priority to \"{}\"?",
                picker.target_label, item.label
            ),
        ),
    };

    Outcome::set(Overlay::Confirm(Confirm {
        message,
        command: Effect::Api(ApiCommand::UpdateIssue {
            id: picker.target_issue.clone(),
            update,
        }),
    }))
}
