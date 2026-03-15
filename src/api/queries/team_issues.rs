use cynic::{QueryFragment, QueryVariables};

mod schema {
    cynic::use_schema!("schema.graphql");
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
pub struct Issue {
    pub id: cynic::Id,
    pub identifier: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub priority: f64,
    pub priority_label: String,
    pub state: WorkflowState,
    pub assignee: Option<User>,
    pub labels: IssueLabelConnection,
}

#[derive(Debug, QueryFragment)]
#[cynic(schema_path = "schema.graphql")]
pub struct IssueConnection {
    pub nodes: Vec<Issue>,
}

#[derive(Debug, QueryFragment)]
#[cynic(schema_path = "schema.graphql", graphql_type = "Team")]
pub struct TeamWithIssues {
    pub issues: IssueConnection,
}

#[derive(Debug, QueryFragment)]
#[cynic(
    schema_path = "schema.graphql",
    graphql_type = "Query",
    variables = "TeamIssuesVariables"
)]
pub struct TeamIssuesQuery {
    #[arguments(id: $id)]
    pub team: Option<TeamWithIssues>,
}

#[derive(Debug, QueryVariables)]
pub struct TeamIssuesVariables {
    pub id: String,
}
