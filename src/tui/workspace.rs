use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::widgets::ListState;

use super::cache::{Cache, CacheStatus, Remote};
use super::feed::{Feed, FeedKey, FeedStore};
use super::markdown;
use super::saved_views::{SavedViewsPanel, ViewSurface};
use super::view::{View, ViewKind};
use crate::api::{
    IssueDetail, IssueSummary, NotificationItem, Session, StateOption, Timestamp, User,
};

#[derive(Default)]
pub struct RenderedDetail {
    pub description: Vec<Line<'static>>,
    pub comment_bodies: Vec<Vec<Line<'static>>>,
}

impl RenderedDetail {
    pub fn render(detail: &IssueDetail) -> Self {
        let description = detail
            .description
            .as_deref()
            .filter(|body| !body.is_empty())
            .map(|body| markdown::render(body, Style::default()))
            .unwrap_or_default();

        let comment_bodies = detail
            .threaded_comments()
            .into_iter()
            .map(|threaded| markdown::render(&threaded.comment.body, Style::default()))
            .collect();

        Self {
            description,
            comment_bodies,
        }
    }
}

pub struct WorkspaceData {
    pub session: Option<Session>,
    pub feeds: FeedStore,
    pub inbox: Feed<NotificationItem>,
    detail: Remote<IssueDetail>,
    detail_markdown: RenderedDetail,
    pub states: Cache<String, Remote<Vec<StateOption>>>,
    pub members: Cache<String, Remote<Vec<User>>>,
    pub saved_views: SavedViewsPanel,
    pub view_open: Option<ViewSurface>,
    pub recently_viewed: Vec<IssueSummary>,
    pub recent_state: ListState,
}

impl WorkspaceData {
    pub fn new() -> Self {
        Self {
            session: None,
            feeds: FeedStore::default(),
            inbox: Feed::default(),
            detail: Remote::default(),
            detail_markdown: RenderedDetail::default(),
            states: Cache::default(),
            members: Cache::default(),
            saved_views: SavedViewsPanel::new(),
            view_open: None,
            recently_viewed: Vec::new(),
            recent_state: ListState::default().with_selected(Some(0)),
        }
    }

    pub fn set_detail(&mut self, detail: IssueDetail, now: Timestamp) {
        self.detail_markdown = RenderedDetail::render(&detail);
        self.detail.set(detail, now);
    }

    pub fn bust_detail(&mut self) {
        self.detail.bust();
        self.detail_markdown = RenderedDetail::default();
    }

    pub fn begin_detail(&mut self) {
        self.detail.begin();
    }

    pub fn fail_detail(&mut self, error: String) {
        self.detail.fail(error);
    }

    pub fn detail(&self) -> &Remote<IssueDetail> {
        &self.detail
    }

    pub fn detail_markdown(&self) -> &RenderedDetail {
        &self.detail_markdown
    }
}

impl WorkspaceData {
    pub fn issues_for(&self, view: &View) -> &[IssueSummary] {
        match &view.kind {
            ViewKind::Issues(filter) => self
                .feeds
                .get(&FeedKey::Issues(filter.clone()))
                .map_or(&[], |feed| feed.items()),
            ViewKind::Inbox => &[],
        }
    }

    pub fn feed_status_for(&self, view: &View) -> CacheStatus {
        match &view.kind {
            ViewKind::Issues(filter) => self
                .feeds
                .get(&FeedKey::Issues(filter.clone()))
                .map_or(CacheStatus::Idle, |feed| feed.status().clone()),
            ViewKind::Inbox => self.inbox.status().clone(),
        }
    }

    pub fn appending_for(&self, view: &View) -> bool {
        match &view.kind {
            ViewKind::Issues(filter) => self
                .feeds
                .get(&FeedKey::Issues(filter.clone()))
                .is_some_and(|feed| feed.appending()),
            ViewKind::Inbox => self.inbox.appending(),
        }
    }
}

impl Default for WorkspaceData {
    fn default() -> Self {
        Self::new()
    }
}
