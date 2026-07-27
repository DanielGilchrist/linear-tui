use ratatui::widgets::{ListState, ScrollbarState};

use super::cache::CacheStatus;
use super::feed::FeedKey;
use super::focus::{DetailFocus, DetailView, Focus, LeftPanel, Nav};
use super::overlay::{Confirm, Editor, Find, Input, Menu, Overlay, Picker, Prefix, Search};
use super::saved_views::ViewSurface;
use super::spinner::Spinner;
use super::status::Status;
use super::view::{View, ViewKind};
use super::workspace::WorkspaceData;
use crate::api::{IssueDetail, IssueId, IssueSummary, NotificationItem, TeamId, Timestamp};
use crate::store::Account;

pub const SCROLL_STEP: usize = 2;

pub const RECENT_CAP: usize = 50;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Zoom {
    Normal,
    Full,
}

impl Zoom {
    pub fn toggle(self) -> Self {
        match self {
            Zoom::Normal => Zoom::Full,
            Zoom::Full => Zoom::Normal,
        }
    }
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

pub struct FocusedIssue {
    pub id: IssueId,
    pub identifier: String,
    pub team_id: TeamId,
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
    pub workspace: WorkspaceData,
    pub views: Vec<View>,
    pub view_state: ListState,
    pub list_state: ListState,
    pub comment_state: ListState,
    pub zoom: Zoom,
    pub stubs: Vec<StubPanel>,
    pub focus: Focus,
    pub status: Option<Status>,
    pub spinner: Spinner,
    pub scroll_position: usize,
    pub scroll_state: ScrollbarState,
    pub viewport: usize,
    pub overlay: Overlay,
    pub find_query: Option<String>,
    pub search_return: Option<Search>,
    pub now: Timestamp,
    pub time_refresh_due: Option<Timestamp>,
    pub accounts: Vec<Account>,
    pub active_workspace: Option<String>,
    pub should_quit: bool,
}

impl App {
    pub fn new() -> Self {
        Self {
            workspace: WorkspaceData::new(),
            views: View::defaults(),
            view_state: ListState::default().with_selected(Some(0)),
            list_state: ListState::default().with_selected(Some(0)),
            comment_state: ListState::default().with_selected(Some(0)),
            zoom: Zoom::Normal,
            stubs: vec![StubPanel::new("Teams", &["Dan's Pizza", "Dan's Donuts"])],
            focus: Focus::MyWork,
            status: None,
            spinner: Spinner::default(),
            scroll_position: 0,
            scroll_state: ScrollbarState::default(),
            viewport: 0,
            overlay: Overlay::None,
            find_query: None,
            search_return: None,
            now: Timestamp::now(),
            time_refresh_due: None,
            accounts: Vec::new(),
            active_workspace: None,
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

    pub fn view(&self) -> Option<&ViewSurface> {
        self.workspace.view_open.as_ref()
    }

    pub fn open_view_surface(&mut self, surface: ViewSurface) {
        self.workspace.view_open = Some(surface);
        self.focus = Focus::View;
    }

    pub fn close_view_surface(&mut self) {
        self.workspace.view_open = None;
        if self.focus == Focus::View {
            self.focus = Focus::SavedViews;
        }
    }

    pub fn view_issues(&self) -> Option<&[IssueSummary]> {
        self.workspace
            .view_open
            .as_ref()?
            .issues(&self.workspace.feeds)
    }

    pub fn view_len(&self) -> usize {
        self.workspace
            .view_open
            .as_ref()
            .map_or(0, |view| view.len(&self.workspace.feeds))
    }

    pub fn view_ordered(&self) -> Vec<usize> {
        self.workspace
            .view_open
            .as_ref()
            .map(|view| view.ordered(&self.workspace.feeds))
            .unwrap_or_default()
    }

    pub fn view_selected_issue(&self) -> Option<&IssueSummary> {
        self.workspace
            .view_open
            .as_ref()?
            .selected_issue(&self.workspace.feeds)
    }

    pub fn editor(&self) -> Option<&Editor> {
        match &self.overlay {
            Overlay::Editor(editor) => Some(editor),
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

    pub fn active_feed_key(&self) -> Option<FeedKey> {
        match &self.active_view().kind {
            ViewKind::Issues(filter) => Some(FeedKey::Issues(filter.clone())),
            ViewKind::Inbox => None,
        }
    }

    pub fn active_issues(&self) -> &[IssueSummary] {
        self.workspace.issues_for(self.active_view())
    }

    pub fn active_feed_status(&self) -> CacheStatus {
        self.workspace.feed_status_for(self.active_view())
    }

    fn feed_in_flight(&self, key: &FeedKey) -> bool {
        match self.workspace.feeds.get(key) {
            Some(feed) => feed.in_flight(),
            None => false,
        }
    }

    pub fn active_appending(&self) -> bool {
        self.workspace.appending_for(self.active_view())
    }

    pub fn main_len(&self) -> usize {
        match self.active_view().kind {
            ViewKind::Issues(_) => self.active_issues().len(),
            ViewKind::Inbox => self.workspace.inbox.items().len(),
        }
    }

    pub fn selected_issue(&self) -> Option<&IssueSummary> {
        self.list_state
            .selected()
            .and_then(|i| self.active_issues().get(i))
    }

    pub fn selected_notification(&self) -> Option<&NotificationItem> {
        self.list_state
            .selected()
            .and_then(|i| self.workspace.inbox.items().get(i))
    }

    fn active_in_flight(&self) -> bool {
        match self.active_feed_key() {
            Some(key) => self.feed_in_flight(&key),
            None => self.workspace.inbox.in_flight(),
        }
    }

    pub fn is_loading(&self) -> bool {
        match self.focus {
            Focus::MyWork => self.active_in_flight(),
            Focus::Recent | Focus::Stub(_) => false,
            Focus::SavedViews => self.workspace.saved_views.views.in_flight(),
            Focus::View => match self.workspace.view_open.as_ref() {
                Some(view) => self.feed_in_flight(&view.key()),
                None => false,
            },
            Focus::Detail(..) => self.workspace.detail().in_flight(),
        }
    }

    pub fn open_detail(&self) -> Option<&IssueDetail> {
        let detail_focus = self.focus.detail()?;

        self.workspace
            .detail()
            .value()
            .filter(|detail| detail_focus.issue.matches_detail(detail))
    }

    pub fn has_comments(&self) -> bool {
        match self.open_detail() {
            Some(detail) => !detail.comments.is_empty(),
            None => false,
        }
    }

    pub fn search_results(&self, query: &str) -> &[IssueSummary] {
        match self
            .workspace
            .feeds
            .get(&FeedKey::Search(query.to_string()))
        {
            Some(feed) => feed.items(),
            None => &[],
        }
    }

    pub fn panels(&self) -> Vec<LeftPanel> {
        let mut panels = vec![LeftPanel::MyWork, LeftPanel::Recent, LeftPanel::SavedViews];
        panels.extend((0..self.stubs.len()).map(LeftPanel::Stub));
        panels
    }

    pub fn panel_count(&self) -> usize {
        self.panels().len()
    }

    pub fn panel_at(&self, index: usize) -> LeftPanel {
        self.panels()[index]
    }

    pub fn panel_len(&self, focus: &Focus) -> usize {
        match focus {
            Focus::MyWork => self.main_len(),
            Focus::Recent => self.workspace.recently_viewed.len(),
            Focus::SavedViews => self.workspace.saved_views.list().len(),
            Focus::View => self.view_len(),
            Focus::Stub(index) => self.stubs[*index].items.len(),
            Focus::Detail(..) => 0,
        }
    }

    pub fn focused_list_len(&self) -> usize {
        self.panel_len(&self.focus)
    }

    pub fn focused_list_mut(&mut self) -> Option<&mut ListState> {
        match self.focus {
            Focus::MyWork => Some(&mut self.list_state),
            Focus::Recent => Some(&mut self.workspace.recent_state),
            Focus::SavedViews => Some(&mut self.workspace.saved_views.state),
            Focus::View => self
                .workspace
                .view_open
                .as_mut()
                .map(|view| &mut view.state),
            Focus::Stub(index) => Some(&mut self.stubs[index].state),
            Focus::Detail(..) => None,
        }
    }

    pub fn nav(&mut self) -> Nav<'_> {
        let viewport = self.viewport;
        match self.focus {
            Focus::Detail(DetailFocus {
                view: DetailView::Reading,
                ..
            }) => Nav::Scroll {
                position: &mut self.scroll_position,
                viewport,
            },
            Focus::Detail(DetailFocus {
                view: DetailView::Comments,
                ..
            }) => {
                let len = self.open_detail().map_or(0, |detail| detail.comments.len());

                Nav::List {
                    state: &mut self.comment_state,
                    len,
                    viewport,
                }
            }
            Focus::MyWork => {
                let len = self.main_len();
                Nav::List {
                    state: &mut self.list_state,
                    len,
                    viewport,
                }
            }
            Focus::Recent => {
                let len = self.workspace.recently_viewed.len();

                Nav::List {
                    state: &mut self.workspace.recent_state,
                    len,
                    viewport,
                }
            }
            Focus::SavedViews => {
                let len = self.workspace.saved_views.list().len();

                Nav::List {
                    state: &mut self.workspace.saved_views.state,
                    len,
                    viewport,
                }
            }
            Focus::View => {
                let len = self.view_len();
                let panel_len = self.workspace.saved_views.list().len();

                match self.workspace.view_open.as_mut() {
                    Some(view) => Nav::List {
                        state: &mut view.state,
                        len,
                        viewport,
                    },
                    None => Nav::List {
                        state: &mut self.workspace.saved_views.state,
                        len: panel_len,
                        viewport,
                    },
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
            Focus::Recent => self.workspace.recent_state.selected(),
            Focus::SavedViews => self.workspace.saved_views.state.selected(),
            Focus::View => self
                .workspace
                .view_open
                .as_ref()
                .and_then(|view| view.state.selected()),
            Focus::Stub(index) => self.stubs[index].state.selected(),
            Focus::Detail(..) => None,
        }
    }

    pub fn selected_recent(&self) -> Option<&IssueSummary> {
        self.workspace
            .recent_state
            .selected()
            .and_then(|i| self.workspace.recently_viewed.get(i))
    }

    pub fn record_recent(&mut self, issue: IssueSummary) {
        let position = match self
            .workspace
            .recently_viewed
            .iter()
            .position(|i| i.id == issue.id)
        {
            Some(position) => position,
            None => {
                self.workspace.recently_viewed.insert(0, issue);
                self.workspace.recently_viewed.truncate(RECENT_CAP);
                0
            }
        };

        self.workspace.recent_state.select(Some(position));
    }

    pub fn open_recent_pos(&self) -> Option<usize> {
        let detail_focus = self.focus.detail()?;

        self.workspace
            .recently_viewed
            .iter()
            .position(|issue| detail_focus.issue.matches_summary(issue))
    }

    fn focused_row_texts(&self) -> Vec<String> {
        match self.focus {
            Focus::MyWork => match self.active_view().kind {
                ViewKind::Issues(_) => self.active_issues().iter().map(issue_search_text).collect(),
                ViewKind::Inbox => self
                    .workspace
                    .inbox
                    .items()
                    .iter()
                    .map(|n| n.title.clone())
                    .collect(),
            },
            Focus::Recent => self
                .workspace
                .recently_viewed
                .iter()
                .map(issue_search_text)
                .collect(),
            Focus::SavedViews => self
                .workspace
                .saved_views
                .list()
                .iter()
                .map(|v| v.name.clone())
                .collect(),
            Focus::View => match self.view_issues() {
                Some(issues) => self
                    .view_ordered()
                    .iter()
                    .filter_map(|&index| issues.get(index))
                    .map(issue_search_text)
                    .collect(),
                None => Vec::new(),
            },
            Focus::Stub(index) => self.stubs[index].items.clone(),
            Focus::Detail(..) => Vec::new(),
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
            Focus::SavedViews => None,
            Focus::View => self.view_selected_issue().map(FocusedIssue::from_summary),
            Focus::Detail(..) => self
                .open_detail()
                .map(FocusedIssue::from_detail)
                .or_else(|| self.selected_issue().map(FocusedIssue::from_summary)),
            Focus::Stub(_) => None,
        }
    }

    pub fn action_target(&self) -> Option<FocusedIssue> {
        match self.focus {
            Focus::Detail(..) => self.open_detail().map(FocusedIssue::from_detail),
            Focus::View => self.view_selected_issue().map(FocusedIssue::from_summary),
            Focus::MyWork | Focus::Recent | Focus::SavedViews | Focus::Stub(_) => None,
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
