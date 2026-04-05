//! Tier 1 `frappe.client.*` method handlers.
//! These mirror the methods in frappe/client.py.

use super::{BoxFuture, MethodRegistry};
use crate::state::SiteState;
use serde_json::Value;
use spotledger_db::child_table::{extract_child_tables, save_children};
use spotledger_db::document::{
    cancel_doc, delete_doc, get_count, get_doc, get_list, get_value, insert_doc, set_field,
    submit_doc, upsert_doc,
};
use spotledger_db::hooks::{
    run_after_save_hooks, run_before_cancel_hooks, run_before_submit_hooks, run_on_cancel_hooks,
    run_on_submit_hooks, run_save_hooks,
};
use spotledger_db::naming::resolve_name;
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
    // The Desk JS sends `doc` as a JSON-encoded string or Value
    let doc_val = params
        .get("doc")
        .ok_or_else(|| SpotError::Validation("'doc' is required".into()))?
        .clone();

    let doctype = doc_val
        .get("doctype")
        .and_then(Value::as_str)
        .ok_or_else(|| SpotError::Validation("doc.doctype is required".into()))?
        .to_owned();

    let raw_name = doc_val
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_owned();

    let is_new = raw_name.is_empty();
    let user = params.get("__current_user").and_then(Value::as_str).unwrap_or("Administrator").to_owned();

    // Resolve name for new docs via naming series
    let name = if is_new {
        resolve_name(&site.db, &doctype, &doc_val)
            .await
            .map_err(SpotError::from)?
    } else {
        raw_name.clone()
    };

    // Build Document struct for hook dispatch
    let mut doc = Document { doctype: doctype.clone(), name: name.clone(), fields: Default::default() };
    if let Value::Object(map) = &doc_val {
        for (k, v) in map {
            if k != "doctype" && k != "name" {
                doc.set(k.clone(), v.clone());
            }
        }
    }

    // Run before-save hooks
    let doc = run_save_hooks(&site.hook_registry, doc, is_new).await?;

    // Snapshot old doc before write (for version diff on updates)
    let old_doc_snap: Option<Value> = if !is_new {
        spotledger_db::document::get_doc(&site.db, &doctype, &name)
            .await
            .ok()
            .map(|d| d.as_dict())
    } else {
        None
    };

    // DB write
    let mut fields_val = doc.as_dict();
    if let Value::Object(ref mut m) = fields_val {
        m.insert("name".into(), Value::String(name.clone()));
    }

    let saved = if is_new {
        insert_doc(&site.db, &doctype, &fields_val)
            .await
            .map_err(SpotError::from)?
    } else {
        upsert_doc(&site.db, &doctype, &name, &fields_val)
            .await
            .map_err(SpotError::from)?
    };

    // Scatter child table arrays into their own tables
    for (parentfield, child_doctype, children) in extract_child_tables(&fields_val) {
        let children_vals: Vec<Value> = children.into_iter().collect();
        save_children(&site.db, &child_doctype, &name, &doctype, &parentfield, &children_vals)
            .await
            .map_err(SpotError::from)?;
    }

    // Invalidate cache
    site.doc_cache.remove(&(doctype.clone(), name.clone())).await;

    // B6: create version snapshot on update, log activity for both paths
    let action = if is_new { "created" } else { "saved" };
    let new_snap = saved.as_dict();
    if !is_new {
        if let Some(old) = old_doc_snap {
            let _ = site.db
                .execute(
                    "RETURN fn::create_version($user, $dt, $dn, $old, $new)",
                    vec![
                        ("user".into(), user.clone().into()),
                        ("dt".into(),   doctype.clone().into()),
                        ("dn".into(),   name.clone().into()),
                        ("old".into(),  old),
                        ("new".into(),  new_snap),
                    ],
                )
                .await;
        }
    }
    let _ = site.db
        .execute(
            "RETURN fn::log_activity($user, $dt, $dn, $action, $data)",
            vec![
                ("user".into(),   user.clone().into()),
                ("dt".into(),     doctype.clone().into()),
                ("dn".into(),     name.clone().into()),
                ("action".into(), action.into()),
                ("data".into(),   Value::Null),
            ],
        )
        .await;

    // Run after-save hooks
    let saved = run_after_save_hooks(&site.hook_registry, saved, is_new).await?;

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

    // Resolve name
    let name = resolve_name(&site.db, &doctype, &doc_val)
        .await
        .map_err(SpotError::from)?;

    let mut doc = Document { doctype: doctype.clone(), name: name.clone(), fields: Default::default() };
    if let Value::Object(map) = &doc_val {
        for (k, v) in map {
            if k != "doctype" && k != "name" {
                doc.set(k.clone(), v.clone());
            }
        }
    }

    let doc = run_save_hooks(&site.hook_registry, doc, true).await?;

    let mut fields_val = doc.as_dict();
    if let Value::Object(ref mut m) = fields_val {
        m.insert("name".into(), Value::String(name.clone()));
    }

    let inserted = insert_doc(&site.db, &doctype, &fields_val)
        .await
        .map_err(SpotError::from)?;

    // Scatter child tables
    for (parentfield, child_doctype, children) in extract_child_tables(&fields_val) {
        let children_vals: Vec<Value> = children.into_iter().collect();
        save_children(&site.db, &child_doctype, &name, &doctype, &parentfield, &children_vals)
            .await
            .map_err(SpotError::from)?;
    }

    site.doc_cache.remove(&(doctype.clone(), name.clone())).await;

    // B6: log activity for insert path
    let user = params.get("__current_user").and_then(Value::as_str).unwrap_or("Administrator").to_owned();
    let _ = site.db
        .execute(
            "RETURN fn::log_activity($user, $dt, $dn, $action, $data)",
            vec![
                ("user".into(),   user.into()),
                ("dt".into(),     doctype.clone().into()),
                ("dn".into(),     name.clone().into()),
                ("action".into(), "created".into()),
                ("data".into(),   Value::Null),
            ],
        )
        .await;

    let inserted = run_after_save_hooks(&site.hook_registry, inserted, true).await?;
    Ok(inserted.as_dict())
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
    let name = require_str(&params, "name")?;
    delete_doc(&site.db, doctype, name).await.map_err(SpotError::from)?;
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
