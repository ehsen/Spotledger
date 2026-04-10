//! Graph operations — upsert app/module nodes and their edges to DocTypes.
//!
//! These functions are called by `install_app` during app installation and
//! upgrade.  All operations are idempotent: re-running them adds only what is
//! missing and never removes existing edges or nodes.
//!
//! ## Graph model
//!
//! ```text
//! app:erpnext -[provides_module]-> module:Accounts
//! module:Accounts -[contains]-> doctype:account
//! doctype:account -[has_field { introduced_by:"erpnext", idx:0 }]-> docfield:account_account_name
//! ```

use serde_json::{json, Value};

use crate::adapter::DbAdapter;
use crate::error::DbError;

// ── App node ─────────────────────────────────────────────────────────────────

/// Upsert the `app` graph node.  On upgrade, updates `version` and `updated_at`.
pub async fn upsert_app_node(
    adapter:  &DbAdapter,
    app_name: &str,
    title:    Option<&str>,
    version:  Option<&str>,
    logo_url: Option<&str>,
) -> Result<(), DbError> {
    let id = graph_id(app_name);
    let sql = format!(
        "UPSERT app:{id} MERGE $patch;",
    );
    let patch = json!({
        "name":       app_name,
        "title":      title,
        "version":    version,
        "logo_url":   logo_url,
        "updated_at": surql_now(),
    });
    adapter.execute(&sql, vec![("patch".into(), patch)]).await
}

// ── Module node ───────────────────────────────────────────────────────────────

/// Upsert a `module` graph node and RELATE it to its owning `app`.
pub async fn upsert_module_node(
    adapter:     &DbAdapter,
    module_name: &str,
    label:       Option<&str>,
    app_name:    &str,
) -> Result<(), DbError> {
    let m_id  = graph_id(module_name);
    let a_id  = graph_id(app_name);

    // Upsert the module node
    adapter
        .execute(
            &format!("UPSERT module:{m_id} MERGE $patch;"),
            vec![(
                "patch".into(),
                json!({
                    "name":  module_name,
                    "label": label,
                    "app":   app_name,
                }),
            )],
        )
        .await?;

    // RELATE app -[provides_module]-> module (skip if edge already exists)
    adapter
        .execute(
            &format!(
                "IF NOT (SELECT * FROM provides_module WHERE in = app:{a_id} AND out = module:{m_id}) THEN \
                   RELATE app:{a_id} -> provides_module -> module:{m_id}; \
                 END;"
            ),
            vec![],
        )
        .await?;

    Ok(())
}

// ── module -[contains]-> doctype edge ─────────────────────────────────────────

/// Ensure a `module -[contains]-> doctype` edge exists.
pub async fn relate_module_contains_doctype(
    adapter:     &DbAdapter,
    module_name: &str,
    doctype:     &str,
) -> Result<(), DbError> {
    let m_id = graph_id(module_name);
    let dt_id = graph_id(doctype);

    // Ensure the doctype node exists first (idempotent no-op if already there)
    adapter
        .execute(
            &format!(
                "IF NOT (SELECT * FROM doctype WHERE name = $name) THEN \
                   UPSERT doctype:{dt_id} SET name = $name; \
                 END;"
            ),
            vec![("name".into(), json!(doctype))],
        )
        .await?;

    // Relate
    adapter
        .execute(
            &format!(
                "IF NOT (SELECT * FROM contains WHERE in = module:{m_id} AND out = doctype:{dt_id}) THEN \
                   RELATE module:{m_id} -> contains -> doctype:{dt_id}; \
                 END;"
            ),
            vec![],
        )
        .await?;

    Ok(())
}

// ── schema_change_log ─────────────────────────────────────────────────────────

/// Write one entry to the `schema_change_log` audit table.
///
/// `change_type` should be `"added"`, `"modified"`, or `"removed"`.
/// `diff` is a before/after snapshot as a JSON object.
pub async fn log_schema_change(
    adapter:     &DbAdapter,
    target_type: &str,  // "DocType" | "DocField"
    target_name: &str,
    change_type: &str,
    app:         &str,
    app_version: &str,
    diff:        Option<Value>,
) -> Result<(), DbError> {
    let sql = "INSERT INTO schema_change_log { \
                 target_type: $tt, target_name: $tn, change_type: $ct, \
                 app: $app, app_version: $av, diff: $diff \
               };";
    adapter
        .execute(
            sql,
            vec![
                ("tt".into(),   json!(target_type)),
                ("tn".into(),   json!(target_name)),
                ("ct".into(),   json!(change_type)),
                ("app".into(),  json!(app)),
                ("av".into(),   json!(app_version)),
                ("diff".into(), diff.unwrap_or(Value::Null)),
            ],
        )
        .await
}

// ── docfield graph node + has_field edge (called by install_app) ──────────────

/// Upsert a `docfield` graph node from a JSON DocField record and RELATE it to
/// its parent `doctype` node with a `has_field` edge carrying provenance metadata.
///
/// Returns `true` when the edge was newly created, `false` when it already existed.
pub async fn upsert_docfield_graph(
    adapter:             &DbAdapter,
    doctype:             &str,
    fieldname:           &str,
    field_json:          &Value,
    idx:                 usize,
    introduced_by:       &str,
    introduced_version:  Option<&str>,
    is_custom:           bool,
) -> Result<bool, DbError> {
    let dt_id = graph_id(doctype);
    let df_id = format!("{}_{}", dt_id, fieldname.to_lowercase());

    // Upsert docfield graph node
    let df_content = json!({
        "name":             format!("{}_{}", doctype, fieldname),
        "fieldname":        fieldname,
        "label":            field_json.get("label").cloned().unwrap_or(Value::Null),
        "fieldtype":        field_json.get("fieldtype").cloned().unwrap_or(Value::Null),
        "options":          field_json.get("options").cloned().unwrap_or(Value::Null),
        "reqd":             field_json.get("reqd").cloned().unwrap_or(json!(0)),
        "unique":           field_json.get("unique").cloned().unwrap_or(json!(0)),
        "read_only":        field_json.get("read_only").cloned().unwrap_or(json!(0)),
        "hidden":           field_json.get("hidden").cloned().unwrap_or(json!(0)),
        "in_list_view":     field_json.get("in_list_view").cloned().unwrap_or(json!(0)),
        "in_standard_filter": field_json.get("in_standard_filter").cloned().unwrap_or(json!(0)),
        "bold":             field_json.get("bold").cloned().unwrap_or(json!(0)),
        "description":      field_json.get("description").cloned().unwrap_or(Value::Null),
        "default_value":    field_json.get("default_value").cloned().unwrap_or(Value::Null),
    });

    adapter
        .execute(
            &format!("UPSERT docfield:{df_id} MERGE $content;"),
            vec![("content".into(), df_content)],
        )
        .await?;

    // Check if edge already exists to know whether to log "added"
    let existing = adapter
        .run(
            &format!(
                "SELECT * FROM has_field WHERE in = doctype:{dt_id} AND out = docfield:{df_id}"
            ),
            vec![],
        )
        .await?;

    let is_new_edge = existing.is_empty();

    if is_new_edge {
        let edge_content = json!({
            "idx":                    idx,
            "introduced_by":          introduced_by,
            "introduced_version":     introduced_version,
            "is_custom":              if is_custom { 1 } else { 0 },
            "protected":              if is_custom { 0 } else { 1 },
            "removable_on_uninstall": if is_custom { 0 } else { 1 },
        });

        adapter
            .execute(
                &format!(
                    "RELATE doctype:{dt_id} -> has_field -> docfield:{df_id} CONTENT $edge;"
                ),
                vec![("edge".into(), edge_content)],
            )
            .await?;
    } else {
        // Edge exists — update idx if it changed
        adapter
            .execute(
                &format!(
                    "UPDATE has_field SET idx = $idx \
                     WHERE in = doctype:{dt_id} AND out = docfield:{df_id};"
                ),
                vec![("idx".into(), json!(idx))],
            )
            .await?;
    }

    Ok(is_new_edge)
}

// ── helpers ───────────────────────────────────────────────────────────────────

/// Convert a human-readable name to a SurrealDB record id component.
/// Lowercase, spaces and hyphens replaced with underscores.
pub fn graph_id(name: &str) -> String {
    name.to_lowercase().replace([' ', '-'], "_")
}

/// Return the current UTC time as an ISO-8601 string for use in SurrealDB
/// content clauses.  SurrealDB will coerce this to a `datetime` value.
fn surql_now() -> String {
    chrono::Utc::now()
        .format("%Y-%m-%dT%H:%M:%S%.3fZ")
        .to_string()
}
