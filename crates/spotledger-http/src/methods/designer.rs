//! DocType Designer API handlers.
//!
//! Seven methods exposed via `POST /api/method/spotledger.designer.*`:
//!
//! | Method                              | Handler                    | Purpose                                    |
//! |-------------------------------------|----------------------------|--------------------------------------------|
//! | `spotledger.designer.get_meta`      | `handle_get_meta`          | Load `tabDocType` + `tabDocField` + children |
//! | `spotledger.designer.save`          | `handle_save`              | Upsert `tabDocType` + full-replace `tabDocField` + DDL |
//! | `spotledger.designer.generate_surql`| `handle_generate_surql`    | Preview SurrealQL — no write               |
//! | `spotledger.designer.get_pipeline`  | `handle_get_pipeline`      | Load stages + nodes + fn_source            |
//! | `spotledger.designer.save_function` | `handle_save_function`     | Upsert `fn_source` + wire `pipeline_node` |
//! | `spotledger.designer.delete_function`| `handle_delete_function`  | Remove node + `REMOVE FUNCTION`            |
//! | `spotledger.designer.reorder_functions`| `handle_reorder_functions`| Bulk-update `has_node.ord`               |

use crate::state::SiteState;
use serde_json::{json, Value};
use spotledger_core::error::SpotError;
use spotledger_db::apply_surql::generate_registry_surql;
use spotledger_db::document::doctype_to_table;
use spotledger_db::schema::apply_pipeline_bootstrap;
use spotledger_db::{
    parse_docfield_from_value, parse_docperm_from_value,
    save_doctype, DoctypeSaveError, DoctypeSaveInput,
};
use std::collections::HashMap;
use std::sync::Arc;

// ── Tier-0 guard ─────────────────────────────────────────────────────────────

/// DocTypes that are compiled into Rust and must never be modified via the designer.
const TIER_0_DOCTYPES: &[&str] = &[
    "DocType",
    "DocField",
    "DocPerm",
    "User",
    "Role",
    "UserPermission",
    "Site",
    "ModuleDef",
];

fn is_tier_0(doctype: &str) -> bool {
    TIER_0_DOCTYPES.iter().any(|&t| t.eq_ignore_ascii_case(doctype))
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn require_str<'a>(params: &'a HashMap<String, Value>, key: &str) -> Result<&'a str, SpotError> {
    params
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| SpotError::Validation(format!("missing required param: `{key}`")))
}

fn require_string(params: &HashMap<String, Value>, key: &str) -> Result<String, SpotError> {
    Ok(require_str(params, key)?.to_owned())
}

// ── spotledger.designer.get_meta ─────────────────────────────────────────────

/// Load the `tabDocType` record and all `tabDocField` rows for `doctype`.
/// Also returns the names of any referenced child doctypes so the UI can load
/// them separately.
///
/// Parameters: `{ "doctype": "<Name>" }`
pub async fn handle_get_meta(
    site: Arc<SiteState>,
    params: HashMap<String, Value>,
) -> Result<Value, SpotError> {
    let doctype = require_string(&params, "doctype")?;

    // Fetch the doctype record
    let dt_rows = site
        .db
        .run(
            "SELECT * FROM tabDocType WHERE name = $dt LIMIT 1",
            vec![("dt".into(), Value::String(doctype.clone()))],
        )
        .await
        .map_err(|e| SpotError::Db(e.to_string()))?;

    let doctype_record = dt_rows.into_iter().next().ok_or_else(|| {
        SpotError::NotFound { doctype: "DocType".into(), name: doctype.clone() }
    })?;

    // Fetch all fields for this doctype ordered by idx
    let field_rows = site
        .db
        .run(
            "SELECT * FROM tabDocField WHERE parent = $dt ORDER BY idx ASC",
            vec![("dt".into(), Value::String(doctype.clone()))],
        )
        .await
        .map_err(|e| SpotError::Db(e.to_string()))?;

    // Collect child-table doctype names from Table-type fields
    let child_doctypes: Vec<String> = field_rows
        .iter()
        .filter(|f| {
            f.get("fieldtype")
                .and_then(Value::as_str)
                .map(|ft| ft.eq_ignore_ascii_case("Table") || ft.eq_ignore_ascii_case("Table MultiSelect"))
                .unwrap_or(false)
        })
        .filter_map(|f| f.get("options").and_then(Value::as_str).map(str::to_owned))
        .filter(|s| !s.is_empty())
        .collect();

    // Fetch DocPerm rows for this doctype
    let perm_rows = site
        .db
        .run(
            "SELECT * FROM tabDocPerm WHERE parent = $dt AND parenttype = 'DocType'",
            vec![("dt".into(), Value::String(doctype.clone()))],
        )
        .await
        .unwrap_or_default();

    Ok(json!({
        "doctype": doctype_record,
        "fields": field_rows,
        "permissions": perm_rows,
        "child_doctypes": child_doctypes,
        "is_tier_0": is_tier_0(&doctype),
    }))
}

// ── spotledger.designer.save ──────────────────────────────────────────────────

/// Orchestrated DocType save — validates, persists DML in a transaction,
/// applies DDL, and invalidates the meta cache.
///
/// Parameters:
/// ```json
/// {
///   "doctype": "...",
///   "meta":    { ...tabDocType scalar fields... },
///   "fields":  [ { ...DocField attrs... }, ... ],
///   "perms":   [ { ...DocPerm attrs...  }, ... ]   (optional)
/// }
/// ```
pub async fn handle_save(
    site: Arc<SiteState>,
    params: HashMap<String, Value>,
) -> Result<Value, SpotError> {
    let doctype = require_string(&params, "doctype")?;

    // Guard: reject Tier-0 compiled types
    if is_tier_0(&doctype) {
        return Err(SpotError::Validation(format!(
            "DocType `{doctype}` is a Tier-0 compiled type and cannot be modified via the designer."
        )));
    }

    // Parse meta scalars from the "meta" sub-object
    let meta_obj = params
        .get("meta")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();

    let module = meta_obj
        .get("module")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_owned();

    let autoname = meta_obj
        .get("autoname")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_owned();

    fn bool_meta(obj: &serde_json::Map<String, Value>, key: &str) -> bool {
        match obj.get(key) {
            Some(Value::Bool(b)) => *b,
            Some(Value::Number(n)) => n.as_u64().unwrap_or(0) != 0,
            _ => false,
        }
    }

    let is_child       = bool_meta(&meta_obj, "is_child_table")
                         || bool_meta(&meta_obj, "istable");
    let is_single      = bool_meta(&meta_obj, "issingle");
    let is_submittable = bool_meta(&meta_obj, "issubmittable")
                         || bool_meta(&meta_obj, "is_submittable");
    let is_tree        = bool_meta(&meta_obj, "is_tree");
    let custom         = bool_meta(&meta_obj, "custom");

    // Build extra_meta: everything in meta except known scalar flags
    let known_meta_keys: std::collections::HashSet<&str> = [
        "name", "module", "autoname",
        "is_child_table", "istable",           // both spellings
        "issingle",
        "is_submittable", "issubmittable",      // both spellings
        "is_tree", "custom",
        "fields", "permissions", "doctype",
    ]
    .into();
    let extra_meta: HashMap<String, Value> = meta_obj
        .into_iter()
        .filter(|(k, _)| !known_meta_keys.contains(k.as_str()))
        .collect();

    // Parse fields
    let fields: Vec<_> = params
        .get("fields")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .iter()
        .map(parse_docfield_from_value)
        .collect();

    // Parse perms — UI sends "permissions" (top-level), CLI/scripts may send "perms"
    let perms: Vec<_> = params
        .get("permissions")
        .or_else(|| params.get("perms"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .iter()
        .map(parse_docperm_from_value)
        .collect();

    let input = DoctypeSaveInput {
        doctype: doctype.clone(),
        module,
        autoname,
        is_child,
        is_single,
        is_submittable,
        is_tree,
        custom,
        fields,
        perms,
        user: params.get("__current_user")
            .and_then(Value::as_str)
            .unwrap_or("Administrator")
            .to_owned(),
        extra_meta,
    };

    let saved = save_doctype(&site.db, &site.meta_cache, input)
        .await
        .map_err(|e| {
            tracing::error!(doctype = %doctype, error = ?e, "DocType save failed");
            match e {
                DoctypeSaveError::Validation(errs) => {
                    let msg = errs
                        .iter()
                        .map(|ve| format!("[{}] {}", ve.code, ve.message))
                        .collect::<Vec<_>>()
                        .join("; ");
                    SpotError::Validation(msg)
                }
                DoctypeSaveError::Db(db_e) => SpotError::Db(db_e.to_string()),
                DoctypeSaveError::DdlFailed { message, .. } => SpotError::Db(message),
            }
        })?;

    tracing::info!(doctype = %doctype, "Designer save complete");

    Ok(json!({ "ok": true, "doctype": saved }))
}

// ── spotledger.designer.generate_surql ───────────────────────────────────────

/// Generate a SurrealQL preview for the given doctype — no DB writes.
///
/// Parameters: `{ "doctype": "<Name>" }`
pub async fn handle_generate_surql(
    site: Arc<SiteState>,
    params: HashMap<String, Value>,
) -> Result<Value, SpotError> {
    let doctype = require_string(&params, "doctype")?;

    let field_rows = site
        .db
        .run(
            "SELECT * FROM tabDocField WHERE parent = $dt ORDER BY idx ASC",
            vec![("dt".into(), Value::String(doctype.clone()))],
        )
        .await
        .map_err(|e| SpotError::Db(e.to_string()))?;

    let table = doctype_to_table(&doctype);
    let safe_name = doctype.replace([' ', '-'], "_").to_lowercase();
    let mut out = String::new();

    out.push_str(&format!(
        "-- DocType: {doctype}  (auto-generated — edit via UI only)\n\
         DEFINE TABLE IF NOT EXISTS `{table}` SCHEMAFULL \
         COMMENT 'SpotLedger DocType: {doctype}';\n\n"
    ));

    for field in &field_rows {
        let fieldname = field
            .get("fieldname")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let fieldtype = field
            .get("fieldtype")
            .and_then(Value::as_str)
            .unwrap_or("Data");

        if fieldname.is_empty() {
            continue;
        }

        let surql_type = match fieldtype.to_lowercase().as_str() {
            "check" | "int" => "none | int",
            "float" | "currency" | "percent" | "rating" => "float",
            "datetime" => "option<datetime>",
            "date" | "time" => "option<string>",
            "table" | "table multiselect" => "array<any>",
            "json" => "any",
            "section break" | "column break" | "tab break" => continue,
            _ => "option<string>",
        };

        let default_clause = field
            .get("default_value")
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
            .map(|dv| format!(" DEFAULT '{}'", dv.replace('\'', "\\'")))
            .unwrap_or_default();

        let assert_clause = field
            .get("assert_expr")
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
            .map(|expr| format!(" ASSERT {expr}"))
            .unwrap_or_default();

        let value_clause = field
            .get("compute_expr")
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
            .map(|expr| format!(" VALUE {expr}"))
            .unwrap_or_default();

        out.push_str(&format!(
            "DEFINE FIELD IF NOT EXISTS `{fieldname}` ON TABLE `{table}` \
             TYPE {surql_type}{default_clause}{assert_clause}{value_clause} PERMISSIONS FULL;\n"
        ));
    }

    out.push('\n');
    out.push_str(&format!(
        "DEFINE INDEX IF NOT EXISTS idx_{safe_name}_name \
         ON TABLE `{table}` FIELDS name UNIQUE;\n"
    ));

    Ok(json!({ "surql": out }))
}

// ── spotledger.designer.get_pipeline ─────────────────────────────────────────

/// Load pipeline stages, nodes, and fn_source records for a doctype.
///
/// Parameters: `{ "doctype": "<Name>" }`
pub async fn handle_get_pipeline(
    site: Arc<SiteState>,
    params: HashMap<String, Value>,
) -> Result<Value, SpotError> {
    let doctype = require_string(&params, "doctype")?;

    // Fetch all pipeline stages for this doctype
    let stages = site
        .db
        .run(
            "SELECT * FROM pipeline_stage WHERE doctype = $dt ORDER BY ord ASC",
            vec![("dt".into(), Value::String(doctype.clone()))],
        )
        .await
        .map_err(|e| SpotError::Db(e.to_string()))?;

    // Fetch all fn_source rows for this doctype
    let fn_sources = site
        .db
        .run(
            "SELECT * FROM fn_source WHERE doctype = $dt ORDER BY stage, fn_name ASC",
            vec![("dt".into(), Value::String(doctype.clone()))],
        )
        .await
        .map_err(|e| SpotError::Db(e.to_string()))?;

    // Fetch pipeline nodes and their has_node edge ordering for this doctype
    let nodes = site
        .db
        .run(
            r#"
            SELECT
                out.fn_name AS fn_name,
                out.name    AS node_name,
                in.name     AS stage_name,
                in.action   AS action_name,
                ord
            FROM has_node
            WHERE in.doctype = $dt
            ORDER BY ord ASC
            "#,
            vec![("dt".into(), Value::String(doctype.clone()))],
        )
        .await
        .unwrap_or_default(); // pipeline may not exist yet — treat as empty

    Ok(json!({
        "stages":     stages,
        "nodes":      nodes,
        "fn_sources": fn_sources,
    }))
}

// ── spotledger.designer.save_function ────────────────────────────────────────

/// Upsert fn_source, apply `DEFINE FUNCTION` DDL, wire pipeline_node into its stage,
/// and regenerate `fn::registry::dispatch` so the runner can call the new function.
///
/// Parameters:
/// ```json
/// {
///   "fn_name": "fn::employee::validate_dob_joining",
///   "doctype": "Employee",
///   "stage":   "validate",
///   "code":    "...",
///   "description": "...",
///   "tags":    []
/// }
/// ```
///
/// ## Lifecycle stage → pipeline action mapping
/// | stage        | action(s)  | stage name in pipeline |
/// |--------------|------------|------------------------|
/// | validate     | save       | validate               |
/// | before_save  | save       | before_save            |
/// | on_save      | save       | on_save                |
/// | on_submit    | submit     | on_submit              |
/// | on_cancel    | cancel     | on_cancel              |
/// | on_amend     | amend      | on_amend               |
/// | scheduled    | (none)     | n/a                    |
///
/// ## DEFINE FUNCTION contract
/// Functions receive `($doc_id: record, $config: object)` — matching what
/// `fn::registry::dispatch` passes when the runner calls them.
/// The body should return `{ ok: true }` on success or `{ error: "..." }` on failure.
pub async fn handle_save_function(
    site: Arc<SiteState>,
    params: HashMap<String, Value>,
) -> Result<Value, SpotError> {
    let fn_name     = require_string(&params, "fn_name")?;
    let doctype     = require_string(&params, "doctype")?;
    let stage       = require_string(&params, "stage")?;
    let code        = require_string(&params, "code")?;
    let description = params.get("description").and_then(Value::as_str).unwrap_or("").to_owned();
    let tags = params
        .get("tags")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    if !fn_name.starts_with("fn::") {
        return Err(SpotError::Validation(format!(
            "fn_name must begin with `fn::`, got `{fn_name}`"
        )));
    }

    // ── 0. Ensure pipeline schema tables, shared fns, universal nodes, and
    //       registry exist.  Delegates to the same bootstrap used at server
    //       startup — fully idempotent (IF NOT EXISTS / OVERWRITE everywhere).
    apply_pipeline_bootstrap(&site.db).await
        .map_err(|e| SpotError::Db(format!("Pipeline bootstrap failed: {e}")))?;

    // Map lifecycle stage name → [(action, stage_name_in_pipeline)] pairs.
    // "validate" runs on both save and submit actions (mirrors Frappe behaviour).
    let stage_actions: &[(&str, &str)] = match stage.as_str() {
        "validate"    => &[("save", "validate"), ("submit", "validate")],
        "before_save" => &[("save", "before_save")],
        "on_save"     => &[("save", "on_save")],
        "on_submit"   => &[("submit", "on_submit")],
        "on_cancel"   => &[("cancel", "on_cancel")],
        "on_amend"    => &[("amend", "on_amend")],
        _             => &[],  // "scheduled" and unknown stages: no pipeline wiring
    };

    let table = doctype_to_table(&doctype);

    // ── 1. Upsert fn_source (source-of-truth for edit round-trips) ────────────
    site.db
        .execute(
            "UPSERT fn_source CONTENT $content WHERE fn_name = $fn_name",
            vec![
                ("fn_name".into(), Value::String(fn_name.clone())),
                (
                    "content".into(),
                    json!({
                        "fn_name":     &fn_name,
                        "doctype":     &doctype,
                        "stage":       &stage,
                        "code":        &code,
                        "description": &description,
                        "tags":        tags,
                    }),
                ),
            ],
        )
        .await
        .map_err(|e| SpotError::Db(e.to_string()))?;

    // ── 2. Apply DEFINE FUNCTION DDL (OVERWRITE so edits take effect) ─────────
    // Signature must match what fn::registry::dispatch calls:
    //   fn_name($doc_id, $config)
    // Body returns { ok: true } or { error: "..." }.
    let fn_ddl = format!(
        "DEFINE FUNCTION OVERWRITE {fn_name}($doc_id: record, $config: object) {{\n{code}\n}};"
    );
    site.db
        .execute(&fn_ddl, vec![])
        .await
        .map_err(|e| SpotError::Db(format!("DEFINE FUNCTION failed: {e}")))?;

    // ── 3. Ensure pipeline_stage + pipeline_node + has_node for each action ───
    for &(action, stage_name) in stage_actions {
        // Deterministic stage record ID: {table}_{action}_{stage_name}
        // Matches the pattern used by fn::pipeline::wire_generic.
        let stage_id = format!("{table}_{action}_{stage_name}");

        // Upsert the pipeline_stage record (name + action + ord + doctype).
        site.db
            .execute(
                "UPSERT type::record('pipeline_stage', $sid) CONTENT $content",
                vec![
                    ("sid".into(), Value::String(stage_id.clone())),
                    (
                        "content".into(),
                        json!({
                            "name":    stage_name,
                            "action":  action,
                            "ord":     1,
                            "doctype": &doctype,
                        }),
                    ),
                ],
            )
            .await
            .map_err(|e| SpotError::Db(e.to_string()))?;

        // Deterministic node record ID derived from fn_name (:: → _).
        let node_id = fn_name.replace("::", "_");

        // Upsert the pipeline_node record.
        site.db
            .execute(
                "UPSERT type::record('pipeline_node', $nid) CONTENT $content",
                vec![
                    ("nid".into(), Value::String(node_id.clone())),
                    (
                        "content".into(),
                        json!({
                            "name":    &fn_name,
                            "fn_name": &fn_name,
                        }),
                    ),
                ],
            )
            .await
            .map_err(|e| SpotError::Db(e.to_string()))?;

        // Compute next ord from existing has_node edges for this stage.
        let max_ord = site
            .db
            .run(
                "LET $stage_rec = type::record('pipeline_stage', $sid); \
                 RETURN (SELECT math::max(ord) AS max_ord FROM has_node WHERE in = $stage_rec)[0].max_ord;",
                vec![("sid".into(), Value::String(stage_id.clone()))],
            )
            .await
            .unwrap_or_default()
            .into_iter()
            .next()
            .and_then(|r| r.as_u64())
            .unwrap_or(0);

        // Idempotent edge creation: only RELATE if edge doesn't already exist.
        site.db
            .execute(
                "LET $stage_rec = type::record('pipeline_stage', $sid); \
                 LET $node_rec  = type::record('pipeline_node',  $nid); \
                 IF (SELECT id FROM has_node \
                       WHERE in = $stage_rec AND out = $node_rec \
                       LIMIT 1) = [] \
                 THEN RELATE $stage_rec -> has_node -> $node_rec \
                          CONTENT { ord: $ord }; \
                 END;",
                vec![
                    ("sid".into(), Value::String(stage_id)),
                    ("nid".into(), Value::String(node_id)),
                    ("ord".into(), Value::Number((max_ord + 10).into())),
                ],
            )
            .await
            .map_err(|e| SpotError::Db(e.to_string()))?;
    }

    // ── 4. Regenerate fn::registry::dispatch ──────────────────────────────────
    // Collect all fn_names currently in pipeline_node — this is the full set of
    // functions the runner can invoke.  We include the new fn_name even if the
    // UPSERT above hasn't been flushed yet.
    let node_rows = site
        .db
        .run("SELECT fn_name FROM pipeline_node ORDER BY fn_name ASC", vec![])
        .await
        .unwrap_or_default();

    let mut all_fn_names: std::collections::BTreeSet<String> = node_rows
        .iter()
        .filter_map(|r| r.get("fn_name").and_then(Value::as_str).map(String::from))
        .collect();

    // Always include the newly saved function.
    all_fn_names.insert(fn_name.clone());

    // Exclude infrastructure functions — they are not dispatched through the registry.
    all_fn_names.retain(|n| {
        !n.starts_with("fn::pipeline::") && !n.starts_with("fn::registry::")
    });

    let fn_names_vec: Vec<String> = all_fn_names.into_iter().collect();
    let registry_sql = generate_registry_surql(&fn_names_vec);
    site.db
        .execute(&registry_sql, vec![])
        .await
        .map_err(|e| SpotError::Db(format!("Failed to rebuild fn::registry::dispatch: {e}")))?;

    tracing::info!(
        fn_name = %fn_name,
        doctype = %doctype,
        stage   = %stage,
        registry_size = fn_names_vec.len(),
        "Function saved, DDL applied, registry rebuilt"
    );

    Ok(json!({ "ok": true, "fn_name": fn_name }))
}

// ── spotledger.designer.delete_function ──────────────────────────────────────

/// Remove a function: deletes `has_node` edge, `pipeline_node`, fn_source, and removes the DDL.
///
/// Parameters: `{ "fn_name": "fn::..." }`
pub async fn handle_delete_function(
    site: Arc<SiteState>,
    params: HashMap<String, Value>,
) -> Result<Value, SpotError> {
    let fn_name = require_string(&params, "fn_name")?;

    if !fn_name.starts_with("fn::") {
        return Err(SpotError::Validation(format!(
            "fn_name must begin with `fn::`, got `{fn_name}`"
        )));
    }

    // 1. Remove has_node edges pointing to this node
    site.db
        .execute(
            "DELETE has_node WHERE out.fn_name = $fn",
            vec![("fn".into(), Value::String(fn_name.clone()))],
        )
        .await
        .map_err(|e| SpotError::Db(e.to_string()))?;

    // 2. Remove pipeline_node
    site.db
        .execute(
            "DELETE pipeline_node WHERE fn_name = $fn",
            vec![("fn".into(), Value::String(fn_name.clone()))],
        )
        .await
        .map_err(|e| SpotError::Db(e.to_string()))?;

    // 3. Remove fn_source
    site.db
        .execute(
            "DELETE fn_source WHERE fn_name = $fn",
            vec![("fn".into(), Value::String(fn_name.clone()))],
        )
        .await
        .map_err(|e| SpotError::Db(e.to_string()))?;

    // 4. Remove the DEFINE FUNCTION DDL
    let remove_sql = format!("REMOVE FUNCTION IF EXISTS {fn_name};");
    // Non-fatal: function may have never been defined (e.g. saved but not applied yet)
    let _ = site.db.execute(&remove_sql, vec![]).await;

    tracing::info!(fn_name = %fn_name, "Function deleted");

    Ok(json!({ "ok": true, "fn_name": fn_name }))
}

// ── spotledger.designer.reorder_functions ────────────────────────────────────

/// Bulk-update `has_node.ord` for all functions in a stage.
///
/// Parameters:
/// ```json
/// {
///   "doctype": "Sales Invoice",
///   "stage":   "validate",
///   "ordered_fn_names": ["fn::sales_invoice::check_a", "fn::sales_invoice::check_b"]
/// }
/// ```
pub async fn handle_reorder_functions(
    site: Arc<SiteState>,
    params: HashMap<String, Value>,
) -> Result<Value, SpotError> {
    let doctype = require_string(&params, "doctype")?;
    let stage   = require_string(&params, "stage")?;

    let ordered: Vec<String> = params
        .get("ordered_fn_names")
        .and_then(Value::as_array)
        .ok_or_else(|| SpotError::Validation("`ordered_fn_names` array is required".into()))?
        .iter()
        .filter_map(|v| v.as_str().map(str::to_owned))
        .collect();

    for (new_ord, fn_name) in ordered.iter().enumerate() {
        site.db
            .execute(
                "UPDATE has_node SET ord = $ord \
                 WHERE in.doctype = $dt AND in.stage = $stage AND out.fn_name = $fn",
                vec![
                    ("dt".into(), Value::String(doctype.clone())),
                    ("stage".into(), Value::String(stage.clone())),
                    ("fn".into(), Value::String(fn_name.clone())),
                    ("ord".into(), Value::Number(new_ord.into())),
                ],
            )
            .await
            .map_err(|e| SpotError::Db(e.to_string()))?;
    }

    tracing::info!(
        doctype = %doctype,
        stage = %stage,
        count = ordered.len(),
        "Functions reordered"
    );

    Ok(json!({ "ok": true, "count": ordered.len() }))
}
