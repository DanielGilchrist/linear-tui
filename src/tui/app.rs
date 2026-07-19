use ratatui::widgets::{ListState, ScrollbarState};

use super::focus::{Focus, LeftPanel, Nav};
use super::overlay::{Confirm, Find, Input, Menu, Overlay, Picker, Prefix, Search};
use super::spinner::Spinner;
use super::view::{View, ViewKind};
use crate::api::{IssueDetail, IssueSummary, NotificationItem, Session};

pub const SCROLL_STEP: usize = 2;

pub const RECENT_CAP: usize = 50;

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

pub struct App {
    pub session: Option<Session>,
    pub views: Vec<View>,
    pub view_state: ListState,
    pub issues: Vec<IssueSummary>,
    pub notifications: Vec<NotificationItem>,
    pub list_state: ListState,
    pub recently_viewed: Vec<IssueSummary>,
    pub recent_state: ListState,
    pub stubs: Vec<StubPanel>,
    pub detail: Option<IssueDetail>,
    pub detail_loading: bool,
    pub focus: Focus,
    pub loading: bool,
    pub status: Option<String>,
    pub spinner: Spinner,
    pub scroll_position: usize,
    pub scroll_state: ScrollbarState,
    pub viewport: usize,
    pub overlay: Overlay,
    pub find_query: Option<String>,
    pub search_return: Option<Search>,
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
            recently_viewed: Vec::new(),
            recent_state: ListState::default().with_selected(Some(0)),
            stubs: vec![
                StubPanel::new(
                    "Saved Views",
                    &["#payroll", "#nest-sync", "High priority", "Created by me"],
                ),
                StubPanel::new("Teams", &["Dan's Pizza", "Dan's Donuts"]),
            ],
            detail: None,
            detail_loading: false,
            focus: Focus::MyWork,
            loading: false,
            status: None,
            spinner: Spinner::default(),
            scroll_position: 0,
            scroll_state: ScrollbarState::default(),
            viewport: 0,
            overlay: Overlay::None,
            find_query: None,
            search_return: None,
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

    pub fn prefix(&self) -> Option<&Prefix> {
        match &self.overlay {
            Overlay::Prefix(prefix) => Some(prefix),
            _ => None,
        }
    }

    pub fn input(&self) -> Option<&Input> {
        match &self.overlay {
            Overlay::Input(input) => Some(input),
            _ => None,
        }
    }

    pub fn search(&self) -> Option<&Search> {
        match &self.overlay {
            Overlay::Search(search) => Some(search),
            _ => None,
        }
    }

    pub fn find(&self) -> Option<&Find> {
        match &self.overlay {
            Overlay::Find(find) => Some(find),
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

    pub fn panels(&self) -> Vec<LeftPanel> {
        let mut panels = vec![LeftPanel::MyWork, LeftPanel::Recent];
        panels.extend((0..self.stubs.len()).map(LeftPanel::Stub));
        panels
    }

    pub fn panel_count(&self) -> usize {
        self.panels().len()
    }

    pub fn panel_at(&self, index: usize) -> LeftPanel {
        self.panels()[index]
    }

    pub fn panel_len(&self, focus: Focus) -> usize {
        match focus {
            Focus::MyWork => self.main_len(),
            Focus::Recent => self.recently_viewed.len(),
            Focus::Stub(index) => self.stubs[index].items.len(),
            Focus::Detail(_) => 0,
        }
    }

    pub fn focused_list_len(&self) -> usize {
        self.panel_len(self.focus)
    }

    pub fn focused_list_mut(&mut self) -> Option<&mut ListState> {
        match self.focus {
            Focus::MyWork => Some(&mut self.list_state),
            Focus::Recent => Some(&mut self.recent_state),
            Focus::Stub(index) => Some(&mut self.stubs[index].state),
            Focus::Detail(_) => None,
        }
    }

    pub fn nav(&mut self) -> Nav<'_> {
        let viewport = self.viewport;
        match self.focus {
            Focus::Detail(_) => Nav::Scroll {
                position: &mut self.scroll_position,
                viewport,
            },
            Focus::MyWork => {
                let len = self.main_len();
                Nav::List {
                    state: &mut self.list_state,
                    len,
                    viewport,
                }
            }
            Focus::Recent => {
                let len = self.recently_viewed.len();
                Nav::List {
                    state: &mut self.recent_state,
                    len,
                    viewport,
                }
            }
            Focus::Stub(index) => {
                let len = self.stubs[index].items.len();
                Nav::List {
                    state: &mut self.stubs[index].state,
                    len,
                    viewport,
                }
            }
        }
    }

    pub fn focused_selection(&self) -> Option<usize> {
        match self.focus {
            Focus::MyWork => self.list_state.selected(),
            Focus::Recent => self.recent_state.selected(),
            Focus::Stub(index) => self.stubs[index].state.selected(),
            Focus::Detail(_) => None,
        }
    }

    pub fn selected_recent(&self) -> Option<&IssueSummary> {
        self.recent_state
            .selected()
            .and_then(|i| self.recently_viewed.get(i))
    }

    pub fn record_recent(&mut self, issue: IssueSummary) {
        let position = match self.recently_viewed.iter().position(|i| i.id == issue.id) {
            Some(position) => position,
            None => {
                self.recently_viewed.insert(0, issue);
                self.recently_viewed.truncate(RECENT_CAP);
                0
            }
        };
        self.recent_state.select(Some(position));
    }

    pub fn open_recent_pos(&self) -> Option<usize> {
        let detail = self.detail.as_ref()?;
        self.recently_viewed.iter().position(|i| i.id == detail.id)
    }

    fn focused_row_texts(&self) -> Vec<String> {
        match self.focus {
            Focus::MyWork => match self.active_view().kind {
                ViewKind::Issues(_) => self.issues.iter().map(issue_search_text).collect(),
                ViewKind::Inbox => self.notifications.iter().map(|n| n.title.clone()).collect(),
            },
            Focus::Recent => self.recently_viewed.iter().map(issue_search_text).collect(),
            Focus::Stub(index) => self.stubs[index].items.clone(),
            Focus::Detail(_) => Vec::new(),
        }
    }

    pub fn focused_matches(&self, query: &str) -> Vec<usize> {
        let needle = query.to_lowercase();
        self.focused_row_texts()
            .iter()
            .enumerate()
            .filter(|(_, text)| text.to_lowercase().contains(&needle))
            .map(|(index, _)| index)
            .collect()
    }

    pub fn open_target(&self) -> Option<FocusedIssue> {
        match self.focus {
            Focus::MyWork => self.selected_issue().map(FocusedIssue::from_summary),
            Focus::Recent => self.selected_recent().map(FocusedIssue::from_summary),
            Focus::Detail(_) => self
                .detail
                .as_ref()
                .map(FocusedIssue::from_detail)
                .or_else(|| self.selected_issue().map(FocusedIssue::from_summary)),
            Focus::Stub(_) => None,
        }
    }

    pub fn action_target(&self) -> Option<FocusedIssue> {
        match self.focus {
            Focus::Detail(_) => self.detail.as_ref().map(FocusedIssue::from_detail),
            Focus::MyWork | Focus::Recent | Focus::Stub(_) => None,
        }
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

fn issue_search_text(issue: &IssueSummary) -> String {
    let mut parts = vec![issue.identifier.clone(), issue.state.name.clone()];
    if let Some(title) = &issue.title {
        parts.push(title.clone());
    }
    if let Some(assignee) = &issue.assignee {
        parts.push(assignee.display_name.clone());
    }
    parts.extend(issue.labels.iter().map(|label| label.name.clone()));
    parts.join(" ")
}
