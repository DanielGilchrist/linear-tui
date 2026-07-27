use cynic::{QueryFragment, QueryVariables};

mod schema {
    cynic::use_schema!("schema.graphql");
}

#[derive(Debug, Clone, QueryFragment)]
#[cynic(schema_path = "schema.graphql")]
pub struct Issue {
    pub id: cynic::Id,
    pub title: Option<String>,
    pub description: Option<String>,
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

