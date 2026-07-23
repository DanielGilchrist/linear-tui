use serde::{Deserialize, Serialize};

use super::scalar::{Priority, Rgb, StateType, Timestamp};
use super::user::User;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkflowState {
    pub name: String,
    #[serde(rename = "type")]
    pub state_type: StateType,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Label {
    pub name: String,
    pub colour: Rgb,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StateOption {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub state_type: StateType,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IssueSummary {
    pub id: String,
    pub identifier: String,
    #[serde(default)]
    pub title: Option<String>,
    pub state: WorkflowState,
    #[serde(default)]
    pub priority: Priority,
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
    #[serde(default)]
    pub updated_at: Timestamp,
}

impl IssueSummary {
    pub fn from_detail(detail: &IssueDetail) -> Self {
        Self {
            id: detail.id.clone(),
            identifier: detail.identifier.clone(),
            title: detail.title.clone(),
            state: detail.state.clone(),
            priority: detail.priority,
            assignee: detail.assignee.clone(),
            labels: detail.labels.clone(),
            url: detail.url.clone(),
            branch_name: detail.branch_name.clone(),
            team_id: detail.team_id.clone(),
            updated_at: detail.updated_at,
        }
    }
}

#[derive(Debug, Clone)]
pub struct IssuePage {
    pub issues: Vec<IssueSummary>,
    pub truncated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Comment {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub parent_id: Option<String>,
    #[serde(default)]
    pub author: Option<String>,
    #[serde(default)]
    pub is_mine: bool,
    pub body: String,
    #[serde(default)]
    pub created_at: Timestamp,
}

impl Comment {
    pub fn reply_parent(&self) -> String {
        self.parent_id.clone().unwrap_or_else(|| self.id.clone())
    }
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
    pub priority: Priority,
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
    #[serde(default)]
    pub updated_at: Timestamp,
}

#[derive(Debug, Clone, Copy)]
pub struct ThreadedComment<'a> {
    pub comment: &'a Comment,
    pub depth: usize,
}

impl IssueDetail {
    pub fn threaded_comments(&self) -> Vec<ThreadedComment<'_>> {
        use std::collections::{HashMap, HashSet};

        let known: HashSet<&str> = self
            .comments
            .iter()
            .map(|comment| comment.id.as_str())
            .filter(|id| !id.is_empty())
            .collect();

        let mut children: HashMap<&str, Vec<&Comment>> = HashMap::new();
        let mut roots: Vec<&Comment> = Vec::new();

        for comment in &self.comments {
            match &comment.parent_id {
                Some(parent) if known.contains(parent.as_str()) => {
                    children.entry(parent.as_str()).or_default().push(comment);
                }
                _ => roots.push(comment),
            }
        }

        roots.sort_by(by_created_at);
        for replies in children.values_mut() {
            replies.sort_by(by_created_at);
        }

        let mut ordered = Vec::new();
        for root in roots {
            ordered.push(ThreadedComment {
                comment: root,
                depth: 0,
            });
            if let Some(replies) = children.get(root.id.as_str()) {
                for reply in replies {
                    ordered.push(ThreadedComment {
                        comment: reply,
                        depth: 1,
                    });
                }
            }
        }
        ordered
    }
}

fn by_created_at(a: &&Comment, b: &&Comment) -> std::cmp::Ordering {
    a.created_at.epoch().cmp(&b.created_at.epoch())
}
