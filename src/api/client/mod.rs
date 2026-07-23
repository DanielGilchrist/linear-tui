mod map;

use cynic::{GraphQlError, GraphQlResponse, MutationBuilder, QueryBuilder};
use reqwest::Client as HttpClient;
use serde::Deserialize;

use crate::api::error::{ApiError, ApiResult};
use crate::api::model::{
    IssueDetail, IssueFilter, IssuePage, IssueSummary, IssueUpdate, NotificationItem, SavedView,
    Session, StateOption, StateType, User,
};
use crate::api::queries::actions::{
    AssigneeInput, AssigneeMutation, AssigneeVariables, CommentCreateInput, CommentCreateMutation,
    CommentCreateVariables, CommentDeleteMutation, CommentDeleteVariables, CommentUpdateInput,
    CommentUpdateMutation, CommentUpdateVariables, StatusInput, StatusMutation, StatusVariables,
    TeamMembersQuery, TeamStatesQuery, TeamVariables,
};
use crate::api::queries::custom_views::{
    CustomViewIssuesQuery, CustomViewIssuesVariables, CustomViewsQuery, CustomViewsVariables,
};
use crate::api::queries::issue::{IssueQuery, IssueVariables};
use crate::api::queries::my_issues::{IssuesQuery, IssuesVariables};
use crate::api::queries::notifications::{NotificationsQuery, NotificationsVariables};
use crate::api::queries::search::{SearchIssuesQuery, SearchVariables};
use crate::api::queries::viewer::ViewerQuery;
use crate::api::LinearApi;

use map::build_cynic_filter;

const API_ENDPOINT: &str = "https://api.linear.app/graphql";

const PAGE_SIZE: i32 = 100;

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

        let result: GraphQlResponse<T, ErrorExtensions> = response.json().await?;

        if let Some(errors) = result.errors {
            return Err(ApiError::GraphQl(error_messages(errors)));
        }

        result.data.ok_or(ApiError::Empty)
    }

    async fn run_mutation<T, V>(&self, operation: cynic::Operation<T, V>) -> ApiResult<()>
    where
        T: for<'de> serde::Deserialize<'de>,
        V: serde::Serialize,
    {
        self.fetch_json(operation).await.map(|_| ())
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ErrorExtensions {
    user_presentable_message: Option<String>,
}

fn error_messages(errors: Vec<GraphQlError<ErrorExtensions>>) -> Vec<String> {
    errors
        .into_iter()
        .map(|error| {
            error
                .extensions
                .and_then(|extensions| extensions.user_presentable_message)
                .unwrap_or(error.message)
        })
        .collect()
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

    async fn custom_views(&self) -> ApiResult<Vec<SavedView>> {
        let operation = CustomViewsQuery::build(CustomViewsVariables {
            first: Some(PAGE_SIZE),
        });
        let result = self.fetch_json(operation).await?;

        Ok(result
            .custom_views
            .nodes
            .into_iter()
            .map(SavedView::from)
            .collect())
    }

    async fn custom_view_issues(&self, id: &str) -> ApiResult<IssuePage> {
        let operation = CustomViewIssuesQuery::build(CustomViewIssuesVariables {
            id: id.to_string(),
            first: Some(PAGE_SIZE),
        });
        let result = self.fetch_json(operation).await?;
        let connection = result.custom_view.issues;

        Ok(IssuePage {
            truncated: connection.page_info.has_next_page,
            issues: connection
                .nodes
                .into_iter()
                .map(IssueSummary::from)
                .collect(),
        })
    }

    async fn issues(&self, filter: &IssueFilter) -> ApiResult<Vec<IssueSummary>> {
        let operation = IssuesQuery::build(IssuesVariables {
            filter: Some(build_cynic_filter(filter)),
            first: Some(PAGE_SIZE),
        });
        let result = self.fetch_json(operation).await?;

        Ok(result
            .issues
            .nodes
            .into_iter()
            .map(IssueSummary::from)
            .collect())
    }

    async fn search_issues(&self, term: &str) -> ApiResult<Vec<IssueSummary>> {
        let operation = SearchIssuesQuery::build(SearchVariables {
            term: term.to_string(),
            first: Some(PAGE_SIZE),
        });
        let result = self.fetch_json(operation).await?;

        Ok(result
            .search_issues
            .nodes
            .into_iter()
            .map(IssueSummary::from)
            .collect())
    }

    async fn issue_detail(&self, id: &str) -> ApiResult<Option<IssueDetail>> {
        let operation = IssueQuery::build(IssueVariables { id: id.to_string() });
        let result = self.fetch_json(operation).await?;

        Ok(result.issue.map(IssueDetail::from))
    }

    async fn notifications(&self) -> ApiResult<Vec<NotificationItem>> {
        let operation = NotificationsQuery::build(NotificationsVariables {
            first: Some(PAGE_SIZE),
        });
        let result = self.fetch_json(operation).await?;

        Ok(result
            .notifications
            .nodes
            .iter()
            .map(NotificationItem::from)
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
        let id = id.to_string();
        match update {
            IssueUpdate::Status(state_id) => {
                self.run_mutation(StatusMutation::build(StatusVariables {
                    id,
                    input: StatusInput {
                        state_id: Some(state_id),
                    },
                }))
                .await
            }
            IssueUpdate::Assignee(assignee_id) => {
                self.run_mutation(AssigneeMutation::build(AssigneeVariables {
                    id,
                    input: AssigneeInput { assignee_id },
                }))
                .await
            }
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

        self.run_mutation(operation).await
    }

    async fn update_comment(&self, comment_id: &str, body: &str) -> ApiResult<()> {
        let operation = CommentUpdateMutation::build(CommentUpdateVariables {
            id: comment_id.to_string(),
            input: CommentUpdateInput {
                body: Some(body.to_string()),
            },
        });

        self.run_mutation(operation).await
    }

    async fn delete_comment(&self, comment_id: &str) -> ApiResult<()> {
        let operation = CommentDeleteMutation::build(CommentDeleteVariables {
            id: comment_id.to_string(),
        });

        self.run_mutation(operation).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_messages_prefer_the_user_presentable_message() {
        let errors = vec![
            GraphQlError::new(
                "Argument Validation Error".into(),
                None,
                None,
                Some(ErrorExtensions {
                    user_presentable_message: Some("The assignee must be on the team.".into()),
                }),
            ),
            GraphQlError::new("Something broke".into(), None, None, None),
        ];

        assert_eq!(
            error_messages(errors),
            vec![
                "The assignee must be on the team.".to_string(),
                "Something broke".to_string(),
            ]
        );
    }
}
