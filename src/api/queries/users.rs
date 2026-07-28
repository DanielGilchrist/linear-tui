use cynic::{InputObject, QueryFragment, QueryVariables};

use super::schema;

#[derive(Debug, QueryVariables)]
pub struct UserSearchVariables {
    pub filter: Option<UserFilter>,
    pub first: Option<i32>,
}

#[derive(Debug, Clone, InputObject)]
#[cynic(schema_path = "schema.graphql")]
pub struct UserFilter {
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub or: Option<Vec<UserFilter>>,
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub name: Option<StringComparator>,
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<StringComparator>,
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub email: Option<StringComparator>,
}

#[derive(Debug, Clone, InputObject)]
#[cynic(schema_path = "schema.graphql")]
pub struct StringComparator {
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub contains_ignore_case: Option<String>,
}

#[derive(Debug, Clone, QueryFragment)]
#[cynic(schema_path = "schema.graphql", graphql_type = "User")]
pub struct SearchedUser {
    pub id: cynic::Id,
    pub name: String,
    pub display_name: String,
    pub url: String,
    pub is_me: bool,
}

#[derive(Debug, Clone, QueryFragment)]
#[cynic(schema_path = "schema.graphql", graphql_type = "UserConnection")]
pub struct UserConnection {
    pub nodes: Vec<SearchedUser>,
}

#[derive(Debug, QueryFragment)]
#[cynic(
    schema_path = "schema.graphql",
    graphql_type = "Query",
    variables = "UserSearchVariables"
)]
pub struct UserSearchQuery {
    #[arguments(filter: $filter, first: $first)]
    pub users: UserConnection,
}

impl UserFilter {
    pub fn matching(term: &str) -> Self {
        let contains = |term: &str| StringComparator {
            contains_ignore_case: Some(term.to_string()),
        };

        Self {
            or: Some(vec![
                Self {
                    display_name: Some(contains(term)),
                    ..Self::empty()
                },
                Self {
                    name: Some(contains(term)),
                    ..Self::empty()
                },
                Self {
                    email: Some(contains(term)),
                    ..Self::empty()
                },
            ]),
            ..Self::empty()
        }
    }

    fn empty() -> Self {
        Self {
            or: None,
            name: None,
            display_name: None,
            email: None,
        }
    }
}
