use super::focus::Reveal;
use super::overlay::PickerItem;
use crate::api::{
    IssueDetail, IssueFilter, IssueSummary, IssueUpdate, NotificationItem, Session, User,
};

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
    DetailLoaded {
        detail: Box<IssueDetail>,
        reveal: Reveal,
    },
    SearchResults(Vec<IssueSummary>),
    RecentLoaded(Vec<IssueSummary>),
    RecentCleared,
    PickerLoaded(Vec<PickerItem>),
    MentionMembersLoaded(Vec<User>),
    IssueUpdated {
        id: String,
    },
    CommentPosted {
        id: String,
    },
    CommentEdited {
        id: String,
    },
    Failed(String),
}

#[derive(Debug, Clone)]
pub enum Command {
    LoadSession,
    LoadIssues {
        view: usize,
        filter: IssueFilter,
    },
    LoadInbox {
        view: usize,
    },
    LoadDetail {
        id: String,
        reveal: Reveal,
    },
    LoadStates {
        team_id: String,
    },
    LoadMembers {
        team_id: String,
    },
    LoadMentionMembers {
        team_id: String,
    },
    UpdateIssue {
        id: String,
        update: IssueUpdate,
    },
    CreateComment {
        issue_id: String,
        body: String,
        parent_id: Option<String>,
    },
    UpdateComment {
        issue_id: String,
        comment_id: String,
        body: String,
    },
    Search(String),
    LoadRecent,
    SaveRecent(Vec<IssueSummary>),
    ClearRecent,
    OpenUrl(String),
    CopyToClipboard(String),
    Batch(Vec<Command>),
}
