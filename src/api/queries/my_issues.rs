use cynic::{InputObject, QueryFragment, QueryVariables};

mod schema {
    cynic::use_schema!("schema.graphql");
}

#[derive(Debug, Clone, InputObject)]
#[cynic(schema_path = "schema.graphql")]
pub struct BooleanComparator {
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub eq: Option<bool>,
}

#[derive(Debug, Clone, InputObject)]
#[cynic(schema_path = "schema.graphql")]
pub struct StringComparator {
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub eq: Option<String>,
    #[cynic(skip_serializing_if = "Option::is_none", rename = "in")]
    pub in_: Option<Vec<String>>,
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub nin: Option<Vec<String>>,
}

#[derive(Debug, Clone, InputObject)]
#[cynic(schema_path = "schema.graphql")]
pub struct NullableUserFilter {
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub is_me: Option<BooleanComparator>,
}

#[derive(Debug, Clone, InputObject)]
#[cynic(schema_path = "schema.graphql")]
pub struct WorkflowStateFilter {
    #[cynic(skip_serializing_if = "Option::is_none", rename = "type")]
    pub type_: Option<StringComparator>,
}

#[derive(Debug, Clone, InputObject)]
#[cynic(schema_path = "schema.graphql")]
pub struct IssueFilter {
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub assignee: Option<NullableUserFilter>,
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub creator: Option<NullableUserFilter>,
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub state: Option<WorkflowStateFilter>,
}

#[derive(Debug, QueryVariables)]
pub struct IssuesVariables {
    pub filter: Option<IssueFilter>,
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
    pub color: String,
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
    pub state: WorkflowState,
    pub team: Team,
    pub assignee: Option<User>,
    pub labels: IssueLabelConnection,
}

#[derive(Debug, QueryFragment)]
#[cynic(schema_path = "schema.graphql")]
pub struct IssueConnection {
    pub nodes: Vec<Issue>,
}

#[derive(Debug, QueryFragment)]
#[cynic(
    schema_path = "schema.graphql",
    graphql_type = "Query",
    variables = "IssuesVariables"
)]
pub struct IssuesQuery {
    #[arguments(filter: $filter, first: $first)]
    pub issues: IssueConnection,
}
