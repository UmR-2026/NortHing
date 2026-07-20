//! Kernel error types.

#[derive(Debug, thiserror::Error)]
pub enum KernelError {
    #[error("internal error: {0}")]
    Internal(String),

    #[error("validation error: {0}")]
    Validation(String),

    #[error("not found: {0}")]
    NotFound(String),

    #[error("config error: {0}")]
    Config(String),

    #[error("runtime error: {0}")]
    Runtime(String),

    #[error("unauthorized: {0}")]
    Unauthorized(String),

    #[error("operation timed out")]
    Timeout,

    #[error("cancelled")]
    Cancelled,
}

pub type KernelResult<T> = Result<T, KernelError>;
