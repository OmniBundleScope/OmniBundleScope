use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("malformed input: {0}")]
    Malformed(String),

    #[error("unsupported schema: {0}")]
    Unsupported(String),

    #[error("fusion invariant violated: {0}")]
    Invariant(String),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
}
