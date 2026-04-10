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
use spotledger_db::save_proxy::{save_doc_proxy, submit_doc_proxy, cancel_doc_proxy};
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
