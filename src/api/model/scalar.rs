use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(from = "String", into = "String")]
pub struct Timestamp(i64);

impl Timestamp {
    pub fn now() -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};

        let epoch = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|elapsed| elapsed.as_secs() as i64)
            .unwrap_or(0);

        Self(epoch)
    }

    pub fn from_epoch(epoch: i64) -> Self {
        Self(epoch)
    }

    pub fn seconds_since(self, earlier: Timestamp) -> i64 {
        self.0 - earlier.0
    }

    pub fn next_change(self, now: Timestamp) -> Option<Timestamp> {
        const MINUTE: i64 = 60;
        const HOUR: i64 = 60 * MINUTE;
        const DAY: i64 = 24 * HOUR;
        const WEEK: i64 = 7 * DAY;
        const MONTH: i64 = 30 * DAY;

        let seconds = now.seconds_since(self);

        let boundary = if seconds < MINUTE {
            MINUTE
        } else if seconds < HOUR {
            (seconds / MINUTE + 1) * MINUTE
        } else if seconds < DAY {
            (seconds / HOUR + 1) * HOUR
        } else if seconds < WEEK {
            (seconds / DAY + 1) * DAY
        } else if seconds < MONTH {
            let next_week_day = (seconds / DAY / 7 + 1) * 7;
            next_week_day.min(30) * DAY
        } else {
            return None;
        };

        Some(Timestamp(self.0 + boundary))
    }

    pub fn humanise(self, now: Timestamp) -> String {
        match self.age(now) {
            Age::JustNow => "just now".into(),
            Age::Relative(text) => format!("{text} ago"),
            Age::Date(text) => text,
        }
    }

    pub fn age_short(self, now: Timestamp) -> String {
        match self.age(now) {
            Age::JustNow => "just now".into(),
            Age::Relative(text) | Age::Date(text) => text,
        }
    }

    fn age(self, now: Timestamp) -> Age {
        let seconds = now.seconds_since(self);

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

#[cfg(test)]
mod tests {
    use super::*;

    const DAY: i64 = 24 * 60 * 60;

    #[test]
    fn humanise_changes_exactly_at_next_change() {
        let stamp = Timestamp::from_epoch(0);
        let ages = [
            0,
            30,
            59,
            90,
            3_500,
            3_600,
            7_000,
            DAY - 1,
            DAY + 5,
            8 * DAY,
            20 * DAY,
        ];

        for age in ages {
            let now = Timestamp::from_epoch(age);
            let due = stamp
                .next_change(now)
                .unwrap_or_else(|| panic!("age {age} should still change"));
            let boundary = due.seconds_since(stamp);

            let just_before = Timestamp::from_epoch(boundary - 1);
            let at_due = Timestamp::from_epoch(boundary);

            assert_eq!(
                stamp.humanise(just_before),
                stamp.humanise(now),
                "age {age}: display should hold until the boundary"
            );
            assert_ne!(
                stamp.humanise(at_due),
                stamp.humanise(now),
                "age {age}: display should flip at the boundary"
            );
        }
    }

    #[test]
    fn a_fixed_date_no_longer_needs_refreshing() {
        let stamp = Timestamp::from_epoch(0);
        let now = Timestamp::from_epoch(40 * DAY);
        assert!(stamp.next_change(now).is_none());
    }
}
