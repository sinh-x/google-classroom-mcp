use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("not authenticated — run `auth` first to set up credentials")]
    NotAuthenticated,

    #[error("failed to read credentials: {0}")]
    CredentialRead(String),

    #[error("Google API error: {0}")]
    GoogleApi(String),

    #[error("Drive API error: {0}")]
    DriveApi(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("OAuth2 error: {0}")]
    OAuth2(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}
