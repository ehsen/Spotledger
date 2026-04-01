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

impl From<DbError> for spotledger_types::error::SpotError {
    fn from(e: DbError) -> Self {
        match e {
            DbError::NotFound { doctype, name } => {
                spotledger_types::error::SpotError::NotFound { doctype, name }
            }
            other => spotledger_types::error::SpotError::Db(other.to_string()),
        }
    }
}
