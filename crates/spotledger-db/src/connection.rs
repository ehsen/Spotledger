//! SurrealDB connection — construct a [`DbAdapter`] from a [`DatabaseConfig`].
//!
//! Every other part of the codebase receives a cloned `DbAdapter`; only this
//! module deals with the raw SurrealDB SDK types.

use spotledger_core::config::DatabaseConfig;

use crate::adapter::DbAdapter;
use crate::error::DbError;

/// Connect to SurrealDB at `config.url` via WebSocket and return a ready [`DbAdapter`].
pub async fn connect(config: &DatabaseConfig) -> Result<DbAdapter, DbError> {
    DbAdapter::connect(config).await
}

#[cfg(test)]
mod tests {
    /// Connection tests require a live SurrealDB instance on ws://127.0.0.1:8500.
    /// They are gated behind the `integration` feature flag to keep unit tests fast.
    ///
    /// Run with: `cargo test --features integration -- connection`
    #[cfg(feature = "integration")]
    #[tokio::test]
    async fn connect_to_local_surreal() {
        use super::*;
        use spotledger_core::config::DatabaseConfig;

        let cfg = DatabaseConfig {
            url: "ws://127.0.0.1:8500".into(),
            ns: "test_spotledger".into(),
            db: "test_spotledger".into(),
            user: "root".into(),
            pass: "root".into(),
        };

        let db = connect(&cfg).await.expect("should connect");
        // Simple smoke test — verify connection is live
        let rows = db.run("RETURN 'hello';", vec![]).await.expect("should query");
        assert!(!rows.is_empty(), "should return at least one row");
    }
}
