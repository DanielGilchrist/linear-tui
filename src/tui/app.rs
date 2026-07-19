use ratatui::widgets::{ListState, ScrollbarState};

use super::action::{self, Action};
use super::message::Command;
use crate::api::{
    IssueDetail, IssueFilter, IssueSummary, NotificationItem, Session, StateOption, User,
};

#[derive(Debug, Clone)]
pub enum ViewKind {
    Issues(IssueFilter),
    Inbox,
}

#[derive(Debug, Clone)]
pub struct View {
    pub name: String,
    pub kind: ViewKind,
}

impl View {
    pub fn defaults() -> Vec<View> {
        vec![
            View {
                name: "Assigned to me".into(),
                kind: ViewKind::Issues(IssueFilter::assigned_to_me()),
            },
            View {
                name: "In Progress".into(),
                kind: ViewKind::Issues(IssueFilter::in_progress_mine()),
            },
            View {
                name: "Inbox".into(),
                kind: ViewKind::Inbox,
            },
        ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    MyWork,
    Stub(usize),
    Detail,
}

pub struct StubPanel {
    pub title: String,
    pub items: Vec<String>,
    pub state: ListState,
}

impl StubPanel {
    pub fn new(title: &str, items: &[&str]) -> Self {
        Self {
            title: title.to_string(),
            items: items.iter().map(|s| s.to_string()).collect(),
            state: ListState::default().with_selected(Some(0)),
        }
    }
}

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
            hint: state.state_type,
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
            Focus::Detail => action::DETAIL_MENU,
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
                rows.push(MenuRow::Item { action, keys, label });
            }
        }
    }

    pub fn selected_action(&self) -> Option<Action> {
        match self.rows.get(self.state.selected()?)? {
            MenuRow::Item { action, .. } => Some(*action),
            MenuRow::Header(_) => None,
        }
    }

    pub fn move_selection(&mut self, forward: bool) {
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

        let next = if forward {
            (position + 1) % items.len()
        } else {
            (position + items.len() - 1) % items.len()
        };

        self.state.select(Some(items[next]));
    }

    pub fn jump_section(&mut self, forward: bool) {
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
        let target = if forward {
            (section + 1) % headers.len()
        } else {
            (section + headers.len() - 1) % headers.len()
        };

        let first_item = (headers[target] + 1..self.rows.len())
            .find(|&index| matches!(self.rows[index], MenuRow::Item { .. }));
        if let Some(index) = first_item {
            self.state.select(Some(index));
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
}

pub struct FocusedIssue {
    pub id: String,
    pub identifier: String,
    pub team_id: String,
    pub url: String,
    pub branch_name: String,
}

impl FocusedIssue {
    fn from_summary(issue: &IssueSummary) -> Self {
        Self {
            id: issue.id.clone(),
            identifier: issue.identifier.clone(),
            team_id: issue.team_id.clone(),
            url: issue.url.clone(),
            branch_name: issue.branch_name.clone(),
        }
    }

    fn from_detail(detail: &IssueDetail) -> Self {
        Self {
            id: detail.id.clone(),
            identifier: detail.identifier.clone(),
            team_id: detail.team_id.clone(),
            url: detail.url.clone(),
            branch_name: detail.branch_name.clone(),
        }
    }
}

pub const SCROLL_STEP: usize = 2;

pub struct App {
    pub session: Option<Session>,
    pub views: Vec<View>,
    pub view_state: ListState,
    pub issues: Vec<IssueSummary>,
    pub notifications: Vec<NotificationItem>,
    pub list_state: ListState,
    pub stubs: Vec<StubPanel>,
    pub detail: Option<IssueDetail>,
    pub detail_loading: bool,
    pub focus: Focus,
    pub loading: bool,
    pub status: Option<String>,
    pub spinner_frame: usize,
    pub scroll_position: usize,
    pub scroll_state: ScrollbarState,
    pub overlay: Overlay,
    pub should_quit: bool,
}

impl App {
    pub fn new() -> Self {
        Self {
            session: None,
            views: View::defaults(),
            view_state: ListState::default().with_selected(Some(0)),
            issues: Vec::new(),
            notifications: Vec::new(),
            list_state: ListState::default().with_selected(Some(0)),
            stubs: vec![
                StubPanel::new(
                    "Saved Views",
                    &["#payroll", "#nest-sync", "High priority", "Created by me"],
                ),
                StubPanel::new(
                    "Recently viewed",
                    &[
                        "DAN2-7 Wood-fired oven",
                        "DAN-10 Sprinkle dispenser",
                        "DAN2-5 Pineapple debate",
                    ],
                ),
                StubPanel::new("Teams", &["Dan's Pizza", "Dan's Donuts"]),
            ],
            detail: None,
            detail_loading: false,
            focus: Focus::MyWork,
            loading: false,
            status: None,
            spinner_frame: 0,
            scroll_position: 0,
            scroll_state: ScrollbarState::default(),
            overlay: Overlay::None,
            should_quit: false,
        }
    }

    pub fn picker(&self) -> Option<&Picker> {
        match &self.overlay {
            Overlay::Picker(picker) => Some(picker),
            _ => None,
        }
    }

    pub fn confirm(&self) -> Option<&Confirm> {
        match &self.overlay {
            Overlay::Confirm(confirm) => Some(confirm),
            _ => None,
        }
    }

    pub fn menu(&self) -> Option<&Menu> {
        match &self.overlay {
            Overlay::Menu(menu) => Some(menu),
            _ => None,
        }
    }

    pub fn active_view_index(&self) -> usize {
        self.view_state.selected().unwrap_or(0)
    }

    pub fn active_view(&self) -> &View {
        &self.views[self.active_view_index().min(self.views.len() - 1)]
    }

    pub fn main_len(&self) -> usize {
        match self.active_view().kind {
            ViewKind::Issues(_) => self.issues.len(),
            ViewKind::Inbox => self.notifications.len(),
        }
    }

    pub fn selected_issue(&self) -> Option<&IssueSummary> {
        self.list_state.selected().and_then(|i| self.issues.get(i))
    }

    pub fn selected_notification(&self) -> Option<&NotificationItem> {
        self.list_state
            .selected()
            .and_then(|i| self.notifications.get(i))
    }

    pub fn detail_ready(&self) -> bool {
        match (&self.detail, self.selected_issue()) {
            (Some(detail), Some(selected)) => detail.id == selected.id,
            _ => false,
        }
    }

    pub fn panel_count(&self) -> usize {
        1 + self.stubs.len()
    }

    pub fn focus_at(&self, index: usize) -> Focus {
        if index == 0 {
            Focus::MyWork
        } else {
            Focus::Stub(index - 1)
        }
    }

    pub fn expanded_panel(&self) -> usize {
        match self.focus {
            Focus::Stub(index) => index + 1,
            _ => 0,
        }
    }

    pub fn panel_len(&self, index: usize) -> usize {
        match index {
            0 => self.main_len(),
            _ => self.stubs[index - 1].items.len(),
        }
    }

    pub fn focused_list_len(&self) -> usize {
        match self.focus {
            Focus::MyWork => self.main_len(),
            Focus::Stub(index) => self.stubs[index].items.len(),
            Focus::Detail => 0,
        }
    }

    pub fn focused_list_mut(&mut self) -> Option<&mut ListState> {
        match self.focus {
            Focus::MyWork => Some(&mut self.list_state),
            Focus::Stub(index) => Some(&mut self.stubs[index].state),
            Focus::Detail => None,
        }
    }

    pub fn open_target(&self) -> Option<FocusedIssue> {
        match self.focus {
            Focus::MyWork => self.selected_issue().map(FocusedIssue::from_summary),
            Focus::Detail => self
                .detail
                .as_ref()
                .map(FocusedIssue::from_detail)
                .or_else(|| self.selected_issue().map(FocusedIssue::from_summary)),
            Focus::Stub(_) => None,
        }
    }

    pub fn action_target(&self) -> Option<FocusedIssue> {
        if self.focus == Focus::Detail {
            self.detail.as_ref().map(FocusedIssue::from_detail)
        } else {
            None
        }
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}
