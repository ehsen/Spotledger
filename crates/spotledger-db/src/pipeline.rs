//! Pipeline infrastructure: apply surql files and run the pipeline.
//!
//! ## Responsibilities
//!
//! 1. `apply_pipeline_functions` — apply all `surql/` files for an app in the
//!    correct order: schema → shared fn:: → domain functions → registry →
//!    runner → universal nodes → wire_generic helper → domain wiring.surql.
//!    Wiring is pure SurrealDB — no Rust code encodes stage topology.
//!
//! 2. `is_pipeline_registered` — fast check used by save_proxy to decide
//!    whether to call fn::pipeline::run.
//!
//! 3. `run_pipeline` — calls fn::pipeline::run, interprets the result.

use serde_json::{json, Value};

use crate::adapter::DbAdapter;
use crate::apply_surql::{
    apply_registry, apply_surql_dir, apply_surql_dir_named, apply_surql_file, collect_fn_names,
};
use crate::error::DbError;

// ── apply_pipeline_functions ──────────────────────────────────────────────────

/// Apply all pipeline surql files for an app, in strict dependency order.
///
/// Order:
///   1. `surql/framework/01_schema.surql`  — DDL for pipeline tables
///   2. `surql/shared/`                    — universal fn:: definitions
///   3. `surql/doctypes/*/functions.surql` — domain fn:: definitions
///   4. registry (generated in memory)     — fn::registry::dispatch
///   5. `surql/framework/02_runner.surql`  — fn::pipeline::run (needs registry)
///   6. `surql/framework/03_universal_nodes.surql` — shared pipeline_node records
///   7. `surql/framework/04_wire_generic.surql`     — fn::pipeline::wire_generic
///   8. `surql/doctypes/*/wiring.surql`    — doctype wiring (calls wire_generic)
///
/// Wiring and node topology live entirely in SurrealDB. Rust never encodes
/// stage graphs — that is the job of the wiring.surql files.
///
/// Returns the number of fn:: names registered in the dispatcher.
pub async fn apply_pipeline_functions(
    adapter:  &DbAdapter,
    app_root: &std::path::Path,
) -> Result<usize, DbError> {
    let surql_dir   = app_root.join("surql");
    let fw_dir      = surql_dir.join("framework");
    let shared_dir  = surql_dir.join("shared");
    let doctypes_dir = surql_dir.join("doctypes");

    // ── 1. Schema DDL ─────────────────────────────────────────────────────────
    let schema_file = fw_dir.join("01_schema.surql");
    if schema_file.exists() {
        apply_surql_file(adapter, &schema_file).await?;
        tracing::debug!("pipeline: applied schema DDL");
    }

    // ── 2. Shared fn:: definitions ────────────────────────────────────────────
    let shared_count = apply_surql_dir(adapter, &shared_dir).await?;
    tracing::debug!("pipeline: applied {} shared fn:: files", shared_count);

    // ── 3. Domain fn:: definitions (functions.surql only) ────────────────────
    let domain_count =
        apply_surql_dir_named(adapter, &doctypes_dir, Some("functions.surql")).await?;
    tracing::debug!("pipeline: applied {} domain functions.surql files", domain_count);

    // ── 4. Build registry from shared + domain function files ────────────────
    let fn_names = collect_fn_names_from_dirs(&[&shared_dir, &doctypes_dir]);
    apply_registry(adapter, &fn_names).await?;
    tracing::debug!("pipeline: registry applied ({} fn::)", fn_names.len());

    // ── 5. Runner (needs registry) ────────────────────────────────────────────
    let runner_file = fw_dir.join("02_runner.surql");
    if runner_file.exists() {
        apply_surql_file(adapter, &runner_file).await?;
        tracing::debug!("pipeline: applied runner");
    }

    // ── 6. Universal pipeline_node records ───────────────────────────────────
    let universal_nodes_file = fw_dir.join("03_universal_nodes.surql");
    if universal_nodes_file.exists() {
        apply_surql_file(adapter, &universal_nodes_file).await?;
        tracing::debug!("pipeline: applied universal nodes");
    }

    // ── 7. wire_generic helper fn ─────────────────────────────────────────────
    let wire_generic_file = fw_dir.join("04_wire_generic.surql");
    if wire_generic_file.exists() {
        apply_surql_file(adapter, &wire_generic_file).await?;
        tracing::debug!("pipeline: applied wire_generic");
    }

    // ── 8. Domain wiring (wiring.surql — calls wire_generic + adds edges) ────
    let wiring_count =
        apply_surql_dir_named(adapter, &doctypes_dir, Some("wiring.surql")).await?;
    tracing::debug!("pipeline: applied {} domain wiring.surql files", wiring_count);

    // ── 9. Naming functions (framework-level, not a pipeline stage) ──────────
    let naming_file = fw_dir.join("05_naming.surql");
    if naming_file.exists() {
        apply_surql_file(adapter, &naming_file).await?;
        tracing::debug!("pipeline: applied naming functions");
    }

    // ── 10. Permission functions (framework-level graph-based RBAC) ──────────
    let permissions_file = fw_dir.join("06_permissions.surql");
    if permissions_file.exists() {
        apply_surql_file(adapter, &permissions_file).await?;
        tracing::debug!("pipeline: applied permission functions");
    }

    Ok(fn_names.len())
}

/// Collect all fn:: names declared in `.surql` files under the given directories.
fn collect_fn_names_from_dirs(dirs: &[&std::path::Path]) -> Vec<String> {
    let mut all = std::collections::BTreeSet::new();
    for dir in dirs {
        if !dir.exists() { continue; }
        // Walk one level deep (same as apply_surql_dir)
        if let Ok(entries) = std::fs::read_dir(dir) {
            let mut paths = vec![];
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_dir() {
                    if let Ok(subs) = std::fs::read_dir(&p) {
                        for sub in subs.flatten() {
                            let sp = sub.path();
                            if sp.extension().and_then(|e| e.to_str()) == Some("surql") {
                                paths.push(sp);
                            }
                        }
                    }
                } else if p.extension().and_then(|e| e.to_str()) == Some("surql") {
                    paths.push(p);
                }
            }
            for path in paths {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    for name in collect_fn_names(&content) {
                        all.insert(name);
                    }
                }
            }
        }
    }
    all.into_iter().collect()
}

// ── is_pipeline_registered ────────────────────────────────────────────────────

/// Returns true if this doctype exists in tabDocType (i.e. it has been seeded).
///
/// Every seeded doctype participates in the pipeline — the runner returns
/// `{status:"skipped",reason:"no_stages"}` if nothing is wired, which is fine.
/// Returns false on any DB error (safe fallback to direct UPSERT).
pub async fn is_pipeline_registered(adapter: &DbAdapter, doctype: &str) -> bool {
    adapter
        .run(
            "SELECT name FROM tabDocType WHERE name = $dt LIMIT 1",
            vec![("dt".into(), json!(doctype))],
        )
        .await
        .map(|r| !r.is_empty())
        .unwrap_or(false)
}

// ── run_pipeline ──────────────────────────────────────────────────────────────

/// Pipeline status returned from fn::pipeline::run.
#[derive(Debug, PartialEq, Eq)]
pub enum PipelineResult {
    /// Pipeline ran and all stages passed.
    Ok,
    /// Doctype not registered or no stages — Rust should keep the saved doc as-is.
    Skipped,
}

/// Call `fn::pipeline::run` and interpret the result.
///
/// Returns:
/// - `Ok(PipelineResult::Ok)`      — pipeline ran successfully
/// - `Ok(PipelineResult::Skipped)` — no pipeline registered; caller falls back
/// - `Err(DbError)`               — validation/logic failure; caller should clean up
pub async fn run_pipeline(
    adapter:  &DbAdapter,
    table:    &str,
    name:     &str,
    doctype:  &str,
    action:   &str,
) -> Result<PipelineResult, DbError> {
    let rows = adapter
        .run(
            "RETURN fn::pipeline::run(type::record($table, $name), $doctype, $action);",
            vec![
                ("table".into(),   json!(table)),
                ("name".into(),    json!(name)),
                ("doctype".into(), json!(doctype)),
                ("action".into(),  json!(action)),
            ],
        )
        .await?;

    let result = rows.into_iter().next().unwrap_or(Value::Null);
    let status = result
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("ok");

    if status == "skipped" {
        Ok(PipelineResult::Skipped)
    } else if status == "error" {
        let msg = result
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or("Pipeline validation error")
            .to_owned();
        Err(DbError::Other(msg))
    } else {
        Ok(PipelineResult::Ok)
    }
}

