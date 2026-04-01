use thiserror::Error;

#[derive(Debug, Error)]
pub enum SpotError {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Database error: {0}")]
    Db(String),

    #[error("Document not found: {doctype}/{name}")]
    NotFound { doctype: String, name: String },

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("{0}")]
    Other(String),
}

impl SpotError {
    /// HTTP status code that maps to this error (matches Frappe behaviour).
    pub fn http_status(&self) -> u16 {
        match self {
            SpotError::NotFound { .. } => 404,
            SpotError::PermissionDenied(_) => 403,
            SpotError::Validation(_) => 417, // Frappe uses 417 for ValidationError
            _ => 500,
        }
    }
}
