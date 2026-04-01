//! SurrealDB connection via WebSocket.
//!
//! Spotledger always connects to an external running SurrealDB process.
//! In dev: `surreal start --bind 127.0.0.1:8500`
//! In prod: point `config.url` at the production instance.
//!
//! The `Db` handle is cheaply cloneable (Arc internally).

use spotledger_types::config::DatabaseConfig;
use surrealdb::engine::remote::ws::{Client, Ws};
use surrealdb::opt::auth::Root;
use surrealdb::Surreal;

use crate::error::DbError;

/// A connected, ready-to-use SurrealDB client scoped to a namespace+database.
/// Clone is cheap — the inner connection is reference-counted.
pub type Db = Surreal<Client>;

/// Connect to SurrealDB at `config.url` via WebSocket.
pub async fn connect(config: &DatabaseConfig) -> Result<Db, DbError> {
    let url = config.url.clone();

    tracing::debug!(url = %url, "Connecting to SurrealDB");

    let db: Surreal<Client> = Surreal::new::<Ws>(url.as_str()).await?;

    db.signin(Root {
        username: config.user.clone(),
        password: config.pass.clone(),
    })
    .await?;

    let ns = if config.ns.is_empty() { &config.db } else { &config.ns };
    let database = if config.db.is_empty() { &config.ns } else { &config.db };

    db.use_ns(ns).use_db(database).await?;

    tracing::info!(ns = %ns, db = %database, "SurrealDB ready");

    Ok(db)
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
        use spotledger_types::config::DatabaseConfig;

        let cfg = DatabaseConfig {
            url: "ws://127.0.0.1:8500".into(),
            ns: "test_spotledger".into(),
            db: "test_spotledger".into(),
            user: "root".into(),
            pass: "root".into(),
        };

        let db = connect(&cfg).await.expect("should connect");
        // Simple smoke test — version query
        let _ver: Option<String> = db
            .query("RETURN meta::id(meta::id('test:hello'))")
            .await
            .unwrap()
            .take(0)
            .unwrap();
    }
}
