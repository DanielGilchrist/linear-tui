use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::api::model::{
    Comment, IssueDetail, IssueFilter, IssueSummary, IssueUpdate, Label, NotificationItem,
    Priority, Rgb, Session, StateOption, StateType, User, WorkflowState,
};
use crate::api::{ApiResult, LinearApi};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fixture {
    pub viewer: User,
    #[serde(default)]
    pub org_name: String,
    #[serde(default)]
    pub org_url_key: String,
    #[serde(default)]
    pub notifications: Vec<NotificationItem>,
    #[serde(default)]
    pub issues: Vec<IssueSummary>,
    #[serde(default)]
    pub details: Vec<IssueDetail>,
}

pub struct FixtureClient {
    fixture: Fixture,
}

impl FixtureClient {
    pub fn new(fixture: Fixture) -> Self {
        Self { fixture }
    }

    pub fn from_path(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let raw = std::fs::read_to_string(path)
            .with_context(|| format!("reading fixture {}", path.display()))?;
        let fixture: Fixture = serde_json::from_str(&raw)
            .with_context(|| format!("parsing fixture {}", path.display()))?;
        Ok(Self::new(fixture))
    }

    pub fn sample() -> Self {
        Self::new(sample_fixture())
    }
}

fn matches(issue: &IssueSummary, filter: &IssueFilter) -> bool {
    let ty = &issue.state.state_type;
    if !filter.state_types_in.is_empty() && !filter.state_types_in.contains(ty) {
        return false;
    }
    if filter.state_types_nin.contains(ty) {
        return false;
    }
    true
}

#[async_trait::async_trait]
impl LinearApi for FixtureClient {
    async fn session(&self) -> ApiResult<Session> {
        Ok(Session {
            user: self.fixture.viewer.clone(),
            org_name: self.fixture.org_name.clone(),
            org_url_key: self.fixture.org_url_key.clone(),
        })
    }

    async fn issues(&self, filter: &IssueFilter) -> ApiResult<Vec<IssueSummary>> {
        Ok(self
            .fixture
            .issues
            .iter()
            .filter(|issue| matches(issue, filter))
            .cloned()
            .collect())
    }

    async fn search_issues(&self, term: &str) -> ApiResult<Vec<IssueSummary>> {
        let needle = term.to_lowercase();
        Ok(self
            .fixture
            .issues
            .iter()
            .filter(|issue| {
                issue.identifier.to_lowercase().contains(&needle)
                    || issue
                        .title
                        .as_deref()
                        .is_some_and(|title| title.to_lowercase().contains(&needle))
            })
            .cloned()
            .collect())
    }

    async fn issue_detail(&self, id: &str) -> ApiResult<Option<IssueDetail>> {
        Ok(self
            .fixture
            .details
            .iter()
            .find(|d| d.id == id || d.identifier == id)
            .cloned())
    }

    async fn notifications(&self) -> ApiResult<Vec<NotificationItem>> {
        Ok(self.fixture.notifications.clone())
    }

    async fn workflow_states(&self, _team_id: &str) -> ApiResult<Vec<StateOption>> {
        Ok(vec![
            StateOption {
                id: "s_backlog".into(),
                name: "Backlog".into(),
                state_type: StateType::Backlog,
            },
            StateOption {
                id: "s_todo".into(),
                name: "Todo".into(),
                state_type: StateType::Unstarted,
            },
            StateOption {
                id: "s_started".into(),
                name: "In Progress".into(),
                state_type: StateType::Started,
            },
            StateOption {
                id: "s_done".into(),
                name: "Done".into(),
                state_type: StateType::Completed,
            },
            StateOption {
                id: "s_canceled".into(),
                name: "Cancelled".into(),
                state_type: StateType::Cancelled,
            },
        ])
    }

    async fn team_members(&self, _team_id: &str) -> ApiResult<Vec<User>> {
        Ok(vec![
            person("dan", true),
            person("sam", false),
            person("alex", false),
        ])
    }

    async fn update_issue(&self, _id: &str, _update: IssueUpdate) -> ApiResult<()> {
        Ok(())
    }

    async fn create_comment(
        &self,
        _issue_id: &str,
        _body: &str,
        _parent_id: Option<&str>,
    ) -> ApiResult<()> {
        Ok(())
    }
}

fn state(name: &str, state_type: StateType) -> WorkflowState {
    WorkflowState {
        name: name.into(),
        state_type,
    }
}

fn person(display_name: &str, is_me: bool) -> User {
    User {
        id: format!("u_{display_name}"),
        name: display_name.into(),
        display_name: display_name.into(),
        url: format!("https://linear.app/dans-donuts/profiles/{display_name}"),
        is_me,
    }
}

fn summary(
    id: &str,
    identifier: &str,
    title: &str,
    st: WorkflowState,
    priority: Priority,
    assignee: &str,
    labels: &[(&str, &str)],
) -> IssueSummary {
    IssueSummary {
        id: id.into(),
        identifier: identifier.into(),
        title: Some(title.into()),
        state: st,
        priority,
        assignee: Some(person(assignee, assignee == "dan")),
        labels: labels
            .iter()
            .map(|(name, colour)| Label {
                name: (*name).into(),
                colour: Rgb::parse_hex(colour),
            })
            .collect(),
        url: format!("https://linear.app/dans-donuts/issue/{identifier}"),
        branch_name: format!("dan/{}", identifier.to_lowercase()),
        team_id: team_for(identifier),
    }
}

fn team_for(identifier: &str) -> String {
    if identifier.starts_with("DAN2") {
        "t_pizza".into()
    } else {
        "t_donut".into()
    }
}

fn sample_fixture() -> Fixture {
    let issues = vec![
        summary(
            "i1",
            "DAN2-7",
            "Wood-fired oven runs 40°C too hot on Friday nights",
            state("In Progress", StateType::Started),
            Priority::Urgent,
            "dan",
            &[("oven", "#eb5757")],
        ),
        summary(
            "i2",
            "DAN-10",
            "Sprinkle dispenser jams during the morning rush",
            state("In Progress", StateType::Started),
            Priority::Urgent,
            "dan",
            &[("production", "#eb5757")],
        ),
        summary(
            "i3",
            "DAN2-2",
            "Delivery driver GPS points to the old shopfront",
            state("In Progress", StateType::Started),
            Priority::High,
            "dan",
            &[("delivery", "#5e6ad2")],
        ),
        summary(
            "i4",
            "DAN2-3",
            "Add gluten-free base option to the online menu",
            state("Todo", StateType::Unstarted),
            Priority::Urgent,
            "dan",
            &[("menu", "#0f9d58")],
        ),
        summary(
            "i5",
            "DAN-13",
            "Introduce a maple-bacon donut for the winter menu",
            state("Todo", StateType::Unstarted),
            Priority::High,
            "dan",
            &[("menu", "#0f9d58")],
        ),
        summary(
            "i6",
            "DAN2-5",
            "Settle the pineapple-on-pizza debate once and for all",
            state("Backlog", StateType::Backlog),
            Priority::High,
            "dan",
            &[("customer-poll", "#f2c94c")],
        ),
        summary(
            "i7",
            "DAN-15",
            "Coffee pairing bundle for donut boxes",
            state("Backlog", StateType::Backlog),
            Priority::None,
            "dan",
            &[("upsell", "#f2c94c")],
        ),
    ];

    let details = vec![IssueDetail {
        id: "i1".into(),
        identifier: "DAN2-7".into(),
        title: Some("Wood-fired oven runs 40°C too hot on Friday nights".into()),
        description: Some(
            r#"## Symptoms

During the Friday rush the stone oven creeps past **480°C** and bases scorch before the cheese melts.

- Expected: steady `430°C`
- Actual: `470-480°C`
- Suspect the flue damper is sticking open

### Checklist

- [x] Swap the thermocouple
- [ ] Inspect the flue damper
- [ ] Recalibrate the PID loop

> Damper was replaced 6 months ago, should still be under warranty.

See the [vendor runbook](https://example.com/runbook) for the reset steps:

```
sudo oven-ctl --reset-pid
oven-ctl --set-target 430
```
"#
            .into(),
        ),
        url: "https://linear.app/dans-donuts/issue/DAN2-7/wood-fired-oven-runs-too-hot".into(),
        state: state("In Progress", StateType::Started),
        priority: Priority::Urgent,
        assignee: Some(person("dan", true)),
        labels: vec![Label {
            name: "oven".into(),
            colour: Rgb::parse_hex("#eb5757"),
        }],
        comments: vec![
            Comment {
                id: "c1".into(),
                parent_id: None,
                author: Some("dan".into()),
                body: "Swapped the thermocouple this morning. Readings so far:\n\n1. 6pm - `445°C`\n2. 7pm - `462°C`".into(),
                created_at: "2026-07-16T09:24:00Z".into(),
            },
            Comment {
                id: "c1a".into(),
                parent_id: Some("c1".into()),
                author: Some("danniiee".into()),
                body: "Agreed, the sensor looks fine. Next suspect is the `flue damper`.".into(),
                created_at: "2026-07-16T10:02:00Z".into(),
            },
            Comment {
                id: "c1b".into(),
                parent_id: Some("c1".into()),
                author: Some("dan".into()),
                body: "Adding the damper check to the list.".into(),
                created_at: "2026-07-16T10:05:00Z".into(),
            },
            Comment {
                id: "c2".into(),
                parent_id: None,
                author: Some("dan".into()),
                body: "Still climbing. Confirmed the flue damper is sticking open.".into(),
                created_at: "2026-07-16T18:40:00Z".into(),
            },
        ],
        branch_name: "dan/dan2-7".into(),
        team_id: "t_pizza".into(),
    }];

    let notifications = vec![
        NotificationItem {
            title: "New comment on DAN2-7 (wood-fired oven)".into(),
            issue_id: Some("i1".into()),
            is_read: false,
            grouping_key: "g1".into(),
        },
        NotificationItem {
            title: "You were assigned DAN-10 (sprinkle dispenser jams)".into(),
            issue_id: Some("i2".into()),
            is_read: false,
            grouping_key: "g2".into(),
        },
        NotificationItem {
            title: "DAN2-5 moved to Backlog (pineapple debate)".into(),
            issue_id: Some("i6".into()),
            is_read: true,
            grouping_key: "g3".into(),
        },
    ];

    Fixture {
        viewer: person("dan", true),
        org_name: "Dan's Donuts".into(),
        org_url_key: "dans-donuts".into(),
        notifications,
        issues,
        details,
    }
}
