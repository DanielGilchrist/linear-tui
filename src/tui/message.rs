use crate::api::{IssueDetail, IssueFilter, IssueSummary, NotificationItem, Session};

#[derive(Debug)]
pub enum Message {
    SessionLoaded(Session),
    IssuesLoaded {
        view: usize,
        issues: Vec<IssueSummary>,
    },
    InboxLoaded {
        view: usize,
        items: Vec<NotificationItem>,
    },
    DetailLoaded(Box<IssueDetail>),
    Failed(String),
}

#[derive(Debug)]
pub enum Command {
    LoadSession,
    LoadIssues { view: usize, filter: IssueFilter },
    LoadInbox { view: usize },
    LoadDetail(String),
}
