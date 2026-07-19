use crate::api::model::{
    Comment, IssueDetail, IssueFilter, IssueSummary, Label, NotificationItem, Priority, Rgb,
    StateType, User, WorkflowState,
};
use crate::api::queries::my_issues::{
    self, BooleanComparator, NullableUserFilter, StringComparator, WorkflowStateFilter,
};
use crate::api::queries::notifications::Notification;
use crate::api::queries::{issue, search};

pub(super) fn build_cynic_filter(filter: &IssueFilter) -> my_issues::IssueFilter {
    let me = || NullableUserFilter {
        is_me: Some(BooleanComparator { eq: Some(true) }),
    };

    let api_types = |types: &[StateType]| -> Option<Vec<String>> {
        (!types.is_empty()).then(|| types.iter().map(|t| t.as_api().to_string()).collect())
    };

    let state = if !filter.state_types_in.is_empty() || !filter.state_types_nin.is_empty() {
        Some(WorkflowStateFilter {
            type_: Some(StringComparator {
                eq: None,
                in_: api_types(&filter.state_types_in),
                nin: api_types(&filter.state_types_nin),
            }),
        })
    } else {
        None
    };

    my_issues::IssueFilter {
        assignee: filter.assigned_to_me.then(me),
        creator: filter.created_by_me.then(me),
        state,
    }
}

pub(super) fn map_summary(issue: my_issues::Issue) -> IssueSummary {
    IssueSummary {
        id: issue.id.into_inner(),
        identifier: issue.identifier,
        title: issue.title,
        state: WorkflowState {
            name: issue.state.name,
            state_type: StateType::from_api(&issue.state.state_type),
        },
        priority: Priority::from(issue.priority as u8),
        assignee: issue.assignee.map(|a| User {
            id: String::new(),
            name: String::new(),
            display_name: a.display_name,
            url: String::new(),
            is_me: false,
        }),
        labels: issue
            .labels
            .nodes
            .into_iter()
            .map(|l| Label {
                name: l.name,
                colour: Rgb::parse_hex(&l.colour),
            })
            .collect(),
        url: issue.url,
        branch_name: issue.branch_name,
        team_id: issue.team.id.into_inner(),
    }
}

pub(super) fn map_search_result(issue: search::IssueSearchResult) -> IssueSummary {
    IssueSummary {
        id: issue.id.into_inner(),
        identifier: issue.identifier,
        title: Some(issue.title),
        state: WorkflowState {
            name: issue.state.name,
            state_type: StateType::from_api(&issue.state.state_type),
        },
        priority: Priority::from(issue.priority as u8),
        assignee: issue.assignee.map(|a| User {
            id: String::new(),
            name: String::new(),
            display_name: a.display_name,
            url: String::new(),
            is_me: false,
        }),
        labels: issue
            .labels
            .nodes
            .into_iter()
            .map(|l| Label {
                name: l.name,
                colour: Rgb::parse_hex(&l.colour),
            })
            .collect(),
        url: issue.url,
        branch_name: issue.branch_name,
        team_id: issue.team.id.into_inner(),
    }
}

pub(super) fn map_detail(issue: issue::Issue) -> IssueDetail {
    IssueDetail {
        id: issue.id.into_inner(),
        identifier: issue.identifier,
        title: issue.title,
        description: issue.description,
        url: issue.url,
        state: WorkflowState {
            name: issue.state.name,
            state_type: StateType::from_api(&issue.state.state_type),
        },
        priority: Priority::from(issue.priority as u8),
        assignee: issue.assignee.map(|a| User {
            id: String::new(),
            name: String::new(),
            display_name: a.display_name,
            url: String::new(),
            is_me: false,
        }),
        labels: issue
            .labels
            .nodes
            .into_iter()
            .map(|l| Label {
                name: l.name,
                colour: Rgb::parse_hex(&l.colour),
            })
            .collect(),
        comments: issue
            .comments
            .nodes
            .into_iter()
            .map(|c| Comment {
                id: c.id.into_inner(),
                parent_id: c.parent.map(|p| p.id.into_inner()),
                author: c.user.map(|u| u.display_name),
                body: c.body,
                created_at: c.created_at.0.into(),
            })
            .collect(),
        branch_name: issue.branch_name,
        team_id: issue.team.id.into_inner(),
    }
}

pub(super) fn map_notification(notification: &Notification) -> NotificationItem {
    NotificationItem {
        title: notification.title().to_string(),
        issue_id: notification.issue_id().map(|s| s.to_string()),
        is_read: notification.is_read(),
        grouping_key: notification.grouping_key().to_string(),
    }
}
