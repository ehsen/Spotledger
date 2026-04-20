//! `spotledger install-app` — replicates what Frappe's `bench install-app` pipeline does:
//!
//! 1.  Schema fixups + DocType definitions  (`seed_doctypes_for_app`)
//! 2.  Module Def records                   (from `{app}/modules.txt`)
//! 3.  Importable fixture records           (Workspace, Page, Report, …)
//! 4.  Patch Log "done" entries             (from `{app}/patches.txt`)
//! 5.  Pipeline framework functions + wiring seed  (graph compute)
//!
//! This mirrors the sequence in `frappe/installer.py::install_app()`:
//!   - `sync_for(name)`         → steps 1 + 3   (DocTypes + importable fixture records)
//!   - `add_module_defs(name)`  → step 2
//!   - `set_all_patches_as_completed(name)` → step 4
//!
//! Run: `spotledger install-app frappe --bench /path/to/bench --site mysite.localhost`

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use flate2::read::GzDecoder;
use serde_json::{json, Value};
use spotledger_db::connection::connect;
use spotledger_db::DbAdapter;
use spotledger_db::document::upsert_doc;
use spotledger_db::graph_ops::{
    log_schema_change, relate_module_contains_doctype, upsert_app_node, upsert_docfield_graph,
    upsert_module_node,
};
use spotledger_db::pipeline::apply_pipeline_functions;
use spotledger_core::config::SiteConfig;
use tar::Archive;

use crate::cli::InstallAppArgs;
use crate::seed_doctypes::{build_child_row, seed_doctypes_for_app};

// ---------------------------------------------------------------------------
// Importable fixture type registry
//
// Each entry is: (filesystem_dir_name, DocType_name, child_fields)
// child_fields:  [(json_array_key, child_doctype_name), …]
//
// This mirrors Frappe's IMPORTABLE_DOCTYPES list in frappe/model/sync.py.
// "doctype" is intentionally absent — handled by seed_doctypes_for_app.
// ---------------------------------------------------------------------------
const IMPORTABLE_TYPES: &[(&str, &str, &[(&str, &str)])] = &[
    ("permission_type", "Permission Type", &[]),
    (
        "page",
        "Page",
        &[("roles", "Has Role")],
    ),
    (
        "report",
        "Report",
        &[
            ("roles", "Has Role"),
            ("columns", "Report Column"),
            ("filters", "Report Filter"),
        ],
    ),
    ("dashboard_chart_source", "Dashboard Chart Source", &[]),
    ("print_format", "Print Format", &[]),
    ("website_theme", "Website Theme", &[]),
    (
        "web_form",
        "Web Form",
        &[("web_form_fields", "Web Form Field")],
    ),
    ("web_template", "Web Template", &[]),
    (
        "notification",
        "Notification",
        &[("recipients", "Notification Recipient")],
    ),
    ("print_style", "Print Style", &[]),
    (
        "workspace",
        "Workspace",
        &[
            ("links",         "Workspace Link"),
            ("shortcuts",     "Workspace Shortcut"),
            ("charts",        "Workspace Chart"),
            ("number_cards",  "Workspace Number Card"),
            ("custom_blocks", "Workspace Custom Block"),
            ("quick_lists",   "Workspace Quick List"),
        ],
    ),
    (
        "workspace_sidebar",
        "Workspace Sidebar",
        &[("sidebar_items", "Workspace Sidebar Item")],
    ),
    (
        "onboarding_step",
        "Onboarding Step",
        &[("action_fields", "Onboarding Step Field")],
    ),
    (
        "module_onboarding",
        "Module Onboarding",
        &[("steps", "Onboarding Step Map")],
    ),
    (
        "form_tour",
        "Form Tour",
        &[("steps", "Form Tour Step")],
    ),
    ("client_script",    "Client Script",    &[]),
    ("server_script",    "Server Script",    &[]),
    ("custom_field",     "Custom Field",     &[]),
    ("property_setter",  "Property Setter",  &[]),
];

// ---------------------------------------------------------------------------
// app.json manifest
// ---------------------------------------------------------------------------

/// Read and return the `app.json` manifest for an app.
/// Returns `None` if no `app.json` exists (legacy apps without one).
async fn read_app_manifest(bench: &Path, app_name: &str) -> Option<Value> {
    let path = bench.join("apps").join(app_name).join("app.json");
    let raw = tokio::fs::read_to_string(&path).await.ok()?;
    serde_json::from_str(&raw).ok()
}

/// Check whether an app already exists in `installed_app`.
async fn is_app_installed(db: &DbAdapter, app_name: &str) -> Result<bool> {
    let query = "SELECT name FROM tabinstalled_app WHERE name = $name LIMIT 1";
    let rows = db
        .run(query, vec![("name".to_owned(), Value::String(app_name.to_owned()))])
        .await
        .with_context(|| format!("Checking installed_app for '{}'", app_name))?;
    Ok(!rows.is_empty())
}

/// Ensure all manifest dependencies are already installed.
async fn enforce_manifest_dependencies(
    db: &DbAdapter,
    app_name: &str,
    manifest: Option<&Value>,
) -> Result<()> {
    let Some(manifest) = manifest else {
        return Ok(());
    };

    let Some(depends_on) = manifest.get("depends_on") else {
        return Ok(());
    };

    let Some(deps) = depends_on.as_array() else {
        anyhow::bail!("Invalid app.json for '{}': depends_on must be an array", app_name);
    };

    for dep in deps.iter().filter_map(|v| v.as_str()) {
        if dep.trim().is_empty() || dep == app_name {
            continue;
        }
        if !is_app_installed(db, dep).await? {
            anyhow::bail!(
                "Dependency missing: '{}' must be installed before '{}'",
                dep,
                app_name
            );
        }
    }

    Ok(())
}

/// Record an installed app in the `installed_app` table.
async fn record_installed_app(
    db: &DbAdapter,
    app_name: &str,
    version: &str,
    app_path: Option<&str>,
    manifest: Option<&Value>,
) {
    let row = json!({
        "name":         app_name,
        "version":      version,
        "app_path":     app_path,
        "manifest":     manifest,
    });
    match upsert_doc(db, "installed_app", app_name, &row).await {
        Ok(_) => tracing::debug!(app = %app_name, "Recorded in installed_app"),
        Err(e) => tracing::warn!(app = %app_name, error = %e, "Could not write to installed_app"),
    }
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

pub async fn install_app(args: InstallAppArgs) -> Result<()> {
    let bench = args
        .bench
        .canonicalize()
        .unwrap_or_else(|_| args.bench.clone());

    let db = connect_to_site(&bench, &args.site).await?;

    let requested = PathBuf::from(&args.app);
    let installed_name = if requested
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| s.eq_ignore_ascii_case("slpkg"))
        .unwrap_or(false)
    {
        install_slpkg_into_db(&db, &requested, args.version.as_deref()).await?
    } else {
        install_app_into_db(&db, &bench, &args.app, args.version.as_deref()).await?;
        args.app.clone()
    };

    println!(
        "\n✓  App '{}' installed into site '{}'.",
        installed_name, args.site
    );
    Ok(())
}

fn read_manifest_from_outer_app_dir(app_outer_dir: &Path) -> Result<Value> {
    let path = app_outer_dir.join("app.json");
    let raw = std::fs::read_to_string(&path)
        .with_context(|| format!("Reading {}", path.display()))?;
    let manifest: Value = serde_json::from_str(&raw)
        .with_context(|| format!("Parsing {}", path.display()))?;
    Ok(manifest)
}

fn extract_app_name_from_manifest(manifest: &Value) -> Result<String> {
    let app_name = manifest
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim()
        .to_owned();
    if app_name.is_empty() {
        anyhow::bail!("Invalid app.json: missing non-empty 'name'");
    }
    Ok(app_name)
}

fn app_version_from_manifest(manifest: &Value, cli_version: Option<&str>) -> String {
    cli_version
        .map(|s| s.to_owned())
        .or_else(|| {
            manifest
                .get("version")
                .and_then(|v| v.as_str())
                .map(|s| s.to_owned())
        })
        .unwrap_or_else(|| "unknown".to_owned())
}

async fn install_app_from_resolved_source(
    db: &DbAdapter,
    app_name: &str,
    app_root: &Path,
    manifest: Option<Value>,
    version: String,
    app_path_for_tracking: Option<&str>,
) -> Result<()> {
    enforce_manifest_dependencies(db, app_name, manifest.as_ref()).await?;

    if !app_root.exists() {
        anyhow::bail!("App root not found: {}", app_root.display());
    }

    // ── 0. Upsert app graph node ──────────────────────────────────────────────
    println!("\n[0/5] Registering app node for '{}' (v{}) …", app_name, version);
    upsert_app_node(db, app_name, None, Some(&version), None)
        .await
        .context("Upserting app graph node")?;

    // ── 1. DocType definitions ────────────────────────────────────────────────
    println!("\n[1/5] Seeding DocType definitions for '{}' …", app_name);
    let (dt_seeded, dt_errors) = seed_doctypes_for_app(db, app_root, app_name).await?;
    println!("      {} DocTypes seeded, {} errors.", dt_seeded, dt_errors);
    if dt_errors > 0 {
        anyhow::bail!("{} DocType(s) failed — check output above", dt_errors);
    }

    // ── 2. Module Def records from modules.txt ────────────────────────────────
    println!("\n[2/5] Seeding Module Def records …");
    let mod_count = seed_module_defs(db, app_root, app_name).await?;
    println!("      {} Module Defs seeded.", mod_count);

    // ── 3. Importable fixture records (Workspace, Page, Report, …) ───────────
    println!("\n[3/5] Seeding importable fixture records …");
    let fixture_count = seed_fixture_records(db, app_root).await?;
    println!("      {} fixture records seeded.", fixture_count);

    // ── 3b. Generic module fixtures (countries, currencies, …) ───────────────
    match seed_module_fixtures(db, app_root).await {
        Ok(n) if n > 0 => println!("      {} module fixture records seeded.", n),
        Ok(_) => {}
        Err(e) => eprintln!("WARN: module fixture seeding failed (non-fatal): {:?}", e),
    }

    // ── 4. Patch Log — mark all patches as completed ──────────────────────────
    println!("\n[4/5] Marking patches as completed …");
    let patch_count = seed_patch_log(db, app_root).await?;
    println!("      {} patches marked done.", patch_count);

    // ── 5. Pipeline framework — fn:: files + Tier A wiring ──────────────────
    println!("\n[5/5] Applying pipeline fn:: functions …");
    match seed_pipeline_for_app(db, app_root).await {
        Ok(fn_count) => println!("      {} fn:: registered.", fn_count),
        Err(e) => eprintln!("WARN: pipeline seeding failed (non-fatal): {:?}", e),
    }

    // ── Record in installed_app ───────────────────────────────────────────────
    record_installed_app(db, app_name, &version, app_path_for_tracking, manifest.as_ref()).await;

    // ── Post-install: link check ──────────────────────────────────────────────
    post_install_link_check(db).await;

    Ok(())
}

fn find_packaged_app_outer_dir(unpack_root: &Path) -> Result<PathBuf> {
    let entries = std::fs::read_dir(unpack_root)
        .with_context(|| format!("Reading {}", unpack_root.display()))?;
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() && path.join("app.json").exists() {
            return Ok(path);
        }
    }
    anyhow::bail!("Invalid .slpkg: expected one top-level app directory with app.json");
}

async fn install_slpkg_into_db(
    db: &DbAdapter,
    slpkg_path: &Path,
    cli_version: Option<&str>,
) -> Result<String> {
    let archive_path = slpkg_path
        .canonicalize()
        .with_context(|| format!("Archive not found: {}", slpkg_path.display()))?;

    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let unpack_root = std::env::temp_dir().join(format!("spotledger-slpkg-{}", unique));
    std::fs::create_dir_all(&unpack_root)
        .with_context(|| format!("Creating {}", unpack_root.display()))?;

    let file = std::fs::File::open(&archive_path)
        .with_context(|| format!("Opening {}", archive_path.display()))?;
    let gz = GzDecoder::new(file);
    let mut tar = Archive::new(gz);
    tar.unpack(&unpack_root)
        .with_context(|| format!("Unpacking {}", archive_path.display()))?;

    let app_outer = find_packaged_app_outer_dir(&unpack_root)?;
    let manifest = read_manifest_from_outer_app_dir(&app_outer)?;
    let app_name = extract_app_name_from_manifest(&manifest)?;
    let app_root = app_outer.join(&app_name);
    let version = app_version_from_manifest(&manifest, cli_version);

    install_app_from_resolved_source(db, &app_name, &app_root, Some(manifest), version, None)
        .await
        .with_context(|| format!("Installing app from {}", archive_path.display()))?;

    Ok(app_name)
}

/// Install an app into an already-connected database.
///
/// This is the inner implementation called by both the CLI `install-app` command
/// and by `new-site`'s auto-install step.
pub async fn install_app_into_db(
    db: &DbAdapter,
    bench: &std::path::Path,
    app_name: &str,
    cli_version: Option<&str>,
) -> Result<()> {
    // ── Read app.json manifest if present ─────────────────────────────────────
    let manifest = read_app_manifest(bench, app_name).await;
    let version = manifest
        .as_ref()
        .map(|m| app_version_from_manifest(m, cli_version))
        .unwrap_or_else(|| cli_version.unwrap_or("unknown").to_owned());

    let app_root = bench.join("apps").join(app_name).join(app_name);
    let app_path_str = app_root.to_string_lossy().to_string();
    install_app_from_resolved_source(
        db,
        app_name,
        &app_root,
        manifest,
        version,
        Some(&app_path_str),
    )
    .await
}

// ---------------------------------------------------------------------------
// Post-install graph health check
// ---------------------------------------------------------------------------

/// Query SurrealDB graph for Link fields that reference a DocType not yet installed.
/// Prints warnings — never fails the install.
async fn post_install_link_check(db: &DbAdapter) {
    // Query tabDocField (already populated by seed_doctypes_for_app) directly.
    // No graph node tables needed — tabDocType is the source of truth for
    // whether a DocType is installed.
    let query = "
        SELECT
            parent AS source_doctype,
            fieldname,
            options AS links_to
        FROM tabDocField
        WHERE fieldtype = 'Link'
          AND options != NONE
          AND options != ''
          AND NOT (SELECT 1 FROM tabDocType WHERE name = $parent.options LIMIT 1)
    ";

    match db.run(query, vec![]).await {
        Ok(rows) => {
            if rows.is_empty() {
                println!("\n✓  Dependency check: all Link fields resolved.");
                return;
            }
            println!("\n⚠  Dependency check: {} unresolved Link field(s):", rows.len());
            for row in &rows {
                let src    = row.get("source_doctype").and_then(|v| v.as_str()).unwrap_or("?");
                let field  = row.get("fieldname").and_then(|v| v.as_str()).unwrap_or("?");
                let target = row.get("links_to").and_then(|v| v.as_str()).unwrap_or("?");
                println!("   {}.{} → '{}' (DocType not installed)", src, field, target);
            }
            println!("   Install the app that provides the missing DocTypes to resolve.");
        }
        Err(e) => {
            eprintln!("WARN: post-install link check failed: {}", e);
        }
    }
}

// ---------------------------------------------------------------------------
// Step helpers
// ---------------------------------------------------------------------------

async fn connect_to_site(bench: &Path, site: &str) -> Result<DbAdapter> {
    let config_path = bench.join("sites").join(site).join("site_config.toml");
    let config_str = tokio::fs::read_to_string(&config_path)
        .await
        .with_context(|| format!("Reading {}", config_path.display()))?;
    let cfg: SiteConfig =
        toml::from_str(&config_str).context("Parsing site_config.toml")?;
    println!(
        "Connecting to {} (ns={}) …",
        cfg.database.url, cfg.database.ns
    );
    let db = connect(&cfg.database)
        .await
        .context("Connecting to SurrealDB")?;
    println!("Connected.");
    Ok(db)
}

async fn seed_module_defs(db: &DbAdapter, app_root: &Path, app_name: &str) -> Result<usize> {
    let modules_txt = app_root.join("modules.txt");
    let content = match tokio::fs::read_to_string(&modules_txt).await {
        Ok(s) => s,
        Err(_) => {
            println!("  (no modules.txt — skipping)");
            return Ok(0);
        }
    };
    let mut count = 0usize;
    for line in content.lines() {
        let module_name = line.trim();
        if module_name.is_empty() || module_name.starts_with('#') {
            continue;
        }
        let row = json!({
            "name":        module_name,
            "doctype":     "Module Def",
            "module_name": module_name,
            "app_name":    app_name,
            "label":       module_name,
            "show_in_menu": 1,
            "order":       count as i64,
            "owner":       "Administrator",
            "modified_by": "Administrator",
        });
        match upsert_doc(db, "Module Def", module_name, &row).await {
            Ok(_) => count += 1,
            Err(e) => eprintln!("WARN: Module Def '{}': {}", module_name, e),
        }
    }
    Ok(count)
}

// ── Phase C: app/module/doctype graph edges ────────────────────────────────────

/// Ensure that all modules listed in `modules.txt` have graph nodes and edges:
///  `app -[provides_module]-> module -[contains]-> doctype`
///
/// Reads the doctype JSON files to discover which DocTypes belong to each module.
/// Returns the total number of edges created.
async fn seed_app_module_edges(
    db:       &DbAdapter,
    app_root: &Path,
    app_name: &str,
) -> Result<usize> {
    use crate::seed_doctypes::collect_doctype_jsons_pub;

    let json_files = collect_doctype_jsons_pub(app_root)?;
    let mut edges = 0usize;

    for path in &json_files {
        let raw = match tokio::fs::read_to_string(path).await {
            Ok(r) => r,
            Err(_) => continue,
        };
        let doc: Value = match serde_json::from_str(&raw) {
            Ok(v) => v,
            Err(_) => continue,
        };

        let doctype_name = match doc.get("name").and_then(Value::as_str) {
            Some(n) => n.to_owned(),
            None => continue,
        };

        if doc.get("doctype").and_then(Value::as_str) != Some("DocType") {
            continue;
        }

        let module_name = match doc.get("module").and_then(Value::as_str) {
            Some(m) if !m.is_empty() => m.to_owned(),
            _ => continue,
        };

        // Ensure module graph node + app-[provides_module]-> module edge
        if let Err(e) = upsert_module_node(db, &module_name, None, app_name).await {
            eprintln!("WARN: module node '{}': {}", module_name, e);
        } else {
            edges += 1;
        }

        // Ensure module-[contains]-> doctype edge
        if let Err(e) = relate_module_contains_doctype(db, &module_name, &doctype_name).await {
            eprintln!("WARN: module-contains-doctype edge for '{}': {}", doctype_name, e);
        } else {
            edges += 1;
        }
    }

    Ok(edges)
}

/// Upsert docfield graph nodes + has_field edges and write schema_change_log entries
/// for every field that was added or modified in this install/upgrade run.
///
/// Returns the total number of change log entries written.
async fn seed_docfield_graph(
    db:          &DbAdapter,
    app_root:    &Path,
    app_name:    &str,
    app_version: &str,
) -> Result<usize> {
    use crate::seed_doctypes::collect_doctype_jsons_pub;

    let json_files = collect_doctype_jsons_pub(app_root)?;
    let mut changes = 0usize;

    for path in &json_files {
        let raw = match tokio::fs::read_to_string(path).await {
            Ok(r) => r,
            Err(_) => continue,
        };
        let doc: Value = match serde_json::from_str(&raw) {
            Ok(v) => v,
            Err(_) => continue,
        };

        if doc.get("doctype").and_then(Value::as_str) != Some("DocType") {
            continue;
        }

        let doctype_name = match doc.get("name").and_then(Value::as_str) {
            Some(n) => n.to_owned(),
            None => continue,
        };

        let fields = match doc.get("fields").and_then(Value::as_array) {
            Some(f) => f,
            None => continue,
        };

        // Skip pure layout fields (Section Break, Column Break, Tab Break)
        const LAYOUT_TYPES: &[&str] = &[
            "Section Break", "Column Break", "Tab Break", "HTML",
        ];

        for (idx, field) in fields.iter().enumerate() {
            let fieldtype = field
                .get("fieldtype")
                .and_then(Value::as_str)
                .unwrap_or("Data");

            if LAYOUT_TYPES.contains(&fieldtype) {
                continue;
            }

            let fieldname = match field.get("fieldname").and_then(Value::as_str) {
                Some(f) if !f.is_empty() => f,
                _ => continue,
            };

            match upsert_docfield_graph(
                db,
                &doctype_name,
                fieldname,
                field,
                idx,
                app_name,
                Some(app_version),
                false, // is_custom = false for app-seeded fields
            )
            .await
            {
                Ok(is_new) if is_new => {
                    let target_name = format!("{}-{}", doctype_name, fieldname);
                    if let Err(e) = log_schema_change(
                        db,
                        "DocField",
                        &target_name,
                        "added",
                        app_name,
                        app_version,
                        Some(json!({
                            "after": field,
                        })),
                    )
                    .await
                    {
                        eprintln!("WARN: schema_change_log '{}': {}", target_name, e);
                    } else {
                        changes += 1;
                    }
                }
                Ok(_) => {} // existing edge — idx may have been updated, no log needed
                Err(e) => {
                    eprintln!(
                        "WARN: docfield graph '{}-{}': {}",
                        doctype_name, fieldname, e
                    );
                }
            }
        }
    }

    Ok(changes)
}

/// Seed all importable fixture types (Workspace, Page, Report, …) for the app.
///
/// Walk pattern: `app_root/{module}/{dir_name}/{record_name}/{record_name}.json`
/// This is the Frappe `get_doc_files()` pattern from `frappe/model/sync.py`.
async fn seed_fixture_records(db: &DbAdapter, app_root: &Path) -> Result<usize> {
    let mut total = 0usize;

    for &(dir_name, doctype_name, child_fields) in IMPORTABLE_TYPES {
        let files = collect_fixture_jsons(app_root, dir_name);
        if files.is_empty() {
            continue;
        }

        let mut count = 0usize;
        for path in &files {
            let raw = match tokio::fs::read_to_string(path).await {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("WARN: read {}: {}", path.display(), e);
                    continue;
                }
            };
            let doc: Value = match serde_json::from_str(&raw) {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("WARN: parse {}: {}", path.display(), e);
                    continue;
                }
            };
            let record_name = match doc.get("name").and_then(Value::as_str) {
                Some(n) => n.to_owned(),
                None => {
                    eprintln!("WARN: no 'name' in {}", path.display());
                    continue;
                }
            };

            // Upsert top-level record, stripping known child-array fields.
            let child_keys: Vec<&str> = child_fields.iter().map(|(k, _)| *k).collect();
            let top = build_top_level(doc.clone(), doctype_name, &child_keys);
            if let Err(e) = upsert_doc(db, doctype_name, &record_name, &top).await {
                eprintln!("ERROR: {} '{}': {}", doctype_name, record_name, e);
                continue;
            }
            count += 1;

            // Upsert child records with deterministic names.
            for &(parentfield, child_doctype) in child_fields {
                if let Some(Value::Array(items)) = doc.get(parentfield) {
                    for (idx, item) in items.iter().enumerate() {
                        let child_name =
                            format!("{}-{}-{}", record_name, parentfield, idx);
                        let child_row = build_child_row(
                            item,
                            &child_name,
                            &record_name,
                            doctype_name,   // parenttype
                            child_doctype,
                            parentfield,
                            idx,
                        );
                        if let Err(e) =
                            upsert_doc(db, child_doctype, &child_name, &child_row).await
                        {
                            eprintln!(
                                "WARN: {} '{}': {}",
                                child_doctype, child_name, e
                            );
                        }
                    }
                }
            }
        }

        if count > 0 {
            println!("  {}: {} records", doctype_name, count);
        }
        total += count;
    }
    Ok(total)
}

/// Seed Patch Log records from `{app_root}/patches.txt`.
///
/// All patches are recorded as already completed — equivalent to Frappe's
/// `set_all_patches_as_completed()` which inserts a `Patch Log` row for
/// every patch in patches.txt so they are never re-executed.
///
/// patches.txt uses an INI-like format with `[pre_model_sync]` / `[post_model_sync]`
/// sections; comments start with `#`.
async fn seed_patch_log(db: &DbAdapter, app_root: &Path) -> Result<usize> {
    let patches_txt = app_root.join("patches.txt");
    let content = match tokio::fs::read_to_string(&patches_txt).await {
        Ok(s) => s,
        Err(_) => {
            println!("  (no patches.txt — skipping)");
            return Ok(0);
        }
    };
    let mut count = 0usize;
    for line in content.lines() {
        let trimmed = line.trim();
        // Skip section headers, empty lines, and lines that are pure comments.
        if trimmed.is_empty() || trimmed.starts_with('[') || trimmed.starts_with('#') {
            continue;
        }
        // Strip inline comment (everything after `#`), then trim.
        let patch_name = trimmed
            .split('#')
            .next()
            .unwrap_or(trimmed)
            .trim()
            .to_owned();
        if patch_name.is_empty() {
            continue;
        }
        let row = json!({
            "name":        &patch_name,
            "doctype":     "Patch Log",
            "patch":       &patch_name,
            "owner":       "Administrator",
            "modified_by": "Administrator",
        });
        match upsert_doc(db, "Patch Log", &patch_name, &row).await {
            Ok(_) => count += 1,
            Err(e) => eprintln!("WARN: Patch Log '{}': {}", patch_name, e),
        }
    }
    Ok(count)
}

// ---------------------------------------------------------------------------
// Filesystem helpers
// ---------------------------------------------------------------------------

/// Walk `app_root/{module}/{dir_name}/{name}/{name}.json`.
///
/// This is the canonical Frappe fixture layout used by `get_doc_files()` in
/// `frappe/model/sync.py`.
fn collect_fixture_jsons(app_root: &Path, dir_name: &str) -> Vec<PathBuf> {
    let mut result = Vec::new();
    let Ok(module_iter) = std::fs::read_dir(app_root) else {
        return result;
    };
    for module_entry in module_iter.flatten() {
        let module_dir = module_entry.path();
        if !module_dir.is_dir() {
            continue;
        }
        let type_dir = module_dir.join(dir_name);
        if !type_dir.is_dir() {
            continue;
        }
        let Ok(item_iter) = std::fs::read_dir(&type_dir) else {
            continue;
        };
        for item_entry in item_iter.flatten() {
            let item_dir = item_entry.path();
            if !item_dir.is_dir() {
                continue;
            }
            let basename = item_dir
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            let json_path = item_dir.join(format!("{}.json", basename));
            if json_path.exists() {
                result.push(json_path);
            }
        }
    }
    result.sort();
    result
}

/// Build the top-level record value, stripping child-array fields.
fn build_top_level(mut doc: Value, doctype_name: &str, child_field_keys: &[&str]) -> Value {
    if let Value::Object(ref mut map) = doc {
        for key in child_field_keys {
            map.remove(*key);
        }
        map.insert("doctype".to_owned(), json!(doctype_name));
        map.entry("owner".to_owned())
            .or_insert_with(|| json!("Administrator"));
        map.entry("modified_by".to_owned())
            .or_insert_with(|| json!("Administrator"));
    }
    doc
}

// ---------------------------------------------------------------------------
// Step 5: pipeline seeding
// ---------------------------------------------------------------------------

/// Apply pipeline fn:: files and wiring.surql files to SurrealDB.
///
/// Order: schema DDL → shared fn:: → domain functions → registry → runner →
///        universal nodes → wire_generic helper → domain wiring.
/// All wiring lives in surql/doctypes/*/wiring.surql — no Rust wiring code.
async fn seed_pipeline_for_app(db: &DbAdapter, app_root: &Path) -> Result<usize> {
    apply_pipeline_functions(db, app_root)
        .await
        .context("Applying pipeline surql files")
}

// ---------------------------------------------------------------------------
// Module-level generic fixtures
// ---------------------------------------------------------------------------

/// Seed generic fixture records from `{app_root}/{module}/fixtures/{DocType}.json`.
///
/// Each file is a JSON array of records. The filename (without extension) is used
/// as the DocType name (title-cased). Example:
///
///   `geo/fixtures/Country.json`  → array of Country records upserted to tabCountry
///
/// Records must each have a `"name"` field.
async fn seed_module_fixtures(db: &DbAdapter, app_root: &Path) -> Result<usize> {
    let mut total = 0usize;
    let Ok(module_iter) = std::fs::read_dir(app_root) else {
        return Ok(0);
    };
    for module_entry in module_iter.flatten() {
        let module_dir = module_entry.path();
        if !module_dir.is_dir() {
            continue;
        }
        let fixtures_dir = module_dir.join("fixtures");
        if !fixtures_dir.is_dir() {
            continue;
        }
        let Ok(files_iter) = std::fs::read_dir(&fixtures_dir) else {
            continue;
        };
        for file_entry in files_iter.flatten() {
            let path = file_entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("json") {
                continue;
            }
            // DocType name is the filename stem (preserving case)
            let doctype = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_owned();
            if doctype.is_empty() {
                continue;
            }

            let raw = match tokio::fs::read_to_string(&path).await {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("WARN: read {}: {}", path.display(), e);
                    continue;
                }
            };
            let records: Vec<Value> = match serde_json::from_str(&raw) {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("WARN: parse {}: {}", path.display(), e);
                    continue;
                }
            };

            let mut count = 0usize;
            for mut record in records {
                let name = match record.get("name").and_then(Value::as_str) {
                    Some(n) if !n.is_empty() => n.to_owned(),
                    _ => {
                        eprintln!("WARN: fixture record in {} missing 'name', skipping", path.display());
                        continue;
                    }
                };
                // Stamp system fields if absent
                if let Value::Object(ref mut map) = record {
                    map.entry("owner".to_owned())
                        .or_insert_with(|| json!("Administrator"));
                    map.entry("modified_by".to_owned())
                        .or_insert_with(|| json!("Administrator"));
                    map.entry("docstatus".to_owned())
                        .or_insert_with(|| json!(0));
                }
                match upsert_doc(db, &doctype, &name, &record).await {
                    Ok(_) => count += 1,
                    Err(e) => eprintln!("WARN: {} '{}': {}", doctype, name, e),
                }
            }
            if count > 0 {
                println!("      {} {} fixture record(s) from {}", count, doctype, path.file_name().unwrap_or_default().to_string_lossy());
            }
            total += count;
        }
    }
    Ok(total)
}
