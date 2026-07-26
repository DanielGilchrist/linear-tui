use ratatui::widgets::ListState;

use super::cache::{Cache, CacheStatus, Remote};
use super::feed::{Feed, FeedKey, FeedStore};
use super::saved_views::{SavedViewsPanel, ViewSurface};
use super::view::{View, ViewKind};
use crate::api::{IssueDetail, IssueSummary, NotificationItem, Session, StateOption, User};

pub struct WorkspaceData {
    pub session: Option<Session>,
    pub feeds: FeedStore,
    pub inbox: Feed<NotificationItem>,
    pub detail: Remote<IssueDetail>,
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
            states: Cache::default(),
            members: Cache::default(),
            saved_views: SavedViewsPanel::new(),
            view_open: None,
            recently_viewed: Vec::new(),
            recent_state: ListState::default().with_selected(Some(0)),
        }
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
