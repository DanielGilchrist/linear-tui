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

        let response = self
            .http_client
            .post(API_ENDPOINT)
            .header("Content-Type", "application/json")
            .header("Authorization", &self.api_key)
            .json(&operation)
            .send()
            .await?;

        let result: GraphQlResponse<TeamsQuery> = response.json().await?;

        if let Some(errors) = result.errors {
            return Err(anyhow::anyhow!("GraphQL errors: {:?}", errors));
        }

        Ok(result.data.unwrap().teams.nodes)
    }

    pub async fn get_issue(&self, issue_id: &str) -> Result<Option<DetailedIssue>> {
        let operation = IssueQuery::build(IssueVariables {
            id: issue_id.to_string(),
        });

        let response = self
            .http_client
            .post(API_ENDPOINT)
            .header("Content-Type", "application/json")
            .header("Authorization", &self.api_key)
            .json(&operation)
            .send()
            .await?;

        let result: GraphQlResponse<IssueQuery> = response.json().await?;

        if let Some(errors) = result.errors {
            return Err(anyhow::anyhow!("GraphQL errors: {:?}", errors));
        }

        Ok(result.data.unwrap().issue)
    }

    pub async fn get_team_issues(&self, team_id: &str) -> Result<Vec<TeamIssue>> {
        let operation = TeamIssuesQuery::build(TeamIssuesVariables {
            id: team_id.to_string(),
        });

        let response = self
            .http_client
            .post(API_ENDPOINT)
            .header("Content-Type", "application/json")
            .header("Authorization", &self.api_key)
            .json(&operation)
            .send()
            .await?;

        let result: GraphQlResponse<TeamIssuesQuery> = response.json().await?;

        if let Some(errors) = result.errors {
            return Err(anyhow::anyhow!("GraphQL errors: {:?}", errors));
        }

        Ok(result.data.unwrap().team.unwrap().issues.nodes)
    }
}
