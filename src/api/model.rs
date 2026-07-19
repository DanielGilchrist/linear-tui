use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct User {
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub display_name: String,
    #[serde(default)]
    pub is_me: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Session {
    pub user: User,
    pub org_name: String,
    #[serde(default)]
    pub org_url_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkflowState {
    pub name: String,
    #[serde(rename = "type")]
    pub state_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Label {
    pub name: String,
    pub color: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IssueSummary {
    pub id: String,
    pub identifier: String,
    #[serde(default)]
    pub title: Option<String>,
    pub state: WorkflowState,
    #[serde(default)]
    pub priority: u8,
    #[serde(default)]
    pub assignee: Option<User>,
    #[serde(default)]
    pub labels: Vec<Label>,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub branch_name: String,
    #[serde(default)]
    pub team_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Comment {
    #[serde(default)]
    pub author: Option<String>,
    pub body: String,
    #[serde(default)]
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IssueDetail {
    pub id: String,
    pub identifier: String,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    pub url: String,
    pub state: WorkflowState,
    #[serde(default)]
    pub priority: u8,
    #[serde(default)]
    pub assignee: Option<User>,
    #[serde(default)]
    pub labels: Vec<Label>,
    #[serde(default)]
    pub comments: Vec<Comment>,
    #[serde(default)]
    pub branch_name: String,
    #[serde(default)]
    pub team_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StateOption {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub state_type: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct IssueUpdate {
    pub state_id: Option<String>,
    pub assignee_id: Option<String>,
}

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

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct IssueFilter {
    #[serde(default)]
    pub assigned_to_me: bool,
    #[serde(default)]
    pub created_by_me: bool,
    #[serde(default)]
    pub state_types_in: Vec<String>,
    #[serde(default)]
    pub state_types_nin: Vec<String>,
    #[serde(default)]
    pub label: Option<String>,
}

impl IssueFilter {
    pub fn assigned_to_me() -> Self {
        Self {
            assigned_to_me: true,
            state_types_nin: vec!["completed".into(), "canceled".into()],
            ..Default::default()
        }
    }

    pub fn in_progress_mine() -> Self {
        Self {
            assigned_to_me: true,
            state_types_in: vec!["started".into()],
            ..Default::default()
        }
    }
}
