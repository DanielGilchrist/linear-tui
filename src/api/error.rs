use thiserror::Error;

pub type ApiResult<T> = Result<T, ApiError>;

#[derive(Debug, Error)]
pub enum ApiError {
    #[error("network request to Linear failed: {0}")]
    Http(#[from] reqwest::Error),
    #[error("Linear returned errors: {}", .0.join("; "))]
    GraphQl(Vec<String>),
    #[error("Linear returned an empty response")]
    Empty,
    #[error("{resource} {id} was not found")]
    NotFound { resource: &'static str, id: String },
}
