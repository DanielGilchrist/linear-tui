use ratatui::widgets::ListState;

use super::cache::Remote;
use super::display::Display;
use super::feed::{FeedKey, FeedStore};
use super::focus::{Direction, LeftPanel};
use super::team::{TeamMode, TeamSurface};
use crate::api::{IssueSummary, SavedView, Team};

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

#[derive(Debug, Clone)]
pub enum SurfaceSource {
    Saved(SavedView),
    Team(TeamSurface),
}

impl SurfaceSource {
    fn key(&self) -> FeedKey {
        match self {
            SurfaceSource::Saved(saved) => FeedKey::View(saved.id.clone()),
            SurfaceSource::Team(team) => team.key(),
        }
    }

    fn name(&self) -> &str {
        match self {
            SurfaceSource::Saved(saved) => &saved.name,
            SurfaceSource::Team(team) => team.name(),
        }
    }

    fn mode(&self) -> Option<TeamMode> {
        match self {
            SurfaceSource::Saved(_) => None,
            SurfaceSource::Team(team) => Some(team.mode()),
        }
    }

    fn panel(&self) -> LeftPanel {
        match self {
            SurfaceSource::Saved(_) => LeftPanel::SavedViews,
            SurfaceSource::Team(_) => LeftPanel::Teams,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ViewSurface {
    source: SurfaceSource,
    pub display: Display,
    pub state: ListState,
    pub layout: ListState,
}

impl ViewSurface {
    pub fn new(source: SurfaceSource) -> Self {
        Self {
            source,
            display: Display::new(),
            state: ListState::default().with_selected(Some(0)),
            layout: ListState::default(),
        }
    }

    pub fn saved(view: SavedView) -> Self {
        Self::new(SurfaceSource::Saved(view))
    }

    pub fn team(team: &Team) -> Self {
        Self::new(SurfaceSource::Team(TeamSurface::new(team)))
    }

    pub fn name(&self) -> &str {
        self.source.name()
    }

    pub fn mode(&self) -> Option<TeamMode> {
        self.source.mode()
    }

    pub fn panel(&self) -> LeftPanel {
        self.source.panel()
    }

    pub fn cycle_mode(&mut self, direction: Direction) -> bool {
        match &mut self.source {
            SurfaceSource::Team(team) => {
                team.cycle(direction);
                self.state.select(Some(0));
                self.layout = ListState::default();

                true
            }
            SurfaceSource::Saved(_) => false,
        }
    }

    pub fn key(&self) -> FeedKey {
        self.source.key()
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
