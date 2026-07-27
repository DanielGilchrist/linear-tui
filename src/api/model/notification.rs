use serde::{Deserialize, Serialize};

use super::id::IssueId;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NotificationItem {
    pub title: String,
    #[serde(default)]
    pub issue_id: Option<IssueId>,
    #[serde(default)]
    pub is_read: bool,
    #[serde(default)]
    pub grouping_key: String,
}
