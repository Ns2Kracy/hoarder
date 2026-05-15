use thiserror::Error;

pub type AppResult<T> = Result<T, AppError>;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("configuration error: {0}")]
    Config(String),

    #[error("connector error: {0}")]
    Connector(String),

    #[error("conflict: {0}")]
    Conflict(String),

    #[error("database error: {0}")]
    Database(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("not found: {0}")]
    NotFound(String),

    #[error("path error: {0}")]
    Path(String),

    #[error("validation error: {0}")]
    Validation(String),

    #[error("unprocessable entity: {0}")]
    Unprocessable(String),
}
