use ratatui::widgets::ListState;

use super::cache::{Cache, Remote};
use super::feed::{Feed, FeedStore};
use super::saved_views::{SavedViewsPanel, ViewSurface};
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

impl Default for WorkspaceData {
    fn default() -> Self {
        Self::new()
    }
}
