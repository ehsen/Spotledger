//! Tier 1 `frappe.client.*` method handlers.
//! These mirror the methods in frappe/client.py.

use super::{BoxFuture, MethodRegistry};
use crate::state::SiteState;
use serde_json::Value;
use spotledger_db::controller::{delete_doc_checked, get_compiled_meta, save_doc};
use spotledger_db::document::{
    cancel_doc, get_count, get_doc, get_list, get_value, set_field, submit_doc, upsert_doc,
};
use spotledger_db::hooks::{
    run_before_cancel_hooks, run_before_submit_hooks, run_on_cancel_hooks, run_on_submit_hooks,
};
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

    // Build the in-memory Document from the incoming JSON
    let doc = doc_from_value(&doctype, &doc_val);

    // Look up compiled meta (fall back to hook-only pipeline if not compiled)
    let saved = if let Some(meta) = get_compiled_meta(&doctype) {
        save_doc(&site.db, &site.hook_registry, &meta, doc, &user).await?
    } else {
        save_doc_no_meta(&site, doc, &user).await?
    };

    // Invalidate cache
    site.doc_cache
        .remove(&(doctype.clone(), saved.name.clone()))
        .await;

    // Activity log (best-effort — failures are silently ignored)
    let action = if saved.fields.get("creation") == saved.fields.get("modified") {
        "created"
    } else {
        "saved"
    };
    let _ = site
        .db
        .execute(
            "RETURN fn::log_activity($user, $dt, $dn, $action, $data)",
            vec![
                ("user".into(),   user.into()),
                ("dt".into(),     doctype.into()),
                ("dn".into(),     saved.name.clone().into()),
                ("action".into(), action.into()),
                ("data".into(),   Value::Null),
            ],
        )
        .await;

    Ok(saved.as_dict())
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

    // Force is_new by clearing name so the controller assigns one via naming series
    let mut doc = doc_from_value(&doctype, &doc_val);
    doc.name.clear();
    doc.fields.insert("__islocal".into(), Value::Number(1.into()));

    let saved = if let Some(meta) = get_compiled_meta(&doctype) {
        save_doc(&site.db, &site.hook_registry, &meta, doc, &user).await?
    } else {
        save_doc_no_meta(&site, doc, &user).await?
    };

    site.doc_cache
        .remove(&(doctype.clone(), saved.name.clone()))
        .await;

    let _ = site
        .db
        .execute(
            "RETURN fn::log_activity($user, $dt, $dn, $action, $data)",
            vec![
                ("user".into(),   user.into()),
                ("dt".into(),     doctype.into()),
                ("dn".into(),     saved.name.clone().into()),
                ("action".into(), "created".into()),
                ("data".into(),   Value::Null),
            ],
        )
        .await;

    Ok(saved.as_dict())
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

    // Fetch latest from DB
    let doc = get_doc(&site.db, &doctype, &name)
        .await
        .map_err(SpotError::from)?;

    // Run before_submit + validate hooks
    let doc = run_before_submit_hooks(&site.hook_registry, doc).await?;

    // DB: set docstatus = 1 (also re-saves any hook-modified fields)
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

    site.doc_cache.remove(&(doctype.clone(), name.clone())).await;

    // on_submit hooks
    let submitted = run_on_submit_hooks(&site.hook_registry, submitted).await?;
    Ok(submitted.as_dict())
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

    let doc = get_doc(&site.db, &doctype, &name)
        .await
        .map_err(SpotError::from)?;

    let _doc = run_before_cancel_hooks(&site.hook_registry, doc).await?;

    let cancelled = cancel_doc(&site.db, &doctype, &name)
        .await
        .map_err(SpotError::from)?;

    site.doc_cache.remove(&(doctype.clone(), name.clone())).await;

    let cancelled = run_on_cancel_hooks(&site.hook_registry, cancelled).await?;
    Ok(cancelled.as_dict())
}

// ── helpers ───────────────────────────────────────────────────────────────────

fn require_str<'a>(params: &'a HashMap<String, Value>, key: &str) -> Result<&'a str, SpotError> {
    params
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| SpotError::Validation(format!("'{key}' is required")))
}

/// Build a [`Document`] from the raw JSON value sent by the client.
fn doc_from_value(doctype: &str, val: &Value) -> Document {
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

/// Fallback save pipeline for DocTypes without a compiled `DocTypeMeta`.
///
/// Runs hooks only — no validation, no naming series from meta.
/// Use `get_compiled_meta` to check in advance and prefer `save_doc`.
async fn save_doc_no_meta(
    site: &Arc<SiteState>,
    doc: Document,
    user: &str,
) -> Result<Document, SpotError> {
    use spotledger_db::document::{insert_doc, upsert_doc};
    use spotledger_db::hooks::{run_after_save_hooks, run_save_hooks};
    use spotledger_db::naming::resolve_name;

    let doctype = doc.doctype.clone();
    let is_new  = doc.name.is_empty()
        || doc.fields.get("__islocal").and_then(|v| v.as_i64()).unwrap_or(0) == 1;

    let name = if is_new {
        resolve_name(&site.db, &doctype, &doc.as_dict())
            .await
            .map_err(SpotError::from)?
    } else {
        doc.name.clone()
    };

    let mut doc = doc;
    doc.name = name.clone();
    doc.set_user_and_timestamp(user, &chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string(), is_new);

    let doc = run_save_hooks(&site.hook_registry, doc, is_new).await?;
    let fields_val = doc.as_dict();

    let saved = if is_new {
        insert_doc(&site.db, &doctype, &fields_val).await.map_err(SpotError::from)?
    } else {
        upsert_doc(&site.db, &doctype, &name, &fields_val).await.map_err(SpotError::from)?
    };

    run_after_save_hooks(&site.hook_registry, saved, is_new).await
}
