use super::feed::{FeedKey, FeedRequest};
use super::focus::Reveal;
use super::overlay::Compose;
use crate::api::{
    CommentId, Credential, IssueDetail, IssueId, IssueRef, IssueSummary, IssueUpdate, Label,
    NotificationItem, Page, ReactionId, ReactionTarget, SavedView, Session, StateOption, Team,
    TeamId, User,
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
    TeamsLoaded {
        teams: Vec<Team>,
    },
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
    LabelsFound {
        query: String,
        labels: Vec<Label>,
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
    TokenRefreshed {
        workspace_key: String,
        credential: Credential,
    },
    RefreshFailed {
        workspace_key: String,
    },
    Failed {
        target: FailureTarget,
        error: RequestError,
    },
}

#[derive(Debug, Clone)]
pub enum RequestError {
    Unauthorised(String),
    Other(String),
}

impl From<&crate::api::ApiError> for RequestError {
    fn from(error: &crate::api::ApiError) -> Self {
        let message = error.to_string();

        if error.is_auth() {
            RequestError::Unauthorised(message)
        } else {
            RequestError::Other(message)
        }
    }
}

#[derive(Debug, Clone)]
pub enum FailureTarget {
    Session,
    Feed(FeedKey),
    Inbox,
    CustomViews,
    Teams,
    Detail,
    States { team_id: TeamId },
    Members { team_id: TeamId },
    UserSearch,
    LabelSearch,
    Compose(Box<ComposeRecovery>),
    Ephemeral,
}

#[derive(Debug, Clone)]
pub struct ComposeRecovery {
    pub issue_id: IssueId,
    pub team_id: TeamId,
    pub compose: Compose,
    pub body: String,
}

#[derive(Debug, Clone)]
pub enum Effect {
    Api(ApiCommand),
    Store(StoreCommand),
    Platform(PlatformCommand),
}

#[derive(Debug, Clone, Default)]
pub struct Effects(Vec<Effect>);

impl Effects {
    pub fn one(effect: Effect) -> Self {
        Effects(vec![effect])
    }

    pub fn when(condition: bool, effect: Effect) -> Self {
        if condition {
            Effects::one(effect)
        } else {
            Effects::default()
        }
    }

    pub fn push(&mut self, effect: Effect) {
        self.0.push(effect);
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn iter(&self) -> std::slice::Iter<'_, Effect> {
        self.0.iter()
    }

    pub fn or_else(self, other: impl FnOnce() -> Effects) -> Effects {
        if self.0.is_empty() {
            other()
        } else {
            self
        }
    }
}

impl From<Effect> for Effects {
    fn from(effect: Effect) -> Self {
        Effects::one(effect)
    }
}

impl From<Option<Effect>> for Effects {
    fn from(effect: Option<Effect>) -> Self {
        match effect {
            Some(effect) => Effects::one(effect),
            None => Effects::default(),
        }
    }
}

impl IntoIterator for Effects {
    type Item = Effect;
    type IntoIter = std::vec::IntoIter<Effect>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl Extend<Effect> for Effects {
    fn extend<T: IntoIterator<Item = Effect>>(&mut self, effects: T) {
        self.0.extend(effects);
    }
}

impl FromIterator<Effect> for Effects {
    fn from_iter<T: IntoIterator<Item = Effect>>(effects: T) -> Self {
        Effects(effects.into_iter().collect())
    }
}

#[derive(Debug, Clone)]
pub enum Commands {
    Effects(Effects),
    Runtime(RuntimeCommand),
}

impl Default for Commands {
    fn default() -> Self {
        Commands::Effects(Effects::default())
    }
}

impl Commands {
    pub fn runtime(command: RuntimeCommand) -> Self {
        Commands::Runtime(command)
    }

    pub fn is_empty(&self) -> bool {
        match self {
            Commands::Effects(effects) => effects.is_empty(),
            Commands::Runtime(_) => false,
        }
    }
}

impl From<Effects> for Commands {
    fn from(effects: Effects) -> Self {
        Commands::Effects(effects)
    }
}

impl From<Effect> for Commands {
    fn from(effect: Effect) -> Self {
        Commands::Effects(Effects::one(effect))
    }
}

#[derive(Debug, Clone)]
pub enum ApiCommand {
    LoadSession,
    LoadFeed {
        key: FeedKey,
        request: FeedRequest,
    },
    LoadInboxFeed {
        request: FeedRequest,
    },
    LoadCustomViews,
    LoadTeams,
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
    SearchLabels {
        query: String,
    },
    UpdateIssue {
        id: IssueId,
        update: IssueUpdate,
    },
    CreateComment {
        issue_id: IssueId,
        team_id: TeamId,
        body: String,
        parent_id: Option<CommentId>,
    },
    UpdateComment {
        issue_id: IssueId,
        team_id: TeamId,
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
}

#[derive(Debug, Clone)]
pub enum StoreCommand {
    SaveFeeds(PersistedCache),
    LoadRecent,
    SaveRecent(Vec<IssueSummary>),
    ClearRecent,
}

#[derive(Debug, Clone)]
pub enum PlatformCommand {
    OpenUrl(String),
    CopyToClipboard(String),
}

#[derive(Debug, Clone)]
pub enum RuntimeCommand {
    SwitchWorkspace(Box<Account>),
    AddAccount { credential: Credential },
    RefreshToken { workspace_key: String },
    Reconnect,
    BeginLogin,
}

impl ApiCommand {
    pub fn failure_target(&self) -> FailureTarget {
        match self {
            ApiCommand::LoadSession => FailureTarget::Session,
            ApiCommand::LoadFeed { key, .. } => FailureTarget::Feed(key.clone()),
            ApiCommand::LoadInboxFeed { .. } => FailureTarget::Inbox,
            ApiCommand::LoadCustomViews => FailureTarget::CustomViews,
            ApiCommand::LoadTeams => FailureTarget::Teams,
            ApiCommand::LoadDetail { .. } => FailureTarget::Detail,
            ApiCommand::LoadStates { team_id } => FailureTarget::States {
                team_id: team_id.clone(),
            },
            ApiCommand::LoadMembers { team_id } => FailureTarget::Members {
                team_id: team_id.clone(),
            },
            ApiCommand::SearchUsers { .. } => FailureTarget::UserSearch,
            ApiCommand::SearchLabels { .. } => FailureTarget::LabelSearch,
            ApiCommand::CreateComment {
                issue_id,
                team_id,
                body,
                parent_id,
            } => FailureTarget::Compose(Box::new(ComposeRecovery {
                issue_id: issue_id.clone(),
                team_id: team_id.clone(),
                compose: match parent_id {
                    Some(parent_id) => Compose::Reply {
                        parent_id: parent_id.clone(),
                    },
                    None => Compose::Comment,
                },
                body: body.clone(),
            })),
            ApiCommand::UpdateComment {
                issue_id,
                team_id,
                comment_id,
                body,
            } => FailureTarget::Compose(Box::new(ComposeRecovery {
                issue_id: issue_id.clone(),
                team_id: team_id.clone(),
                compose: Compose::Edit {
                    comment_id: comment_id.clone(),
                },
                body: body.clone(),
            })),
            ApiCommand::UpdateIssue { .. }
            | ApiCommand::DeleteComment { .. }
            | ApiCommand::CreateReaction { .. }
            | ApiCommand::DeleteReaction { .. } => FailureTarget::Ephemeral,
        }
    }
}
