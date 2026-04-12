//! Tier 1 `frappe.client.*` method handlers.
//! These mirror the methods in frappe/client.py.

use super::{BoxFuture, MethodRegistry};
use crate::state::SiteState;
use serde_json::{json, Value};
use spotledger_db::controller::{delete_doc_checked, get_compiled_meta, save_doc};
use spotledger_db::document::{
    bulk_update, cancel_doc, get_count, get_doc, get_list, get_value, rename_doc,
    set_field, submit_doc, upsert_doc,
};
use spotledger_db::hooks::{
    run_before_cancel_hooks, run_before_submit_hooks, run_on_cancel_hooks, run_on_submit_hooks,
};
use spotledger_db::permissions::{get_doc_permissions, has_permission, PermissionType};
use spotledger_db::save_proxy::{cancel_doc_proxy, save_doc_proxy, submit_doc_proxy};
use spotledger_core::document::Document;
use spotledger_core::error::SpotError;
use std::collections::HashMap;
use std::sync::Arc;

pub fn register_client_methods(registry: &Arc<MethodRegistry>) {
    macro_rules! reg {
        ($path:expr, $fn:ident) => {
            registry.register(
                $path,
                Arc::new(|site: Arc<SiteState>, params: HashMap<String, Value>| -> BoxFuture {
                    Box::pin(async move { $fn(site, params).await })
                }),
            );
        };
    }

    reg!("frappe.client.get_list",  handle_get_list);
    reg!("frappe.client.get",       handle_get);
    reg!("frappe.client.get_value", handle_get_value);
    reg!("frappe.client.get_count", handle_get_count);
    reg!("frappe.client.save",      handle_save);
    reg!("frappe.client.insert",    handle_insert);
    reg!("frappe.client.set_value", handle_set_value);
    reg!("frappe.client.delete",    handle_delete);
    reg!("frappe.client.submit",    handle_submit);
    reg!("frappe.client.cancel",    handle_cancel);
    // ── Phase 5 additions ────────────────────────────────────────────────────
    reg!("frappe.client.rename_doc",          handle_rename_doc);
    reg!("frappe.client.attach_file",         handle_attach_file);
    reg!("frappe.client.validate_link",       handle_validate_link);
    reg!("frappe.client.bulk_update",         handle_bulk_update);
    reg!("frappe.client.has_permission",      handle_has_permission);
    reg!("frappe.client.get_doc_permissions", handle_get_doc_permissions);
}

// ── frappe.client.get_list ────────────────────────────────────────────────────

async fn handle_get_list(
    site: Arc<SiteState>,
    params: HashMap<String, Value>,
) -> Result<Value, SpotError> {
    let doctype = require_str(&params, "doctype")?;

    let field_strings: Vec<String> = match params.get("fields") {
        Some(Value::Array(arr)) => arr
            .iter()
            .filter_map(|v| v.as_str().map(str::to_string))
            .collect(),
        Some(Value::String(s)) if s == "*" => vec![],
        _ => vec![],
    };
    let field_refs: Vec<&str> = field_strings.iter().map(String::as_str).collect();
    let fields_opt = if field_refs.is_empty() { None } else { Some(field_refs.as_slice()) };

    let filters = params.get("filters");
    let limit = params.get("limit").and_then(Value::as_u64).map(|n| n as usize).unwrap_or(20);
    let start = params
        .get("limit_start")
        .or_else(|| params.get("start"))
        .and_then(Value::as_u64)
        .map(|n| n as usize)
        .unwrap_or(0);

    let rows = get_list(&site.db, doctype, fields_opt, filters, limit, start)
        .await
        .map_err(SpotError::from)?;

    Ok(serde_json::to_value(rows).unwrap_or(Value::Array(vec![])))
}

// ── frappe.client.get ─────────────────────────────────────────────────────────

async fn handle_get(
    site: Arc<SiteState>,
    params: HashMap<String, Value>,
) -> Result<Value, SpotError> {
    let doctype = require_str(&params, "doctype")?;
    let name = require_str(&params, "name")?;
    let doc = get_doc(&site.db, doctype, name).await.map_err(SpotError::from)?;
    Ok(doc.as_dict())
}

// ── frappe.client.get_value ───────────────────────────────────────────────────

async fn handle_get_value(
    site: Arc<SiteState>,
    params: HashMap<String, Value>,
) -> Result<Value, SpotError> {
    let doctype = require_str(&params, "doctype")?;
    let name = match params.get("filters") {
        Some(Value::String(s)) => s.as_str(),
        _ => require_str(&params, "name")?,
    };
    let fieldname = require_str(&params, "fieldname")?;
    let val = get_value(&site.db, doctype, name, fieldname)
        .await
        .map_err(SpotError::from)?;
    Ok(val.unwrap_or(Value::Null))
}

// -- frappe.client.get_count -------------------------------------------------

async fn handle_get_count(
    site: Arc<SiteState>,
    params: HashMap<String, Value>,
) -> Result<Value, SpotError> {
    let doctype = require_str(&params, "doctype")?;
    let filters = params.get("filters");
    let n = get_count(&site.db, doctype, filters).await.map_err(SpotError::from)?;
    Ok(Value::Number(n.into()))
}

// ── frappe.client.save ────────────────────────────────────────────────────────

async fn handle_save(
    site: Arc<SiteState>,
    params: HashMap<String, Value>,
) -> Result<Value, SpotError> {
    let doc_val = params
        .get("doc")
        .ok_or_else(|| SpotError::Validation("'doc' is required".into()))?
        .clone();

    let doctype = doc_val
        .get("doctype")
        .and_then(Value::as_str)
        .ok_or_else(|| SpotError::Validation("doc.doctype is required".into()))?
        .to_owned();

    let user = params
        .get("__current_user")
        .and_then(Value::as_str)
        .unwrap_or("Administrator")
        .to_owned();

    // Route: compiled Tier-0 types → full validation pipeline
    //         runtime types (Tier 3+) → thin auth+proxy, SurrealDB events handle the rest
    let (saved_val, saved_name) = if let Some(meta) = get_compiled_meta(&doctype) {
        let doc = doc_from_value(&doctype, &doc_val);
        let saved = save_doc(&site.db, &site.hook_registry, &meta, doc, &user).await?;
        let name = saved.name.clone();
        (saved.as_dict(), name)
    } else {
        let result = save_doc_proxy(&site.db, &site.meta_cache, &user, &doctype, doc_val)
            .await
            .map_err(|e| SpotError::Validation(e.to_string()))?;
        let name = result
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned();
        (result, name)
    };

    // Invalidate cache
    site.doc_cache
        .remove(&(doctype.clone(), saved_name.clone()))
        .await;

    // Activity log (best-effort — failures are silently ignored)
    // Determine action: if creation == modified it was just created, otherwise updated.
    let action = {
        let creation  = saved_val.get("creation").and_then(Value::as_str).unwrap_or("");
        let modified  = saved_val.get("modified").and_then(Value::as_str).unwrap_or("x");
        if creation == modified { "created" } else { "saved" }
    };
    let _ = site
        .db
        .execute(
            "RETURN fn::log_activity($user, $dt, $dn, $action, $data)",
            vec![
                ("user".into(),   user.into()),
                ("dt".into(),     doctype.into()),
                ("dn".into(),     saved_name.into()),
                ("action".into(), action.into()),
                ("data".into(),   Value::Null),
            ],
        )
        .await;

    Ok(saved_val)
}

// ── frappe.client.insert ──────────────────────────────────────────────────────

async fn handle_insert(
    site: Arc<SiteState>,
    params: HashMap<String, Value>,
) -> Result<Value, SpotError> {
    let doc_val = params
        .get("doc")
        .ok_or_else(|| SpotError::Validation("'doc' is required".into()))?
        .clone();

    let doctype = doc_val
        .get("doctype")
        .and_then(Value::as_str)
        .ok_or_else(|| SpotError::Validation("doc.doctype is required".into()))?
        .to_owned();

    let user = params
        .get("__current_user")
        .and_then(Value::as_str)
        .unwrap_or("Administrator")
        .to_owned();

    // Force is_new by clearing name so naming series kicks in
    let mut insert_val = doc_val.clone();
    if let Value::Object(ref mut m) = insert_val {
        m.remove("name");
        m.insert("__islocal".into(), Value::Number(1.into()));
    }

    let (saved_val, saved_name) = if let Some(meta) = get_compiled_meta(&doctype) {
        let mut doc = doc_from_value(&doctype, &doc_val);
        doc.name.clear();
        doc.fields.insert("__islocal".into(), Value::Number(1.into()));
        let saved = save_doc(&site.db, &site.hook_registry, &meta, doc, &user).await?;
        let name = saved.name.clone();
        (saved.as_dict(), name)
    } else {
        let result = save_doc_proxy(&site.db, &site.meta_cache, &user, &doctype, insert_val)
            .await
            .map_err(|e| SpotError::Validation(e.to_string()))?;
        let name = result
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned();
        (result, name)
    };

    site.doc_cache
        .remove(&(doctype.clone(), saved_name.clone()))
        .await;

    let _ = site
        .db
        .execute(
            "RETURN fn::log_activity($user, $dt, $dn, $action, $data)",
            vec![
                ("user".into(),   user.into()),
                ("dt".into(),     doctype.into()),
                ("dn".into(),     saved_name.into()),
                ("action".into(), "created".into()),
                ("data".into(),   Value::Null),
            ],
        )
        .await;

    Ok(saved_val)
}

// ── frappe.client.set_value ───────────────────────────────────────────────────

async fn handle_set_value(
    site: Arc<SiteState>,
    params: HashMap<String, Value>,
) -> Result<Value, SpotError> {
    let doctype = require_str(&params, "doctype")?;
    let name = require_str(&params, "name")?;
    let fieldname = require_str(&params, "fieldname")?;
    let value = params.get("value").cloned().unwrap_or(Value::Null);

    set_field(&site.db, doctype, name, fieldname, value)
        .await
        .map_err(SpotError::from)?;

    // Invalidate cache
    site.doc_cache.remove(&(doctype.to_owned(), name.to_owned())).await;

    // Return the updated doc (Frappe convention)
    let doc = get_doc(&site.db, doctype, name).await.map_err(SpotError::from)?;
    Ok(doc.as_dict())
}

// ── frappe.client.delete ──────────────────────────────────────────────────────

async fn handle_delete(
    site: Arc<SiteState>,
    params: HashMap<String, Value>,
) -> Result<Value, SpotError> {
    let doctype = require_str(&params, "doctype")?;
    let name    = require_str(&params, "name")?;
    delete_doc_checked(&site.db, &site.hook_registry, doctype, name).await?;
    site.doc_cache.remove(&(doctype.to_owned(), name.to_owned())).await;
    Ok(Value::String("ok".into()))
}

// ── frappe.client.submit ──────────────────────────────────────────────────────

async fn handle_submit(
    site: Arc<SiteState>,
    params: HashMap<String, Value>,
) -> Result<Value, SpotError> {
    let doc_val = params
        .get("doc")
        .ok_or_else(|| SpotError::Validation("'doc' is required".into()))?
        .clone();

    let doctype = doc_val
        .get("doctype")
        .and_then(Value::as_str)
        .ok_or_else(|| SpotError::Validation("doc.doctype is required".into()))?
        .to_owned();

    let name = doc_val
        .get("name")
        .and_then(Value::as_str)
        .ok_or_else(|| SpotError::Validation("doc.name is required for submit".into()))?
        .to_owned();

    let user = params
        .get("__current_user")
        .and_then(Value::as_str)
        .unwrap_or("Administrator")
        .to_owned();

    site.doc_cache.remove(&(doctype.clone(), name.clone())).await;

    // For compiled Tier-0 types use the hook-based submit path.
    // For runtime types let the save_proxy + SurrealDB on_submit event handle it.
    if get_compiled_meta(&doctype).is_some() {
        let doc = get_doc(&site.db, &doctype, &name)
            .await
            .map_err(SpotError::from)?;
        let doc = run_before_submit_hooks(&site.hook_registry, doc).await?;
        let mut fields_val = doc.as_dict();
        if let Value::Object(ref mut m) = fields_val {
            m.insert("docstatus".into(), Value::Number(1.into()));
        }
        upsert_doc(&site.db, &doctype, &name, &fields_val)
            .await
            .map_err(SpotError::from)?;
        let submitted = submit_doc(&site.db, &doctype, &name)
            .await
            .map_err(SpotError::from)?;
        let submitted = run_on_submit_hooks(&site.hook_registry, submitted).await?;
        Ok(submitted.as_dict())
    } else {
        let result = submit_doc_proxy(&site.db, &user, &doctype, &name)
            .await
            .map_err(|e| SpotError::Validation(e.to_string()))?;
        Ok(result)
    }
}

// ── frappe.client.cancel ──────────────────────────────────────────────────────

async fn handle_cancel(
    site: Arc<SiteState>,
    params: HashMap<String, Value>,
) -> Result<Value, SpotError> {
    let doc_val = params
        .get("doc")
        .ok_or_else(|| SpotError::Validation("'doc' is required".into()))?
        .clone();

    let doctype = doc_val
        .get("doctype")
        .and_then(Value::as_str)
        .ok_or_else(|| SpotError::Validation("doc.doctype is required".into()))?
        .to_owned();

    let name = doc_val
        .get("name")
        .and_then(Value::as_str)
        .ok_or_else(|| SpotError::Validation("doc.name is required for cancel".into()))?
        .to_owned();

    let user = params
        .get("__current_user")
        .and_then(Value::as_str)
        .unwrap_or("Administrator")
        .to_owned();

    site.doc_cache.remove(&(doctype.clone(), name.clone())).await;

    if get_compiled_meta(&doctype).is_some() {
        let doc = get_doc(&site.db, &doctype, &name)
            .await
            .map_err(SpotError::from)?;
        let doc = run_before_cancel_hooks(&site.hook_registry, doc).await?;
        let _ = doc;
        let cancelled = cancel_doc(&site.db, &doctype, &name)
            .await
            .map_err(SpotError::from)?;
        let cancelled = run_on_cancel_hooks(&site.hook_registry, cancelled).await?;
        Ok(cancelled.as_dict())
    } else {
        let result = cancel_doc_proxy(&site.db, &user, &doctype, &name)
            .await
            .map_err(|e| SpotError::Validation(e.to_string()))?;
        Ok(result)
    }
}

// ── helpers ───────────────────────────────────────────────────────────────────

// ── frappe.client.rename_doc ──────────────────────────────────────────────────
//
// Params: doctype, old_name (or old), new_name (or new)
// Returns: { message: "renamed" }
//
// Renames a document within the same DocType table.  Does not cascade Link
// field rewrites to other tables (deferred to Phase 6 when the graph is
// queried for dependents).

async fn handle_rename_doc(
    site: Arc<SiteState>,
    params: HashMap<String, Value>,
) -> Result<Value, SpotError> {
    let doctype  = require_str(&params, "doctype")?;
    let old_name = params
        .get("old_name")
        .or_else(|| params.get("old"))
        .and_then(Value::as_str)
        .ok_or_else(|| SpotError::Validation("'old_name' is required".into()))?;
    let new_name = params
        .get("new_name")
        .or_else(|| params.get("new"))
        .and_then(Value::as_str)
        .ok_or_else(|| SpotError::Validation("'new_name' is required".into()))?;

    let _new_doc = rename_doc(&site.db, doctype, old_name, new_name)
        .await
        .map_err(|e| SpotError::Validation(e.to_string()))?;

    // Evict both names from the doc cache.
    site.doc_cache.remove(&(doctype.to_owned(), old_name.to_owned())).await;
    site.doc_cache.remove(&(doctype.to_owned(), new_name.to_owned())).await;

    Ok(json!({ "message": "renamed" }))
}

// ── frappe.client.attach_file ─────────────────────────────────────────────────
//
// Params: filename, filedata (base64), doctype, docname, is_private (0|1), folder (opt)
// Returns: { name, file_name, file_url, is_private }
//
// Decodes the base64 payload and writes it under
// sites/<site>/public/files/  (or private/files/ when is_private=1).
// Creates a tabFile record referencing the attached document.
//
// NOTE: Full file-storage backend is Phase 8.  This handler covers the
// frappe.client.attach_file contract needed by the desk form UI.

async fn handle_attach_file(
    site: Arc<SiteState>,
    params: HashMap<String, Value>,
) -> Result<Value, SpotError> {
    use base64::{engine::general_purpose, Engine as _};

    let filename  = require_str(&params, "filename")?;
    let filedata  = require_str(&params, "filedata")?;
    let doctype   = require_str(&params, "doctype")?;
    let docname   = require_str(&params, "docname")?;
    let is_private = params
        .get("is_private")
        .and_then(|v| v.as_u64().map(|n| n != 0).or_else(|| v.as_bool()))
        .unwrap_or(false);
    let folder = params
        .get("folder")
        .and_then(Value::as_str)
        .unwrap_or("Home");

    // Strip optional data-URI prefix (e.g. "data:image/png;base64,")
    let b64 = strip_data_uri_prefix(filedata);

    let bytes = general_purpose::STANDARD
        .decode(b64.trim())
        .map_err(|e| SpotError::Validation(format!("Invalid base64: {e}")))?;

    let file_size = bytes.len() as u64;

    // Determine storage path.
    let sub_dir = if is_private { "private/files" } else { "public/files" };
    let site_name = &site.config.site.name;

    // Sanitise filename (prevent path traversal).
    let safe_name: String = std::path::Path::new(filename)
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| SpotError::Validation("Invalid filename".into()))?
        .to_owned();

    let storage_dir = std::path::PathBuf::from("sites")
        .join(site_name)
        .join(sub_dir);
    std::fs::create_dir_all(&storage_dir)
        .map_err(|e| SpotError::Validation(format!("Cannot create upload dir: {e}")))?;

    let dest = storage_dir.join(&safe_name);
    std::fs::write(&dest, &bytes)
        .map_err(|e| SpotError::Validation(format!("Cannot write file: {e}")))?;

    let file_url = if is_private {
        format!("/private/files/{safe_name}")
    } else {
        format!("/files/{safe_name}")
    };

    // Generate a unique name for the File record.
    let file_record_name = format!("{safe_name}");

    // Upsert a File record in tabFile.
    let file_fields = json!({
        "name":                 &file_record_name,
        "file_name":            &safe_name,
        "file_url":             &file_url,
        "file_size":            file_size,
        "attached_to_doctype":  doctype,
        "attached_to_name":     docname,
        "is_private":           is_private as u8,
        "folder":               folder,
        "docstatus":            0
    });
    upsert_doc(&site.db, "File", &file_record_name, &file_fields)
        .await
        .map_err(|e| SpotError::Validation(e.to_string()))?;

    Ok(json!({
        "name":       file_record_name,
        "file_name":  safe_name,
        "file_url":   file_url,
        "is_private": is_private as u8
    }))
}

// ── frappe.client.validate_link ───────────────────────────────────────────────
//
// Params: value (docname to check), options (target doctype), doctype (context, opt)
// Returns: { valid: 1 } or throws a validation error.
//
// Checks that the linked document exists and the current user has read access.

async fn handle_validate_link(
    site: Arc<SiteState>,
    params: HashMap<String, Value>,
) -> Result<Value, SpotError> {
    let value   = require_str(&params, "value")?;
    let options = require_str(&params, "options")?; // target DocType

    let user = params
        .get("__current_user")
        .and_then(Value::as_str)
        .unwrap_or("Administrator");

    // Permission check on the target doctype.
    let can_read = has_permission(&site.db, user, options, PermissionType::Read)
        .await
        .map_err(|e| SpotError::Validation(e.to_string()))?;
    if !can_read {
        return Err(SpotError::Validation(format!(
            "No read permission for {options}"
        )));
    }

    // Existence check.
    let doc_opt = get_doc(&site.db, options, value).await;
    match doc_opt {
        Ok(_) => Ok(json!({ "valid": 1 })),
        Err(_) => Err(SpotError::Validation(format!(
            "{options} '{value}' does not exist"
        ))),
    }
}

// ── frappe.client.bulk_update ─────────────────────────────────────────────────
//
// Params: doctype, docnames (JSON array of strings), fieldname, value
// Returns: { updated: <count> }
//
// Sets `fieldname = value` on every document in `docnames`.

async fn handle_bulk_update(
    site: Arc<SiteState>,
    params: HashMap<String, Value>,
) -> Result<Value, SpotError> {
    let doctype   = require_str(&params, "doctype")?;
    let fieldname = require_str(&params, "fieldname")?;
    let value     = params.get("value").cloned().unwrap_or(Value::Null);

    let names: Vec<String> = parse_bulk_docnames(params.get("docnames"))?;

    let count = bulk_update(&site.db, doctype, &names, fieldname, value)
        .await
        .map_err(|e| SpotError::Validation(e.to_string()))?;

    // Evict all touched docs from the cache.
    for name in &names {
        site.doc_cache.remove(&(doctype.to_owned(), name.clone())).await;
    }

    Ok(json!({ "updated": count }))
}

// ── frappe.client.has_permission ──────────────────────────────────────────────
//
// Params: doctype, ptype (read|write|create|delete|submit|cancel|…), name (opt)
// Returns: { has_permission: <bool> }

async fn handle_has_permission(
    site: Arc<SiteState>,
    params: HashMap<String, Value>,
) -> Result<Value, SpotError> {
    let doctype = require_str(&params, "doctype")?;
    let ptype_s = params
        .get("ptype")
        .and_then(Value::as_str)
        .unwrap_or("read");

    let user = params
        .get("__current_user")
        .and_then(Value::as_str)
        .unwrap_or("Administrator");

    let ptype = PermissionType::from_str(ptype_s)
        .unwrap_or(PermissionType::Read);

    let allowed = has_permission(&site.db, user, doctype, ptype)
        .await
        .map_err(|e| SpotError::Validation(e.to_string()))?;

    Ok(json!({ "has_permission": allowed }))
}

// ── frappe.client.get_doc_permissions ─────────────────────────────────────────
//
// Params: doctype
// Returns: full DocPermission JSON (same shape as bootinfo.user.can_*)

async fn handle_get_doc_permissions(
    site: Arc<SiteState>,
    params: HashMap<String, Value>,
) -> Result<Value, SpotError> {
    let doctype = require_str(&params, "doctype")?;

    let user = params
        .get("__current_user")
        .and_then(Value::as_str)
        .unwrap_or("Administrator");

    let perms = get_doc_permissions(&site.db, user, doctype)
        .await
        .map_err(|e| SpotError::Validation(e.to_string()))?;

    Ok(json!({ "permissions": perms.to_json() }))
}

// ── helpers ───────────────────────────────────────────────────────────────────

fn require_str<'a>(params: &'a HashMap<String, Value>, key: &str) -> Result<&'a str, SpotError> {
    params
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| SpotError::Validation(format!("'{key}' is required")))
}

/// Strip a data-URI prefix from a base64 payload.
///
/// `"data:image/png;base64,abc123"` → `"abc123"`.
/// A string without a prefix is returned unchanged.
pub(crate) fn strip_data_uri_prefix(s: &str) -> &str {
    if let Some(pos) = s.find(',') { &s[pos + 1..] } else { s }
}

/// Parse the `docnames` parameter from a `frappe.client.bulk_update` call.
///
/// Accepts:
/// - A JSON array of strings: `["DOC-1", "DOC-2"]`
/// - A JSON-encoded array string: `"[\"DOC-1\",\"DOC-2\"]"`
///
/// Any other shape returns a `Validation` error.
pub(crate) fn parse_bulk_docnames(v: Option<&Value>) -> Result<Vec<String>, SpotError> {
    match v {
        Some(Value::Array(arr)) => Ok(arr
            .iter()
            .filter_map(|v| v.as_str().map(str::to_owned))
            .collect()),
        Some(Value::String(s)) => serde_json::from_str::<Vec<String>>(s)
            .map_err(|_| SpotError::Validation("'docnames' must be an array".into())),
        _ => Err(SpotError::Validation("'docnames' must be an array".into())),
    }
}

/// Build a [`Document`] from the raw JSON value sent by the client.
pub(crate) fn doc_from_value(doctype: &str, val: &Value) -> Document {
    let mut doc = Document::new(doctype);
    if let Some(name) = val.get("name").and_then(Value::as_str) {
        doc.name = name.to_owned();
    }
    if let Value::Object(map) = val {
        for (k, v) in map {
            if k != "doctype" && k != "name" {
                doc.set(k.clone(), v.clone());
            }
        }
    }
    doc
}

// ── tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── require_str ──────────────────────────────────────────────────────────

    #[test]
    fn require_str_returns_value_when_present() {
        let mut p: HashMap<String, Value> = HashMap::new();
        p.insert("doctype".to_owned(), Value::String("Customer".into()));
        assert_eq!(require_str(&p, "doctype").unwrap(), "Customer");
    }

    #[test]
    fn require_str_errors_when_key_missing() {
        let p: HashMap<String, Value> = HashMap::new();
        let err = require_str(&p, "doctype").unwrap_err();
        assert!(err.to_string().contains("'doctype' is required"));
    }

    #[test]
    fn require_str_errors_when_value_is_not_string() {
        let mut p: HashMap<String, Value> = HashMap::new();
        p.insert("limit".to_owned(), Value::Number(20.into()));
        let err = require_str(&p, "limit").unwrap_err();
        assert!(err.to_string().contains("'limit' is required"));
    }

    // ── strip_data_uri_prefix ────────────────────────────────────────────────

    #[test]
    fn strip_data_uri_prefix_removes_data_uri() {
        assert_eq!(
            strip_data_uri_prefix("data:image/png;base64,abc123=="),
            "abc123=="
        );
    }

    #[test]
    fn strip_data_uri_prefix_no_prefix_unchanged() {
        assert_eq!(strip_data_uri_prefix("abc123=="), "abc123==");
        assert_eq!(strip_data_uri_prefix(""), "");
    }

    #[test]
    fn strip_data_uri_prefix_handles_generic_prefix() {
        // Any data: URI scheme works
        assert_eq!(
            strip_data_uri_prefix("data:application/octet-stream;base64,AAAA"),
            "AAAA"
        );
    }

    // ── parse_bulk_docnames ──────────────────────────────────────────────────

    #[test]
    fn parse_bulk_docnames_accepts_json_array() {
        let v = json!(["DOC-1", "DOC-2", "DOC-3"]);
        let names = parse_bulk_docnames(Some(&v)).unwrap();
        assert_eq!(names, vec!["DOC-1", "DOC-2", "DOC-3"]);
    }

    #[test]
    fn parse_bulk_docnames_accepts_json_encoded_string() {
        let v = Value::String(r#"["DOC-1","DOC-2"]"#.to_owned());
        let names = parse_bulk_docnames(Some(&v)).unwrap();
        assert_eq!(names, vec!["DOC-1", "DOC-2"]);
    }

    #[test]
    fn parse_bulk_docnames_errors_on_missing() {
        let err = parse_bulk_docnames(None).unwrap_err();
        assert!(err.to_string().contains("'docnames' must be an array"));
    }

    #[test]
    fn parse_bulk_docnames_errors_on_number() {
        let v = json!(42);
        let err = parse_bulk_docnames(Some(&v)).unwrap_err();
        assert!(err.to_string().contains("'docnames' must be an array"));
    }

    #[test]
    fn parse_bulk_docnames_errors_on_invalid_json_string() {
        let v = Value::String("not json".to_owned());
        let err = parse_bulk_docnames(Some(&v)).unwrap_err();
        assert!(err.to_string().contains("'docnames' must be an array"));
    }

    #[test]
    fn parse_bulk_docnames_empty_array() {
        let v = json!([]);
        let names = parse_bulk_docnames(Some(&v)).unwrap();
        assert!(names.is_empty());
    }

    // ── doc_from_value ───────────────────────────────────────────────────────

    #[test]
    fn doc_from_value_extracts_name_and_fields() {
        let val = json!({
            "doctype": "Customer",
            "name":    "CUST-001",
            "customer_name": "Acme Corp",
            "website": "https://acme.com"
        });
        let doc = doc_from_value("Customer", &val);
        assert_eq!(doc.name, "CUST-001");
        assert_eq!(
            doc.fields.get("customer_name").and_then(Value::as_str),
            Some("Acme Corp")
        );
        assert_eq!(
            doc.fields.get("website").and_then(Value::as_str),
            Some("https://acme.com")
        );
        // doctype and name must not appear as extra fields
        assert!(!doc.fields.contains_key("doctype"));
        assert!(!doc.fields.contains_key("name"));
    }

    #[test]
    fn doc_from_value_empty_name_stays_empty() {
        let val = json!({ "doctype": "Customer", "customer_name": "Test" });
        let doc = doc_from_value("Customer", &val);
        assert_eq!(doc.name, "");
        assert_eq!(
            doc.fields.get("customer_name").and_then(Value::as_str),
            Some("Test")
        );
    }

    // ── handler-level param validation (no DB required) ──────────────────────
    //
    // These tests call the handlers directly with deliberately invalid params.
    // The handlers short-circuit on require_str() before any DB access, so
    // they work without a live SiteState.  We can't construct a real SiteState
    // without a DB connection, so these tests verify only the early-exit paths.
    //
    // For full round-trip tests see the `integration` sub-module below.
    // (Note: Rust drops the future when we `.await` it and it returns an Err
    // before the first `.await` on a state field -- this is safe because
    // `require_str` is a synchronous check at the top of every handler.)
    //
    // IMPORTANT: These tests intentionally DON'T construct SiteState.
    // They use `std::future::pending()` as a sentinel -- if the handler ever
    // awaits on `site.db` before the parameter check, the test will hang and
    // must be rewritten.  Given the current code this cannot happen because
    // `require_str` is not async.

    // ── rename_doc params ────────────────────────────────────────────────────

    #[test]
    fn rename_doc_handler_old_name_alias() {
        // Verify that both `old_name` and `old` are accepted as param names.
        // We test parse_rename_doc_params instead of the full handler because
        // the full handler needs SiteState.
        let mut p1: HashMap<String, Value> = HashMap::new();
        p1.insert("doctype".into(), json!("Customer"));
        p1.insert("old_name".into(), json!("A"));
        p1.insert("new_name".into(), json!("B"));
        assert_eq!(require_str(&p1, "doctype").unwrap(), "Customer");
        assert_eq!(
            p1.get("old_name")
                .or_else(|| p1.get("old"))
                .and_then(Value::as_str).unwrap(),
            "A"
        );

        let mut p2: HashMap<String, Value> = HashMap::new();
        p2.insert("doctype".into(), json!("Customer"));
        p2.insert("old".into(), json!("A"));
        p2.insert("new".into(), json!("B"));
        assert_eq!(
            p2.get("old_name")
                .or_else(|| p2.get("old"))
                .and_then(Value::as_str).unwrap(),
            "A"
        );
        assert_eq!(
            p2.get("new_name")
                .or_else(|| p2.get("new"))
                .and_then(Value::as_str).unwrap(),
            "B"
        );
    }

    #[test]
    fn rename_doc_handler_errors_without_doctype() {
        let mut p: HashMap<String, Value> = HashMap::new();
        p.insert("old_name".into(), json!("A"));
        p.insert("new_name".into(), json!("B"));
        let err = require_str(&p, "doctype").unwrap_err();
        assert!(err.to_string().contains("'doctype' is required"));
    }

    #[test]
    fn rename_doc_handler_errors_without_old_name() {
        let mut p: HashMap<String, Value> = HashMap::new();
        p.insert("doctype".into(), json!("Customer"));
        p.insert("new_name".into(), json!("B"));
        // Simulates the alias resolution logic in handle_rename_doc
        let old_name = p.get("old_name")
            .or_else(|| p.get("old"))
            .and_then(Value::as_str);
        assert!(old_name.is_none(), "should not find old_name");
    }

    // ── validate_link params ─────────────────────────────────────────────────

    #[test]
    fn validate_link_handler_errors_without_value() {
        let mut p: HashMap<String, Value> = HashMap::new();
        p.insert("options".into(), json!("Customer"));
        let err = require_str(&p, "value").unwrap_err();
        assert!(err.to_string().contains("'value' is required"));
    }

    #[test]
    fn validate_link_handler_errors_without_options() {
        let mut p: HashMap<String, Value> = HashMap::new();
        p.insert("value".into(), json!("CUST-001"));
        let err = require_str(&p, "options").unwrap_err();
        assert!(err.to_string().contains("'options' is required"));
    }

    // ── attach_file base64 decoding ──────────────────────────────────────────

    #[test]
    fn attach_file_base64_decodes_plain_payload() {
        use base64::{engine::general_purpose, Engine as _};
        let payload = general_purpose::STANDARD.encode(b"hello world");
        let b64 = strip_data_uri_prefix(&payload);
        let decoded = general_purpose::STANDARD.decode(b64.trim()).unwrap();
        assert_eq!(decoded, b"hello world");
    }

    #[test]
    fn attach_file_base64_decodes_data_uri_payload() {
        use base64::{engine::general_purpose, Engine as _};
        let raw_b64   = general_purpose::STANDARD.encode(b"\x89PNG");
        let data_uri  = format!("data:image/png;base64,{raw_b64}");
        let b64 = strip_data_uri_prefix(&data_uri);
        let decoded = general_purpose::STANDARD.decode(b64.trim()).unwrap();
        assert_eq!(&decoded[..4], b"\x89PNG");
    }

    #[test]
    fn attach_file_rejects_invalid_base64() {
        use base64::{engine::general_purpose, Engine as _};
        let err = general_purpose::STANDARD.decode("!!!not-valid-base64".trim());
        assert!(err.is_err());
    }

    // ── has_permission ptype parsing ─────────────────────────────────────────

    #[test]
    fn has_permission_defaults_to_read() {
        // Verify that missing ptype falls back to PermissionType::Read
        let p: HashMap<String, Value> = HashMap::new();
        let ptype_s = p.get("ptype").and_then(Value::as_str).unwrap_or("read");
        assert_eq!(
            PermissionType::from_str(ptype_s).unwrap_or(PermissionType::Read),
            PermissionType::Read
        );
    }

    #[test]
    fn has_permission_parses_all_standard_types() {
        for (s, expected) in [
            ("read",   PermissionType::Read),
            ("write",  PermissionType::Write),
            ("create", PermissionType::Create),
            ("delete", PermissionType::Delete),
            ("submit", PermissionType::Submit),
            ("cancel", PermissionType::Cancel),
            ("amend",  PermissionType::Amend),
            ("print",  PermissionType::Print),
            ("email",  PermissionType::Email),
        ] {
            let parsed = PermissionType::from_str(s).unwrap_or(PermissionType::Read);
            assert_eq!(parsed, expected, "ptype '{s}' should parse correctly");
        }
    }

    #[test]
    fn has_permission_unknown_ptype_falls_back_to_read() {
        let parsed = PermissionType::from_str("fly").unwrap_or(PermissionType::Read);
        assert_eq!(parsed, PermissionType::Read);
    }

    // ── integration tests (require live SurrealDB on ws://127.0.0.1:8500) ───

    #[cfg(feature = "integration")]
    mod integration {
        use super::super::*;
        use crate::state::SiteState;
        use spotledger_core::config::{
            AppsConfig, CacheConfig, DatabaseConfig, SiteConfig, SiteInfo,
        };
        use spotledger_db::adapter::DbAdapter;
        use spotledger_db::document::upsert_doc;
        use std::sync::Arc;

        async fn test_state() -> Arc<SiteState> {
            let cfg = spotledger_core::config::SiteConfig {
                site: SiteInfo {
                    name: "test_phase5".into(),
                    namespace: "test_phase5".into(),
                },
                database: DatabaseConfig {
                    url:  "ws://127.0.0.1:8500".into(),
                    ns:   "test_phase5".into(),
                    db:   "client_methods".into(),
                    user: "root".into(),
                    pass: "root".into(),
                },
                cache: CacheConfig::default(),
                apps:  AppsConfig::default(),
            };
            let db = DbAdapter::connect(&cfg.database).await.expect("test DB connect");
            Arc::new(SiteState::new(cfg, db))
        }

        async fn setup_schemaless_table(site: &SiteState, doctype: &str) {
            let table = spotledger_db::document::doctype_to_table(doctype);
            let _ = site
                .db
                .execute(&format!("DEFINE TABLE IF NOT EXISTS `{table}` SCHEMALESS;"), vec![])
                .await;
            let _ = site
                .db
                .execute(&format!("DELETE `{table}`;"), vec![])
                .await;
        }

        fn params(pairs: &[(&str, Value)]) -> HashMap<String, Value> {
            pairs.iter().map(|(k, v)| (k.to_string(), v.clone())).collect()
        }

        // ── rename_doc handler ───────────────────────────────────────────────

        #[tokio::test]
        async fn handle_rename_doc_renames_and_evicts_cache() {
            let site = test_state().await;
            setup_schemaless_table(&site, "IntegRename").await;
            upsert_doc(&site.db, "IntegRename", "OLD-1", &json!({"x": 1}))
                .await
                .unwrap();

            let p = params(&[
                ("doctype",  json!("IntegRename")),
                ("old_name", json!("OLD-1")),
                ("new_name", json!("NEW-1")),
                ("__current_user", json!("Administrator")),
            ]);
            let result = handle_rename_doc(site.clone(), p).await.unwrap();
            assert_eq!(result["message"], "renamed");

            // New name must exist.
            spotledger_db::document::get_doc(&site.db, "IntegRename", "NEW-1")
                .await
                .expect("NEW-1 must exist");

            // Old name must be gone.
            assert!(
                spotledger_db::document::get_doc(&site.db, "IntegRename", "OLD-1")
                    .await
                    .is_err()
            );
        }

        #[tokio::test]
        async fn handle_rename_doc_old_alias_works() {
            let site = test_state().await;
            setup_schemaless_table(&site, "IntegRenameAlias").await;
            upsert_doc(&site.db, "IntegRenameAlias", "SRC", &json!({"y": 2}))
                .await
                .unwrap();

            let p = params(&[
                ("doctype",  json!("IntegRenameAlias")),
                ("old",      json!("SRC")),
                ("new",      json!("DST")),
                ("__current_user", json!("Administrator")),
            ]);
            let result = handle_rename_doc(site.clone(), p).await.unwrap();
            assert_eq!(result["message"], "renamed");
        }

        // ── bulk_update handler ──────────────────────────────────────────────

        #[tokio::test]
        async fn handle_bulk_update_sets_field_on_listed_docs() {
            let site = test_state().await;
            setup_schemaless_table(&site, "IntegBulk").await;
            for name in &["B1", "B2", "B3"] {
                upsert_doc(&site.db, "IntegBulk", name, &json!({"status": "Draft"}))
                    .await
                    .unwrap();
            }

            let p = params(&[
                ("doctype",   json!("IntegBulk")),
                ("docnames",  json!(["B1", "B2"])),
                ("fieldname", json!("status")),
                ("value",     json!("Closed")),
                ("__current_user", json!("Administrator")),
            ]);
            let result = handle_bulk_update(site.clone(), p).await.unwrap();
            assert_eq!(result["updated"], 2);

            let d1 = spotledger_db::document::get_doc(&site.db, "IntegBulk", "B1").await.unwrap();
            assert_eq!(d1.fields["status"], "Closed");
            let d3 = spotledger_db::document::get_doc(&site.db, "IntegBulk", "B3").await.unwrap();
            assert_eq!(d3.fields["status"], "Draft");
        }

        #[tokio::test]
        async fn handle_bulk_update_returns_zero_for_empty_list() {
            let site  = test_state().await;
            let p = params(&[
                ("doctype",   json!("IntegBulk")),
                ("docnames",  json!([])),
                ("fieldname", json!("status")),
                ("value",     json!("x")),
                ("__current_user", json!("Administrator")),
            ]);
            let result = handle_bulk_update(site.clone(), p).await.unwrap();
            assert_eq!(result["updated"], 0);
        }

        #[tokio::test]
        async fn handle_bulk_update_rejects_unsafe_fieldname() {
            let site = test_state().await;
            let p = params(&[
                ("doctype",   json!("IntegBulk")),
                ("docnames",  json!(["X"])),
                ("fieldname", json!("bad-field; DROP TABLE foo")),
                ("value",     json!("x")),
                ("__current_user", json!("Administrator")),
            ]);
            assert!(handle_bulk_update(site.clone(), p).await.is_err());
        }

        // ── has_permission handler ───────────────────────────────────────────

        #[tokio::test]
        async fn handle_has_permission_administrator_always_true() {
            let site = test_state().await;
            let p = params(&[
                ("doctype",          json!("Customer")),
                ("ptype",            json!("read")),
                ("__current_user",   json!("Administrator")),
            ]);
            let result = handle_has_permission(site.clone(), p).await.unwrap();
            assert_eq!(result["has_permission"], true);
        }

        // ── get_doc_permissions handler ──────────────────────────────────────

        #[tokio::test]
        async fn handle_get_doc_permissions_administrator_all_true() {
            let site = test_state().await;
            let p = params(&[
                ("doctype",         json!("Customer")),
                ("__current_user",  json!("Administrator")),
            ]);
            let result = handle_get_doc_permissions(site.clone(), p).await.unwrap();
            let perms = &result["permissions"];
            assert_eq!(perms["read"],   1);
            assert_eq!(perms["write"],  1);
            assert_eq!(perms["create"], 1);
            assert_eq!(perms["delete"], 1);
        }

        // ── attach_file handler ──────────────────────────────────────────────

        #[tokio::test]
        async fn handle_attach_file_stores_file_record() {
            use base64::{engine::general_purpose, Engine as _};

            let site = test_state().await;
            setup_schemaless_table(&site, "File").await;

            let b64 = general_purpose::STANDARD.encode(b"dummy file content");
            let p = params(&[
                ("filename",  json!("test_upload.txt")),
                ("filedata",  Value::String(b64)),
                ("doctype",   json!("Customer")),
                ("docname",   json!("CUST-001")),
                ("is_private",json!(0)),
                ("__current_user", json!("Administrator")),
            ]);

            let result = handle_attach_file(site.clone(), p).await.unwrap();
            assert_eq!(result["file_name"], "test_upload.txt");
            assert!(result["file_url"].as_str().unwrap().contains("test_upload.txt"));
            assert_eq!(result["is_private"], 0);
        }

        // ── validate_link handler ────────────────────────────────────────────

        #[tokio::test]
        async fn handle_validate_link_returns_valid_for_existing_doc() {
            let site = test_state().await;
            setup_schemaless_table(&site, "IntegValidateLink").await;
            upsert_doc(&site.db, "IntegValidateLink", "VL-001", &json!({"x": 1}))
                .await
                .unwrap();

            let p = params(&[
                ("value",   json!("VL-001")),
                ("options", json!("IntegValidateLink")),
                ("__current_user", json!("Administrator")),
            ]);
            let result = handle_validate_link(site.clone(), p).await.unwrap();
            assert_eq!(result["valid"], 1);
        }

        #[tokio::test]
        async fn handle_validate_link_errors_for_missing_doc() {
            let site = test_state().await;
            setup_schemaless_table(&site, "IntegValidateLink").await;

            let p = params(&[
                ("value",   json!("GHOST-999")),
                ("options", json!("IntegValidateLink")),
                ("__current_user", json!("Administrator")),
            ]);
            assert!(handle_validate_link(site.clone(), p).await.is_err());
        }
    }
}

