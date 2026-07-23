pub(crate) mod schema {
    cynic::use_schema!("schema.graphql");
}

pub mod actions;
pub mod custom_views;
pub mod issue;
pub mod my_issues;
pub mod notifications;
pub mod scalars;
pub mod search;
pub mod viewer;
