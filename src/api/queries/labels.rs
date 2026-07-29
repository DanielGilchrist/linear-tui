use cynic::{InputObject, QueryFragment, QueryVariables};

use super::schema;

#[derive(Debug, Clone, QueryFragment)]
#[cynic(schema_path = "schema.graphql")]
pub struct IssueLabel {
    pub id: cynic::Id,
    pub name: String,
    #[cynic(rename = "color")]
    pub colour: String,
}

#[derive(Debug, Clone, QueryFragment)]
#[cynic(schema_path = "schema.graphql")]
pub struct IssueLabelConnection {
    pub nodes: Vec<IssueLabel>,
}

#[derive(Debug, Clone, InputObject)]
#[cynic(schema_path = "schema.graphql", graphql_type = "StringComparator")]
pub struct StringComparator {
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub contains_ignore_case: Option<String>,
}

#[derive(Debug, Clone, InputObject)]
#[cynic(schema_path = "schema.graphql")]
pub struct IssueLabelFilter {
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub name: Option<StringComparator>,
}

#[derive(Debug, QueryVariables)]
pub struct LabelSearchVariables {
    pub filter: Option<IssueLabelFilter>,
    pub first: i32,
}

#[derive(Debug, QueryFragment)]
#[cynic(
    schema_path = "schema.graphql",
    graphql_type = "Query",
    variables = "LabelSearchVariables"
)]
pub struct LabelSearchQuery {
    #[arguments(filter: $filter, first: $first)]
    pub issue_labels: IssueLabelConnection,
}
