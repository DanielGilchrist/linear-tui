use ratatui::widgets::ListState;

use super::action::{self, Action};
use super::emoji::{self, PaletteEmoji};
use super::focus::{Direction, Edge, Focus};
use super::message::Command;
use crate::api::{
    CommentId, IssueId, Priority, Reaction, ReactionTarget, StateId, StateOption, User, UserId,
};
use crate::store::Account;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PickerKind {
    Status,
    Assign(AssignOptions),
    Priority,
}

/// Assignees are not enumerated: an account can hold thousands, so the picker
/// offers yourself and searches for anyone else.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AssignOptions {
    Suggested,
    Matching(String),
}

#[derive(Debug, Clone)]
pub enum PickerAction {
    SetStatus(StateId),
    SetAssignee(Option<UserId>),
    SetPriority(Priority),
}

#[derive(Debug, Clone)]
pub struct PickerItem {
    pub label: String,
    pub hint: String,
    pub action: PickerAction,
}

impl PickerItem {
    pub fn hint(&self) -> Option<&str> {
        (!self.hint.is_empty()).then_some(self.hint.as_str())
    }

    pub fn unassign() -> Self {
        Self {
            label: "Unassigned".into(),
            hint: String::new(),
            action: PickerAction::SetAssignee(None),
        }
    }
}

impl From<StateOption> for PickerItem {
    fn from(state: StateOption) -> Self {
        Self {
            hint: state.state_type.as_api().to_string(),
            label: state.name,
            action: PickerAction::SetStatus(state.id),
        }
    }
}

impl From<User> for PickerItem {
    fn from(user: User) -> Self {
        Self {
            hint: if user.is_me {
                "you".into()
            } else {
                String::new()
            },
            label: user.display_name,
            action: PickerAction::SetAssignee(Some(user.id)),
        }
    }
}

impl From<Priority> for PickerItem {
    fn from(priority: Priority) -> Self {
        Self {
            label: priority.label().to_string(),
            hint: String::new(),
            action: PickerAction::SetPriority(priority),
        }
    }
}

pub struct Picker {
    pub kind: PickerKind,
    pub target_issue: IssueId,
    pub target_label: String,
    pub items: Vec<PickerItem>,
    pub state: ListState,
    pub loading: bool,
}

impl Picker {
    pub fn verb(&self) -> &'static str {
        match self.kind {
            PickerKind::Status => "Set status",
            PickerKind::Assign(_) => "Assign",
            PickerKind::Priority => "Set priority",
        }
    }

    pub fn searching(&self) -> Option<&str> {
        match &self.kind {
            PickerKind::Assign(AssignOptions::Matching(query)) => Some(query),
            PickerKind::Status
            | PickerKind::Assign(AssignOptions::Suggested)
            | PickerKind::Priority => None,
        }
    }

    pub fn searchable(&self) -> bool {
        matches!(self.kind, PickerKind::Assign(_))
    }

    pub fn selected(&self) -> Option<&PickerItem> {
        self.state.selected().and_then(|i| self.items.get(i))
    }
}

pub struct Confirm {
    pub message: String,
    pub command: Command,
}

pub enum MenuRow {
    Header(&'static str),
    Item {
        action: Action,
        keys: String,
        label: &'static str,
    },
}

pub struct Menu {
    pub rows: Vec<MenuRow>,
    pub state: ListState,
}

impl Menu {
    pub fn new(rows: Vec<MenuRow>) -> Self {
        let first = rows
            .iter()
            .position(|row| matches!(row, MenuRow::Item { .. }));

        Self {
            rows,
            state: ListState::default().with_selected(first),
        }
    }

    pub fn for_focus(focus: &Focus) -> Self {
        let local = match focus {
            Focus::MyWork => action::MY_WORK_MENU,
            Focus::Recent => action::RECENT_MENU,
            Focus::SavedViews => action::SAVED_VIEWS_MENU,
            Focus::View => action::VIEW_MENU,
            Focus::Detail(..) => action::DETAIL_MENU,
            Focus::Stub(_) => action::STUB_MENU,
        };

        let mut rows = vec![MenuRow::Header("Local")];
        Self::push_items(&mut rows, local);
        rows.push(MenuRow::Header("Global"));
        Self::push_items(&mut rows, action::GLOBAL_MENU);

        Menu::new(rows)
    }

    fn push_items(rows: &mut Vec<MenuRow>, actions: &[Action]) {
        for &action in actions {
            if let Some((keys, label)) = action::BROWSE.describe(action) {
                rows.push(MenuRow::Item {
                    action,
                    keys,
                    label,
                });
            }
        }
    }

    pub fn selected_action(&self) -> Option<Action> {
        match self.rows.get(self.state.selected()?)? {
            MenuRow::Item { action, .. } => Some(*action),
            MenuRow::Header(_) => None,
        }
    }

    pub fn move_selection(&mut self, direction: Direction) {
        let items: Vec<usize> = self
            .rows
            .iter()
            .enumerate()
            .filter(|(_, row)| matches!(row, MenuRow::Item { .. }))
            .map(|(index, _)| index)
            .collect();

        if items.is_empty() {
            return;
        }

        let current = self.state.selected().unwrap_or(items[0]);
        let position = items.iter().position(|&i| i == current).unwrap_or(0);

        self.state
            .select(Some(items[direction.wrap(position, items.len())]));
    }

    pub fn jump_section(&mut self, direction: Direction) {
        let headers: Vec<usize> = self
            .rows
            .iter()
            .enumerate()
            .filter(|(_, row)| matches!(row, MenuRow::Header(_)))
            .map(|(index, _)| index)
            .collect();

        if headers.is_empty() {
            return;
        }

        let current = self.state.selected().unwrap_or(0);
        let section = headers.iter().rposition(|&h| h <= current).unwrap_or(0);
        let target = direction.wrap(section, headers.len());

        let first_item = (headers[target] + 1..self.rows.len())
            .find(|&index| matches!(self.rows[index], MenuRow::Item { .. }));
        if let Some(index) = first_item {
            self.state.select(Some(index));
        }
    }

    pub fn jump_edge(&mut self, edge: Edge) {
        let mut items = self
            .rows
            .iter()
            .enumerate()
            .filter(|(_, row)| matches!(row, MenuRow::Item { .. }))
            .map(|(index, _)| index);

        let target = match edge {
            Edge::Bottom => items.next_back(),
            Edge::Top => items.next(),
        };

        if let Some(index) = target {
            self.state.select(Some(index));
        }
    }
}

pub struct Prefix {
    pub title: &'static str,
    pub keymap: &'static action::Keymap<Action>,
    pub under: PrefixUnder,
}

pub enum PrefixUnder {
    Browse,
    Modal(Box<Overlay>),
}

pub struct Find {
    pub query: String,
    pub origin: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputPurpose {
    Jump,
    Search,
    CustomReaction { target: ReactionTarget },
    AssignSearch { issue: IssueId, label: String },
    AddWorkspaceKey,
    AddWorkspaceEnvVar,
}

pub enum WorkspaceRow {
    Account {
        key: String,
        name: String,
        detail: String,
        active: bool,
    },
    AddBrowser,
    AddKey,
    AddEnvVar,
}

pub struct Workspaces {
    pub rows: Vec<WorkspaceRow>,
    pub state: ListState,
}

impl Workspaces {
    pub fn new(accounts: &[Account], active: Option<&str>) -> Self {
        let mut rows: Vec<WorkspaceRow> = accounts
            .iter()
            .map(|account| WorkspaceRow::Account {
                key: account.workspace_key.clone(),
                name: account.org_name.clone(),
                detail: account.credential.describe(),
                active: active == Some(account.workspace_key.as_str()),
            })
            .collect();

        rows.push(WorkspaceRow::AddBrowser);
        rows.push(WorkspaceRow::AddKey);
        rows.push(WorkspaceRow::AddEnvVar);

        let selected = accounts
            .iter()
            .position(|account| active == Some(account.workspace_key.as_str()))
            .unwrap_or(0);

        Self {
            rows,
            state: ListState::default().with_selected(Some(selected)),
        }
    }

    pub fn selected(&self) -> Option<&WorkspaceRow> {
        self.state.selected().and_then(|index| self.rows.get(index))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Section {
    Current,
    Add,
}

pub struct ReactionChoice {
    pub name: String,
    pub glyph: String,
    pub count: usize,
    pub mine: bool,
    pub section: Section,
}

impl ReactionChoice {
    fn current(name: String, mine: bool) -> Self {
        Self {
            glyph: emoji::glyph(&name),
            name,
            count: 1,
            mine,
            section: Section::Current,
        }
    }

    fn palette(entry: &PaletteEmoji) -> Self {
        Self {
            name: entry.name.to_string(),
            glyph: entry.glyph.to_string(),
            count: 0,
            mine: false,
            section: Section::Add,
        }
    }

    pub fn in_section(&self, section: Section) -> bool {
        self.section == section
    }
}

pub struct Reactions {
    pub target: ReactionTarget,
    pub choices: Vec<ReactionChoice>,
    pub state: ListState,
}

impl Reactions {
    pub fn new(target: ReactionTarget, reactions: &[Reaction]) -> Self {
        let mut choices: Vec<ReactionChoice> = Vec::new();

        for reaction in reactions {
            match choices
                .iter_mut()
                .find(|choice| choice.name == reaction.emoji)
            {
                Some(choice) => {
                    choice.count += 1;
                    choice.mine |= reaction.mine;
                }
                None => choices.push(ReactionChoice::current(
                    reaction.emoji.clone(),
                    reaction.mine,
                )),
            }
        }

        for entry in emoji::REACTION_PALETTE {
            if !choices.iter().any(|choice| choice.name == entry.name) {
                choices.push(ReactionChoice::palette(entry));
            }
        }

        let split = Self::split_at(&choices);
        let start = if split < choices.len() { split } else { 0 };

        Self {
            target,
            choices,
            state: ListState::default().with_selected(Some(start)),
        }
    }

    pub fn selected_name(&self) -> Option<&str> {
        self.state
            .selected()
            .and_then(|index| self.choices.get(index))
            .map(|choice| choice.name.as_str())
    }

    fn split_at(choices: &[ReactionChoice]) -> usize {
        choices
            .iter()
            .take_while(|choice| choice.in_section(Section::Current))
            .count()
    }

    fn split(&self) -> usize {
        Self::split_at(&self.choices)
    }

    pub fn move_horizontal(&mut self, direction: Direction) {
        let split = self.split();
        let len = self.choices.len();
        let selected = self.state.selected().unwrap_or(0);

        let (lo, hi) = if selected < split {
            (0, split)
        } else {
            (split, len)
        };

        if hi <= lo {
            return;
        }

        let next = match direction {
            Direction::Next => (selected + 1).min(hi - 1),
            Direction::Prev => selected.saturating_sub(1).max(lo),
        };

        self.state.select(Some(next));
    }

    pub fn move_vertical(&mut self, direction: Direction) {
        let split = self.split();
        let len = self.choices.len();

        if split == 0 || split == len {
            return;
        }

        let selected = self.state.selected().unwrap_or(0);
        let in_current = selected < split;

        let next = match direction {
            Direction::Prev if !in_current => (selected - split).min(split - 1),
            Direction::Next if in_current => split + selected.min(len - split - 1),
            _ => return,
        };

        self.state.select(Some(next));
    }
}

pub struct Input {
    pub purpose: InputPurpose,
    pub prompt: &'static str,
    pub buffer: String,
    pub cursor: usize,
}

impl Input {
    pub fn new(purpose: InputPurpose, prompt: &'static str) -> Self {
        Self {
            purpose,
            prompt,
            buffer: String::new(),
            cursor: 0,
        }
    }

    pub fn insert(&mut self, c: char) {
        let byte = self.byte_offset();
        self.buffer.insert(byte, c);
        self.cursor += 1;
    }

    pub fn backspace(&mut self) {
        if self.cursor == 0 {
            return;
        }

        self.cursor -= 1;

        let byte = self.byte_offset();
        self.buffer.remove(byte);
    }

    pub fn move_left(&mut self) {
        self.cursor = self.cursor.saturating_sub(1);
    }

    pub fn move_right(&mut self) {
        self.cursor = (self.cursor + 1).min(self.char_len());
    }

    pub fn char_len(&self) -> usize {
        self.buffer.chars().count()
    }

    fn byte_offset(&self) -> usize {
        self.buffer
            .char_indices()
            .nth(self.cursor)
            .map(|(index, _)| index)
            .unwrap_or(self.buffer.len())
    }
}

#[derive(Debug, Clone)]
pub enum Cell {
    Char(char),
    Mention(Mention),
}

#[derive(Debug, Clone)]
pub struct Mention {
    pub display: String,
    pub url: String,
}

pub struct MentionMenu {
    pub at: usize,
    pub query: String,
    pub state: ListState,
}

#[derive(Debug, Clone)]
pub enum Compose {
    Comment,
    Reply { parent_id: CommentId },
    Edit { comment_id: CommentId },
}

impl Compose {
    fn title(&self) -> &'static str {
        match self {
            Compose::Comment => "Comment",
            Compose::Reply { .. } => "Reply",
            Compose::Edit { .. } => "Edit",
        }
    }
}

pub struct Editor {
    pub title: &'static str,
    pub lines: Vec<Vec<Cell>>,
    pub row: usize,
    pub col: usize,
    pub compose: Compose,
    pub members: Vec<User>,
    pub mention: Option<MentionMenu>,
}

impl Editor {
    pub fn new(compose: Compose) -> Self {
        Self {
            title: compose.title(),
            lines: vec![Vec::new()],
            row: 0,
            col: 0,
            compose,
            members: Vec::new(),
            mention: None,
        }
    }

    pub fn seeded(compose: Compose, body: &str) -> Self {
        let mut editor = Self::new(compose);

        editor.lines = body
            .split('\n')
            .map(|line| line.chars().map(Cell::Char).collect())
            .collect();

        editor.row = editor.lines.len() - 1;
        editor.col = editor.lines[editor.row].len();

        editor
    }

    pub fn text(&self) -> String {
        self.lines
            .iter()
            .map(|line| {
                line.iter()
                    .map(|cell| match cell {
                        Cell::Char(c) => c.to_string(),
                        Cell::Mention(mention) => mention.url.clone(),
                    })
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn is_empty(&self) -> bool {
        self.lines.iter().all(|line| line.is_empty())
    }

    fn line_len(&self, row: usize) -> usize {
        self.lines[row].len()
    }

    pub fn insert_char(&mut self, c: char) {
        let col = self.col.min(self.line_len(self.row));
        self.lines[self.row].insert(col, Cell::Char(c));
        self.col = col + 1;
    }

    pub fn insert_mention(&mut self, display: String, url: String) {
        let col = self.col.min(self.line_len(self.row));
        self.lines[self.row].insert(col, Cell::Mention(Mention { display, url }));
        self.col = col + 1;
    }

    pub fn newline(&mut self) {
        let col = self.col.min(self.line_len(self.row));
        let tail = self.lines[self.row].split_off(col);

        self.lines.insert(self.row + 1, tail);
        self.row += 1;
        self.col = 0;
    }

    pub fn backspace(&mut self) {
        if self.col > 0 {
            self.col -= 1;
            self.lines[self.row].remove(self.col);
        } else if self.row > 0 {
            let current = self.lines.remove(self.row);

            self.row -= 1;
            self.col = self.line_len(self.row);
            self.lines[self.row].extend(current);
        }
    }

    pub fn move_left(&mut self) {
        if self.col > 0 {
            self.col -= 1;
        } else if self.row > 0 {
            self.row -= 1;
            self.col = self.line_len(self.row);
        }
    }

    pub fn move_right(&mut self) {
        if self.col < self.line_len(self.row) {
            self.col += 1;
        } else if self.row + 1 < self.lines.len() {
            self.row += 1;
            self.col = 0;
        }
    }

    pub fn move_up(&mut self) {
        if self.row > 0 {
            self.row -= 1;
            self.col = self.col.min(self.line_len(self.row));
        }
    }

    pub fn move_down(&mut self) {
        if self.row + 1 < self.lines.len() {
            self.row += 1;
            self.col = self.col.min(self.line_len(self.row));
        }
    }

    pub fn at_word_boundary(&self) -> bool {
        match self
            .col
            .checked_sub(1)
            .and_then(|i| self.lines[self.row].get(i))
        {
            None => true,
            Some(Cell::Char(c)) => c.is_whitespace(),
            Some(Cell::Mention(_)) => true,
        }
    }

    pub fn candidates(&self, query: &str) -> Vec<&User> {
        let needle = query.to_lowercase();

        self.members
            .iter()
            .filter(|user| user.display_name.to_lowercase().contains(&needle))
            .collect()
    }
}

pub struct Search {
    pub query: String,
    pub state: ListState,
}

impl Search {
    pub fn new(query: String) -> Self {
        Self {
            query,
            state: ListState::default().with_selected(Some(0)),
        }
    }
}

#[derive(Default)]
pub enum Overlay {
    #[default]
    None,
    Picker(Picker),
    Confirm(Confirm),
    Menu(Menu),
    Prefix(Prefix),
    Input(Input),
    Editor(Editor),
    Search(Search),
    Find(Find),
    Reactions(Reactions),
    Workspaces(Workspaces),
}
