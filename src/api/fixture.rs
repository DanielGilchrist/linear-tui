use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::api::model::{
    Comment, CommentId, Cursor, IssueDetail, IssueFilter, IssueId, IssueRef, IssueSummary,
    IssueUpdate, Label, LabelId, NotificationItem, Page, Priority, Reaction, ReactionId,
    ReactionTarget, Rgb, SavedView, Session, StateId, StateOption, StateType, Team, TeamId, User,
    UserId, ViewId, WorkflowState,
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
    pub saved_views: Vec<SavedView>,
    #[serde(default)]
    pub issues: Vec<IssueSummary>,
    #[serde(default)]
    pub saved_view_issues: std::collections::HashMap<ViewId, Vec<IssueSummary>>,
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

    async fn custom_views(&self) -> ApiResult<Vec<SavedView>> {
        Ok(self.fixture.saved_views.clone())
    }

    async fn custom_view_issues(
        &self,
        id: &ViewId,
        _after: Option<&Cursor>,
    ) -> ApiResult<Page<IssueSummary>> {
        let issues = self
            .fixture
            .saved_view_issues
            .get(id)
            .cloned()
            .unwrap_or_else(|| self.fixture.issues.clone());

        Ok(Page::single(issues))
    }

    async fn issues(
        &self,
        filter: &IssueFilter,
        _after: Option<&Cursor>,
    ) -> ApiResult<Page<IssueSummary>> {
        Ok(Page::single(
            self.fixture
                .issues
                .iter()
                .filter(|issue| matches(issue, filter))
                .cloned()
                .collect(),
        ))
    }

    async fn search_issues(
        &self,
        term: &str,
        _after: Option<&Cursor>,
    ) -> ApiResult<Page<IssueSummary>> {
        let needle = term.to_lowercase();
        Ok(Page::single(
            self.fixture
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
                .collect(),
        ))
    }

    async fn issue_detail(&self, target: &IssueRef) -> ApiResult<Option<IssueDetail>> {
        Ok(self
            .fixture
            .details
            .iter()
            .find(|detail| target.matches_detail(detail))
            .cloned())
    }

    async fn notifications(&self, _after: Option<&Cursor>) -> ApiResult<Page<NotificationItem>> {
        Ok(Page::single(self.fixture.notifications.clone()))
    }

    async fn workflow_states(&self, _team_id: &TeamId) -> ApiResult<Vec<StateOption>> {
        Ok(vec![
            StateOption {
                id: StateId::from_raw("s_backlog"),
                name: "Backlog".into(),
                state_type: StateType::Backlog,
            },
            StateOption {
                id: StateId::from_raw("s_todo"),
                name: "Todo".into(),
                state_type: StateType::Unstarted,
            },
            StateOption {
                id: StateId::from_raw("s_started"),
                name: "In Progress".into(),
                state_type: StateType::Started,
            },
            StateOption {
                id: StateId::from_raw("s_done"),
                name: "Done".into(),
                state_type: StateType::Completed,
            },
            StateOption {
                id: StateId::from_raw("s_canceled"),
                name: "Cancelled".into(),
                state_type: StateType::Cancelled,
            },
        ])
    }

    async fn teams(&self) -> ApiResult<Vec<Team>> {
        Ok(vec![
            Team {
                id: TeamId::from_raw("t_donut"),
                name: "Donuts".into(),
                key: "DAN".into(),
            },
            Team {
                id: TeamId::from_raw("t_pizza"),
                name: "Pizza".into(),
                key: "DAN2".into(),
            },
        ])
    }

    async fn team_members(&self, _team_id: &TeamId) -> ApiResult<Vec<User>> {
        Ok(vec![
            person("dan", true),
            person("sam", false),
            person("alex", false),
        ])
    }

    async fn search_users(&self, term: &str) -> ApiResult<Vec<User>> {
        let needle = term.to_lowercase();

        Ok(["dan", "sam", "alex", "danniiee", "charlieh"]
            .into_iter()
            .filter(|name| name.contains(&needle))
            .map(|name| person(name, name == "dan"))
            .collect())
    }

    async fn search_labels(&self, term: &str) -> ApiResult<Vec<Label>> {
        let needle = term.to_lowercase();

        Ok([
            ("lbl_oven", "oven", "#eb5757"),
            ("lbl_upsell", "upsell", "#f2c94c"),
            ("lbl_bug", "bug", "#9b51e0"),
            ("lbl_chore", "chore", "#4f4f4f"),
        ]
        .into_iter()
        .filter(|(_, name, _)| name.contains(&needle))
        .map(|(id, name, colour)| Label {
            id: LabelId::from_raw(id),
            name: name.into(),
            colour: Rgb::parse_hex(colour),
        })
        .collect())
    }

    async fn update_issue(&self, _id: &IssueId, _update: IssueUpdate) -> ApiResult<()> {
        Ok(())
    }

    async fn create_comment(
        &self,
        _issue_id: &IssueId,
        _body: &str,
        _parent_id: Option<&CommentId>,
    ) -> ApiResult<()> {
        Ok(())
    }

    async fn update_comment(&self, _comment_id: &CommentId, _body: &str) -> ApiResult<()> {
        Ok(())
    }

    async fn delete_comment(&self, _comment_id: &CommentId) -> ApiResult<()> {
        Ok(())
    }

    async fn create_reaction(&self, _target: &ReactionTarget, _emoji: &str) -> ApiResult<()> {
        Ok(())
    }

    async fn delete_reaction(&self, _reaction_id: &ReactionId) -> ApiResult<()> {
        Ok(())
    }
}

fn state(name: &str, state_type: StateType) -> WorkflowState {
    WorkflowState {
        name: name.into(),
        state_type,
    }
}

fn reaction(id: &str, emoji: &str, mine: bool) -> Reaction {
    Reaction {
        id: ReactionId::from_raw(id),
        emoji: emoji.into(),
        mine,
    }
}

fn person(display_name: &str, is_me: bool) -> User {
    User {
        id: UserId::from_raw(format!("u_{display_name}")),
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
        id: IssueId::from_raw(id),
        identifier: identifier.into(),
        title: Some(title.into()),
        state: st,
        priority,
        assignee: Some(person(assignee, assignee == "dan")),
        labels: labels
            .iter()
            .map(|(name, colour)| Label {
                id: LabelId::from_raw(format!("lbl_{name}")),
                name: (*name).into(),
                colour: Rgb::parse_hex(colour),
            })
            .collect(),
        url: format!("https://linear.app/dans-donuts/issue/{identifier}"),
        branch_name: format!("dan/{}", identifier.to_lowercase()),
        team_id: team_for(identifier),
        updated_at: "2026-07-15T09:00:00Z".into(),
    }
}

fn team_for(identifier: &str) -> TeamId {
    if identifier.starts_with("DAN2") {
        TeamId::from_raw("t_pizza")
    } else {
        TeamId::from_raw("t_donut")
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
        id: IssueId::from_raw("i1"),
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
            id: LabelId::from_raw("lbl_oven"),
            name: "oven".into(),
            colour: Rgb::parse_hex("#eb5757"),
        }],
        comments: vec![
            Comment {
                id: CommentId::from_raw("c1"),
                parent_id: None,
                author: Some("dan".into()),
                is_mine: true,
                body: "Swapped the thermocouple this morning. Readings so far:\n\n1. 6pm - `445°C`\n2. 7pm - `462°C`".into(),
                created_at: "2026-07-16T09:24:00Z".into(),
                reactions: vec![
                    reaction("r1", "+1", true),
                    reaction("r2", "+1", false),
                    reaction("r3", "heart", false),
                ],
            },
            Comment {
                id: CommentId::from_raw("c1a"),
                parent_id: Some(CommentId::from_raw("c1")),
                author: Some("danniiee".into()),
                is_mine: false,
                body: "Agreed, the sensor looks fine. Next suspect is the `flue damper`.".into(),
                created_at: "2026-07-16T10:02:00Z".into(),
                reactions: vec![reaction("r4", "tada", false)],
            },
            Comment {
                id: CommentId::from_raw("c1b"),
                parent_id: Some(CommentId::from_raw("c1")),
                author: Some("dan".into()),
                is_mine: true,
                body: "Adding the damper check to the list.".into(),
                created_at: "2026-07-16T10:05:00Z".into(),
                reactions: Vec::new(),
            },
            Comment {
                id: CommentId::from_raw("c2"),
                parent_id: None,
                author: Some("dan".into()),
                is_mine: true,
                body: "Still climbing. Confirmed the flue damper is sticking open.".into(),
                created_at: "2026-07-16T18:40:00Z".into(),
                reactions: Vec::new(),
            },
        ],
        reactions: vec![reaction("ri1", "eyes", true), reaction("ri2", "rocket", false)],
        branch_name: "dan/dan2-7".into(),
        team_id: TeamId::from_raw("t_pizza"),
        updated_at: "2026-07-16T18:40:00Z".into(),
    }];

    let notifications = vec![
        NotificationItem {
            title: "New comment on DAN2-7 (wood-fired oven)".into(),
            issue_id: Some(IssueId::from_raw("i1")),
            is_read: false,
            grouping_key: "g1".into(),
        },
        NotificationItem {
            title: "You were assigned DAN-10 (sprinkle dispenser jams)".into(),
            issue_id: Some(IssueId::from_raw("i2")),
            is_read: false,
            grouping_key: "g2".into(),
        },
        NotificationItem {
            title: "DAN2-5 moved to Backlog (pineapple debate)".into(),
            issue_id: Some(IssueId::from_raw("i6")),
            is_read: true,
            grouping_key: "g3".into(),
        },
    ];

    let saved_views = vec![
        SavedView {
            id: ViewId::from_raw("v_urgent"),
            name: "Urgent & unassigned".into(),
        },
        SavedView {
            id: ViewId::from_raw("v_oven"),
            name: "Oven incidents".into(),
        },
        SavedView {
            id: ViewId::from_raw("v_menu"),
            name: "Menu ideas".into(),
        },
    ];

    let pick = |ids: &[&str]| -> Vec<IssueSummary> {
        ids.iter()
            .filter_map(|id| {
                issues
                    .iter()
                    .find(|issue| issue.id.as_str() == *id)
                    .cloned()
            })
            .collect()
    };
    let saved_view_issues = std::collections::HashMap::from([
        (
            ViewId::from_raw("v_urgent"),
            pick(&["i1", "i2", "i3", "i4", "i5", "i6"]),
        ),
        (ViewId::from_raw("v_oven"), pick(&["i1", "i3"])),
        (ViewId::from_raw("v_menu"), pick(&["i4", "i5", "i7"])),
    ]);

    Fixture {
        viewer: person("dan", true),
        org_name: "Dan's Donuts".into(),
        org_url_key: "dans-donuts".into(),
        notifications,
        saved_views,
        issues,
        saved_view_issues,
        details,
    }
}
