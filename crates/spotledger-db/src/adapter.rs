//! `DbAdapter` — the single chokepoint for all SurrealDB interaction.
//!
//! **Rule**: no other file in this crate (or any crate) calls `.query()` on the
//! SurrealDB client directly.  Every SQL statement goes through `run`,
//! `run_one`, or `execute` here.
//!
//! This keeps DB concerns quarantined: connection pooling, retry logic, table-
//! not-found handling, and future driver swaps are all one-place changes.

use serde_json::Value;
use surrealdb::engine::remote::ws::{Client, Ws};
use surrealdb::opt::auth::Root;
use surrealdb::Surreal;

use spotledger_core::config::DatabaseConfig;

use crate::error::DbError;

// ── DbAdapter ─────────────────────────────────────────────────────────────────

/// A connected, authenticated SurrealDB handle.
/// Clone is cheap — the inner `Surreal<Client>` is reference-counted.
///
/// All SQL statements in this crate flow through `run`, `run_one`, or `execute`.
/// No other code should call `.query()` on a raw SurrealDB client.
#[derive(Clone)]
pub struct DbAdapter {
    db: Surreal<Client>,
}

impl DbAdapter {
    // ── construction ──────────────────────────────────────────────────────────

    /// Connect to SurrealDB via WebSocket and return a ready adapter.
    pub async fn connect(config: &DatabaseConfig) -> Result<Self, DbError> {
        let url = config.url.clone();
        let endpoint = url
            .strip_prefix("ws://")
            .or_else(|| url.strip_prefix("wss://"))
            .unwrap_or(&url)
            .to_owned();

        tracing::debug!(endpoint = %endpoint, "Connecting to SurrealDB");

        let db: Surreal<Client> = Surreal::new::<Ws>(endpoint.as_str()).await?;

        db.signin(Root {
            username: config.user.clone(),
            password: config.pass.clone(),
        })
        .await?;

        let ns = if config.ns.is_empty() { &config.db } else { &config.ns };
        let database = if config.db.is_empty() { &config.ns } else { &config.db };

        db.use_ns(ns).use_db(database).await?;

        tracing::info!(ns = %ns, db = %database, "SurrealDB ready");

        Ok(Self { db })
    }

    // ── query helpers ─────────────────────────────────────────────────────────

    /// Execute SQL, return all rows.
    ///
    /// Returns `Ok(vec![])` when the queried table does not exist yet,
    /// so callers never see a spurious error for freshly bootstrapped sites.
    pub async fn run(
        &self,
        sql: &str,
        bindings: Vec<(String, Value)>,
    ) -> Result<Vec<Value>, DbError> {
        let mut q = self.db.query(sql);
        for (k, v) in bindings {
            q = q.bind((k, v));
        }
        match q.await {
            Err(e) if is_table_not_found(&e) => Ok(vec![]),
            Err(e) => Err(DbError::Surreal(e)),
            Ok(mut resp) => match resp.take::<Vec<Value>>(0) {
                Err(e) if is_table_not_found(&e) => Ok(vec![]),
                Err(e) => Err(DbError::Surreal(e)),
                Ok(rows) => Ok(rows),
            },
        }
    }

    /// Execute SQL, expect exactly one row.
    ///
    /// Returns `DbError::NotFound` when the result set is empty.
    pub async fn run_one(
        &self,
        sql: &str,
        bindings: Vec<(String, Value)>,
        doctype: &str,
        name: &str,
    ) -> Result<Value, DbError> {
        self.run(sql, bindings)
            .await?
            .into_iter()
            .next()
            .ok_or_else(|| DbError::NotFound {
                doctype: doctype.into(),
                name: name.into(),
            })
    }

    /// Execute SQL with no return value (DELETE, plain UPDATE, DDL).
    pub async fn execute(
        &self,
        sql: &str,
        bindings: Vec<(String, Value)>,
    ) -> Result<(), DbError> {
        let mut q = self.db.query(sql);
        for (k, v) in bindings {
            q = q.bind((k, v));
        }
        q.await.map(|_| ()).map_err(DbError::Surreal)
    }
}

// ── helpers ───────────────────────────────────────────────────────────────────

/// Returns `true` when the SurrealDB error message indicates the table has not
/// been defined yet.  Callers use this to return empty results rather than errors.
pub(crate) fn is_table_not_found(e: &surrealdb::Error) -> bool {
    e.to_string().contains("does not exist")
}

// ── tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    /// Integration tests require a live SurrealDB instance on ws://127.0.0.1:8500.
    /// Gate them behind `--features integration` to keep `cargo test` fast.
    #[cfg(feature = "integration")]
    #[tokio::test]
    async fn connect_and_round_trip() {
        use super::*;
        use spotledger_core::config::DatabaseConfig;

        let cfg = DatabaseConfig {
            url: "ws://127.0.0.1:8500".into(),
            ns: "test_spotledger".into(),
            db: "test_spotledger".into(),
            user: "root".into(),
            pass: "root".into(),
        };

        let adapter = DbAdapter::connect(&cfg).await.expect("should connect");
        let rows = adapter
            .run("RETURN 42", vec![])
            .await
            .expect("should run");
        assert_eq!(rows.first(), Some(&serde_json::json!(42)));
    }
}
