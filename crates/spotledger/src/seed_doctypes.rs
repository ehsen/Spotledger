//! `spotledger seed-doctypes` — walks apps/frappe/frappe/**/doctype/{name}/{name}.json
//! and UPSERTs every DocType + its DocField + DocPerm rows into SurrealDB.
//!
//! This replicates what Frappe's `before_install` / `reload_doc` pipeline does during
//! first-time site setup, giving us the full `tabDocType` metadata that `getdoctype`
//! needs to serve the Desk UI.
//!
//! Names for child rows are derived deterministically so re-seeding is idempotent:
//!   DocField:  "{DocTypeName}-{fieldname}"
//!   DocPerm:   "{DocTypeName}-{role}-{idx}"

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde_json::{json, Value};
use spotledger_db::connection::{connect, Db};
use spotledger_db::document::upsert_doc;
use spotledger_types::config::SiteConfig;

use crate::cli::SeedDoctypesArgs;

pub async fn seed_doctypes(args: SeedDoctypesArgs) -> Result<()> {
    let bench = args
        .bench
        .canonicalize()
        .unwrap_or_else(|_| args.bench.clone());

    // ── 1. Load site config ──────────────────────────────────────────────────
    let config_path = bench
        .join("sites")
        .join(&args.site)
        .join("site_config.toml");
    let config_str = tokio::fs::read_to_string(&config_path)
        .await
        .with_context(|| format!("Reading {}", config_path.display()))?;
    let cfg: SiteConfig =
        toml::from_str(&config_str).context("Parsing site_config.toml")?;

    // ── 2. Connect ───────────────────────────────────────────────────────────
    println!(
        "Connecting to {} (ns={}) …",
        cfg.database.url, cfg.database.ns
    );
    let db = connect(&cfg.database)
        .await
        .context("Connecting to SurrealDB")?;
    println!("Connected.");

    let app_root = bench.join("apps").join("frappe").join("frappe");
    let (seeded, errors) = seed_doctypes_for_app(&db, &app_root, "frappe").await?;
    println!("\nDone. {} DocTypes seeded, {} errors.", seeded, errors);
    if errors > 0 {
        anyhow::bail!("{} DocType(s) failed to seed — check output above", errors);
    }
    Ok(())
}

/// Inner implementation — takes an already-connected `Db` and the app root directory.
/// Returns `(seeded_count, error_count)`.
pub async fn seed_doctypes_for_app(
    db: &Db,
    app_root: &Path,
    app_name: &str,
) -> Result<(usize, usize)> {
    // ── 3. Apply any schema additions that may be missing on older sites ────
    // Add fields introduced in newer Frappe versions or missing from the first
    // bootstrap schema pass.  All statements are idempotent (IF NOT EXISTS).
    println!("Applying schema fixups …");
    db.query(
        "DEFINE FIELD IF NOT EXISTS doctype      ON tabDocType TYPE none | string; \
         DEFINE FIELD IF NOT EXISTS quick_entry  ON tabDocType TYPE none | bool | int; \
         DEFINE FIELD IF NOT EXISTS grid_page_length ON tabDocType TYPE none | int; \
         DEFINE FIELD IF NOT EXISTS index_web_pages_for_search ON tabDocType TYPE none | bool | int; \
         DEFINE FIELD IF NOT EXISTS make_attachments_public ON tabDocType TYPE none | bool | int; \
         DEFINE FIELD IF NOT EXISTS row_format   ON tabDocType TYPE none | string; \
         DEFINE FIELD IF NOT EXISTS name_case    ON tabDocType TYPE none | string; \
         DEFINE FIELD IF NOT EXISTS default_view ON tabDocType TYPE none | string; \
         DEFINE FIELD IF NOT EXISTS track_views  ON tabDocType TYPE none | bool | int; \
         DEFINE FIELD IF NOT EXISTS allow_events_in_timeline ON tabDocType TYPE none | bool | int; \
         DEFINE FIELD IF NOT EXISTS force_re_route_to_default_view ON tabDocType TYPE none | bool | int; \
         DEFINE FIELD IF NOT EXISTS rows_threshold_for_grid_search ON tabDocType TYPE none | int; \
         DEFINE FIELD IF NOT EXISTS doctype      ON tabModule_Def TYPE none | string; \
         DEFINE FIELD IF NOT EXISTS doctype      ON tabDocField TYPE none | string; \
         DEFINE FIELD IF NOT EXISTS hidden       ON tabDocField TYPE none | bool | int; \
         DEFINE FIELD IF NOT EXISTS read_only    ON tabDocField TYPE none | bool | int; \
         DEFINE FIELD IF NOT EXISTS no_copy      ON tabDocField TYPE none | bool | int; \
         DEFINE FIELD IF NOT EXISTS print_hide   ON tabDocField TYPE none | bool | int; \
         DEFINE FIELD IF NOT EXISTS depends_on   ON tabDocField TYPE none | string; \
         DEFINE FIELD IF NOT EXISTS mandatory_depends_on   ON tabDocField TYPE none | string; \
         DEFINE FIELD IF NOT EXISTS read_only_depends_on   ON tabDocField TYPE none | string; \
         DEFINE FIELD IF NOT EXISTS collapsible            ON tabDocField TYPE none | bool | int; \
         DEFINE FIELD IF NOT EXISTS collapsible_depends_on ON tabDocField TYPE none | string; \
         DEFINE FIELD IF NOT EXISTS ignore_user_permissions ON tabDocField TYPE none | bool | int; \
         DEFINE FIELD IF NOT EXISTS ignore_xss_filter      ON tabDocField TYPE none | bool | int; \
         DEFINE FIELD IF NOT EXISTS allow_on_submit        ON tabDocField TYPE none | bool | int; \
         DEFINE FIELD IF NOT EXISTS set_only_once          ON tabDocField TYPE none | bool | int; \
         DEFINE FIELD IF NOT EXISTS allow_bulk_edit        ON tabDocField TYPE none | bool | int; \
         DEFINE FIELD IF NOT EXISTS allow_in_quick_entry   ON tabDocField TYPE none | bool | int; \
         DEFINE FIELD IF NOT EXISTS in_global_search       ON tabDocField TYPE none | bool | int; \
         DEFINE FIELD IF NOT EXISTS in_standard_filter     ON tabDocField TYPE none | bool | int; \
         DEFINE FIELD IF NOT EXISTS in_preview             ON tabDocField TYPE none | bool | int; \
         DEFINE FIELD IF NOT EXISTS fetch_from             ON tabDocField TYPE none | string; \
         DEFINE FIELD IF NOT EXISTS fetch_if_empty         ON tabDocField TYPE none | bool | int; \
         DEFINE FIELD IF NOT EXISTS show_dashboard         ON tabDocField TYPE none | bool | int; \
         DEFINE FIELD IF NOT EXISTS permlevel              ON tabDocField TYPE none | int; \
         DEFINE FIELD IF NOT EXISTS columns                ON tabDocField TYPE none | int; \
         DEFINE FIELD IF NOT EXISTS width                  ON tabDocField TYPE none | string; \
         DEFINE FIELD IF NOT EXISTS max_height             ON tabDocField TYPE none | string; \
         DEFINE FIELD IF NOT EXISTS print_width            ON tabDocField TYPE none | string; \
         DEFINE FIELD IF NOT EXISTS non_negative           ON tabDocField TYPE none | bool | int; \
         DEFINE FIELD IF NOT EXISTS hide_days              ON tabDocField TYPE none | bool | int; \
         DEFINE FIELD IF NOT EXISTS hide_seconds           ON tabDocField TYPE none | bool | int; \
         DEFINE FIELD IF NOT EXISTS hide_border            ON tabDocField TYPE none | bool | int; \
         DEFINE FIELD IF NOT EXISTS sort_options           ON tabDocField TYPE none | bool | int; \
         DEFINE FIELD IF NOT EXISTS oldfieldname           ON tabDocField TYPE none | string; \
         DEFINE FIELD IF NOT EXISTS oldfieldtype           ON tabDocField TYPE none | string; \
         DEFINE FIELD IF NOT EXISTS documentation_url      ON tabDocField TYPE none | string; \
         DEFINE FIELD IF NOT EXISTS doctype ON tabHas_Role  TYPE none | string; \
         DEFINE FIELD IF NOT EXISTS doctype ON tabPage      TYPE none | string; \
         DEFINE FIELD IF NOT EXISTS doctype ON tabReport    TYPE none | string; \
         DEFINE FIELD IF NOT EXISTS doctype ON tabWorkspace TYPE none | string;",
    )
    .await
    .context("Applying schema fixups")?;

    // ── 4. Collect DocType JSON files ────────────────────────────────────────
    let json_files = collect_doctype_jsons(app_root)?;
    println!(
        "Found {} DocType JSON files under {}",
        json_files.len(),
        app_root.display()
    );

    let mut seeded: usize = 0;
    let mut errors: usize = 0;
    let mut modules: BTreeSet<(String, String)> = BTreeSet::new(); // (module_name, app_name)

    // ── 5. Seed each DocType ─────────────────────────────────────────────────
    for path in &json_files {
        let raw = tokio::fs::read_to_string(path)
            .await
            .with_context(|| format!("Reading {}", path.display()))?;

        let doc: Value = match serde_json::from_str(&raw) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("WARN: parse error in {}: {}", path.display(), e);
                continue;
            }
        };

        let doctype_name = match doc.get("name").and_then(Value::as_str) {
            Some(n) => n.to_owned(),
            None => {
                eprintln!("WARN: no 'name' field in {}", path.display());
                continue;
            }
        };

        // Verify this is a real DocType JSON (guard against stray files)
        if doc.get("doctype").and_then(Value::as_str) != Some("DocType") {
            eprintln!(
                "WARN: skipping {} — doctype != 'DocType'",
                path.display()
            );
            continue;
        }

        let module = doc
            .get("module")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_owned();
        if !module.is_empty() {
            modules.insert((module.clone(), app_name.to_owned()));
        }

        // ── 4a. Upsert the DocType row (top-level metadata only) ─────────────
        let dt_row = build_doctype_row(&doc);
        match upsert_doc(&db, "DocType", &doctype_name, &dt_row).await {
            Ok(_) => {}
            Err(e) => {
                eprintln!("ERROR seeding DocType '{}': {}", doctype_name, e);
                errors += 1;
                continue;
            }
        }

        // ── 4b. Upsert DocField child rows ───────────────────────────────────
        if let Some(Value::Array(fields)) = doc.get("fields") {
            for (idx, field) in fields.iter().enumerate() {
                let fieldname = field
                    .get("fieldname")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                if fieldname.is_empty() {
                    continue;
                }
                // Deterministic, unique name: ParentName-fieldname
                let row_name = format!("{}-{}", doctype_name, fieldname);
                let row = build_child_row(
                    field,
                    &row_name,
                    &doctype_name,
                    "DocType",
                    "DocField",
                    "fields",
                    idx,
                );
                if let Err(e) = upsert_doc(&db, "DocField", &row_name, &row).await {
                    eprintln!("WARN: DocField '{}': {}", row_name, e);
                }
            }
        }

        // ── 4c. Upsert DocPerm child rows ────────────────────────────────────
        if let Some(Value::Array(perms)) = doc.get("permissions") {
            for (idx, perm) in perms.iter().enumerate() {
                let role = perm
                    .get("role")
                    .and_then(Value::as_str)
                    .unwrap_or("Guest");
                // Deterministic name: ParentName-RoleName-idx
                let row_name = format!("{}-{}-{}", doctype_name, role, idx);
                let row = build_child_row(
                    perm,
                    &row_name,
                    &doctype_name,
                    "DocType",
                    "DocPerm",
                    "permissions",
                    idx,
                );
                if let Err(e) = upsert_doc(&db, "DocPerm", &row_name, &row).await {
                    eprintln!("WARN: DocPerm '{}': {}", row_name, e);
                }
            }
        }

        seeded += 1;
        if seeded % 50 == 0 {
            println!("  … {}/{} seeded", seeded, json_files.len());
        }
    }

    // ── 5. Seed Module Def rows (deduplicated) ───────────────────────────────
    println!("Seeding {} Module Def rows …", modules.len());
    for (module_name, app_name) in &modules {
        let row = json!({
            "name":        module_name,
            "doctype":     "Module Def",
            "module_name": module_name,
            "app_name":    app_name,
            "owner":       "Administrator",
            "modified_by": "Administrator",
        });
        if let Err(e) = upsert_doc(&db, "Module Def", module_name, &row).await {
            eprintln!("WARN: Module Def '{}': {}", module_name, e);
        }
    }

    Ok((seeded, errors))
}

// ── helpers ──────────────────────────────────────────────────────────────────

/// Build the top-level DocType row, stripping child-table arrays.
fn build_doctype_row(doc: &Value) -> Value {
    let mut row = doc.clone();
    if let Value::Object(ref mut map) = row {
        // Remove child-table arrays — stored as separate table rows
        for key in &["fields", "permissions", "actions", "links", "states", "field_order"] {
            map.remove(*key);
        }
        map.insert("doctype".to_owned(), json!("DocType"));
        map.entry("owner".to_owned())
            .or_insert_with(|| json!("Administrator"));
        map.entry("modified_by".to_owned())
            .or_insert_with(|| json!("Administrator"));
    }
    row
}

/// Build a child row with standard parent-link fields.
pub fn build_child_row(
    source: &Value,
    name: &str,
    parent: &str,
    parenttype: &str,
    doctype: &str,
    parentfield: &str,
    idx: usize,
) -> Value {
    let mut row = source.clone();
    if let Value::Object(ref mut map) = row {
        map.insert("name".to_owned(), json!(name));
        map.insert("parent".to_owned(), json!(parent));
        map.insert("parenttype".to_owned(), json!(parenttype));
        map.insert("parentfield".to_owned(), json!(parentfield));
        map.insert("doctype".to_owned(), json!(doctype));
        map.insert("idx".to_owned(), json!(idx));
        map.entry("owner".to_owned())
            .or_insert_with(|| json!("Administrator"));
        map.entry("modified_by".to_owned())
            .or_insert_with(|| json!("Administrator"));
    }
    row
}

/// Walk `frappe_root/**/doctype/{name}/{name}.json` using only std::fs (no glob crate).
///
/// The canonical Frappe layout is:
///   frappe/<module>/doctype/<dt_name>/<dt_name>.json
///
/// We only pick files where the JSON filename (without `.json`) matches the parent
/// directory name — this filters out test_records.json and other fixtures.
fn collect_doctype_jsons(frappe_root: &Path) -> Result<Vec<PathBuf>> {
    let mut result = Vec::new();

    for module_entry in std::fs::read_dir(frappe_root)
        .with_context(|| format!("Reading {}", frappe_root.display()))?
    {
        let module_dir = module_entry?.path();
        if !module_dir.is_dir() {
            continue;
        }
        let doctype_dir = module_dir.join("doctype");
        if !doctype_dir.is_dir() {
            continue;
        }
        for dt_entry in std::fs::read_dir(&doctype_dir)? {
            let dt_dir = dt_entry?.path();
            if !dt_dir.is_dir() {
                continue;
            }
            let dt_dir_name = dt_dir
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            let json_path = dt_dir.join(format!("{}.json", dt_dir_name));
            if json_path.exists() {
                result.push(json_path);
            }
        }
    }

    result.sort();
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collect_finds_known_doctype() {
        let frappe_root =
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../../../apps/frappe/frappe");
        if !frappe_root.exists() {
            return; // Skip in CI where apps/ may not be present
        }
        let files = collect_doctype_jsons(&frappe_root).unwrap();
        assert!(files.len() >= 200, "expected ≥200 doctype JSONs, got {}", files.len());
        // The canonical DocType doctype must be present
        assert!(
            files.iter().any(|p| p.ends_with("doctype/doctype/doctype.json")),
            "doctype/doctype/doctype.json not found in list"
        );
    }

    #[test]
    fn build_doctype_row_strips_children() {
        let doc = serde_json::json!({
            "name": "User",
            "doctype": "DocType",
            "module": "Core",
            "fields": [{"fieldname": "email"}],
            "permissions": [{"role": "Administrator"}],
        });
        let row = build_doctype_row(&doc);
        assert!(row.get("fields").is_none());
        assert!(row.get("permissions").is_none());
        assert_eq!(row["module"], "Core");
    }

    #[test]
    fn build_child_row_sets_parent_fields() {
        let source = serde_json::json!({"fieldname": "email", "fieldtype": "Data"});
        let row = build_child_row(&source, "User-email", "User", "DocType", "DocField", "fields", 0);
        assert_eq!(row["name"], "User-email");
        assert_eq!(row["parent"], "User");
        assert_eq!(row["parenttype"], "DocType");
        assert_eq!(row["parentfield"], "fields");
        assert_eq!(row["idx"], 0);
    }
}
