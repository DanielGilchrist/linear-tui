use cynic::{InlineFragments, QueryFragment, QueryVariables};

use super::scalars::DateTime;
use super::schema;

#[derive(Debug, QueryFragment)]
#[cynic(
    schema_path = "schema.graphql",
    graphql_type = "Query",
    variables = "NotificationsVariables"
)]
pub struct NotificationsQuery {
    #[arguments(first: $first)]
    pub notifications: NotificationConnection,
}

#[derive(Debug, QueryVariables)]
pub struct NotificationsVariables {
    pub first: Option<i32>,
}

#[derive(Debug, QueryFragment)]
#[cynic(schema_path = "schema.graphql")]
pub struct NotificationConnection {
    pub nodes: Vec<Notification>,
}

#[derive(Debug, InlineFragments)]
#[cynic(schema_path = "schema.graphql", graphql_type = "Notification")]
#[allow(clippy::enum_variant_names)]
pub enum Notification {
    IssueNotification(IssueNotificationFields),
    ProjectNotification(ProjectNotificationFields),
    DocumentNotification(DocumentNotificationFields),
    #[cynic(fallback)]
    Other(OtherNotificationFields),
}

#[derive(Debug, QueryFragment)]
#[cynic(schema_path = "schema.graphql", graphql_type = "IssueNotification")]
pub struct IssueNotificationFields {
    pub title: String,
    pub issue_id: Option<String>,
    pub read_at: Option<DateTime>,
    pub grouping_key: String,
}

#[derive(Debug, QueryFragment)]
#[cynic(schema_path = "schema.graphql", graphql_type = "ProjectNotification")]
pub struct ProjectNotificationFields {
    pub title: String,
    pub read_at: Option<DateTime>,
    pub grouping_key: String,
}

#[derive(Debug, QueryFragment)]
#[cynic(schema_path = "schema.graphql", graphql_type = "DocumentNotification")]
pub struct DocumentNotificationFields {
    pub title: String,
    pub read_at: Option<DateTime>,
    pub grouping_key: String,
}

#[derive(Debug, QueryFragment)]
#[cynic(schema_path = "schema.graphql", graphql_type = "Notification")]
pub struct OtherNotificationFields {
    pub title: String,
    pub read_at: Option<DateTime>,
    pub grouping_key: String,
}

impl Notification {
    pub fn title(&self) -> &str {
        match self {
            Notification::IssueNotification(n) => &n.title,
            Notification::ProjectNotification(n) => &n.title,
            Notification::DocumentNotification(n) => &n.title,
            Notification::Other(n) => &n.title,
        }
    }

    pub fn is_read(&self) -> bool {
        match self {
            Notification::IssueNotification(n) => n.read_at.is_some(),
            Notification::ProjectNotification(n) => n.read_at.is_some(),
            Notification::DocumentNotification(n) => n.read_at.is_some(),
            Notification::Other(n) => n.read_at.is_some(),
        }
    }

    pub fn grouping_key(&self) -> &str {
        match self {
            Notification::IssueNotification(n) => &n.grouping_key,
            Notification::ProjectNotification(n) => &n.grouping_key,
            Notification::DocumentNotification(n) => &n.grouping_key,
            Notification::Other(n) => &n.grouping_key,
        }
    }

    pub fn issue_id(&self) -> Option<&str> {
        match self {
            Notification::IssueNotification(n) => n.issue_id.as_deref(),
            _ => None,
        }
    }
}
