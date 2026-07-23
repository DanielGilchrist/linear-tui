use cynic::{QueryFragment, QueryVariables};

use super::scalars::DateTime;
use super::schema;

#[derive(Debug, QueryVariables)]
pub struct CustomViewsVariables {
    pub first: Option<i32>,
}

#[derive(Debug, Clone, QueryFragment)]
#[cynic(schema_path = "schema.graphql")]
pub struct CustomView {
    pub id: cynic::Id,
    pub name: String,
}

#[derive(Debug, QueryFragment)]
#[cynic(schema_path = "schema.graphql")]
pub struct CustomViewConnection {
    pub nodes: Vec<CustomView>,
}

#[derive(Debug, QueryFragment)]
#[cynic(
    schema_path = "schema.graphql",
    graphql_type = "Query",
    variables = "CustomViewsVariables"
)]
pub struct CustomViewsQuery {
    #[arguments(first: $first)]
    pub custom_views: CustomViewConnection,
}

#[derive(Debug, QueryVariables)]
pub struct CustomViewIssuesVariables {
    pub id: String,
    pub first: Option<i32>,
}

#[derive(Debug, Clone, QueryFragment)]
#[cynic(schema_path = "schema.graphql")]
pub struct WorkflowState {
    pub name: String,
    #[cynic(rename = "type")]
    pub state_type: String,
}

#[derive(Debug, Clone, QueryFragment)]
#[cynic(schema_path = "schema.graphql")]
pub struct User {
    pub display_name: String,
}

#[derive(Debug, Clone, QueryFragment)]
#[cynic(schema_path = "schema.graphql")]
pub struct IssueLabel {
    pub name: String,
    #[cynic(rename = "color")]
    pub colour: String,
}

#[derive(Debug, Clone, QueryFragment)]
#[cynic(schema_path = "schema.graphql")]
pub struct IssueLabelConnection {
    pub nodes: Vec<IssueLabel>,
}

#[derive(Debug, Clone, QueryFragment)]
#[cynic(schema_path = "schema.graphql")]
pub struct Team {
    pub id: cynic::Id,
}

#[derive(Debug, Clone, QueryFragment)]
#[cynic(schema_path = "schema.graphql")]
pub struct Issue {
    pub id: cynic::Id,
    pub identifier: String,
    pub title: Option<String>,
    pub priority: f64,
    pub url: String,
    pub branch_name: String,
    pub updated_at: DateTime,
    pub state: WorkflowState,
    pub team: Team,
    pub assignee: Option<User>,
    pub labels: IssueLabelConnection,
}

#[derive(Debug, QueryFragment)]
#[cynic(schema_path = "schema.graphql")]
pub struct PageInfo {
    pub has_next_page: bool,
}

#[derive(Debug, QueryFragment)]
#[cynic(schema_path = "schema.graphql")]
pub struct IssueConnection {
    pub nodes: Vec<Issue>,
    pub page_info: PageInfo,
}

#[derive(Debug, QueryFragment)]
#[cynic(
    schema_path = "schema.graphql",
    graphql_type = "CustomView",
    variables = "CustomViewIssuesVariables"
)]
pub struct CustomViewIssues {
    #[arguments(first: $first)]
    pub issues: IssueConnection,
}

#[derive(Debug, QueryFragment)]
#[cynic(
    schema_path = "schema.graphql",
    graphql_type = "Query",
    variables = "CustomViewIssuesVariables"
)]
pub struct CustomViewIssuesQuery {
    #[arguments(id: $id)]
    pub custom_view: CustomViewIssues,
}
