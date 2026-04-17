//! Save proxy — the thin auth + DB pass-through that replaces the 15-step
//! compiled save controller for non-Tier-0 DocTypes.
//!
//! ## Responsibility split
//!
//! | Layer | Responsibility |
//! |-------|---------------|
//! | **save_proxy** (this file) | Auth check → single SurrealDB UPSERT/INSERT |
//! | **SurrealDB DEFINE EVENT** | Validation, computed fields, cascaded writes |
//! | **SurrealDB DEFINE FIELD ASSERT** | Type + constraint enforcement |
//!
//! Tier 0 compiled DocTypes (User, Role, DocType, …) continue to use the full
//! `controller::save_doc` pipeline because those types are bootstrapped before
//! SurrealDB events are in place.  The `save_proxy` is for all runtime-seeded
//! types: everything `install_app` loaded from JSON.
//!
//! ## Custom-field prefix enforcement
//!
//! When a field inside the document has `is_custom = true` on its `docfield`
//! graph node **and** the fieldname does not already start with `custom_`, the
//! save proxy rewrites the fieldname.  This is enforced at the API boundary so
//! no custom field can silently shadow a standard field.

use serde_json::{json, Value};

use crate::adapter::DbAdapter;
use crate::document::{doctype_to_table, get_doc};
use crate::error::DbError;
use crate::meta_cache::MetaCache;
use crate::naming::resolve_name;
use crate::permissions::{has_permission, PermissionType};
use crate::pipeline::{run_pipeline, PipelineResult};

// ── SaveProxyError ───────────────────────────────────────────────────────────

/// Errors returned by save_proxy operations.
#[derive(Debug, thiserror::Error)]
pub enum SaveProxyError {
    #[error("Permission denied: {user} cannot {ptype:?} {doctype}")]
    PermissionDenied {
        user:    String,
        ptype:   PermissionType,
        doctype: String,
    },

    #[error("Pipeline error: {0}")]
    Pipeline(String),

    #[error(transparent)]
    Db(#[from] DbError),
}

// ── save_doc_proxy ────────────────────────────────────────────────────────────

/// Thin save handler for runtime-seeded DocTypes.
///
/// Steps:
/// 1.  Permission check (Write / Create).
/// 2.  Enforce `custom_` prefix on `is_custom` fields.
/// 3.  `UPSERT` the document into SurrealDB — all validation, computed fields,
///     and cascaded writes fire automatically via `DEFINE EVENT` / `DEFINE FIELD`.
///
/// Returns the persisted record as returned by SurrealDB.
pub async fn save_doc_proxy(
    adapter:    &DbAdapter,
    meta_cache: &MetaCache,
    user:       &str,
    doctype:    &str,
    mut doc:    Value,
) -> Result<Value, SaveProxyError> {
    // ── 1. Determine whether this is an insert or an update ──────────────────
    let name = doc
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_owned();
    let is_new = name.is_empty()
        || doc
            .get("__islocal")
            .and_then(Value::as_i64)
            .unwrap_or(0)
            == 1;

    let ptype = if is_new {
        PermissionType::Create
    } else {
        PermissionType::Write
    };

    // ── 2. Permission check ───────────────────────────────────────────────────
    let allowed = has_permission(adapter, user, doctype, ptype)
        .await
        .map_err(SaveProxyError::Db)?;

    if !allowed {
        return Err(SaveProxyError::PermissionDenied {
            user:    user.to_owned(),
            ptype,
            doctype: doctype.to_owned(),
        });
    }

    // ── 3. custom_ prefix enforcement ─────────────────────────────────────────
    // Fetch the live meta to identify is_custom fields.  On error (meta not in DB
    // yet, first-time seeding path) we skip this step gracefully.
    if let Ok(meta) = meta_cache.get(adapter, doctype).await {
        let custom_fieldnames: std::collections::HashSet<String> = meta
            .fields
            .iter()
            .filter(|f| f.is_custom)
            .map(|f| f.fieldname.clone())
            .collect();

        if let Value::Object(ref mut map) = doc {
            let keys_to_fix: Vec<String> = map
                .keys()
                .filter(|k| {
                    custom_fieldnames.contains(*k) && !k.starts_with("custom_")
                })
                .cloned()
                .collect();

            for key in keys_to_fix {
                if let Some(val) = map.remove(&key) {
                    let new_key = format!("custom_{key}");
                    map.insert(new_key, val);
                }
            }
        }
    }

    // ── 3b. Resolve name for new documents ───────────────────────────────────
    // type::record() requires an explicit name; resolve it from doc fields /
    // DocumentNamingRule / UUID fallback, matching the Tier-0 path.
    if is_new && name.is_empty() {
        let resolved = resolve_name(adapter, doctype, &doc, None)
            .await
            .map_err(|e| SaveProxyError::Db(e))?;
        if let Value::Object(ref mut map) = doc {
            map.insert("name".into(), Value::String(resolved));
        }
    }

    // ── 4. Strip internal meta fields before writing ──────────────────────────
    if let Value::Object(ref mut map) = doc {
        map.remove("__islocal");
        map.remove("__unsaved");
        map.remove("doctype"); // SurrealDB record type already encodes this
    }

    // ── 4b. Stamp system fields ───────────────────────────────────────────────
    // creation/modified are handled by SurrealDB VALUE expressions in the schema.
    // Only stamp owner/modified_by (string type, no coercion issue) and
    // docstatus/idx defaults.
    if let Value::Object(ref mut map) = doc {
        // modified_by: always overwrite with current user.
        map.insert("modified_by".into(), Value::String(user.to_owned()));

        // owner: keep existing non-empty value; fill in when absent or blank.
        let needs_owner = map.get("owner")
            .map(|v| v.as_str().map(|s| s.is_empty()).unwrap_or(true))
            .unwrap_or(true);
        if needs_owner {
            map.insert("owner".into(), Value::String(user.to_owned()));
        }

        // docstatus: default 0 for new documents.
        if is_new {
            map.entry("docstatus").or_insert_with(|| json!(0));
        }
    }

    // ── 5. DB write ────────────────────────────────────────────────────────────
    // Use the same type::record(table, name) pattern as document.rs so that
    // both the compiled (Tier-0) and proxy (Tier-3+) paths share one SQL dialect
    // compatible with SurrealDB v3.
    let table = doctype_to_table(doctype);
    let result = if is_new {
        // For new docs the name was already resolved by save_doc_proxy caller
        // or is present in the doc itself.  Ensure it is in the doc.
        let doc_name = doc
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_owned();
        let sql = "CREATE type::record($table, $name) CONTENT $doc RETURN AFTER;";
        adapter
            .run(
                sql,
                vec![
                    ("table".into(), json!(table)),
                    ("name".into(),  json!(doc_name)),
                    ("doc".into(),   doc),
                ],
            )
            .await?
    } else {
        // UPSERT by record id
        let sql = "UPSERT type::record($table, $name) CONTENT $doc RETURN AFTER;";
        adapter
            .run(
                sql,
                vec![
                    ("table".into(), json!(table)),
                    ("name".into(),  json!(name)),
                    ("doc".into(),   doc),
                ],
            )
            .await?
    };

    let saved = result.into_iter().next().unwrap_or(Value::Null);

    // ── 6. Pipeline (graph-compute) ────────────────────────────────────────────
    // Wired doctypes run validation + compute + side-effects in SurrealDB.
    // Unwired doctypes return Skipped; we keep the already-saved result.
    let doc_name = saved
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or(if is_new { "" } else { &name })
        .to_owned();

    match run_pipeline(adapter, &table, &doc_name, doctype, "save").await {
        Ok(PipelineResult::Skipped) => {
            // No pipeline registered — keep the saved document as-is.
            return Ok(saved);
        }
        Ok(PipelineResult::Ok) => {
            // Pipeline may have mutated computed fields — re-read the record.
            match get_doc(adapter, doctype, &doc_name).await {
                Ok(doc) => return Ok(serde_json::to_value(doc).unwrap_or(Value::Null)),
                Err(_)  => return Ok(saved), // fallback: return what was already saved
            }
        }
        Err(e) => {
            // Pipeline validation/logic failure — roll back new inserts.
            if is_new && !doc_name.is_empty() {
                let _ = adapter
                    .execute(
                        "DELETE type::record($t, $n);",
                        vec![
                            ("t".into(), json!(table)),
                            ("n".into(), json!(doc_name)),
                        ],
                    )
                    .await;
            }
            return Err(SaveProxyError::Pipeline(e.to_string()));
        }
    }
}

// ── submit_doc_proxy ──────────────────────────────────────────────────────────

/// Thin submit handler — checks Submit permission, sets `docstatus = 1`.
/// The `on_submit` SurrealDB event fires inside the same transaction.
pub async fn submit_doc_proxy(
    adapter: &DbAdapter,
    user:    &str,
    doctype: &str,
    name:    &str,
) -> Result<Value, SaveProxyError> {
    let allowed = has_permission(adapter, user, doctype, PermissionType::Submit)
        .await
        .map_err(SaveProxyError::Db)?;

    if !allowed {
        return Err(SaveProxyError::PermissionDenied {
            user:    user.to_owned(),
            ptype:   PermissionType::Submit,
            doctype: doctype.to_owned(),
        });
    }

    let table = doctype_to_table(doctype);

    // Run the submit pipeline (validates + posts GL entries, etc.).
    // For unwired doctypes, fall back to direct docstatus update.
    match run_pipeline(adapter, &table, name, doctype, "submit").await {
        Ok(PipelineResult::Ok) => {
            match get_doc(adapter, doctype, name).await {
                Ok(doc) => return Ok(serde_json::to_value(doc).unwrap_or(Value::Null)),
                Err(e)  => return Err(SaveProxyError::Db(e)),
            }
        }
        Ok(PipelineResult::Skipped) => {
            // No pipeline → plain docstatus flip.
            let sql = "UPDATE type::record($table, $name) SET docstatus = 1 RETURN AFTER;";
            let rows = adapter
                .run(
                    sql,
                    vec![
                        ("table".into(), json!(table)),
                        ("name".into(),  json!(name)),
                    ],
                )
                .await?;
            return Ok(rows.into_iter().next().unwrap_or(Value::Null));
        }
        Err(e) => return Err(SaveProxyError::Pipeline(e.to_string())),
    }
}

// ── cancel_doc_proxy ──────────────────────────────────────────────────────────

/// Thin cancel handler — checks Cancel permission, sets `docstatus = 2`.
/// The `on_cancel` SurrealDB event fires inside the same transaction.
pub async fn cancel_doc_proxy(
    adapter: &DbAdapter,
    user:    &str,
    doctype: &str,
    name:    &str,
) -> Result<Value, SaveProxyError> {
    let allowed = has_permission(adapter, user, doctype, PermissionType::Cancel)
        .await
        .map_err(SaveProxyError::Db)?;

    if !allowed {
        return Err(SaveProxyError::PermissionDenied {
            user:    user.to_owned(),
            ptype:   PermissionType::Cancel,
            doctype: doctype.to_owned(),
        });
    }

    let table = doctype_to_table(doctype);

    // Run the cancel pipeline (reverses GL entries, marks cancelled, cascades).
    // For unwired doctypes, fall back to direct docstatus update.
    match run_pipeline(adapter, &table, name, doctype, "cancel").await {
        Ok(PipelineResult::Ok) => {
            match get_doc(adapter, doctype, name).await {
                Ok(doc) => return Ok(serde_json::to_value(doc).unwrap_or(Value::Null)),
                Err(e)  => return Err(SaveProxyError::Db(e)),
            }
        }
        Ok(PipelineResult::Skipped) => {
            // No pipeline → plain docstatus flip.
            let sql = "UPDATE type::record($table, $name) SET docstatus = 2 RETURN AFTER;";
            let rows = adapter
                .run(
                    sql,
                    vec![
                        ("table".into(), json!(table)),
                        ("name".into(),  json!(name)),
                    ],
                )
                .await?;
            return Ok(rows.into_iter().next().unwrap_or(Value::Null));
        }
        Err(e) => return Err(SaveProxyError::Pipeline(e.to_string())),
    }
}
