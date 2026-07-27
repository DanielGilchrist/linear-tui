use cynic::QueryFragment;

mod schema {
    cynic::use_schema!("schema.graphql");
}

#[derive(Debug, Clone, QueryFragment)]
#[cynic(schema_path = "schema.graphql")]
pub struct Team {
    pub id: cynic::Id,
    pub name: String,
    #[cynic(rename = "issueCount")]
    pub issue_count: i32,
}

#[derive(Debug, QueryFragment)]
#[cynic(schema_path = "schema.graphql")]
pub struct TeamConnection {
    pub nodes: Vec<Team>,
}

#[derive(Debug, QueryFragment)]
#[cynic(schema_path = "schema.graphql", graphql_type = "Query")]
pub struct TeamsQuery {
    pub teams: TeamConnection,
}
