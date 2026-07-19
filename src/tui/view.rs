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
