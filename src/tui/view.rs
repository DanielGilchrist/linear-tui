use std::num::NonZeroUsize;

use ratatui::widgets::ListState;

use crate::api::IssueFilter;

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

#[derive(Debug, Clone)]
pub struct Views(Vec<View>);

impl Views {
    pub fn defaults() -> Self {
        Views(View::defaults())
    }

    pub fn active(&self, state: &ListState) -> &View {
        let index = state.selected().unwrap_or(0).min(self.0.len() - 1);

        &self.0[index]
    }

    pub fn as_slice(&self) -> &[View] {
        &self.0
    }

    pub fn len(&self) -> NonZeroUsize {
        NonZeroUsize::MIN.saturating_add(self.0.len() - 1)
    }
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
