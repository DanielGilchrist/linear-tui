use cynic::{QueryFragment, QueryVariables};

use super::schema;

#[derive(Debug, QueryVariables)]
pub struct TeamsVariables {
    pub first: Option<i32>,
}

#[derive(Debug, Clone, QueryFragment)]
#[cynic(schema_path = "schema.graphql", graphql_type = "Team")]
pub struct TeamNode {
    pub id: cynic::Id,
    pub name: String,
    pub key: String,
    pub triage_enabled: bool,
}

#[derive(Debug, QueryFragment)]
#[cynic(schema_path = "schema.graphql")]
pub struct TeamConnection {
    pub nodes: Vec<TeamNode>,
}

#[derive(Debug, QueryFragment)]
#[cynic(
    schema_path = "schema.graphql",
    graphql_type = "Query",
    variables = "TeamsVariables"
)]
pub struct TeamsQuery {
    #[arguments(first: $first)]
    pub teams: TeamConnection,
}
