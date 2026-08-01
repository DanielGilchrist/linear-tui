use thiserror::Error;

pub type ApiResult<T> = Result<T, ApiError>;

const ERROR_BODY_SNIPPET: usize = 200;

#[derive(Debug, Error)]
pub enum ApiError {
    #[error("network request to Linear failed: {0}")]
    Http(#[from] reqwest::Error),
    #[error("Linear returned HTTP {status}{}", detail(.body))]
    Status { status: u16, body: String },
    #[error("{0}")]
    Auth(String),
    #[error("Linear returned errors: {}", .0.join("; "))]
    GraphQl(Vec<String>),
    #[error("Linear returned an empty response")]
    Empty,
    #[error("{resource} {id} was not found")]
    NotFound { resource: &'static str, id: String },
}

impl ApiError {
    pub fn is_auth(&self) -> bool {
        match self {
            ApiError::Auth(_) => true,
            ApiError::Status { status, .. } => *status == 401 || *status == 403,
            ApiError::GraphQl(messages) => messages
                .iter()
                .any(|message| message.to_lowercase().contains("authenticat")),
            ApiError::Http(_) | ApiError::Empty | ApiError::NotFound { .. } => false,
        }
    }
}

fn detail(body: &str) -> String {
    let trimmed = body.trim();

    if trimmed.is_empty() {
        return String::new();
    }

    let snippet: String = trimmed.chars().take(ERROR_BODY_SNIPPET).collect();

    format!(": {snippet}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auth_errors_are_recognised() {
        assert!(ApiError::Auth("expired".into()).is_auth());
        assert!(ApiError::Status {
            status: 401,
            body: String::new()
        }
        .is_auth());
        assert!(ApiError::Status {
            status: 403,
            body: String::new()
        }
        .is_auth());
        assert!(ApiError::GraphQl(vec!["Authentication required".into()]).is_auth());
    }

    #[test]
    fn non_auth_errors_are_not_flagged() {
        assert!(!ApiError::Status {
            status: 500,
            body: String::new()
        }
        .is_auth());
        assert!(!ApiError::Empty.is_auth());
        assert!(!ApiError::GraphQl(vec!["rate limited".into()]).is_auth());
    }
}
