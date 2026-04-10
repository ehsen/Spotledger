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
use crate::document::doctype_to_table;
use crate::error::DbError;
use crate::meta_cache::MetaCache;
use crate::permissions::{has_permission, PermissionType};

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

    // ── 4. Strip internal meta fields before writing ──────────────────────────
    if let Value::Object(ref mut map) = doc {
        map.remove("__islocal");
        map.remove("__unsaved");
        map.remove("doctype"); // SurrealDB record type already encodes this
    }

    // ── 5. DB write — single UPSERT; SurrealDB events fire here ──────────────
    let table = doctype_to_table(doctype);
    let result = if is_new {
        // INSERT: let SurrealDB generate the record id if name is absent
        let sql = "INSERT INTO type::table($table) $doc RETURN AFTER;";
        adapter
            .run(
                sql,
                vec![
                    ("table".into(), json!(table)),
                    ("doc".into(), doc),
                ],
            )
            .await?
    } else {
        // UPSERT by record id
        let sql = "UPSERT type::thing($table, $name) CONTENT $doc RETURN AFTER;";
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

    Ok(result
        .into_iter()
        .next()
        .unwrap_or(Value::Null))
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
    let sql = "UPDATE type::thing($table, $name) SET docstatus = 1 RETURN AFTER;";
    let rows = adapter
        .run(
            sql,
            vec![
                ("table".into(), json!(table)),
                ("name".into(),  json!(name)),
            ],
        )
        .await?;

    Ok(rows.into_iter().next().unwrap_or(Value::Null))
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
    let sql = "UPDATE type::thing($table, $name) SET docstatus = 2 RETURN AFTER;";
    let rows = adapter
        .run(
            sql,
            vec![
                ("table".into(), json!(table)),
                ("name".into(),  json!(name)),
            ],
        )
        .await?;

    Ok(rows.into_iter().next().unwrap_or(Value::Null))
}
