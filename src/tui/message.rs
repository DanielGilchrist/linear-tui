use super::app::PickerItem;
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
    OpenUrl(String),
    CopyToClipboard(String),
}
