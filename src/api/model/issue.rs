use serde::{Deserialize, Serialize};

use super::id::{CommentId, IssueId, LabelId, ReactionId, StateId, TeamId};
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
    #[serde(default)]
    pub id: LabelId,
    pub name: String,
    pub colour: Rgb,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StateOption {
    pub id: StateId,
    pub name: String,
    #[serde(rename = "type")]
    pub state_type: StateType,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IssueSummary {
    pub id: IssueId,
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
    pub team_id: TeamId,
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Reaction {
    pub id: ReactionId,
    pub emoji: String,
    #[serde(default)]
    pub mine: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Comment {
    #[serde(default)]
    pub id: CommentId,
    #[serde(default)]
    pub parent_id: Option<CommentId>,
    #[serde(default)]
    pub author: Option<String>,
    #[serde(default)]
    pub is_mine: bool,
    pub body: String,
    #[serde(default)]
    pub created_at: Timestamp,
    #[serde(default)]
    pub reactions: Vec<Reaction>,
}

impl Comment {
    pub fn reply_parent(&self) -> CommentId {
        self.parent_id.clone().unwrap_or_else(|| self.id.clone())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IssueDetail {
    pub id: IssueId,
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
    pub reactions: Vec<Reaction>,
    #[serde(default)]
    pub branch_name: String,
    #[serde(default)]
    pub team_id: TeamId,
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
            push_thread(&mut ordered, root, 0, &children);
        }

        ordered
    }

    pub fn thread_len(&self) -> usize {
        self.threaded_comments().len()
    }
}

fn push_thread<'a>(
    ordered: &mut Vec<ThreadedComment<'a>>,
    comment: &'a Comment,
    depth: usize,
    children: &std::collections::HashMap<&str, Vec<&'a Comment>>,
) {
    ordered.push(ThreadedComment { comment, depth });

    if let Some(replies) = children.get(comment.id.as_str()) {
        for reply in replies {
            push_thread(ordered, reply, depth + 1, children);
        }
    }
}

fn by_created_at(a: &&Comment, b: &&Comment) -> std::cmp::Ordering {
    a.created_at.cmp(&b.created_at)
}
