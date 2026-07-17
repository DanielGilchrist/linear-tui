use ratatui::widgets::{ListState, ScrollbarState};

use crate::api::{IssueDetail, IssueFilter, IssueSummary, NotificationItem, Session};

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
pub enum Pane {
    Sidebar,
    Main,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Home,
    Detail,
}

pub const SCROLL_STEP: usize = 2;

pub struct App {
    pub session: Option<Session>,
    pub views: Vec<View>,
    pub view_state: ListState,
    pub issues: Vec<IssueSummary>,
    pub notifications: Vec<NotificationItem>,
    pub list_state: ListState,
    pub detail: Option<IssueDetail>,
    pub screen: Screen,
    pub pane: Pane,
    pub loading: bool,
    pub status: Option<String>,
    pub spinner_frame: usize,
    pub scroll_position: usize,
    pub scroll_state: ScrollbarState,
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
            detail: None,
            screen: Screen::Home,
            pane: Pane::Sidebar,
            loading: false,
            status: None,
            spinner_frame: 0,
            scroll_position: 0,
            scroll_state: ScrollbarState::default(),
            should_quit: false,
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
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}
