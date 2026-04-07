//! `spotledger cleanup` — remove orphaned DocField / DocPerm rows.
//!
//! When a DocType is deleted from Rust code and `ensure_all_schemas` is
//! re-run, the `tab<Name>` table still exists in SurrealDB (by design —
//! dropping it would risk data loss).  Similarly, any `tabDocField` and
//! `tabDocPerm` rows that were seeded for that DocType become orphans.
//!
//! This command:
//! 1. Builds the set of DocType names currently in the compiled inventory.
//! 2. Queries `tabDocField` and `tabDocPerm` for rows whose `parent` is NOT
//!    in that set.
//! 3. Prints a report of what it found.
//! 4. With `--apply`, deletes those rows.
//!
//! The table itself (`tab<Name>`) is never touched — that requires an explicit
//! manual `REMOVE TABLE` statement after verifying no live data remains.

use std::collections::HashSet;

use anyhow::Context;
use serde_json::Value;
use spotledger_core::config::SiteConfig;
use spotledger_db::connection::connect;
use spotledger_db::schema::compiled_doctype_names;

use crate::cli::CleanupArgs;

pub async fn cleanup(args: CleanupArgs) -> anyhow::Result<()> {
    let bench = args.bench.canonicalize()
        .unwrap_or_else(|_| args.bench.clone());

    // ── 1. Load site config + connect ─────────────────────────────────────────
    let cfg_path = bench.join("sites").join(&args.site).join("site_config.toml");
    let cfg: SiteConfig = {
        let s = tokio::fs::read_to_string(&cfg_path)
            .await
            .with_context(|| format!("Reading {}", cfg_path.display()))?;
        toml::from_str(&s).context("Parsing site_config.toml")?
    };

    println!("Connecting to {} (ns={}) …", cfg.database.url, cfg.database.ns);
    let db = connect(&cfg.database)
        .await
        .context("Connecting to SurrealDB")?;
    println!("Connected.");

    // ── 2. Build the live inventory set ──────────────────────────────────────
    let compiled: HashSet<String> = compiled_doctype_names();

    println!("Compiled DocTypes in inventory: {}", compiled.len());

    // ── 3. Find orphaned rows in tabDocField ──────────────────────────────────
    let field_orphans = find_orphans(&db, "tabDocField", &compiled).await?;
    // ── 4. Find orphaned rows in tabDocPerm ───────────────────────────────────
    let perm_orphans  = find_orphans(&db, "tabDocPerm",  &compiled).await?;

    if field_orphans.is_empty() && perm_orphans.is_empty() {
        println!("No orphaned rows found. Nothing to clean up.");
        return Ok(());
    }

    // ── 5. Report ─────────────────────────────────────────────────────────────
    println!();
    if !field_orphans.is_empty() {
        println!("Orphaned tabDocField parents ({}):", field_orphans.len());
        for p in &field_orphans {
            println!("  - {p}");
        }
    }
    if !perm_orphans.is_empty() {
        println!("Orphaned tabDocPerm parents ({}):", perm_orphans.len());
        for p in &perm_orphans {
            println!("  - {p}");
        }
    }

    // ── 6. Delete (only with --apply) ─────────────────────────────────────────
    if !args.apply {
        println!();
        println!("Dry run — no rows deleted. Re-run with --apply to delete.");
        return Ok(());
    }

    println!();
    println!("Deleting orphaned rows …");

    let mut deleted = 0usize;
    for parent in &field_orphans {
        db.execute(
            "DELETE tabDocField WHERE parent = $parent",
            vec![("parent".into(), Value::String(parent.clone()))],
        )
        .await
        .with_context(|| format!("Deleting tabDocField rows for {parent}"))?;
        println!("  Deleted tabDocField rows for '{parent}'");
        deleted += 1;
    }
    for parent in &perm_orphans {
        db.execute(
            "DELETE tabDocPerm WHERE parent = $parent",
            vec![("parent".into(), Value::String(parent.clone()))],
        )
        .await
        .with_context(|| format!("Deleting tabDocPerm rows for {parent}"))?;
        println!("  Deleted tabDocPerm rows for '{parent}'");
        deleted += 1;
    }

    println!();
    println!("Cleanup complete. Removed orphaned rows for {deleted} DocType(s).");
    println!();
    println!(
        "Note: the tab<Name> tables themselves were NOT dropped.\n\
         Verify no live data remains, then run:\n\
         \n  REMOVE TABLE `tab<Name>`;\n\
         \nin the SurrealDB CLI if you wish to drop them."
    );

    Ok(())
}

/// Query `table` for distinct `parent` values not present in `compiled`.
async fn find_orphans(
    db: &spotledger_db::DbAdapter,
    table: &str,
    compiled: &HashSet<String>,
) -> anyhow::Result<Vec<String>> {
    let rows = db
        .run(
            &format!("SELECT DISTINCT parent FROM {table}"),
            vec![],
        )
        .await
        .with_context(|| format!("Querying {table}"))?;

    let orphans = rows
        .into_iter()
        .filter_map(|v| v.get("parent").and_then(Value::as_str).map(str::to_owned))
        .filter(|p| !p.is_empty() && !compiled.contains(p))
        .collect();

    Ok(orphans)
}
