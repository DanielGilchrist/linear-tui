use cynic::{QueryFragment, QueryVariables};

mod schema {
    cynic::use_schema!("schema.graphql");
}

#[derive(Debug, QueryVariables)]
pub struct SearchVariables {
    pub term: String,
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
pub struct IssueSearchResult {
    pub id: cynic::Id,
    pub identifier: String,
    pub title: String,
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
pub struct IssueSearchPayload {
    pub nodes: Vec<IssueSearchResult>,
}

#[derive(Debug, QueryFragment)]
#[cynic(
    schema_path = "schema.graphql",
    graphql_type = "Query",
    variables = "SearchVariables"
)]
pub struct SearchIssuesQuery {
    #[arguments(term: $term, first: $first)]
    pub search_issues: IssueSearchPayload,
}
