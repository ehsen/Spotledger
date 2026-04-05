use thiserror::Error;

/// Central error type for the SpotledgerCore framework.
/// HTTP status codes match Frappe behaviour so existing desk responses are unchanged.
#[derive(Debug, Error)]
pub enum CoreError {
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

    #[error("Unknown DocType: {0}")]
    UnknownDocType(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("{0}")]
    Other(String),
}

impl CoreError {
    /// HTTP status code that maps to this error (matches Frappe behaviour).
    pub fn http_status(&self) -> u16 {
        match self {
            CoreError::NotFound { .. }      => 404,
            CoreError::PermissionDenied(_)  => 403,
            CoreError::Validation(_)        => 417, // Frappe uses 417 for ValidationError
            CoreError::UnknownDocType(_)    => 404,
            _                               => 500,
        }
    }
}

/// Legacy alias — existing code that imports `SpotError` still compiles.
/// Prefer `CoreError` in new code.
pub type SpotError = CoreError;
