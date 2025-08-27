use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("Discord error: {0}")]
    Discord(#[from] serenity::Error),
    #[error("Environment variable error: {0}")]
    EnvVar(#[from] std::env::VarError),
    #[error("Network error: {0}")]
    Network(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, AppError>;
