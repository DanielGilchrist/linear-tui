use cynic::{QueryFragment, QueryVariables};

mod schema {
    cynic::use_schema!("schema.graphql");
}

#[derive(cynic::Scalar, Debug, Clone)]
pub struct DateTime(pub String);

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
#[cynic(schema_path = "schema.graphql", graphql_type = "Comment")]
pub struct CommentParent {
    pub id: cynic::Id,
}

#[derive(Debug, Clone, QueryFragment)]
#[cynic(schema_path = "schema.graphql")]
pub struct Comment {
    pub id: cynic::Id,
    pub body: String,
    pub created_at: DateTime,
    pub user: Option<User>,
    pub parent: Option<CommentParent>,
}

#[derive(Debug, Clone, QueryFragment)]
#[cynic(schema_path = "schema.graphql")]
pub struct CommentConnection {
    pub nodes: Vec<Comment>,
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
    pub description: Option<String>,
    pub url: String,
    pub branch_name: String,
    pub priority: f64,
    pub state: WorkflowState,
    pub team: Team,
    pub assignee: Option<User>,
    pub labels: IssueLabelConnection,
    pub comments: CommentConnection,
}

#[derive(Debug, QueryFragment)]
#[cynic(
    schema_path = "schema.graphql",
    graphql_type = "Query",
    variables = "IssueVariables"
)]
pub struct IssueQuery {
    #[arguments(id: $id)]
    pub issue: Option<Issue>,
}

#[derive(Debug, QueryVariables)]
pub struct IssueVariables {
    pub id: String,
}
