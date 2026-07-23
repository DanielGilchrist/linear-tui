use serde::{Deserialize, Serialize};

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

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(from = "String", into = "String")]
pub struct Timestamp(i64);

impl Timestamp {
    pub fn epoch(self) -> i64 {
        self.0
    }

    pub fn humanise(self, now: i64) -> String {
        match self.age(now) {
            Age::JustNow => "just now".into(),
            Age::Relative(text) => format!("{text} ago"),
            Age::Date(text) => text,
        }
    }

    pub fn age_short(self, now: i64) -> String {
        match self.age(now) {
            Age::JustNow => "just now".into(),
            Age::Relative(text) | Age::Date(text) => text,
        }
    }

    fn age(self, now: i64) -> Age {
        let seconds = now - self.0;

        if seconds < 60 {
            return Age::JustNow;
        }

        let minutes = seconds / 60;

        if minutes < 60 {
            return Age::Relative(format!("{minutes}m"));
        }

        let hours = minutes / 60;

        if hours < 24 {
            return Age::Relative(format!("{hours}h"));
        }

        let days = hours / 24;

        if days < 7 {
            return Age::Relative(format!("{days}d"));
        }

        if days < 30 {
            return Age::Relative(format!("{}w", days / 7));
        }

        let date = chrono::DateTime::from_timestamp(self.0, 0)
            .map(|dt| dt.format("%b %-d, %Y").to_string())
            .unwrap_or_default();
        Age::Date(date)
    }
}

enum Age {
    JustNow,
    Relative(String),
    Date(String),
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
