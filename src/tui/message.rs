use super::feed::{FeedKey, FeedRequest};
use super::focus::Reveal;
use crate::api::{
    IssueDetail, IssueSummary, IssueUpdate, NotificationItem, Page, ReactionTarget, SavedView,
    Session, StateOption, User,
};
use crate::store::PersistedCache;

#[derive(Debug)]
pub enum Message {
    SessionLoaded(Session),
    FeedLoaded {
        key: FeedKey,
        request: FeedRequest,
        page: Page<IssueSummary>,
    },
    InboxLoaded {
        request: FeedRequest,
        page: Page<NotificationItem>,
    },
    CustomViewsLoaded(Vec<SavedView>),
    DetailLoaded {
        detail: Box<IssueDetail>,
        reveal: Reveal,
    },
    RecentLoaded(Vec<IssueSummary>),
    RecentCleared,
    StatesLoaded {
        team_id: String,
        states: Vec<StateOption>,
    },
    MembersLoaded {
        team_id: String,
        members: Vec<User>,
    },
    IssueUpdated {
        id: String,
    },
    CommentPosted {
        id: String,
    },
    CommentEdited {
        id: String,
    },
    CommentDeleted {
        id: String,
    },
    ReactionToggled {
        id: String,
    },
    Failed {
        target: FailureTarget,
        error: String,
    },
}

#[derive(Debug, Clone)]
pub enum FailureTarget {
    Feed(FeedKey),
    Inbox,
    CustomViews,
    Detail,
    States { team_id: String },
    Members { team_id: String },
    Ephemeral,
}

#[derive(Debug, Clone)]
pub enum Command {
    LoadSession,
    LoadFeed {
        key: FeedKey,
        request: FeedRequest,
    },
    LoadInboxFeed {
        request: FeedRequest,
    },
    SaveFeeds(PersistedCache),
    LoadCustomViews,
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
    DeleteComment {
        issue_id: String,
        comment_id: String,
    },
    CreateReaction {
        issue_id: String,
        target: ReactionTarget,
        emoji: String,
    },
    DeleteReaction {
        issue_id: String,
        reaction_id: String,
    },
    LoadRecent,
    SaveRecent(Vec<IssueSummary>),
    ClearRecent,
    OpenUrl(String),
    CopyToClipboard(String),
    Batch(Vec<Command>),
}
