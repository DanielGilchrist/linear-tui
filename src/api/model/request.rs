use serde::{Deserialize, Serialize};

use super::id::{CommentId, IssueId, StateId, UserId};
use super::scalar::{Priority, StateType};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IssueUpdate {
    Status(StateId),
    Assignee(Option<UserId>),
    Priority(Priority),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReactionTarget {
    Issue(IssueId),
    Comment(CommentId),
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq, Hash)]
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
