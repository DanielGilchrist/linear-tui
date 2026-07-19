use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NotificationItem {
    pub title: String,
    #[serde(default)]
    pub issue_id: Option<String>,
    #[serde(default)]
    pub is_read: bool,
    #[serde(default)]
    pub grouping_key: String,
}
