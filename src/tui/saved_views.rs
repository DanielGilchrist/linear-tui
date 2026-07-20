use std::collections::HashMap;

use ratatui::widgets::ListState;

use super::display::Display;
use crate::api::{IssueSummary, SavedView};

pub enum ViewIssues {
    Loading,
    Loaded {
        issues: Vec<IssueSummary>,
        truncated: bool,
    },
    Failed,
}

pub struct SavedViewsPanel {
    pub views: Vec<SavedView>,
    pub state: ListState,
    pub loading: bool,
    pub issues: HashMap<String, ViewIssues>,
}

impl SavedViewsPanel {
    pub fn new() -> Self {
        Self {
            views: Vec::new(),
            state: ListState::default().with_selected(Some(0)),
            loading: true,
            issues: HashMap::new(),
        }
    }

    pub fn selected_view(&self) -> Option<&SavedView> {
        self.state.selected().and_then(|i| self.views.get(i))
    }

    pub fn issues_for(&self, id: &str) -> Option<&ViewIssues> {
        self.issues.get(id)
    }

    pub fn loaded(&self, id: &str) -> Option<&[IssueSummary]> {
        match self.issues.get(id) {
            Some(ViewIssues::Loaded { issues, .. }) => Some(issues),
            _ => None,
        }
    }
}

impl Default for SavedViewsPanel {
    fn default() -> Self {
        Self::new()
    }
}

/// A saved view opened as a right-pane surface: its display options and the
/// selection/scroll cursors over the issues held in the panel cache.
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

    pub fn id(&self) -> &str {
        &self.saved.id
    }

    pub fn name(&self) -> &str {
        &self.saved.name
    }

    pub fn issues<'a>(&self, panel: &'a SavedViewsPanel) -> Option<&'a [IssueSummary]> {
        panel.loaded(self.id())
    }

    pub fn len(&self, panel: &SavedViewsPanel) -> usize {
        self.issues(panel).map_or(0, |issues| issues.len())
    }

    pub fn ordered(&self, panel: &SavedViewsPanel) -> Vec<usize> {
        self.issues(panel)
            .map(|issues| self.display.order(issues))
            .unwrap_or_default()
    }

    pub fn selected_issue<'a>(&self, panel: &'a SavedViewsPanel) -> Option<&'a IssueSummary> {
        let issues = self.issues(panel)?;
        let pos = self.state.selected()?;
        let index = *self.display.order(issues).get(pos)?;
        issues.get(index)
    }
}
