pub mod client;
pub mod fixture;
pub mod model;
pub mod queries;

use anyhow::Result;

pub use client::Client;
pub use fixture::FixtureClient;
pub use model::*;

#[async_trait::async_trait]
pub trait LinearApi: Send + Sync {
    async fn session(&self) -> Result<Session>;
    async fn issues(&self, filter: &IssueFilter) -> Result<Vec<IssueSummary>>;
    async fn issue_detail(&self, id: &str) -> Result<Option<IssueDetail>>;
    async fn notifications(&self) -> Result<Vec<NotificationItem>>;
    async fn workflow_states(&self, team_id: &str) -> Result<Vec<StateOption>>;
    async fn team_members(&self, team_id: &str) -> Result<Vec<User>>;
    async fn update_issue(&self, id: &str, update: IssueUpdate) -> Result<()>;
}
