use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::widgets::ListState;

use super::feed::{force_feed, load_more, reload};
use super::issue::{
    clear_recent, enter_comments, open_assign_picker, open_comment_input, open_delete_comment,
    open_edit_editor, open_in_browser, open_issue, open_reactions, open_reply_editor,
    open_status_picker, toggle_reaction, yank_url,
};
use super::nav::{
    ascend, cycle_panel, cycle_view, cycle_view_group, cycle_view_sort, descend, history_step,
    jump_edge, jump_panel, move_selection, navigate_list, scroll_half, select_edge,
};
use crate::api::IssueRef;
use crate::api::IssueUpdate;
use crate::tui::action::{
    self, Action, ConfirmInput, EditorInput, InputInput, MenuInput, PickerInput, ReactionInput,
};
use crate::tui::app::App;
use crate::tui::feed::FeedKey;
use crate::tui::focus::{DetailView, Direction, Edge, Focus};
use crate::tui::message::Command;
use crate::tui::overlay::{
    Compose, Confirm, Editor, Find, Input, InputPurpose, MentionMenu, Menu, Overlay, Picker,
    PickerAction, Prefix, PrefixUnder, Reactions, Search,
};
use crate::tui::status::Status;

pub(super) fn resolve_browse(app: &App, key: KeyEvent) -> Option<Action> {
    if is_plain(key) {
        if let Some(action) = context_keymap(&app.focus).and_then(|keymap| keymap.resolve(key)) {
            return Some(action);
        }
    }

    Action::from_key(key)
}

pub(super) fn context_keymap(focus: &Focus) -> Option<&'static action::Keymap<Action>> {
    match focus {
        Focus::Detail(detail) => match detail.view {
            DetailView::Reading => Some(&action::DETAIL_KEYS),
            DetailView::Comments => Some(&action::COMMENTS_KEYS),
        },
        Focus::View => Some(&action::VIEW_KEYS),
        Focus::MyWork | Focus::Recent | Focus::SavedViews | Focus::Stub(_) => None,
    }
}

pub(super) fn open_prefix(under: Overlay) -> Overlay {
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

pub(super) fn open_display_prefix() -> Overlay {
    Overlay::Prefix(Prefix {
        title: "Display",
        keymap: &action::VIEW_GROUP,
        under: PrefixUnder::Browse,
    })
}

pub(super) fn apply_prefix(app: &mut App, prefix: Prefix, key: KeyEvent) -> Option<Command> {
    let action = prefix.keymap.resolve(key);

    app.overlay = match prefix.under {
        PrefixUnder::Browse => Overlay::None,
        PrefixUnder::Modal(modal) => *modal,
    };
    action.and_then(|action| apply_action(app, action))
}

pub(super) fn apply_find(app: &mut App, mut find: Find, key: KeyEvent) -> Option<Command> {
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

pub(super) fn refresh_find(app: &mut App, query: &str) {
    if query.is_empty() {
        return;
    }

    if let Some(&first) = app.focused_matches(query).first() {
        if let Some(state) = app.focused_list_mut() {
            state.select(Some(first));
        }
    }
}

pub(super) fn find_step(app: &mut App, direction: Direction) {
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

pub(super) fn open_find(app: &mut App) -> Option<Command> {
    match app.focus {
        Focus::Detail(..) => {
            app.status = Some(Status::FindInList);
            return None;
        }
        Focus::MyWork | Focus::Recent | Focus::SavedViews | Focus::View | Focus::Stub(_) => {}
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

pub(super) fn apply_input(app: &mut App, mut input: Input, key: KeyEvent) -> Option<Command> {
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

pub(super) fn restore_input(app: &mut App, input: Input) -> Option<Command> {
    app.overlay = Overlay::Input(input);
    None
}

pub(super) fn submit_input(app: &mut App, input: Input) -> Option<Command> {
    let query = input.buffer.trim().to_string();

    if query.is_empty() {
        return None;
    }

    match input.purpose {
        InputPurpose::Jump => open_issue(app, IssueRef::parse(&query)),
        InputPurpose::Search => {
            let key = FeedKey::Search(query.clone());

            app.workspace
                .feeds
                .retain(|existing, _| !existing.is_search() || *existing == key);

            let command = force_feed(app, key);

            app.overlay = Overlay::Search(Search::new(query));
            Some(command)
        }
        InputPurpose::CustomReaction { target } => toggle_reaction(app, target, &query),
    }
}

pub(super) fn apply_editor(app: &mut App, mut editor: Editor, key: KeyEvent) -> Option<Command> {
    if action::is_editor_submit(key) {
        return submit_editor(app, editor);
    }

    match editor.mention.take() {
        Some(mention) => apply_mention(app, editor, mention, key),
        None => edit(app, editor, key),
    }
}

pub(super) fn edit(app: &mut App, mut editor: Editor, key: KeyEvent) -> Option<Command> {
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
            KeyCode::Char('@') if is_plain(key) && editor.at_word_boundary() => {
                editor.insert_char('@');
                editor.mention = Some(MentionMenu {
                    at: editor.col - 1,
                    query: String::new(),
                    state: ListState::default().with_selected(Some(0)),
                });

                restore_editor(app, editor)
            }
            KeyCode::Char(c) if is_plain(key) => {
                editor.insert_char(c);
                restore_editor(app, editor)
            }
            _ => restore_editor(app, editor),
        },
    }
}

pub(super) fn is_plain(key: KeyEvent) -> bool {
    !key.modifiers
        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
}

pub(super) fn apply_mention(
    app: &mut App,
    mut editor: Editor,
    mut mention: MentionMenu,
    key: KeyEvent,
) -> Option<Command> {
    match key.code {
        KeyCode::Up => {
            mention_move(&editor, &mut mention, Direction::Prev);
            editor.mention = Some(mention);
            restore_editor(app, editor)
        }
        KeyCode::Down => {
            mention_move(&editor, &mut mention, Direction::Next);
            editor.mention = Some(mention);
            restore_editor(app, editor)
        }
        KeyCode::Enter => {
            accept_mention(&mut editor, mention);
            restore_editor(app, editor)
        }
        KeyCode::Esc | KeyCode::Left | KeyCode::Right => restore_editor(app, editor),
        KeyCode::Backspace => {
            editor.backspace();
            if !mention.query.is_empty() {
                mention.query.pop();
                mention.state.select(Some(0));
                editor.mention = Some(mention);
            }
            restore_editor(app, editor)
        }
        KeyCode::Char(c) if is_plain(key) => {
            editor.insert_char(c);
            mention.query.push(c);
            mention.state.select(Some(0));
            editor.mention = Some(mention);
            restore_editor(app, editor)
        }
        _ => {
            editor.mention = Some(mention);
            restore_editor(app, editor)
        }
    }
}

pub(super) fn mention_move(editor: &Editor, mention: &mut MentionMenu, direction: Direction) {
    let len = editor.candidates(&mention.query).len();
    if len == 0 {
        return;
    }
    let current = mention.state.selected().unwrap_or(0);
    mention.state.select(Some(direction.wrap(current, len)));
}

pub(super) fn accept_mention(editor: &mut Editor, mention: MentionMenu) {
    let Some(selected) = mention.state.selected() else {
        return;
    };

    let query = mention.query.to_lowercase();
    let picked = editor
        .members
        .iter()
        .filter(|user| user.display_name.to_lowercase().contains(&query))
        .nth(selected)
        .map(|user| (user.display_name.clone(), user.url.clone()));

    let Some((display, url)) = picked else {
        return;
    };

    editor.lines[editor.row].drain(mention.at..editor.col);
    editor.col = mention.at;
    editor.insert_mention(display, url);
}

pub(super) fn restore_editor(app: &mut App, editor: Editor) -> Option<Command> {
    app.overlay = Overlay::Editor(editor);
    None
}

pub(super) fn submit_editor(app: &mut App, editor: Editor) -> Option<Command> {
    if editor.is_empty() {
        return None;
    }

    let issue_id = app
        .workspace
        .detail()
        .value()
        .map(|detail| detail.id.clone())?;

    let body = editor.text();

    match editor.compose {
        Compose::Comment => {
            app.status = Some(Status::PostingComment);
            Some(Command::CreateComment {
                issue_id,
                body,
                parent_id: None,
            })
        }
        Compose::Reply { parent_id } => {
            app.status = Some(Status::PostingComment);
            Some(Command::CreateComment {
                issue_id,
                body,
                parent_id: Some(parent_id),
            })
        }
        Compose::Edit { comment_id } => {
            app.status = Some(Status::SavingComment);
            Some(Command::UpdateComment {
                issue_id,
                comment_id,
                body,
            })
        }
    }
}

pub(super) fn apply_search(app: &mut App, mut search: Search, key: KeyEvent) -> Option<Command> {
    let feed_key = FeedKey::Search(search.query.clone());
    let len = app.search_results(&search.query).len();

    let input = PickerInput::from_key(key);

    if key.code == KeyCode::Char('g') {
        app.overlay = open_prefix(Overlay::Search(search));
        return None;
    }

    match input {
        Some(PickerInput::Accept) => return accept_search(app, search),
        Some(PickerInput::Cancel) => {
            app.search_return = None;
            return None;
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
                None
            }
            _ => None,
        },
    };

    app.overlay = Overlay::Search(search);
    command
}

pub(super) fn accept_search(app: &mut App, search: Search) -> Option<Command> {
    let selected = search
        .state
        .selected()
        .and_then(|i| app.search_results(&search.query).get(i))
        .map(|issue| issue.id.clone());

    match selected {
        Some(id) => {
            let command = open_issue(app, id);
            app.search_return = Some(search);
            command
        }
        None => {
            app.overlay = Overlay::Search(search);
            None
        }
    }
}

pub(super) fn apply_menu(app: &mut App, mut menu: Menu, key: KeyEvent) -> Option<Command> {
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

pub(super) fn open_menu(app: &mut App) {
    app.overlay = Overlay::Menu(Menu::for_focus(&app.focus));
}

pub(super) fn apply_reactions(
    app: &mut App,
    mut reactions: Reactions,
    key: KeyEvent,
) -> Option<Command> {
    let Some(input) = ReactionInput::from_key(key) else {
        app.overlay = Overlay::Reactions(reactions);
        return None;
    };

    match input {
        ReactionInput::Left => {
            reactions.move_horizontal(Direction::Prev);
            app.overlay = Overlay::Reactions(reactions);
            None
        }
        ReactionInput::Right => {
            reactions.move_horizontal(Direction::Next);
            app.overlay = Overlay::Reactions(reactions);
            None
        }
        ReactionInput::Up => {
            reactions.move_vertical(Direction::Prev);
            app.overlay = Overlay::Reactions(reactions);
            None
        }
        ReactionInput::Down => {
            reactions.move_vertical(Direction::Next);
            app.overlay = Overlay::Reactions(reactions);
            None
        }
        ReactionInput::Toggle => match reactions.selected_name().map(str::to_string) {
            Some(name) => toggle_reaction(app, reactions.target, &name),
            None => None,
        },
        ReactionInput::Custom => {
            app.overlay = Overlay::Input(Input::new(
                InputPurpose::CustomReaction {
                    target: reactions.target,
                },
                "React with an emoji",
            ));
            None
        }
        ReactionInput::Cancel => None,
    }
}

pub(super) fn apply_action(app: &mut App, action: Action) -> Option<Command> {
    match action {
        Action::Quit => {
            app.should_quit = true;
            None
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
        Action::Reload => Some(reload(app)),
        Action::OpenInBrowser => open_in_browser(app),
        Action::YankUrl => yank_url(app),
        Action::SetStatus => open_status_picker(app),
        Action::Assign => open_assign_picker(app),
        Action::Comment => open_comment_input(app),
        Action::EnterComments => enter_comments(app),
        Action::Reply => open_reply_editor(app),
        Action::EditComment => open_edit_editor(app),
        Action::DeleteComment => open_delete_comment(app),
        Action::React => open_reactions(app),
        Action::CycleGroup => {
            cycle_view_group(app);
            None
        }
        Action::CycleSort => {
            cycle_view_sort(app);
            None
        }
        Action::ToggleZoom => {
            app.zoom = app.zoom.toggle();
            None
        }
        Action::ViewDisplay => {
            app.overlay = open_display_prefix();
            None
        }
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
        Action::HalfPageDown => scroll_half(app, Direction::Next),
        Action::HalfPageUp => scroll_half(app, Direction::Prev),
        Action::HistoryBack => history_step(app, Direction::Prev),
        Action::HistoryForward => history_step(app, Direction::Next),
        Action::JumpToTop => jump_edge(app, Edge::Top),
        Action::JumpToBottom => jump_edge(app, Edge::Bottom),
        Action::Help => {
            open_menu(app);
            None
        }
    }
}

pub(super) fn apply_confirm(
    app: &mut App,
    confirm: Confirm,
    input: Option<ConfirmInput>,
) -> Option<Command> {
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

pub(super) fn apply_picker(app: &mut App, mut picker: Picker, key: KeyEvent) -> Option<Command> {
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

pub(super) fn confirm_picker(app: &mut App, picker: Picker) -> Option<Command> {
    let Some(item) = picker.selected() else {
        app.overlay = Overlay::Picker(picker);
        return None;
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
