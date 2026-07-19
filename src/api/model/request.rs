use serde::{Deserialize, Serialize};

use super::scalar::StateType;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct IssueUpdate {
    pub state_id: Option<String>,
    pub assignee_id: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct IssueFilter {
    #[serde(default)]
    pub assigned_to_me: bool,
    #[serde(default)]
    pub created_by_me: bool,
    #[serde(default)]
    pub state_types_in: Vec<StateType>,
    #[serde(default)]
    pub state_types_nin: Vec<StateType>,
    #[serde(default)]
    pub label: Option<String>,
}

impl IssueFilter {
    pub fn assigned_to_me() -> Self {
        Self {
            assigned_to_me: true,
            state_types_nin: vec![StateType::Completed, StateType::Cancelled],
            ..Default::default()
        }
    }

    pub fn in_progress_mine() -> Self {
        Self {
            assigned_to_me: true,
            state_types_in: vec![StateType::Started],
            ..Default::default()
        }
    }
}
