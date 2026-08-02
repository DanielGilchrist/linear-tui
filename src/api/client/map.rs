use crate::api::model::{
    Comment, IssueDetail, IssueFilter, IssueSummary, Label, NotificationItem, Priority, Reaction,
    Rgb, SavedView, StateOption, StateType, Team, User, UserId, WorkflowState,
};
use crate::api::queries::my_issues::{
    self, BooleanComparator, IdComparator, NullableUserFilter, StringComparator, TeamFilter,
    WorkflowStateFilter,
};
use crate::api::queries::notifications::Notification;
use crate::api::queries::{actions, custom_views, issue, search, teams, users, viewer};

pub(super) fn build_cynic_filter(filter: &IssueFilter) -> my_issues::IssueFilter {
    let me = || NullableUserFilter {
        is_me: Some(BooleanComparator { eq: Some(true) }),
    };

    let api_types = |types: &[StateType]| -> Option<Vec<String>> {
        (!types.is_empty()).then(|| types.iter().map(|t| t.as_api().to_string()).collect())
    };

    let state = if !filter.state_types_in.is_empty() || !filter.state_types_not_in.is_empty() {
        Some(WorkflowStateFilter {
            type_: Some(StringComparator {
                eq: None,
                in_: api_types(&filter.state_types_in),
                nin: api_types(&filter.state_types_not_in),
            }),
        })
    } else {
        None
    };

    let team = filter.team.as_ref().map(|team| TeamFilter {
        id: Some(IdComparator {
            eq: Some(team.to_string().into()),
        }),
    });

    my_issues::IssueFilter {
        assignee: filter.assigned_to_me.then(me),
        creator: filter.created_by_me.then(me),
        state,
        team,
    }
}

impl From<my_issues::Issue> for IssueSummary {
    fn from(issue: my_issues::Issue) -> Self {
        Self {
            id: issue.id.into(),
            identifier: issue.identifier,
            title: issue.title,
            state: WorkflowState {
                name: issue.state.name,
                state_type: StateType::from_api(&issue.state.state_type),
            },
            priority: Priority::from(issue.priority as u8),
            assignee: issue.assignee.map(|a| named_user(a.display_name)),
            labels: issue
                .labels
                .nodes
                .into_iter()
                .map(|l| Label {
                    id: l.id.into(),
                    name: l.name,
                    colour: Rgb::parse_hex(&l.colour),
                })
                .collect(),
            url: issue.url,
            branch_name: issue.branch_name,
            team_id: issue.team.id.into(),
            updated_at: issue.updated_at.0.into(),
        }
    }
}

impl From<search::IssueSearchResult> for IssueSummary {
    fn from(issue: search::IssueSearchResult) -> Self {
        Self {
            id: issue.id.into(),
            identifier: issue.identifier,
            title: Some(issue.title),
            state: WorkflowState {
                name: issue.state.name,
                state_type: StateType::from_api(&issue.state.state_type),
            },
            priority: Priority::from(issue.priority as u8),
            assignee: issue.assignee.map(|a| named_user(a.display_name)),
            labels: issue
                .labels
                .nodes
                .into_iter()
                .map(|l| Label {
                    id: l.id.into(),
                    name: l.name,
                    colour: Rgb::parse_hex(&l.colour),
                })
                .collect(),
            url: issue.url,
            branch_name: issue.branch_name,
            team_id: issue.team.id.into(),
            updated_at: issue.updated_at.0.into(),
        }
    }
}

impl From<custom_views::Issue> for IssueSummary {
    fn from(issue: custom_views::Issue) -> Self {
        Self {
            id: issue.id.into(),
            identifier: issue.identifier,
            title: issue.title,
            state: WorkflowState {
                name: issue.state.name,
                state_type: StateType::from_api(&issue.state.state_type),
            },
            priority: Priority::from(issue.priority as u8),
            assignee: issue.assignee.map(|a| named_user(a.display_name)),
            labels: issue
                .labels
                .nodes
                .into_iter()
                .map(|l| Label {
                    id: l.id.into(),
                    name: l.name,
                    colour: Rgb::parse_hex(&l.colour),
                })
                .collect(),
            url: issue.url,
            branch_name: issue.branch_name,
            team_id: issue.team.id.into(),
            updated_at: issue.updated_at.0.into(),
        }
    }
}

impl From<issue::Issue> for IssueDetail {
    fn from(issue: issue::Issue) -> Self {
        Self {
            id: issue.id.into(),
            identifier: issue.identifier,
            title: issue.title,
            description: issue.description,
            url: issue.url,
            state: WorkflowState {
                name: issue.state.name,
                state_type: StateType::from_api(&issue.state.state_type),
            },
            priority: Priority::from(issue.priority as u8),
            assignee: issue.assignee.map(|a| named_user(a.display_name)),
            labels: issue
                .labels
                .nodes
                .into_iter()
                .map(|l| Label {
                    id: l.id.into(),
                    name: l.name,
                    colour: Rgb::parse_hex(&l.colour),
                })
                .collect(),
            comments: issue
                .comments
                .nodes
                .into_iter()
                .map(|c| {
                    let (author, is_mine) = match c.user {
                        Some(user) => (Some(user.display_name), user.is_me),
                        None => (None, false),
                    };

                    Comment {
                        id: c.id.into(),
                        parent_id: c.parent.map(|p| p.id.into()),
                        author,
                        is_mine,
                        body: c.body,
                        created_at: c.created_at.0.into(),
                        reactions: c.reactions.into_iter().map(Reaction::from).collect(),
                    }
                })
                .collect(),
            reactions: issue.reactions.into_iter().map(Reaction::from).collect(),
            branch_name: issue.branch_name,
            team_id: issue.team.id.into(),
            updated_at: issue.updated_at.0.into(),
        }
    }
}

impl From<crate::api::queries::labels::IssueLabel> for Label {
    fn from(label: crate::api::queries::labels::IssueLabel) -> Self {
        Self {
            id: label.id.into(),
            name: label.name,
            colour: Rgb::parse_hex(&label.colour),
        }
    }
}

impl From<issue::Reaction> for Reaction {
    fn from(reaction: issue::Reaction) -> Self {
        Self {
            id: reaction.id.into(),
            emoji: reaction.emoji,
            mine: reaction.user.is_some_and(|user| user.is_me),
        }
    }
}

impl From<&Notification> for NotificationItem {
    fn from(notification: &Notification) -> Self {
        Self {
            title: notification.title().to_string(),
            issue_id: notification.issue_id(),
            is_read: notification.is_read(),
            grouping_key: notification.grouping_key().to_string(),
        }
    }
}

impl From<custom_views::CustomView> for SavedView {
    fn from(view: custom_views::CustomView) -> Self {
        Self {
            id: view.id.into(),
            name: view.name,
        }
    }
}

fn named_user(display_name: String) -> User {
    User {
        id: UserId::default(),
        name: String::new(),
        display_name,
        url: String::new(),
        is_me: false,
    }
}

impl From<teams::TeamNode> for Team {
    fn from(team: teams::TeamNode) -> Self {
        Self {
            id: team.id.into(),
            name: team.name,
            key: team.key,
            triage_enabled: team.triage_enabled,
        }
    }
}

impl From<actions::User> for User {
    fn from(user: actions::User) -> Self {
        Self {
            id: user.id.into(),
            name: user.name,
            display_name: user.display_name,
            url: user.url,
            is_me: user.is_me,
        }
    }
}

impl From<users::SearchedUser> for User {
    fn from(user: users::SearchedUser) -> Self {
        Self {
            id: user.id.into(),
            name: user.name,
            display_name: user.display_name,
            url: user.url,
            is_me: user.is_me,
        }
    }
}

impl From<viewer::User> for User {
    fn from(user: viewer::User) -> Self {
        Self {
            id: user.id.into(),
            name: user.name,
            display_name: user.display_name,
            url: user.url,
            is_me: user.is_me,
        }
    }
}

impl From<actions::WorkflowState> for StateOption {
    fn from(state: actions::WorkflowState) -> Self {
        Self {
            id: state.id.into(),
            name: state.name,
            state_type: StateType::from_api(&state.state_type),
        }
    }
}
