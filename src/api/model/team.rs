use serde::{Deserialize, Serialize};

use super::id::TeamId;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Team {
    pub id: TeamId,
    pub name: String,
    pub key: String,
    #[serde(default)]
    pub triage_enabled: bool,
}
