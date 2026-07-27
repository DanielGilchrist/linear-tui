use ratatui::widgets::ListState;

use super::cache::Remote;
use super::display::Display;
use super::feed::{FeedKey, FeedStore};
use crate::api::{IssueSummary, SavedView, ViewId};

pub struct SavedViewsPanel {
    pub views: Remote<Vec<SavedView>>,
    pub state: ListState,
}

impl SavedViewsPanel {
    pub fn new() -> Self {
        Self {
            views: Remote::default(),
            state: ListState::default().with_selected(Some(0)),
        }
    }

    pub fn list(&self) -> &[SavedView] {
        self.views.value().map_or(&[], Vec::as_slice)
    }

    pub fn selected_view(&self) -> Option<&SavedView> {
        self.state.selected().and_then(|i| self.list().get(i))
    }
}

impl Default for SavedViewsPanel {
    fn default() -> Self {
        Self::new()
    }
}

pub struct ViewSurface {
    pub saved: SavedView,
    pub display: Display,
    pub state: ListState,
    pub layout: ListState,
}

impl ViewSurface {
    pub fn new(saved: SavedView) -> Self {
        Self {
            saved,
            display: Display::new(),
            state: ListState::default().with_selected(Some(0)),
            layout: ListState::default(),
        }
    }

    pub fn id(&self) -> &ViewId {
        &self.saved.id
    }

    pub fn name(&self) -> &str {
        &self.saved.name
    }

    pub fn key(&self) -> FeedKey {
        FeedKey::View(self.saved.id.clone())
    }

    pub fn issues<'a>(&self, feeds: &'a FeedStore) -> Option<&'a [IssueSummary]> {
        feeds.get(&self.key()).map(|feed| feed.items())
    }

    pub fn len(&self, feeds: &FeedStore) -> usize {
        self.issues(feeds).map_or(0, |issues| issues.len())
    }

    pub fn ordered(&self, feeds: &FeedStore) -> Vec<usize> {
        self.issues(feeds)
            .map(|issues| self.display.order(issues))
            .unwrap_or_default()
    }

    pub fn selected_issue<'a>(&self, feeds: &'a FeedStore) -> Option<&'a IssueSummary> {
        let issues = self.issues(feeds)?;
        let pos = self.state.selected()?;
        let index = *self.display.order(issues).get(pos)?;
        issues.get(index)
    }
}
