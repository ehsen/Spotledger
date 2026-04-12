//! Per-site versioned migration runner.
//!
//! ## Architecture
//!
//! Migration tracking is **per-site** by design: each Spotledger site is an
//! independent SurrealDB namespace, so `tabMigration` in namespace `acme` is
//! completely separate from `tabMigration` in namespace `beta`.  Running a
//! migration against site A never touches site B.
//!
//! ## Two-tier migration model
//!
//! | Tier | What | When |
//! |---|---|---|
//! | DDL (schema)| `ensure_all_schemas` adds new tables/fields | Always, on startup |
//! | Data        | `MigrationEntry` + `run_pending_migrations` | Once per site, tracked |
//!
//! DDL migrations are always idempotent (`DEFINE … IF NOT EXISTS`) and need no
//! tracking.  Data migrations are one-shot: each carries a sortable name like
//! `"0001_seed_naming_rules"` and is recorded in `tabMigration` so it never
//! runs twice on the same site.
//!
//! ## Registering a migration
//!
//! ```rust
//! use spotledger_db::migrations::{MigrationEntry, MigrationFuture};
//! use spotledger_db::adapter::DbAdapter;
//! use spotledger_db::error::DbError;
//!
//! fn run_0001(db: DbAdapter) -> MigrationFuture {
//!     Box::pin(async move {
//!         db.execute("UPDATE tabFoo SET bar = 'baz'", vec![]).await
//!     })
//! }
//!
//! inventory::submit!(MigrationEntry {
//!     name: "0001_seed_foo_bar",
//!     run: run_0001,
//! });
//! ```
//!
//! Migration names **must** be lexicographically sortable (i.e. zero-padded
//! numeric prefix like `0001_`, `0002_`, …).  They are run in ascending name
//! order.

use std::collections::BTreeSet;
use std::pin::Pin;
use std::future::Future;

use serde_json::Value;

use crate::adapter::DbAdapter;
use crate::error::DbError;

// ── MigrationEntry ────────────────────────────────────────────────────────────

/// The return type of every migration function.
///
/// Using a concrete type alias avoids spelling out the full `Pin<Box<dyn …>>`
/// everywhere.
pub type MigrationFuture = Pin<Box<dyn Future<Output = Result<(), DbError>> + Send>>;

/// A single versioned data migration.
///
/// Register one at module top-level with:
/// ```rust,ignore
/// inventory::submit!(MigrationEntry { name: "0001_...", run: my_fn });
/// ```
pub struct MigrationEntry {
    /// Unique, lexicographically-sortable name, e.g. `"0001_seed_naming_rules"`.
    pub name: &'static str,
    /// The migration function.  Receives an **owned** `DbAdapter` clone
    /// (cheap — the inner SurrealDB handle is reference-counted).
    pub run: fn(DbAdapter) -> MigrationFuture,
}

inventory::collect!(MigrationEntry);

// ── run_pending_migrations ────────────────────────────────────────────────────

/// Return the names of all registered migrations that have **not** yet been
/// applied to this site.  Useful for `--dry-run` display.
pub async fn pending_migration_names(adapter: &DbAdapter) -> Result<Vec<&'static str>, DbError> {
    let applied: BTreeSet<String> = {
        let rows = adapter
            .run("SELECT name FROM tabMigration", vec![])
            .await?;
        rows.into_iter()
            .filter_map(|r| r.get("name").and_then(Value::as_str).map(str::to_owned))
            .collect()
    };

    let mut pending: Vec<&MigrationEntry> = inventory::iter::<MigrationEntry>()
        .filter(|e| !applied.contains(e.name))
        .collect();
    pending.sort_by_key(|e| e.name);
    Ok(pending.iter().map(|e| e.name).collect())
}

/// Apply every registered [`MigrationEntry`] that has not yet been recorded in
/// `tabMigration` for this site's database namespace.
///
/// Migrations are sorted by [`MigrationEntry::name`] and executed in order.
/// If a migration fails the error is returned immediately; subsequent
/// migrations are **not** run (fail-fast).
///
/// This function is idempotent within a site: calling it multiple times only
/// re-runs migrations that haven't been recorded yet.
///
/// # Arguments
/// * `adapter` — connected adapter for the **specific site** to migrate.
/// * `batch`   — numeric batch label written into each applied row (use the
///               current Unix timestamp or a monotonic counter).
pub async fn run_pending_migrations(
    adapter: &DbAdapter,
    batch: u64,
) -> Result<usize, DbError> {
    // ── 1. Collect already-applied names ──────────────────────────────────────
    let applied: BTreeSet<String> = {
        let rows = adapter
            .run("SELECT name FROM tabMigration", vec![])
            .await?;
        rows.into_iter()
            .filter_map(|r| r.get("name").and_then(Value::as_str).map(str::to_owned))
            .collect()
    };

    // ── 2. Collect + sort all registered migrations ───────────────────────────
    let mut pending: Vec<&MigrationEntry> = inventory::iter::<MigrationEntry>()
        .filter(|e| !applied.contains(e.name))
        .collect();
    pending.sort_by_key(|e| e.name);

    if pending.is_empty() {
        tracing::debug!("No pending migrations.");
        return Ok(0);
    }

    tracing::info!(count = pending.len(), "Running pending migrations");

    // ── 3. Run each pending migration ─────────────────────────────────────────
    let mut ran = 0_usize;
    for entry in pending {
        tracing::info!(migration = entry.name, "Applying migration");

        // Pass an owned clone — cheap because DbAdapter is Arc-backed.
        (entry.run)(adapter.clone()).await.map_err(|e| {
            tracing::error!(migration = entry.name, error = %e, "Migration failed");
            e
        })?;

        // Record applied migration — use literal record ID to avoid a separate
        // query for the name.
        let record_id = surql_record_id(entry.name);
        let sql = format!(
            "UPSERT tabMigration:{record_id} SET \
             name = $name, \
             applied_at = time::now(), \
             batch = $batch"
        );
        adapter
            .execute(
                &sql,
                vec![
                    ("name".into(),  Value::String(entry.name.to_owned())),
                    ("batch".into(), Value::Number(batch.into())),
                ],
            )
            .await?;

        tracing::info!(migration = entry.name, "Migration applied");
        ran += 1;
    }

    Ok(ran)
}

// ── current_batch ─────────────────────────────────────────────────────────────

/// Returns a batch number suitable for tagging a group of migrations run
/// together.  Uses the current Unix timestamp in seconds so it's both
/// monotonic and human-readable in the DB.
pub fn current_batch() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn surql_record_id(name: &str) -> String {
    if name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        name.to_string()
    } else {
        format!("⟨{}⟩", name)
    }
}
