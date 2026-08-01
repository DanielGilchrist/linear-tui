use ratatui::widgets::ListState;

use super::cache::{CacheStatus, Remote};
use super::feed::{FeedKey, FeedRequest, FeedStore};
use super::focus::{
    navigate_list, scrolled, select_edge, Cursor, DetailFocus, DetailView, Direction, Edge, Focus,
    LeftPanel, Nav, Origin, Scroll, PANELS,
};
use super::message::{Commands, RuntimeCommand};
use super::overlay::{
    AssignOptions, Confirm, Editor, Find, Input, Labels, Menu, Overlay, Picker, PickerKind, Prefix,
    Search, SearchPhase,
};
use super::saved_views::ViewSurface;
use super::spinner::Spinner;
use super::status::Status;
use super::view::{View, ViewKind, Views};
use super::workspace::{TeamsPanel, WorkspaceData};
use crate::api::{
    Credential, IssueDetail, IssueId, IssueRef, IssueSummary, NotificationItem, OAuthToken, Page,
    TeamId, Timestamp,
};
use crate::store::{Account, PersistedCache};

pub const SCROLL_STEP: usize = 2;
pub const RECENT_CAP: usize = 50;
const REFRESH_SKEW_SECS: i64 = 60;
const REFRESH_DEADLINE_SECS: i64 = 60;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthState {
    Authenticated,
    Refreshing { since: Timestamp },
    Unauthenticated,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Active {
    None,
    Authenticated { workspace: String },
    Refreshing { workspace: String, since: Timestamp },
    Unauthenticated { workspace: String },
}

impl Active {
    pub fn workspace(&self) -> Option<&str> {
        match self {
            Active::None => None,
            Active::Authenticated { workspace }
            | Active::Unauthenticated { workspace }
            | Active::Refreshing { workspace, .. } => Some(workspace),
        }
    }
}

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

pub struct PanelRef<'a> {
    pub len: usize,
    pub state: &'a ListState,
    pub in_flight: bool,
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

pub struct Ui {
    pub views: Views,
    pub view_state: ListState,
    pub list_state: ListState,
    pub zoom: Zoom,
    focus: Focus,
    pub status: Option<Status>,
    pub spinner: Spinner,
    pub viewport: usize,
    pub detail_scroll_max: usize,
    overlay: Overlay,
    pub find_query: Option<String>,
}

pub struct SessionState {
    accounts: Vec<Account>,
    active: Active,
}

impl SessionState {
    fn new() -> Self {
        Self {
            accounts: Vec::new(),
            active: Active::None,
        }
    }

    pub fn auth(&self) -> AuthState {
        match &self.active {
            Active::Authenticated { .. } => AuthState::Authenticated,
            Active::Refreshing { since, .. } => AuthState::Refreshing { since: *since },
            Active::None | Active::Unauthenticated { .. } => AuthState::Unauthenticated,
        }
    }

    pub fn active(&self) -> &Active {
        &self.active
    }

    pub fn accounts(&self) -> &[Account] {
        &self.accounts
    }

    pub fn active_workspace(&self) -> Option<&str> {
        self.active.workspace()
    }

    pub fn begin_refresh(&mut self, now: Timestamp) -> bool {
        match &self.active {
            Active::Authenticated { workspace } => {
                self.active = Active::Refreshing {
                    workspace: workspace.clone(),
                    since: now,
                };

                true
            }
            Active::None | Active::Refreshing { .. } | Active::Unauthenticated { .. } => false,
        }
    }

    pub fn authenticated(&mut self) {
        if let Some(workspace) = self.active.workspace() {
            self.active = Active::Authenticated {
                workspace: workspace.to_string(),
            };
        }
    }

    pub fn expired(&mut self) {
        if let Some(workspace) = self.active.workspace() {
            self.active = Active::Unauthenticated {
                workspace: workspace.to_string(),
            };
        }
    }

    pub fn set_accounts(&mut self, accounts: Vec<Account>) {
        self.accounts = accounts;

        if !self.knows_active() {
            self.active = Active::None;
        }
    }

    fn knows_active(&self) -> bool {
        match self.active.workspace() {
            Some(key) => self.accounts.iter().any(|a| a.workspace_key == key),
            None => true,
        }
    }

    pub fn upsert_account(&mut self, account: Account) {
        self.accounts
            .retain(|existing| existing.workspace_key != account.workspace_key);

        self.accounts.push(account);
    }

    #[must_use]
    pub fn activate(&mut self, workspace_key: &str) -> bool {
        let known = self
            .accounts
            .iter()
            .any(|account| account.workspace_key == workspace_key);

        if known {
            self.active = Active::Authenticated {
                workspace: workspace_key.to_string(),
            };
        }

        known
    }

    pub fn active_account(&self) -> Option<&Account> {
        let key = self.active.workspace()?;
        self.accounts
            .iter()
            .find(|account| account.workspace_key == key)
    }

    pub fn set_credential(&mut self, workspace_key: &str, credential: Credential) {
        if let Some(account) = self
            .accounts
            .iter_mut()
            .find(|account| account.workspace_key == workspace_key)
        {
            account.credential = credential;
        }
    }

    pub fn active_oauth(&self) -> Option<&OAuthToken> {
        match &self.active_account()?.credential {
            Credential::OAuth(token) => Some(token),
            Credential::PersonalKey(_) | Credential::EnvVar(_) => None,
        }
    }

    pub fn active_refresh_token(&self) -> Option<String> {
        self.active_oauth()?.refresh_token.clone()
    }

    pub fn refresh_token_for(&self, workspace_key: &str) -> Option<String> {
        let account = self
            .accounts
            .iter()
            .find(|account| account.workspace_key == workspace_key)?;

        match &account.credential {
            Credential::OAuth(token) => token.refresh_token.clone(),
            Credential::PersonalKey(_) | Credential::EnvVar(_) => None,
        }
    }
}

pub struct App {
    pub ui: Ui,
    pub workspace: WorkspaceData,
    pub session: SessionState,
    pub now: Timestamp,
    pub should_quit: bool,
}

impl App {
    pub fn new() -> Self {
        Self {
            ui: Ui {
                views: Views::defaults(),
                view_state: ListState::default().with_selected(Some(0)),
                list_state: ListState::default().with_selected(Some(0)),
                zoom: Zoom::Normal,
                focus: Focus::MyWork,
                status: None,
                spinner: Spinner::default(),
                viewport: 0,
                detail_scroll_max: 0,
                overlay: Overlay::None,
                find_query: None,
            },
            workspace: WorkspaceData::new(),
            session: SessionState::new(),
            now: Timestamp::now(),
            should_quit: false,
        }
    }

    pub fn overlay(&self) -> &Overlay {
        &self.ui.overlay
    }

    pub fn set_overlay(&mut self, overlay: Overlay) {
        self.ui.overlay = overlay;
    }

    pub fn take_overlay(&mut self) -> Overlay {
        std::mem::take(&mut self.ui.overlay)
    }

    pub fn picker_mut(&mut self) -> Option<&mut Picker> {
        match &mut self.ui.overlay {
            Overlay::Picker(picker) => Some(picker),
            _ => None,
        }
    }

    pub fn editor_mut(&mut self) -> Option<&mut Editor> {
        match &mut self.ui.overlay {
            Overlay::Editor(editor) => Some(editor),
            _ => None,
        }
    }

    pub fn search_mut(&mut self) -> Option<&mut Search> {
        match &mut self.ui.overlay {
            Overlay::Search(search) => Some(search),
            _ => None,
        }
    }

    pub fn labels_mut(&mut self) -> Option<&mut Labels> {
        match &mut self.ui.overlay {
            Overlay::Labels(labels) => Some(labels),
            _ => None,
        }
    }

    pub fn apply_feed(
        &mut self,
        key: &FeedKey,
        request: &FeedRequest,
        page: Page<IssueSummary>,
    ) -> bool {
        self.workspace
            .feeds
            .get_or_default(key)
            .apply(request, page, self.now)
    }

    pub fn apply_inbox(&mut self, request: &FeedRequest, page: Page<NotificationItem>) -> bool {
        self.workspace.inbox.apply(request, page, self.now)
    }

    pub fn persisted_cache(&self) -> PersistedCache {
        crate::store::build_cache(&self.workspace.feeds, &self.workspace.inbox, self.now)
    }

    pub fn reset_workspace(&mut self) {
        self.workspace = WorkspaceData::new();

        let Ui {
            views: _,
            view_state,
            list_state,
            zoom,
            focus,
            status,
            spinner: _,
            viewport: _,
            detail_scroll_max: _,
            overlay,
            find_query,
        } = &mut self.ui;

        *focus = Focus::MyWork;
        *overlay = Overlay::None;
        list_state.select(Some(0));
        view_state.select(Some(0));
        *find_query = None;
        *zoom = Zoom::Normal;
        *status = None;
    }

    pub fn maybe_refresh_token(&mut self) -> Commands {
        if self.session.auth() != AuthState::Authenticated {
            return Commands::default();
        }

        let Some(token) = self.session.active_oauth() else {
            return Commands::default();
        };

        let expiring =
            token.refresh_token.is_some() && token.is_expiring(self.now.epoch(), REFRESH_SKEW_SECS);

        let Some(workspace_key) = self.session.active_workspace().map(str::to_string) else {
            return Commands::default();
        };

        if expiring && self.session.begin_refresh(self.now) {
            Commands::runtime(RuntimeCommand::RefreshToken { workspace_key })
        } else {
            Commands::default()
        }
    }

    pub fn expire_stuck_refresh(&mut self) -> bool {
        let AuthState::Refreshing { since } = self.session.auth() else {
            return false;
        };

        if self.now.seconds_since(since) > REFRESH_DEADLINE_SECS {
            self.session.expired();
            return true;
        }

        false
    }

    pub fn picker(&self) -> Option<&Picker> {
        match &self.ui.overlay {
            Overlay::Picker(picker) => Some(picker),
            _ => None,
        }
    }

    pub fn confirm(&self) -> Option<&Confirm> {
        match &self.ui.overlay {
            Overlay::Confirm(confirm) => Some(confirm),
            _ => None,
        }
    }

    pub fn menu(&self) -> Option<&Menu> {
        match &self.ui.overlay {
            Overlay::Menu(menu) => Some(menu),
            _ => None,
        }
    }

    pub fn prefix(&self) -> Option<&Prefix> {
        match &self.ui.overlay {
            Overlay::Prefix(prefix) => Some(prefix),
            _ => None,
        }
    }

    pub fn input(&self) -> Option<&Input> {
        match &self.ui.overlay {
            Overlay::Input(input) => Some(input),
            _ => None,
        }
    }

    pub fn view(&self) -> Option<&ViewSurface> {
        self.ui.focus.open_view()
    }

    pub fn view_mut(&mut self) -> Option<&mut ViewSurface> {
        self.ui.focus.open_view_mut()
    }

    pub fn view_render_parts(&mut self) -> (Option<&mut ViewSurface>, &FeedStore) {
        (self.ui.focus.open_view_mut(), &self.workspace.feeds)
    }

    pub fn open_view_surface(&mut self, surface: ViewSurface) {
        self.ui.focus = Focus::View(Box::new(surface));
    }

    pub fn close_view_surface(&mut self) {
        if self.ui.focus.is_view() {
            self.ui.focus = Focus::SavedViews;
        }
    }

    pub fn take_origin(&mut self) -> Origin {
        match std::mem::replace(&mut self.ui.focus, Focus::MyWork) {
            Focus::View(surface) => Origin::View(surface),
            Focus::Detail(detail) => detail.origin,
            Focus::MyWork => Origin::Panel(LeftPanel::MyWork),
            Focus::Recent => Origin::Panel(LeftPanel::Recent),
            Focus::SavedViews => Origin::Panel(LeftPanel::SavedViews),
            Focus::Teams => Origin::Panel(LeftPanel::Teams),
        }
    }

    pub fn focus(&self) -> &Focus {
        &self.ui.focus
    }

    pub fn focus_panel(&mut self, panel: LeftPanel) {
        self.ui.focus = panel.focus();
    }

    pub fn focus_my_work(&mut self) {
        self.ui.focus = Focus::MyWork;
    }

    pub fn focus_current_panel(&mut self) {
        self.ui.focus = self.ui.focus.left().focus();
    }

    pub fn open_detail_focus(&mut self, focus: DetailFocus) {
        self.ui.focus = Focus::Detail(focus);
    }

    pub fn set_detail_view(&mut self, view: DetailView) {
        if let Focus::Detail(detail) = &self.ui.focus {
            self.ui.focus = Focus::Detail(detail.with_view(view));
        }
    }

    pub fn refocus_detail_issue(&mut self, issue: IssueRef) {
        if let Focus::Detail(detail) = &self.ui.focus {
            self.ui.focus = Focus::Detail(DetailFocus {
                issue,
                ..detail.clone()
            });
        }
    }

    pub fn view_issues(&self) -> Option<&[IssueSummary]> {
        self.ui.focus.open_view()?.issues(&self.workspace.feeds)
    }

    pub fn view_len(&self) -> usize {
        self.ui
            .focus
            .open_view()
            .map_or(0, |view| view.len(&self.workspace.feeds))
    }

    pub fn view_ordered(&self) -> Vec<usize> {
        self.ui
            .focus
            .open_view()
            .map(|view| view.ordered(&self.workspace.feeds))
            .unwrap_or_default()
    }

    pub fn view_selected_issue(&self) -> Option<&IssueSummary> {
        self.ui
            .focus
            .open_view()?
            .selected_issue(&self.workspace.feeds)
    }

    pub fn editor(&self) -> Option<&Editor> {
        match &self.ui.overlay {
            Overlay::Editor(editor) => Some(editor),
            _ => None,
        }
    }

    pub fn search(&self) -> Option<&Search> {
        match &self.ui.overlay {
            Overlay::Search(search) => Some(search),
            _ => None,
        }
    }

    pub fn labels(&self) -> Option<&Labels> {
        match &self.ui.overlay {
            Overlay::Labels(labels) => Some(labels),
            _ => None,
        }
    }

    pub fn find(&self) -> Option<&Find> {
        match &self.ui.overlay {
            Overlay::Find(find) => Some(find),
            _ => None,
        }
    }

    pub fn active_view_index(&self) -> usize {
        self.ui.view_state.selected().unwrap_or(0)
    }

    pub fn active_view(&self) -> &View {
        self.ui.views.active(&self.ui.view_state)
    }

    pub fn teams(&self) -> &TeamsPanel {
        &self.workspace.teams
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
        self.ui
            .list_state
            .selected()
            .and_then(|i| self.active_issues().get(i))
    }

    pub fn selected_notification(&self) -> Option<&NotificationItem> {
        self.ui
            .list_state
            .selected()
            .and_then(|i| self.workspace.inbox.items().get(i))
    }

    fn active_in_flight(&self) -> bool {
        match self.active_feed_key() {
            Some(key) => self.feed_in_flight(&key),
            None => self.workspace.inbox.in_flight(),
        }
    }

    pub fn overlay_in_flight(&self) -> bool {
        match &self.ui.overlay {
            Overlay::Picker(picker) => self.picker_in_flight(picker),
            Overlay::Labels(labels) => labels.results.is_loading(),
            Overlay::None
            | Overlay::Confirm(_)
            | Overlay::Menu(_)
            | Overlay::Prefix(_)
            | Overlay::Input(_)
            | Overlay::Editor(_)
            | Overlay::Search(_)
            | Overlay::Find(_)
            | Overlay::Reactions(_)
            | Overlay::Workspaces(_) => false,
        }
    }

    fn picker_in_flight(&self, picker: &Picker) -> bool {
        match &picker.kind {
            PickerKind::Status => self
                .workspace
                .states
                .get(&picker.target_team)
                .is_some_and(Remote::in_flight),
            PickerKind::Assign(AssignOptions::Suggested) => self
                .workspace
                .members
                .get(&picker.target_team)
                .is_some_and(Remote::in_flight),
            PickerKind::Assign(AssignOptions::Matching { phase, .. }) => {
                *phase == SearchPhase::InFlight
            }
            PickerKind::Priority => false,
        }
    }

    pub fn cancel_overlay_in_flight(&mut self) {
        match &mut self.ui.overlay {
            Overlay::Picker(picker) => picker.settle_search(),
            Overlay::Labels(labels) => labels.settle(),
            Overlay::None
            | Overlay::Confirm(_)
            | Overlay::Menu(_)
            | Overlay::Prefix(_)
            | Overlay::Input(_)
            | Overlay::Editor(_)
            | Overlay::Search(_)
            | Overlay::Find(_)
            | Overlay::Reactions(_)
            | Overlay::Workspaces(_) => {}
        }
    }

    pub fn is_loading(&self) -> bool {
        if self.overlay_in_flight() {
            return true;
        }

        match &self.ui.focus {
            Focus::MyWork => self.panel(LeftPanel::MyWork).in_flight,
            Focus::Recent => self.panel(LeftPanel::Recent).in_flight,
            Focus::SavedViews => self.panel(LeftPanel::SavedViews).in_flight,
            Focus::Teams => self.panel(LeftPanel::Teams).in_flight,
            Focus::View(view) => self.feed_in_flight(&view.key()),
            Focus::Detail(..) => self.workspace.detail().in_flight(),
        }
    }

    pub fn open_detail(&self) -> Option<&IssueDetail> {
        let detail_focus = self.ui.focus.detail()?;

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

    pub fn comment_cursor(&self) -> Option<usize> {
        match self.ui.focus.detail()?.view {
            DetailView::Comments { at } => Some(at.index()),
            DetailView::Reading { .. } => None,
        }
    }

    pub fn reading_scroll(&self) -> Option<Scroll> {
        match self.ui.focus.detail()?.view {
            DetailView::Reading { scroll } => Some(scroll),
            DetailView::Comments { .. } => None,
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

    pub fn panel_at(&self, index: usize) -> Option<LeftPanel> {
        PANELS.get(index).copied()
    }

    pub fn panel(&self, panel: LeftPanel) -> PanelRef<'_> {
        match panel {
            LeftPanel::MyWork => PanelRef {
                len: self.main_len(),
                state: &self.ui.list_state,
                in_flight: self.active_in_flight(),
            },
            LeftPanel::Recent => PanelRef {
                len: self.workspace.recently_viewed.len(),
                state: &self.workspace.recent_state,
                in_flight: false,
            },
            LeftPanel::SavedViews => PanelRef {
                len: self.workspace.saved_views.list().len(),
                state: &self.workspace.saved_views.state,
                in_flight: self.workspace.saved_views.views.in_flight(),
            },
            LeftPanel::Teams => PanelRef {
                len: self.workspace.teams.list().len(),
                state: &self.workspace.teams.state,
                in_flight: self.workspace.teams.teams.in_flight(),
            },
        }
    }

    pub fn panel_len(&self, focus: &Focus) -> usize {
        match focus {
            Focus::MyWork => self.panel(LeftPanel::MyWork).len,
            Focus::Recent => self.panel(LeftPanel::Recent).len,
            Focus::SavedViews => self.panel(LeftPanel::SavedViews).len,
            Focus::Teams => self.panel(LeftPanel::Teams).len,
            Focus::View(_) => self.view_len(),
            Focus::Detail(..) => 0,
        }
    }

    pub fn focused_list_len(&self) -> usize {
        self.panel_len(&self.ui.focus)
    }

    fn nav(&mut self) -> Nav<'_> {
        let viewport = self.ui.viewport;
        let scroll_max = self.ui.detail_scroll_max;
        let comment_len = self.open_detail().map_or(0, |detail| detail.thread_len());
        let view_len = self.view_len();
        let main_len = self.panel(LeftPanel::MyWork).len;
        let recent_len = self.panel(LeftPanel::Recent).len;
        let saved_len = self.panel(LeftPanel::SavedViews).len;
        let teams_len = self.panel(LeftPanel::Teams).len;

        match &mut self.ui.focus {
            Focus::Detail(DetailFocus {
                view: DetailView::Reading { scroll },
                ..
            }) => Nav::Scroll {
                scroll,
                viewport,
                max: scroll_max,
            },
            Focus::Detail(DetailFocus {
                view: DetailView::Comments { at },
                ..
            }) => Nav::Comments {
                at,
                len: comment_len,
                viewport,
            },
            Focus::MyWork => Nav::List {
                state: &mut self.ui.list_state,
                len: main_len,
                viewport,
            },
            Focus::Recent => Nav::List {
                state: &mut self.workspace.recent_state,
                len: recent_len,
                viewport,
            },
            Focus::SavedViews => Nav::List {
                state: &mut self.workspace.saved_views.state,
                len: saved_len,
                viewport,
            },
            Focus::View(view) => Nav::List {
                state: &mut view.state,
                len: view_len,
                viewport,
            },
            Focus::Teams => Nav::List {
                state: &mut self.workspace.teams.state,
                len: teams_len,
                viewport,
            },
        }
    }

    pub fn focused_selection(&self) -> Option<usize> {
        match self.ui.focus {
            Focus::MyWork => self.panel(LeftPanel::MyWork).state.selected(),
            Focus::Recent => self.panel(LeftPanel::Recent).state.selected(),
            Focus::SavedViews => self.panel(LeftPanel::SavedViews).state.selected(),
            Focus::Teams => self.panel(LeftPanel::Teams).state.selected(),
            Focus::View(_) => self.view().and_then(|view| view.state.selected()),
            Focus::Detail(DetailFocus {
                view: DetailView::Comments { at },
                ..
            }) => Some(at.index()),
            Focus::Detail(DetailFocus {
                view: DetailView::Reading { scroll },
                ..
            }) => Some(scroll.line().unwrap_or(self.ui.detail_scroll_max)),
        }
    }

    pub fn reveal_focused(&mut self, index: Option<usize>) {
        match self.nav() {
            Nav::List { state, len, .. } => {
                state.select(index.map(|index| index.min(len.saturating_sub(1))));
            }
            Nav::Scroll { scroll, .. } => {
                *scroll = index.map_or(Scroll::Top, Scroll::At);
            }
            Nav::Comments { at, len, .. } => {
                if let Some(cursor) = index.and_then(|index| Cursor::new(index, len)) {
                    *at = cursor;
                }
            }
        }
    }

    pub fn step_selection(&mut self, direction: Direction) {
        match self.nav() {
            Nav::List { state, len, .. } => navigate_list(state, len, direction),
            Nav::Scroll { scroll, max, .. } => {
                *scroll = scrolled(*scroll, SCROLL_STEP, direction, max);
            }
            Nav::Comments { at, len, .. } => {
                *at = at.stepped(len, direction);
            }
        }
    }

    pub fn scroll_half_page(&mut self, direction: Direction) {
        match self.nav() {
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
            Nav::Scroll {
                scroll,
                viewport,
                max,
            } => {
                *scroll = scrolled(*scroll, (viewport / 2).max(1), direction, max);
            }
            Nav::Comments { at, len, viewport } => {
                if len == 0 {
                    return;
                }

                let step = (viewport / 2).max(1);
                let next = match direction {
                    Direction::Next => (at.index() + step).min(len - 1),
                    Direction::Prev => at.index().saturating_sub(step),
                };

                *at = Cursor::new(next, len).unwrap_or(*at);
            }
        }
    }

    pub fn jump_to_edge(&mut self, edge: Edge) {
        match self.nav() {
            Nav::List { state, len, .. } => select_edge(state, len, edge),
            Nav::Scroll { scroll, .. } => {
                *scroll = match edge {
                    Edge::Bottom => Scroll::Bottom,
                    Edge::Top => Scroll::Top,
                };
            }
            Nav::Comments { at, len, .. } => {
                if len > 0 {
                    *at = Cursor::edge(len, edge);
                }
            }
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

    pub fn clear_transient_status(&mut self) {
        if matches!(self.ui.status, Some(Status::Error(_))) {
            return;
        }

        self.ui.status = None;
    }

    pub fn merge_recent(&mut self, loaded: Vec<IssueSummary>) {
        for issue in loaded {
            let known = self
                .workspace
                .recently_viewed
                .iter()
                .any(|existing| existing.id == issue.id);

            if !known {
                self.workspace.recently_viewed.push(issue);
            }
        }

        self.workspace.recently_viewed.truncate(RECENT_CAP);
    }

    pub fn open_recent_pos(&self) -> Option<usize> {
        let detail_focus = self.ui.focus.detail()?;

        self.workspace
            .recently_viewed
            .iter()
            .position(|issue| detail_focus.issue.matches_summary(issue))
    }

    pub fn focused_row_texts(&self) -> Vec<String> {
        match self.ui.focus {
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
            Focus::View(_) => match self.view_issues() {
                Some(issues) => self
                    .view_ordered()
                    .iter()
                    .filter_map(|&index| issues.get(index))
                    .map(issue_search_text)
                    .collect(),
                None => Vec::new(),
            },
            Focus::Teams => self.workspace.teams.names(),
            Focus::Detail(DetailFocus {
                view: DetailView::Comments { .. },
                ..
            }) => match self.open_detail() {
                Some(detail) => detail
                    .threaded_comments()
                    .iter()
                    .map(|threaded| comment_search_text(threaded.comment))
                    .collect(),
                None => Vec::new(),
            },
            Focus::Detail(DetailFocus {
                view: DetailView::Reading { .. },
                ..
            }) => match self.open_detail() {
                Some(detail) => super::render::detail_line_texts(
                    detail,
                    self.workspace.detail_markdown(),
                    self.now,
                    None,
                ),
                None => Vec::new(),
            },
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
        match &self.ui.focus {
            Focus::MyWork => self.selected_issue().map(FocusedIssue::from_summary),
            Focus::Recent => self.selected_recent().map(FocusedIssue::from_summary),
            Focus::SavedViews => None,
            Focus::View(_) => self.view_selected_issue().map(FocusedIssue::from_summary),
            Focus::Detail(detail) => self
                .open_detail()
                .map(FocusedIssue::from_detail)
                .or_else(|| detail.summary.as_deref().map(FocusedIssue::from_summary)),
            Focus::Teams => None,
        }
    }

    pub fn action_target(&self) -> Option<FocusedIssue> {
        match self.ui.focus {
            Focus::Detail(..) => self.open_detail().map(FocusedIssue::from_detail),
            Focus::View(_) => self.view_selected_issue().map(FocusedIssue::from_summary),
            Focus::MyWork | Focus::Recent | Focus::SavedViews | Focus::Teams => None,
        }
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

fn comment_search_text(comment: &crate::api::Comment) -> String {
    match &comment.author {
        Some(author) => format!("{author} {}", comment.body),
        None => comment.body.clone(),
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
