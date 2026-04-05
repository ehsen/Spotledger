use thiserror::Error;

#[derive(Debug, Error)]
pub enum DbError {
    #[error("SurrealDB error: {0}")]
    Surreal(#[from] surrealdb::Error),

    #[error("Record not found: {doctype}/{name}")]
    NotFound { doctype: String, name: String },

    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("{0}")]
    Other(String),
}

impl From<DbError> for spotledger_core::error::CoreError {
    fn from(e: DbError) -> Self {
        match e {
            DbError::NotFound { doctype, name } => {
                spotledger_core::error::CoreError::NotFound { doctype, name }
            }
            other => spotledger_core::error::CoreError::Db(other.to_string()),
        }
    }
}
