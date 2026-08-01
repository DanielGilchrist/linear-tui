use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct OAuthToken {
    pub access_token: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<i64>,
}

impl OAuthToken {
    pub fn new(
        access_token: String,
        refresh_token: Option<String>,
        expires_at: Option<i64>,
    ) -> Self {
        Self {
            access_token,
            refresh_token,
            expires_at,
        }
    }

    pub fn is_expiring(&self, now: i64, skew: i64) -> bool {
        self.expires_at
            .is_some_and(|expires_at| now + skew >= expires_at)
    }
}

impl<'de> Deserialize<'de> for OAuthToken {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Wire {
            Legacy(String),
            Full {
                access_token: String,
                #[serde(default)]
                refresh_token: Option<String>,
                #[serde(default)]
                expires_at: Option<i64>,
            },
        }

        Ok(match Wire::deserialize(deserializer)? {
            Wire::Legacy(access_token) => OAuthToken {
                access_token,
                refresh_token: None,
                expires_at: None,
            },
            Wire::Full {
                access_token,
                refresh_token,
                expires_at,
            } => OAuthToken {
                access_token,
                refresh_token,
                expires_at,
            },
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Credential {
    PersonalKey(String),
    OAuth(OAuthToken),
    EnvVar(String),
}

impl Credential {
    pub fn header_value(&self) -> String {
        match self {
            Credential::PersonalKey(key) => key.clone(),
            Credential::OAuth(token) => format!("Bearer {}", token.access_token),
            Credential::EnvVar(name) => std::env::var(name).unwrap_or_default(),
        }
    }

    pub fn secret(&self) -> String {
        match self {
            Credential::PersonalKey(secret) => secret.clone(),
            Credential::OAuth(token) => token.access_token.clone(),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_bare_string_oauth_still_deserialises() {
        let credential: Credential = serde_json::from_str(r#"{"OAuth":"legacy-token"}"#).unwrap();
        match credential {
            Credential::OAuth(token) => {
                assert_eq!(token.access_token, "legacy-token");
                assert_eq!(token.refresh_token, None);
                assert_eq!(token.expires_at, None);
            }
            other => panic!("expected OAuth, got {other:?}"),
        }
    }

    #[test]
    fn full_oauth_token_round_trips() {
        let token = OAuthToken::new("access".into(), Some("refresh".into()), Some(1_000));
        let json = serde_json::to_string(&Credential::OAuth(token.clone())).unwrap();
        let back: Credential = serde_json::from_str(&json).unwrap();
        assert_eq!(back, Credential::OAuth(token));
    }

    #[test]
    fn is_expiring_respects_skew_and_unknown_expiry() {
        let token = OAuthToken::new("a".into(), Some("r".into()), Some(1_000));
        assert!(token.is_expiring(1_000, 0));
        assert!(token.is_expiring(950, 60));
        assert!(!token.is_expiring(800, 60));

        let forever = OAuthToken::new("a".into(), None, None);
        assert!(!forever.is_expiring(i64::MAX, 60));
    }
}
