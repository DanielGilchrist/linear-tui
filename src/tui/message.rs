use super::overlay::PickerItem;
use crate::api::{IssueDetail, IssueFilter, IssueSummary, IssueUpdate, NotificationItem, Session};

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
    SearchResults(Vec<IssueSummary>),
    RecentLoaded(Vec<IssueSummary>),
    RecentCleared,
    PickerLoaded(Vec<PickerItem>),
    IssueUpdated {
        id: String,
    },
    Status(String),
    Failed(String),
}

#[derive(Debug, Clone)]
pub enum Command {
    LoadSession,
    LoadIssues { view: usize, filter: IssueFilter },
    LoadInbox { view: usize },
    LoadDetail(String),
    LoadStates { team_id: String },
    LoadMembers { team_id: String },
    UpdateIssue { id: String, update: IssueUpdate },
    Search(String),
    LoadRecent,
    SaveRecent(Vec<IssueSummary>),
    ClearRecent,
    OpenUrl(String),
    CopyToClipboard(String),
    Batch(Vec<Command>),
}
