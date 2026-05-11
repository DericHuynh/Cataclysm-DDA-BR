use thiserror::Error;

/// Core error type for `cdda_core`.
#[derive(Error, Debug)]
pub enum CoreError {
    #[error("ZLevel overflow: {0}")]
    ZLevelOverflow(i8),

    #[error("Invalid value: {0}")]
    InvalidValue(String),
}

impl CoreError {
    pub fn invalid_value(msg: impl Into<String>) -> Self {
        CoreError::InvalidValue(msg.into())
    }
}
