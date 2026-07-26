pub mod client;
pub mod error;
pub mod fixture;
pub mod model;
pub mod queries;

pub use client::Client;
pub use error::{ApiError, ApiResult};
pub use fixture::FixtureClient;
pub use model::*;

#[async_trait::async_trait]
pub trait LinearApi: Send + Sync {
    async fn session(&self) -> ApiResult<Session>;
    async fn custom_views(&self) -> ApiResult<Vec<SavedView>>;
    async fn custom_view_issues(
        &self,
        id: &str,
        after: Option<&Cursor>,
    ) -> ApiResult<Page<IssueSummary>>;
    async fn issues(
        &self,
        filter: &IssueFilter,
        after: Option<&Cursor>,
    ) -> ApiResult<Page<IssueSummary>>;
    async fn search_issues(
        &self,
        term: &str,
        after: Option<&Cursor>,
    ) -> ApiResult<Page<IssueSummary>>;
    async fn issue_detail(&self, id: &str) -> ApiResult<Option<IssueDetail>>;
    async fn notifications(&self, after: Option<&Cursor>) -> ApiResult<Page<NotificationItem>>;
    async fn workflow_states(&self, team_id: &str) -> ApiResult<Vec<StateOption>>;
    async fn team_members(&self, team_id: &str) -> ApiResult<Vec<User>>;
    async fn update_issue(&self, id: &str, update: IssueUpdate) -> ApiResult<()>;
    async fn create_comment(
        &self,
        issue_id: &str,
        body: &str,
        parent_id: Option<&str>,
    ) -> ApiResult<()>;
    async fn update_comment(&self, comment_id: &str, body: &str) -> ApiResult<()>;
    async fn delete_comment(&self, comment_id: &str) -> ApiResult<()>;
    async fn create_reaction(&self, target: &ReactionTarget, emoji: &str) -> ApiResult<()>;
    async fn delete_reaction(&self, reaction_id: &str) -> ApiResult<()>;
}
