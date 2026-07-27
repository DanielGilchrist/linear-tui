use anyhow::anyhow;
use anyhow::Result;
use cynic::{GraphQlResponse, QueryBuilder};
use reqwest::Client as HttpClient;

use crate::api::queries::issue;
use crate::api::queries::team_issues;
use crate::api::queries::teams;

use issue::{Issue as DetailedIssue, IssueQuery, IssueVariables};
use team_issues::{Issue as TeamIssue, TeamIssuesQuery, TeamIssuesVariables};
use teams::{Team, TeamsQuery};

const API_ENDPOINT: &str = "https://api.linear.app/graphql";

pub struct Client {
    http_client: HttpClient,
    api_key: String,
}

impl Client {
    pub fn new(api_key: String) -> Self {
        Self {
            http_client: HttpClient::new(),
            api_key,
        }
    }

    pub async fn get_teams(&self) -> Result<Vec<Team>> {
        let operation = TeamsQuery::build(());
        let result = self.fetch_json(operation).await?;

        Ok(result.teams.nodes)
    }

    pub async fn get_issue(&self, issue_id: &str) -> Result<Option<DetailedIssue>> {
        let operation = IssueQuery::build(IssueVariables {
            id: issue_id.to_string(),
        });

        let result = self.fetch_json(operation).await?;

        Ok(result.issue)
    }

    pub async fn get_team_issues(&self, team_id: &str) -> Result<Vec<TeamIssue>> {
        let operation = TeamIssuesQuery::build(TeamIssuesVariables {
            id: team_id.to_string(),
        });

        let result = self.fetch_json(operation).await?;
        let team = result
            .team
            .ok_or_else(|| anyhow!("Team doesn't exist for ID {team_id}"))?;

        Ok(team.issues.nodes)
    }

    async fn fetch_json<T, V>(&self, operation: cynic::Operation<T, V>) -> Result<T>
    where
        T: for<'de> serde::Deserialize<'de>,
        V: serde::Serialize,
    {
        let response = self
            .http_client
            .post(API_ENDPOINT)
            .header("Content-Type", "application/json")
            .header("Authorization", &self.api_key)
            .json(&operation)
            .send()
            .await?;

        let result: GraphQlResponse<T> = response.json().await?;

        if let Some(errors) = &result.errors {
            return Err(anyhow!("GraphQL errors: {:?}", errors));
        }

        let data = result.data.ok_or_else(|| anyhow!("Response is empty"))?;

        Ok(data)
    }
}
