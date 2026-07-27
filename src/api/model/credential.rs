use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Credential {
    PersonalKey(String),
    OAuth(String),
    EnvVar(String),
}

impl Credential {
    pub fn header_value(&self) -> String {
        match self {
            Credential::PersonalKey(key) => key.clone(),
            Credential::OAuth(token) => format!("Bearer {token}"),
            Credential::EnvVar(name) => std::env::var(name).unwrap_or_default(),
        }
    }

    pub fn secret(&self) -> String {
        match self {
            Credential::PersonalKey(secret) | Credential::OAuth(secret) => secret.clone(),
            Credential::EnvVar(name) => std::env::var(name).unwrap_or_default(),
        }
    }

    pub fn describe(&self) -> String {
        match self {
            Credential::PersonalKey(_) => "API key".into(),
            Credential::OAuth(_) => "browser sign-in".into(),
            Credential::EnvVar(name) => format!("${name}"),
        }
    }
}
