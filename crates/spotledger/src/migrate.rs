//! `spotledger migrate` — apply pending schema sync and data migrations to a site.
//!
//! This command is the explicit alternative to the automatic migration that runs
//! at `serve` startup.  Useful for:
//!
//! - Controlled rollouts: test migrations on a staging site first.
//! - CI pipelines: migrate after deploying a new binary before switching traffic.
//! - Debugging: `--dry-run` shows what would run without touching data.
//!
//! ## What it does
//!
//! 1. Connects to the site's SurrealDB namespace.
//! 2. Runs `ensure_all_schemas` — additive DDL for any new compiled DocTypes or
//!    fields.  Idempotent; `DEFINE … IF NOT EXISTS` guards mean nothing is lost
//!    even if this has run before.
//! 3. Runs `seed_naming_rules` — upserts `tabDocumentNamingRule` rows for any
//!    DocTypes whose compiled `autoname` has not yet been seeded.  Idempotent.
//! 4. Queries `tabMigration` to find which versioned data migrations have already
//!    been recorded for this site.
//! 5. Runs each pending `MigrationEntry` (sorted by name) and records it with the
//!    current batch number.

use anyhow::Context;
use spotledger_core::config::SiteConfig;
use spotledger_db::bootstrap::seed_naming_rules;
use spotledger_db::connection::connect;
use spotledger_db::migrations::{current_batch, pending_migration_names, run_pending_migrations};
use spotledger_db::schema::ensure_all_schemas;

use crate::cli::MigrateArgs;

pub async fn migrate(args: MigrateArgs) -> anyhow::Result<()> {
    let bench = args.bench.canonicalize().unwrap_or_else(|_| args.bench.clone());
    let config_path = bench.join("sites").join(&args.site).join("site_config.toml");

    let cfg = SiteConfig::from_file(&config_path)
        .with_context(|| format!("Reading {}", config_path.display()))?;

    println!("Migrating site '{}' ({})…", args.site, cfg.database.url);

    let db = connect(&cfg.database)
        .await
        .context("Connecting to SurrealDB")?;

    // ── Dry-run mode: just list pending migrations ────────────────────────────
    if args.dry_run {
        let pending = pending_migration_names(&db).await?;
        if pending.is_empty() {
            println!("  No pending migrations.");
        } else {
            println!("  Pending migrations ({}):", pending.len());
            for name in &pending {
                println!("    - {name}");
            }
        }
        return Ok(());
    }

    // ── 1. DDL sync ───────────────────────────────────────────────────────────
    println!("  [1/3] Syncing schemas from compiled inventory…");
    ensure_all_schemas(&db)
        .await
        .context("Schema sync")?;

    // ── 2. Naming rules ───────────────────────────────────────────────────────
    println!("  [2/3] Seeding DocumentNamingRule rows…");
    seed_naming_rules(&db)
        .await
        .context("Seeding naming rules")?;

    // ── 3. Data migrations ────────────────────────────────────────────────────
    println!("  [3/3] Running pending data migrations…");
    let n = run_pending_migrations(&db, current_batch())
        .await
        .context("Running data migrations")?;

    if n == 0 {
        println!("  Nothing to migrate — site is up to date.");
    } else {
        println!("  {n} migration(s) applied successfully.");
    }

    Ok(())
}
