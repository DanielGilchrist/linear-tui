use crate::api::{IssueSummary, Priority, StateType};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroupBy {
    None,
    Status,
    Priority,
    Assignee,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortBy {
    Manual,
    Priority,
    Title,
    Updated,
}

impl GroupBy {
    pub fn next(self) -> Self {
        match self {
            GroupBy::None => GroupBy::Status,
            GroupBy::Status => GroupBy::Priority,
            GroupBy::Priority => GroupBy::Assignee,
            GroupBy::Assignee => GroupBy::None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            GroupBy::None => "none",
            GroupBy::Status => "status",
            GroupBy::Priority => "priority",
            GroupBy::Assignee => "assignee",
        }
    }
}

impl SortBy {
    pub fn next(self) -> Self {
        match self {
            SortBy::Manual => SortBy::Priority,
            SortBy::Priority => SortBy::Title,
            SortBy::Title => SortBy::Updated,
            SortBy::Updated => SortBy::Manual,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            SortBy::Manual => "manual",
            SortBy::Priority => "priority",
            SortBy::Title => "title",
            SortBy::Updated => "updated",
        }
    }
}

/// How an issue list is presented: which dimension it is grouped by and how each
/// group is sorted. New display options gain their home here rather than as loose
/// fields elsewhere.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Display {
    pub group: GroupBy,
    pub sort: SortBy,
}

impl Display {
    pub fn new() -> Self {
        Self {
            group: GroupBy::Status,
            sort: SortBy::Manual,
        }
    }

    pub fn cycle_group(&mut self) {
        self.group = self.group.next();
    }

    pub fn cycle_sort(&mut self) {
        self.sort = self.sort.next();
    }

    pub fn arrange(&self, issues: &[IssueSummary]) -> Vec<Group> {
        arrange(issues, self.group, self.sort)
    }

    pub fn order(&self, issues: &[IssueSummary]) -> Vec<usize> {
        self.arrange(issues)
            .into_iter()
            .flat_map(|group| group.indices)
            .collect()
    }
}

impl Default for Display {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Group {
    pub label: Option<String>,
    pub indices: Vec<usize>,
}

/// Bucket issue indices into display groups (ordered) and sort within each group.
/// The concatenation of every group's indices is the flat display order.
pub fn arrange(issues: &[IssueSummary], group: GroupBy, sort: SortBy) -> Vec<Group> {
    let mut groups = bucket(issues, group);
    for group in &mut groups {
        sort_indices(&mut group.indices, issues, sort);
    }
    groups
}

fn bucket(issues: &[IssueSummary], group: GroupBy) -> Vec<Group> {
    match group {
        GroupBy::None => vec![Group {
            label: None,
            indices: (0..issues.len()).collect(),
        }],
        GroupBy::Status => {
            let mut buckets: Vec<(u8, String, Vec<usize>)> = Vec::new();
            for (index, issue) in issues.iter().enumerate() {
                match buckets
                    .iter_mut()
                    .find(|(_, name, _)| *name == issue.state.name)
                {
                    Some((_, _, indices)) => indices.push(index),
                    None => buckets.push((
                        status_rank(issue.state.state_type),
                        issue.state.name.clone(),
                        vec![index],
                    )),
                }
            }
            buckets.sort_by_key(|(rank, _, _)| *rank);
            buckets
                .into_iter()
                .map(|(_, name, indices)| Group {
                    label: Some(name),
                    indices,
                })
                .collect()
        }
        GroupBy::Priority => [
            Priority::Urgent,
            Priority::High,
            Priority::Medium,
            Priority::Low,
            Priority::None,
        ]
        .into_iter()
        .filter_map(|priority| {
            let indices: Vec<usize> = issues
                .iter()
                .enumerate()
                .filter(|(_, issue)| issue.priority == priority)
                .map(|(index, _)| index)
                .collect();

            (!indices.is_empty()).then(|| Group {
                label: Some(priority.label().to_string()),
                indices,
            })
        })
        .collect(),
        GroupBy::Assignee => {
            let mut buckets: Vec<(String, Vec<usize>)> = Vec::new();
            for (index, issue) in issues.iter().enumerate() {
                let name = issue
                    .assignee
                    .as_ref()
                    .map(|user| user.display_name.clone())
                    .unwrap_or_else(|| "Unassigned".to_string());

                match buckets.iter_mut().find(|(existing, _)| *existing == name) {
                    Some((_, indices)) => indices.push(index),
                    None => buckets.push((name, vec![index])),
                }
            }
            buckets.sort_by(|(a, _), (b, _)| {
                let key = |name: &str| (name == "Unassigned", name.to_lowercase());
                key(a).cmp(&key(b))
            });
            buckets
                .into_iter()
                .map(|(name, indices)| Group {
                    label: Some(name),
                    indices,
                })
                .collect()
        }
    }
}

fn sort_indices(indices: &mut [usize], issues: &[IssueSummary], sort: SortBy) {
    match sort {
        SortBy::Manual => {}
        SortBy::Priority => indices.sort_by_key(|&index| priority_rank(issues[index].priority)),
        SortBy::Title => indices.sort_by_key(|&index| {
            issues[index]
                .title
                .clone()
                .unwrap_or_default()
                .to_lowercase()
        }),
        SortBy::Updated => {
            indices.sort_by_key(|&index| std::cmp::Reverse(issues[index].updated_at.epoch()))
        }
    }
}

fn status_rank(state_type: StateType) -> u8 {
    match state_type {
        StateType::Triage => 0,
        StateType::Started => 1,
        StateType::Unstarted => 2,
        StateType::Backlog => 3,
        StateType::Completed => 4,
        StateType::Cancelled => 5,
    }
}

fn priority_rank(priority: Priority) -> u8 {
    match priority {
        Priority::None => u8::MAX,
        other => u8::from(other),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::{User, WorkflowState};

    fn issue(identifier: &str, state: &str, ty: StateType, priority: Priority) -> IssueSummary {
        IssueSummary {
            id: identifier.into(),
            identifier: identifier.into(),
            title: Some(identifier.into()),
            state: WorkflowState {
                name: state.into(),
                state_type: ty,
            },
            priority,
            assignee: None,
            labels: Vec::new(),
            url: String::new(),
            branch_name: String::new(),
            team_id: String::new(),
            updated_at: crate::api::Timestamp::default(),
        }
    }

    fn labels(groups: &[Group]) -> Vec<(Option<&str>, usize)> {
        groups
            .iter()
            .map(|group| (group.label.as_deref(), group.indices.len()))
            .collect()
    }

    #[test]
    fn none_is_a_single_unlabelled_group_in_api_order() {
        let issues = vec![
            issue("A", "Todo", StateType::Unstarted, Priority::Low),
            issue("B", "Done", StateType::Completed, Priority::Urgent),
        ];
        let groups = arrange(&issues, GroupBy::None, SortBy::Manual);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].label, None);
        assert_eq!(groups[0].indices, vec![0, 1]);
    }

    #[test]
    fn status_groups_follow_the_workflow_order() {
        let issues = vec![
            issue("A", "Backlog", StateType::Backlog, Priority::None),
            issue("B", "In Progress", StateType::Started, Priority::None),
            issue("C", "Todo", StateType::Unstarted, Priority::None),
            issue("D", "In Progress", StateType::Started, Priority::None),
        ];
        let groups = arrange(&issues, GroupBy::Status, SortBy::Manual);
        assert_eq!(
            labels(&groups),
            vec![
                (Some("In Progress"), 2),
                (Some("Todo"), 1),
                (Some("Backlog"), 1),
            ]
        );
        assert_eq!(groups[0].indices, vec![1, 3]);
    }

    #[test]
    fn priority_sort_orders_urgent_first_and_none_last() {
        let issues = vec![
            issue("A", "Todo", StateType::Unstarted, Priority::None),
            issue("B", "Todo", StateType::Unstarted, Priority::Urgent),
            issue("C", "Todo", StateType::Unstarted, Priority::Medium),
        ];
        let groups = arrange(&issues, GroupBy::None, SortBy::Priority);
        // B (urgent), C (medium), A (none)
        assert_eq!(groups[0].indices, vec![1, 2, 0]);
    }

    #[test]
    fn priority_groups_skip_empty_buckets() {
        let issues = vec![
            issue("A", "Todo", StateType::Unstarted, Priority::Urgent),
            issue("B", "Todo", StateType::Unstarted, Priority::Low),
        ];
        let groups = arrange(&issues, GroupBy::Priority, SortBy::Manual);
        assert_eq!(labels(&groups), vec![(Some("Urgent"), 1), (Some("Low"), 1)]);
    }

    #[test]
    fn title_sort_is_case_insensitive() {
        let issues = vec![
            issue("banana", "Todo", StateType::Unstarted, Priority::None),
            issue("Apple", "Todo", StateType::Unstarted, Priority::None),
        ];
        let groups = arrange(&issues, GroupBy::None, SortBy::Title);
        assert_eq!(groups[0].indices, vec![1, 0]);
    }

    #[test]
    fn display_cycles_group_and_sort_and_orders_flat() {
        let mut display = Display::new();
        assert_eq!(display.group, GroupBy::Status);
        assert_eq!(display.sort, SortBy::Manual);
        display.cycle_group();
        display.cycle_sort();
        assert_eq!(display.group, GroupBy::Priority);
        assert_eq!(display.sort, SortBy::Priority);

        let issues = vec![
            issue("A", "Todo", StateType::Unstarted, Priority::Low),
            issue("B", "Todo", StateType::Unstarted, Priority::Urgent),
        ];
        assert_eq!(display.order(&issues), vec![1, 0]);
    }

    #[test]
    fn assignee_groups_put_unassigned_last() {
        let mut with_assignee = issue("A", "Todo", StateType::Unstarted, Priority::None);
        with_assignee.assignee = Some(User {
            id: "u".into(),
            name: "sam".into(),
            display_name: "sam".into(),
            url: String::new(),
            is_me: false,
        });
        let issues = vec![
            issue("B", "Todo", StateType::Unstarted, Priority::None),
            with_assignee,
        ];
        let groups = arrange(&issues, GroupBy::Assignee, SortBy::Manual);
        assert_eq!(
            labels(&groups),
            vec![(Some("sam"), 1), (Some("Unassigned"), 1)]
        );
    }
}
