mod map;

use cynic::{GraphQlResponse, MutationBuilder, QueryBuilder};
use reqwest::Client as HttpClient;

use crate::api::error::{ApiError, ApiResult};
use crate::api::model::{
    IssueDetail, IssueFilter, IssueSummary, IssueUpdate, NotificationItem, Session, StateOption,
    StateType, User,
};
use crate::api::queries::actions::{
    CommentCreateInput, CommentCreateMutation, CommentCreateVariables, IssueUpdateInput,
    IssueUpdateMutation, IssueUpdateVariables, TeamMembersQuery, TeamStatesQuery, TeamVariables,
};
use crate::api::queries::issue::{IssueQuery, IssueVariables};
use crate::api::queries::my_issues::{IssuesQuery, IssuesVariables};
use crate::api::queries::notifications::{NotificationsQuery, NotificationsVariables};
use crate::api::queries::search::{SearchIssuesQuery, SearchVariables};
use crate::api::queries::viewer::ViewerQuery;
use crate::api::LinearApi;

use map::{build_cynic_filter, map_detail, map_notification, map_search_result, map_summary};

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

    async fn fetch_json<T, V>(&self, operation: cynic::Operation<T, V>) -> ApiResult<T>
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

        if let Some(errors) = result.errors {
            return Err(ApiError::GraphQl(
                errors.into_iter().map(|error| error.message).collect(),
            ));
        }

        result.data.ok_or(ApiError::Empty)
    }
}

#[async_trait::async_trait]
impl LinearApi for Client {
    async fn session(&self) -> ApiResult<Session> {
        let result = self.fetch_json(ViewerQuery::build(())).await?;

        let user = User {
            id: result.viewer.id.into_inner(),
            name: result.viewer.name,
            display_name: result.viewer.display_name,
            url: String::new(),
            is_me: result.viewer.is_me,
        };

        Ok(Session {
            user,
            org_name: result.organization.name,
            org_url_key: result.organization.url_key,
        })
    }

    async fn issues(&self, filter: &IssueFilter) -> ApiResult<Vec<IssueSummary>> {
        let operation = IssuesQuery::build(IssuesVariables {
            filter: Some(build_cynic_filter(filter)),
            first: Some(100),
        });
        let result = self.fetch_json(operation).await?;

        Ok(result.issues.nodes.into_iter().map(map_summary).collect())
    }

    async fn search_issues(&self, term: &str) -> ApiResult<Vec<IssueSummary>> {
        let operation = SearchIssuesQuery::build(SearchVariables {
            term: term.to_string(),
            first: Some(50),
        });
        let result = self.fetch_json(operation).await?;

        Ok(result
            .search_issues
            .nodes
            .into_iter()
            .map(map_search_result)
            .collect())
    }

    async fn issue_detail(&self, id: &str) -> ApiResult<Option<IssueDetail>> {
        let operation = IssueQuery::build(IssueVariables { id: id.to_string() });
        let result = self.fetch_json(operation).await?;

        Ok(result.issue.map(map_detail))
    }

    async fn notifications(&self) -> ApiResult<Vec<NotificationItem>> {
        let operation = NotificationsQuery::build(NotificationsVariables { first: Some(50) });
        let result = self.fetch_json(operation).await?;

        Ok(result
            .notifications
            .nodes
            .iter()
            .map(map_notification)
            .collect())
    }

    async fn workflow_states(&self, team_id: &str) -> ApiResult<Vec<StateOption>> {
        let operation = TeamStatesQuery::build(TeamVariables {
            id: team_id.to_string(),
        });
        let result = self.fetch_json(operation).await?;

        let team = result.team.ok_or_else(|| ApiError::NotFound {
            resource: "team",
            id: team_id.to_string(),
        })?;

        let mut states: Vec<StateOption> = team
            .states
            .nodes
            .into_iter()
            .map(|s| StateOption {
                id: s.id.into_inner(),
                name: s.name,
                state_type: StateType::from_api(&s.state_type),
            })
            .collect();
        states.reverse();

        Ok(states)
    }

    async fn team_members(&self, team_id: &str) -> ApiResult<Vec<User>> {
        let operation = TeamMembersQuery::build(TeamVariables {
            id: team_id.to_string(),
        });
        let result = self.fetch_json(operation).await?;

        let team = result.team.ok_or_else(|| ApiError::NotFound {
            resource: "team",
            id: team_id.to_string(),
        })?;

        Ok(team
            .members
            .nodes
            .into_iter()
            .map(|u| User {
                id: u.id.into_inner(),
                name: u.name,
                display_name: u.display_name,
                url: u.url,
                is_me: u.is_me,
            })
            .collect())
    }

    async fn update_issue(&self, id: &str, update: IssueUpdate) -> ApiResult<()> {
        let operation = IssueUpdateMutation::build(IssueUpdateVariables {
            id: id.to_string(),
            input: IssueUpdateInput {
                state_id: update.state_id,
                assignee_id: update.assignee_id,
            },
        });
        let result = self.fetch_json(operation).await?;

        if result.issue_update.success {
            Ok(())
        } else {
            Err(ApiError::Rejected("issue update"))
        }
    }

    async fn create_comment(
        &self,
        issue_id: &str,
        body: &str,
        parent_id: Option<&str>,
    ) -> ApiResult<()> {
        let operation = CommentCreateMutation::build(CommentCreateVariables {
            input: CommentCreateInput {
                issue_id: Some(issue_id.to_string()),
                body: Some(body.to_string()),
                parent_id: parent_id.map(str::to_string),
            },
        });
        let result = self.fetch_json(operation).await?;

        if result.comment_create.success {
            Ok(())
        } else {
            Err(ApiError::Rejected("comment"))
        }
    }
}
