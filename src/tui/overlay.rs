use ratatui::widgets::ListState;

use super::action::{self, Action};
use super::focus::{Direction, Edge, Focus};
use super::message::Command;
use crate::api::{IssueSummary, StateOption, User};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PickerKind {
    Status,
    Assign,
}

#[derive(Debug, Clone)]
pub struct PickerItem {
    pub id: String,
    pub label: String,
    pub hint: String,
}

impl From<StateOption> for PickerItem {
    fn from(state: StateOption) -> Self {
        Self {
            id: state.id,
            label: state.name,
            hint: state.state_type.as_api().to_string(),
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
            id: user.id,
            label: user.display_name,
        }
    }
}

pub struct Picker {
    pub kind: PickerKind,
    pub target_issue: String,
    pub target_label: String,
    pub items: Vec<PickerItem>,
    pub state: ListState,
    pub loading: bool,
}

impl Picker {
    pub fn verb(&self) -> &'static str {
        match self.kind {
            PickerKind::Status => "Set status",
            PickerKind::Assign => "Assign",
        }
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

    pub fn for_focus(focus: Focus) -> Self {
        let local = match focus {
            Focus::MyWork => action::MY_WORK_MENU,
            Focus::Recent => action::RECENT_MENU,
            Focus::Detail(_) => action::DETAIL_MENU,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputPurpose {
    Jump,
    Search,
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

pub struct Search {
    pub query: String,
    pub results: Vec<IssueSummary>,
    pub state: ListState,
    pub loading: bool,
}

impl Search {
    pub fn new(query: String) -> Self {
        Self {
            query,
            results: Vec::new(),
            state: ListState::default().with_selected(Some(0)),
            loading: true,
        }
    }

    pub fn selected(&self) -> Option<&IssueSummary> {
        self.state.selected().and_then(|i| self.results.get(i))
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
    Search(Search),
    Find(Find),
}
