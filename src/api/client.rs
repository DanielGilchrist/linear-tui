use anyhow::{anyhow, Result};
use cynic::{GraphQlResponse, QueryBuilder};
use reqwest::Client as HttpClient;

use crate::api::model::{
    Comment, IssueDetail, IssueFilter, IssueSummary, IssueUpdate, Label, NotificationItem, Session,
    StateOption, User, WorkflowState,
};
use crate::api::queries::actions::{
    IssueUpdateInput, IssueUpdateMutation, IssueUpdateVariables, TeamMembersQuery, TeamStatesQuery,
    TeamVariables,
};
use crate::api::queries::issue::{self, IssueQuery, IssueVariables};
use crate::api::queries::my_issues::{
    self, BooleanComparator, IssuesQuery, IssuesVariables, NullableUserFilter, StringComparator,
    WorkflowStateFilter,
};
use crate::api::queries::notifications::{
    Notification, NotificationsQuery, NotificationsVariables,
};
use crate::api::queries::viewer::ViewerQuery;
use crate::api::LinearApi;
use cynic::MutationBuilder;

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

        result.data.ok_or_else(|| anyhow!("Response is empty"))
    }
}

fn build_cynic_filter(filter: &IssueFilter) -> my_issues::IssueFilter {
    let me = || NullableUserFilter {
        is_me: Some(BooleanComparator { eq: Some(true) }),
    };

    let state = if !filter.state_types_in.is_empty() || !filter.state_types_nin.is_empty() {
        Some(WorkflowStateFilter {
            type_: Some(StringComparator {
                eq: None,
                in_: (!filter.state_types_in.is_empty()).then(|| filter.state_types_in.clone()),
                nin: (!filter.state_types_nin.is_empty()).then(|| filter.state_types_nin.clone()),
            }),
        })
    } else {
        None
    };

    my_issues::IssueFilter {
        assignee: filter.assigned_to_me.then(me),
        creator: filter.created_by_me.then(me),
        state,
    }
}

fn map_summary(issue: my_issues::Issue) -> IssueSummary {
    IssueSummary {
        id: issue.id.into_inner(),
        identifier: issue.identifier,
        title: issue.title,
        state: WorkflowState {
            name: issue.state.name,
            state_type: issue.state.state_type,
        },
        priority: issue.priority as u8,
        assignee: issue.assignee.map(|a| User {
            id: String::new(),
            name: String::new(),
            display_name: a.display_name,
            is_me: false,
        }),
        labels: issue
            .labels
            .nodes
            .into_iter()
            .map(|l| Label {
                name: l.name,
                color: l.color,
            })
            .collect(),
        url: issue.url,
        branch_name: issue.branch_name,
        team_id: issue.team.id.into_inner(),
    }
}

fn map_detail(issue: issue::Issue) -> IssueDetail {
    IssueDetail {
        id: issue.id.into_inner(),
        identifier: issue.identifier,
        title: issue.title,
        description: issue.description,
        url: issue.url,
        state: WorkflowState {
            name: issue.state.name,
            state_type: issue.state.state_type,
        },
        priority: issue.priority as u8,
        assignee: issue.assignee.map(|a| User {
            id: String::new(),
            name: String::new(),
            display_name: a.display_name,
            is_me: false,
        }),
        labels: issue
            .labels
            .nodes
            .into_iter()
            .map(|l| Label {
                name: l.name,
                color: l.color,
            })
            .collect(),
        comments: issue
            .comments
            .nodes
            .into_iter()
            .map(|c| Comment {
                author: c.user.map(|u| u.display_name),
                body: c.body,
                created_at: c.created_at.0,
            })
            .collect(),
        branch_name: issue.branch_name,
        team_id: issue.team.id.into_inner(),
    }
}

fn map_notification(notification: &Notification) -> NotificationItem {
    NotificationItem {
        title: notification.title().to_string(),
        issue_id: notification.issue_id().map(|s| s.to_string()),
        is_read: notification.is_read(),
        grouping_key: notification.grouping_key().to_string(),
    }
}

#[async_trait::async_trait]
impl LinearApi for Client {
    async fn session(&self) -> Result<Session> {
        let result = self.fetch_json(ViewerQuery::build(())).await?;

        let user = User {
            id: result.viewer.id.into_inner(),
            name: result.viewer.name,
            display_name: result.viewer.display_name,
            is_me: result.viewer.is_me,
        };

        Ok(Session {
            user,
            org_name: result.organization.name,
            org_url_key: result.organization.url_key,
        })
    }

    async fn issues(&self, filter: &IssueFilter) -> Result<Vec<IssueSummary>> {
        let operation = IssuesQuery::build(IssuesVariables {
            filter: Some(build_cynic_filter(filter)),
            first: Some(100),
        });
        let result = self.fetch_json(operation).await?;

        let issues = result.issues.nodes.into_iter().map(map_summary).collect();

        Ok(issues)
    }

    async fn issue_detail(&self, id: &str) -> Result<Option<IssueDetail>> {
        let operation = IssueQuery::build(IssueVariables { id: id.to_string() });
        let result = self.fetch_json(operation).await?;

        Ok(result.issue.map(map_detail))
    }

    async fn notifications(&self) -> Result<Vec<NotificationItem>> {
        let operation = NotificationsQuery::build(NotificationsVariables { first: Some(50) });
        let result = self.fetch_json(operation).await?;

        let notifications = result
            .notifications
            .nodes
            .iter()
            .map(map_notification)
            .collect();

        Ok(notifications)
    }

    async fn workflow_states(&self, team_id: &str) -> Result<Vec<StateOption>> {
        let operation = TeamStatesQuery::build(TeamVariables {
            id: team_id.to_string(),
        });
        let result = self.fetch_json(operation).await?;

        let team = result
            .team
            .ok_or_else(|| anyhow!("Team {team_id} not found"))?;

        let mut states: Vec<StateOption> = team
            .states
            .nodes
            .into_iter()
            .map(|s| StateOption {
                id: s.id.into_inner(),
                name: s.name,
                state_type: s.state_type,
            })
            .collect();
        states.reverse();

        Ok(states)
    }

    async fn team_members(&self, team_id: &str) -> Result<Vec<User>> {
        let operation = TeamMembersQuery::build(TeamVariables {
            id: team_id.to_string(),
        });
        let result = self.fetch_json(operation).await?;

        let team = result
            .team
            .ok_or_else(|| anyhow!("Team {team_id} not found"))?;

        let members = team
            .members
            .nodes
            .into_iter()
            .map(|u| User {
                id: u.id.into_inner(),
                name: u.name,
                display_name: u.display_name,
                is_me: u.is_me,
            })
            .collect();

        Ok(members)
    }

    async fn update_issue(&self, id: &str, update: IssueUpdate) -> Result<()> {
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
            Err(anyhow!("Linear rejected the update"))
        }
    }
}
