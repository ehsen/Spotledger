//! `spotledger emit` — write the full generated SurrealQL schema to a file.
//!
//! Iterates every compiled `inventory::submit!(MetaEntry { … })` and produces
//! the exact `DEFINE TABLE / DEFINE FIELD / DEFINE INDEX` statements that
//! `ensure_all_schemas` would execute.  Nothing is sent to the database.
//!
//! Output path: `<bench>/generated/schema.surql`
//!
//! The file is suitable for:
//! - Code review in pull requests (diff against previous version)
//! - Manual inspection before applying to a production site
//! - Audit trails and compliance

use anyhow::Context;
use spotledger_db::schema::emit_all_schemas_sql;
use spotledger_db::bootstrap;

use crate::cli::EmitArgs;

pub async fn emit(args: EmitArgs) -> anyhow::Result<()> {
    let bench = args.bench.canonicalize()
        .unwrap_or_else(|_| args.bench.clone());

    let out_dir  = bench.join("generated");
    let out_path = out_dir.join("schema.surql");

    // ── Generate SQL from inventory (no DB needed) ────────────────────────────
    let (doctype_sql, dt_count) = emit_all_schemas_sql();

    // Prepend the framework-tables bootstrap block for completeness
    let framework_header =
        "-- ════════════════════════════════════════════════════════════════════\n\
         -- Framework-internal tables (not DocTypes)\n\
         -- ════════════════════════════════════════════════════════════════════\n\n";

    // The FRAMEWORK_TABLES const is not pub, so we expose it via a thin fn
    let framework_sql = bootstrap::framework_tables_sql();

    let doctype_header =
        "\n-- ════════════════════════════════════════════════════════════════════\n\
         -- DocType tables (generated from compiled Rust inventory)\n\
         -- ════════════════════════════════════════════════════════════════════\n\n";

    let full = format!("{framework_header}{framework_sql}{doctype_header}{doctype_sql}");

    // ── Write output ──────────────────────────────────────────────────────────
    tokio::fs::create_dir_all(&out_dir)
        .await
        .with_context(|| format!("Creating {}", out_dir.display()))?;

    tokio::fs::write(&out_path, &full)
        .await
        .with_context(|| format!("Writing {}", out_path.display()))?;

    println!(
        "Emitted {dt_count} DocType(s) → {} ({} bytes)",
        out_path.display(),
        full.len()
    );

    Ok(())
}
