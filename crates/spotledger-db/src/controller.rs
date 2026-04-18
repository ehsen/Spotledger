//! Save controller — ordered document lifecycle pipeline.
//!
//! This is the single chokepoint through which all `frappe.client.save` and
//! `frappe.client.insert` operations flow.  It mirrors Frappe's
//! `BaseDocument.save()` / `BaseDocument.insert()` sequence:
//!
//! ```text
//! 1.  fix_numeric_types         — coerce Check → 0/1, Int, Float
//! 2.  resolve_name              — naming series / hash / field / prompt  (new only)
//! 3.  fetch_old_doc             — load DB copy for change-validation     (existing only)
//! 4.  validate_constants        — set_only_once fields
//! 5.  validate_update_submit    — allow_on_submit guard
//! 6.  set_user_and_timestamp    — owner / creation / modified
//! 7.  set_parent_in_children    — stamp parent / parenttype / idx on rows
//! 8.  mandatory check           — get_missing_mandatory_fields
//! 9.  validate_selects          — Select options
//! 10. validate_length           — max-length on Data/SmallText fields
//! 11. sanitize_content          — XSS strip / escape
//! 12. Validate hook             — fire registered `validate` hooks
//! 13. BeforeSave/BeforeInsert   — hooks
//! 14. DB write                  — insert_doc or upsert_doc
//! 15. AfterSave/AfterInsert     — hooks
//! ```

use chrono::Utc;
use serde_json::Value;
use spotledger_core::document::Document;
use spotledger_core::error::CoreError;
use spotledger_core::meta::DocTypeMeta;
use spotledger_core::validation::{
    get_missing_mandatory_fields, sanitize_content, validate_constants, validate_length,
    validate_selects, validate_update_after_submit,
};

use crate::adapter::DbAdapter;
use crate::document::{get_doc, insert_doc, upsert_doc, delete_doc as db_delete};
use crate::error::DbError;
use crate::hooks::{HookEvent, HookRegistry};

// ── save_doc ──────────────────────────────────────────────────────────────────

/// Run the full save lifecycle (insert or upsert) for a document.
///
/// `is_new` should be `true` when the document has no existing DB record
/// (typically when `doc.name` is empty or `doc.fields["__islocal"]` is set).
/// The controller resolves the name via naming series when `is_new = true` and
/// `doc.name.is_empty()`.
///
/// Returns the persisted document exactly as stored in the DB (includes
/// server-assigned `creation`, `modified`, etc.).
pub async fn save_doc(
    adapter: &DbAdapter,
    hooks: &HookRegistry,
    meta: &DocTypeMeta,
    mut doc: Document,
    user: &str,
) -> Result<Document, CoreError> {
    let is_new = doc.name.is_empty()
        || doc.fields.get("__islocal").and_then(|v| v.as_i64()).unwrap_or(0) == 1;

    // Strip internal UI-only meta fields — these must not reach the DB.
    doc.fields.remove("__islocal");
    doc.fields.remove("__unsaved");
    doc.fields.remove("doctype"); // redundant: doc.doctype already holds this

    // ── 1. Coerce numeric field types ─────────────────────────────────────────
    doc.fix_numeric_types(meta);

    // ── 2. Resolve name (new documents only) ──────────────────────────────────
    if is_new && doc.name.is_empty() {
        let doc_val = doc.as_dict();
        let mut naming_val = doc_val.clone();
        if let Value::Object(ref mut m) = naming_val {
            m.remove("name");
            m.remove("__islocal");
        }
        let name = adapter
            .run(
                "RETURN fn::naming::resolve($doctype, $doc);",
                vec![
                    ("doctype".into(), meta.name.clone().into()),
                    ("doc".into(),     naming_val),
                ],
            )
            .await
            .map_err(|e| CoreError::Other(e.to_string()))?
            .into_iter()
            .next()
            .and_then(|v| v.as_str().map(str::to_owned))
            .filter(|s| !s.is_empty() && s != "NONE")
            .unwrap_or_else(|| {
                // Only fallback: compiled meta autoname hint (Tier-0 doctypes).
                // After install-app, fn::naming::resolve always handles this.
                if let Some(hint) = meta.autoname.as_deref() {
                    if hint.starts_with("field:") {
                        let fieldname = &hint["field:".len()..];
                        if let Some(v) = doc_val.get(fieldname).and_then(|v| v.as_str()).filter(|s| !s.is_empty()) {
                            return v.to_owned();
                        }
                    }
                }
                format!("new-{:08x}",
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default().subsec_nanos())
            });
        doc.name = name;
    }

    // ── 3. Fetch old document from DB (existing docs only) ────────────────────
    let old_doc: Option<Document> = if !is_new {
        match get_doc(adapter, &meta.name, &doc.name).await {
            Ok(d)  => Some(d),
            Err(DbError::NotFound { .. }) => None,
            Err(e) => return Err(CoreError::from(e)),
        }
    } else {
        None
    };

    // ── 4. set_only_once field guard ──────────────────────────────────────────
    validate_constants(&doc, old_doc.as_ref(), meta)?;

    // ── 5. allow_on_submit guard ──────────────────────────────────────────────
    validate_update_after_submit(&doc, old_doc.as_ref(), meta)?;

    // ── 6. Timestamps ─────────────────────────────────────────────────────────
    let now_iso = Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();
    doc.set_user_and_timestamp(user, &now_iso, is_new);

    // ── 7. Stamp child rows ───────────────────────────────────────────────────
    doc.set_parent_in_children();

    // ── 8. Mandatory field check ──────────────────────────────────────────────
    let missing = get_missing_mandatory_fields(&doc, meta);
    if !missing.is_empty() {
        let names: Vec<String> = missing.iter().map(|(_, lbl)| lbl.clone()).collect();
        return Err(CoreError::Validation(format!(
            "Mandatory fields are required: {}",
            names.join(", ")
        )));
    }

    // ── 9. Select options ─────────────────────────────────────────────────────
    validate_selects(&doc, meta)?;

    // ── 10. Length limits ─────────────────────────────────────────────────────
    validate_length(&doc, meta)?;

    // ── 11. XSS sanitization ──────────────────────────────────────────────────
    sanitize_content(&mut doc, meta);

    // ── 12 & 13. Hooks: Validate → BeforeSave/BeforeInsert ───────────────────
    let doc = hooks.fire(&doc.doctype.clone(), &HookEvent::Validate, doc).await?;
    let doc = hooks.fire(&doc.doctype.clone(), &HookEvent::BeforeSave, doc).await?;
    let doc = if is_new {
        hooks.fire(&doc.doctype.clone(), &HookEvent::BeforeInsert, doc).await?
    } else {
        doc
    };

    // ── 14. DB write ──────────────────────────────────────────────────────────
    let fields_val = doc.as_dict();
    let saved = if is_new {
        insert_doc(adapter, &meta.name, &fields_val)
            .await
            .map_err(CoreError::from)?
    } else {
        upsert_doc(adapter, &meta.name, &doc.name, &fields_val)
            .await
            .map_err(CoreError::from)?
    };

    // ── 15. Hooks: AfterSave/AfterInsert ──────────────────────────────────────
    let saved = hooks.fire(&saved.doctype.clone(), &HookEvent::AfterSave, saved).await?;
    let saved = if is_new {
        hooks.fire(&saved.doctype.clone(), &HookEvent::AfterInsert, saved).await?
    } else {
        saved
    };

    Ok(saved)
}

// ── delete_doc_checked ────────────────────────────────────────────────────────

/// Delete a document after checking that it is not in Submitted state.
///
/// Fires `BeforeDelete` and `AfterDelete` hooks.
/// Returns `CoreError::Validation` if the document is submitted (docstatus = 1).
pub async fn delete_doc_checked(
    adapter: &DbAdapter,
    hooks: &HookRegistry,
    doctype: &str,
    name: &str,
) -> Result<(), CoreError> {
    // Load doc to check docstatus and run hooks with the document data
    let doc = get_doc(adapter, doctype, name)
        .await
        .map_err(CoreError::from)?;

    if doc.is_submitted() {
        return Err(CoreError::Validation(format!(
            "Cannot delete a submitted document ({doctype}/{name}). Cancel it first."
        )));
    }

    // BeforeDelete hook
    let doc = hooks.fire(doctype, &HookEvent::BeforeDelete, doc).await?;

    // DB delete
    db_delete(adapter, doctype, name)
        .await
        .map_err(CoreError::from)?;

    // AfterDelete hook (fire with a tombstone doc since the record is gone)
    let _ = hooks.fire(doctype, &HookEvent::AfterDelete, doc).await;

    Ok(())
}

// ── meta lookup helper ────────────────────────────────────────────────────────

/// Look up the compiled `DocTypeMeta` for a given doctype name from the
/// `inventory` of registered [`MetaEntry`] items.
///
/// Returns `None` when no compiled DocType with that name is registered.
/// Dynamic/WASM DocTypes must supply their meta through a separate path.
pub fn get_compiled_meta(doctype: &str) -> Option<DocTypeMeta> {
    use spotledger_core::registry::MetaEntry;
    inventory::iter::<MetaEntry>
        .into_iter()
        .find(|e| e.name == doctype)
        .map(|e| (e.meta)())
}
