use ratatui::widgets::{ListState, ScrollbarState};

use super::message::Command;
use crate::api::{IssueDetail, IssueFilter, IssueSummary, NotificationItem, Session, StateOption, User};

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
            hint: if user.is_me { "you".into() } else { String::new() },
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

#[derive(Default)]
pub enum Overlay {
    #[default]
    None,
    Picker(Picker),
    Confirm(Confirm),
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

    pub fn picker_mut(&mut self) -> Option<&mut Picker> {
        match &mut self.overlay {
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
