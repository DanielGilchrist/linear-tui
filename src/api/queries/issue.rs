use cynic::{QueryFragment, QueryVariables};

mod schema {
    cynic::use_schema!("schema.graphql");
}

#[derive(Debug, Clone, QueryFragment)]
#[cynic(schema_path = "schema.graphql")]
pub struct Comment {
    pub body: String,
}

#[derive(Debug, Clone, QueryFragment)]
#[cynic(schema_path = "schema.graphql")]
pub struct CommentConnection {
    pub nodes: Vec<Comment>,
}

#[derive(Debug, Clone, QueryFragment)]
#[cynic(schema_path = "schema.graphql")]
pub struct Issue {
    pub title: Option<String>,
    pub description: Option<String>,
    pub comments: CommentConnection,
    pub url: String,
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

