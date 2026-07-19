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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StateType {
    Triage,
    Backlog,
    Unstarted,
    Started,
    Completed,
    #[serde(rename = "canceled")]
    Cancelled,
}

impl StateType {
    pub fn from_api(raw: &str) -> Self {
        match raw {
            "triage" => StateType::Triage,
            "unstarted" => StateType::Unstarted,
            "started" => StateType::Started,
            "completed" => StateType::Completed,
            "canceled" | "cancelled" => StateType::Cancelled,
            _ => StateType::Backlog,
        }
    }

    pub fn as_api(self) -> &'static str {
        match self {
            StateType::Triage => "triage",
            StateType::Backlog => "backlog",
            StateType::Unstarted => "unstarted",
            StateType::Started => "started",
            StateType::Completed => "completed",
            StateType::Cancelled => "canceled",
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "u8", into = "u8")]
pub enum Priority {
    #[default]
    None,
    Urgent,
    High,
    Medium,
    Low,
}

impl Priority {
    pub fn label(self) -> &'static str {
        match self {
            Priority::None => "No priority",
            Priority::Urgent => "Urgent",
            Priority::High => "High",
            Priority::Medium => "Medium",
            Priority::Low => "Low",
        }
    }
}

impl From<u8> for Priority {
    fn from(value: u8) -> Self {
        match value {
            1 => Priority::Urgent,
            2 => Priority::High,
            3 => Priority::Medium,
            4 => Priority::Low,
            _ => Priority::None,
        }
    }
}

impl From<Priority> for u8 {
    fn from(priority: Priority) -> Self {
        match priority {
            Priority::None => 0,
            Priority::Urgent => 1,
            Priority::High => 2,
            Priority::Medium => 3,
            Priority::Low => 4,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "String", into = "String")]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb {
    const FALLBACK: Rgb = Rgb {
        r: 128,
        g: 128,
        b: 128,
    };

    pub fn parse_hex(hex: &str) -> Self {
        let hex = hex.trim_start_matches('#');
        if hex.len() != 6 {
            return Rgb::FALLBACK;
        }
        let channel = |range: std::ops::Range<usize>| u8::from_str_radix(&hex[range], 16);
        match (channel(0..2), channel(2..4), channel(4..6)) {
            (Ok(r), Ok(g), Ok(b)) => Rgb { r, g, b },
            _ => Rgb::FALLBACK,
        }
    }
}

impl From<String> for Rgb {
    fn from(hex: String) -> Self {
        Rgb::parse_hex(&hex)
    }
}

impl From<Rgb> for String {
    fn from(colour: Rgb) -> Self {
        format!("#{:02x}{:02x}{:02x}", colour.r, colour.g, colour.b)
    }
}

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
        }
    }
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(from = "String", into = "String")]
pub struct Timestamp(i64);

impl Timestamp {
    pub fn epoch(self) -> i64 {
        self.0
    }

    pub fn humanise(self, now: i64) -> String {
        let seconds = now - self.0;

        if seconds < 60 {
            return "just now".into();
        }

        let minutes = seconds / 60;

        if minutes < 60 {
            return format!("{minutes}m ago");
        }

        let hours = minutes / 60;

        if hours < 24 {
            return format!("{hours}h ago");
        }

        let days = hours / 24;

        if days < 7 {
            return format!("{days}d ago");
        }

        if days < 30 {
            return format!("{}w ago", days / 7);
        }

        chrono::DateTime::from_timestamp(self.0, 0)
            .map(|dt| dt.format("%b %-d, %Y").to_string())
            .unwrap_or_default()
    }
}

impl From<&str> for Timestamp {
    fn from(raw: &str) -> Self {
        let epoch = chrono::DateTime::parse_from_rfc3339(raw)
            .map(|dt| dt.timestamp())
            .unwrap_or(0);

        Self(epoch)
    }
}

impl From<String> for Timestamp {
    fn from(raw: String) -> Self {
        raw.as_str().into()
    }
}

impl From<Timestamp> for String {
    fn from(timestamp: Timestamp) -> Self {
        chrono::DateTime::from_timestamp(timestamp.0, 0)
            .map(|dt| dt.to_rfc3339())
            .unwrap_or_default()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Comment {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub parent_id: Option<String>,
    #[serde(default)]
    pub author: Option<String>,
    pub body: String,
    #[serde(default)]
    pub created_at: Timestamp,
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
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StateOption {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub state_type: StateType,
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
