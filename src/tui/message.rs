use super::feed::{FeedKey, FeedRequest};
use super::focus::Reveal;
use crate::api::{
    CommentId, Credential, IssueDetail, IssueId, IssueRef, IssueSummary, IssueUpdate,
    NotificationItem, Page, ReactionId, ReactionTarget, SavedView, Session, StateOption, TeamId,
    User,
};
use crate::store::{Account, PersistedCache};

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
        team_id: TeamId,
        states: Vec<StateOption>,
    },
    MembersLoaded {
        team_id: TeamId,
        members: Vec<User>,
    },
    UsersFound {
        query: String,
        users: Vec<User>,
    },
    IssueUpdated {
        id: IssueId,
    },
    CommentPosted {
        id: IssueId,
    },
    CommentEdited {
        id: IssueId,
    },
    CommentDeleted {
        id: IssueId,
    },
    ReactionToggled {
        id: IssueId,
    },
    AccountAdded {
        account: Box<Account>,
    },
    LoginSucceeded {
        credential: Credential,
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
    States { team_id: TeamId },
    Members { team_id: TeamId },
    UserSearch,
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
        target: IssueRef,
        reveal: Reveal,
    },
    LoadStates {
        team_id: TeamId,
    },
    LoadMembers {
        team_id: TeamId,
    },
    SearchUsers {
        query: String,
    },
    UpdateIssue {
        id: IssueId,
        update: IssueUpdate,
    },
    CreateComment {
        issue_id: IssueId,
        body: String,
        parent_id: Option<CommentId>,
    },
    UpdateComment {
        issue_id: IssueId,
        comment_id: CommentId,
        body: String,
    },
    DeleteComment {
        issue_id: IssueId,
        comment_id: CommentId,
    },
    CreateReaction {
        issue_id: IssueId,
        target: ReactionTarget,
        emoji: String,
    },
    DeleteReaction {
        issue_id: IssueId,
        reaction_id: ReactionId,
    },
    LoadRecent,
    SaveRecent(Vec<IssueSummary>),
    ClearRecent,
    OpenUrl(String),
    CopyToClipboard(String),
    SwitchWorkspace(Box<Account>),
    AddAccount {
        credential: Credential,
    },
    BeginLogin,
    Batch(Vec<Command>),
}
