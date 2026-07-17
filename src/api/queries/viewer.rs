use cynic::QueryFragment;

mod schema {
    cynic::use_schema!("schema.graphql");
}

#[derive(Debug, Clone, QueryFragment)]
#[cynic(schema_path = "schema.graphql")]
pub struct User {
    pub id: cynic::Id,
    pub name: String,
    pub display_name: String,
    pub is_me: bool,
}

#[derive(Debug, Clone, QueryFragment)]
#[cynic(schema_path = "schema.graphql")]
pub struct Organization {
    pub name: String,
    pub url_key: String,
}

#[derive(Debug, QueryFragment)]
#[cynic(schema_path = "schema.graphql", graphql_type = "Query")]
pub struct ViewerQuery {
    pub viewer: User,
    pub organization: Organization,
}
