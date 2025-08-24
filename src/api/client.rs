use anyhow::Result;
use reqwest::Client as HttpClient;
use serde_json::{json, Value};
use std::collections::HashMap;

use crate::api::models::*;

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
        let query = r#"
            query Teams {
                teams {
                    nodes {
                        id
                        name
                        issueCount
                    }
                }
            }
        "#;

        let response: TeamsResponse = self.make_request(query, None).await?;

        Ok(response.data.teams.nodes)
    }

    pub async fn get_issue(&self, issue_id: &str) -> Result<Issue> {
        let query = r#"
            query Issue($issueId: String!) {
                issue(id: $issueId) {
                    title
                    description
                    comments {
                        nodes {
                            body
                        }
                    }
                    url
                }
            }
        "#;

        let mut variables = HashMap::new();
        variables.insert("issueId".to_string(), json!(issue_id));

        let response: IssueResponse = self.make_request(query, Some(variables)).await?;

        Ok(response.data.issue)
    }

    pub async fn get_team_issues(&self, team_id: &str) -> Result<Vec<TeamIssue>> {
        let query = r#"
            query TeamIssues($teamId: String!) {
                team(id: $teamId) {
                    issues {
                        nodes {
                            id
                            title
                            description
                        }
                    }
                }
            }
        "#;

        let mut variables = HashMap::new();
        variables.insert("teamId".to_string(), json!(team_id));

        let response: TeamIssuesResponse = self.make_request(query, Some(variables)).await?;

        Ok(response.data.team.issues.nodes)
    }

    async fn make_request<T: serde::de::DeserializeOwned>(
        &self,
        query: &str,
        variables: Option<HashMap<String, Value>>,
    ) -> Result<T> {
        let request_body = json!({
            "query": query,
            "variables": variables.unwrap_or_default()
        });

        let response = self
            .http_client
            .post(API_ENDPOINT)
            .header("Content-Type", "application/json")
            .header("Authorization", &self.api_key)
            .json(&request_body)
            .send()
            .await?;

        let result = response.json::<T>().await?;
        Ok(result)
    }
}
