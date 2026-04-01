//! Tier 1 `frappe.client.*` method handlers.
//! These mirror the ~21 methods in frappe/client.py.

use super::{BoxFuture, MethodRegistry};
use crate::state::SiteState;
use serde_json::Value;
use spotledger_db::document::{get_doc, get_list, get_value};
use spotledger_types::error::SpotError;
use std::collections::HashMap;
use std::sync::Arc;

pub fn register_client_methods(registry: &Arc<MethodRegistry>) {
    // frappe.client.get_list
    registry.register(
        "frappe.client.get_list",
        Arc::new(|site: Arc<SiteState>, params: HashMap<String, Value>| -> BoxFuture {
            Box::pin(async move { handle_get_list(site, params).await })
        }),
    );

    // frappe.client.get
    registry.register(
        "frappe.client.get",
        Arc::new(|site: Arc<SiteState>, params: HashMap<String, Value>| -> BoxFuture {
            Box::pin(async move { handle_get(site, params).await })
        }),
    );

    // frappe.client.get_value
    registry.register(
        "frappe.client.get_value",
        Arc::new(|site: Arc<SiteState>, params: HashMap<String, Value>| -> BoxFuture {
            Box::pin(async move { handle_get_value(site, params).await })
        }),
    );

    // frappe.client.get_count
    registry.register(
        "frappe.client.get_count",
        Arc::new(|site: Arc<SiteState>, params: HashMap<String, Value>| -> BoxFuture {
            Box::pin(async move { handle_get_count(site, params).await })
        }),
    );
}

// ── frappe.client.get_list ────────────────────────────────────────────────────
//
// Accepted params (all optional except doctype):
//   doctype  : string (required)
//   fields   : JSON array of field names, or "*"  → default ["name"]
//   filters  : JSON object {"field": "val"} or {"field": [">=", val]}
//   limit    : integer (default 20)
//   limit_start / start : integer (default 0)

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

    let limit = params
        .get("limit")
        .and_then(Value::as_u64)
        .map(|n| n as usize)
        .unwrap_or(20);

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
//
//   doctype : string (required)
//   name    : string (required)

async fn handle_get(
    site: Arc<SiteState>,
    params: HashMap<String, Value>,
) -> Result<Value, SpotError> {
    let doctype = require_str(&params, "doctype")?;
    let name = require_str(&params, "name")?;

    let doc = get_doc(&site.db, doctype, name)
        .await
        .map_err(SpotError::from)?;

    Ok(doc.as_dict())
}

// ── frappe.client.get_value ───────────────────────────────────────────────────
//
//   doctype   : string (required)
//   filters   : string (name) or dict
//   fieldname : string or list of strings

async fn handle_get_value(
    site: Arc<SiteState>,
    params: HashMap<String, Value>,
) -> Result<Value, SpotError> {
    let doctype = require_str(&params, "doctype")?;

    // `filters` can be a plain name string or a dict
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

// ── frappe.client.get_count ───────────────────────────────────────────────────
//
//   doctype : string (required)
//   filters : optional JSON object

async fn handle_get_count(
    site: Arc<SiteState>,
    params: HashMap<String, Value>,
) -> Result<Value, SpotError> {
    let doctype = require_str(&params, "doctype")?;
    let table = spotledger_db::document::doctype_to_table(doctype);

    let filters = params.get("filters");
    let (where_clause, bindings) = spotledger_db::document::build_where(filters);

    // SurrealDB: SELECT count() FROM table GROUP ALL returns [{"count": N}]
    let surql = format!("SELECT count() FROM `{table}`{where_clause} GROUP ALL");
    let mut q = site.db.query(&surql);
    for (k, v) in bindings {
        q = q.bind((k, v));
    }

    let mut resp = q.await.map_err(|e| SpotError::Db(e.to_string()))?;
    let rows: Vec<Value> = resp.take(0).unwrap_or_default();

    let n = rows
        .first()
        .and_then(|obj| obj.get("count"))
        .and_then(Value::as_u64)
        .unwrap_or(0);

    Ok(Value::Number(n.into()))
}

// ── helpers ───────────────────────────────────────────────────────────────────

fn require_str<'a>(params: &'a HashMap<String, Value>, key: &str) -> Result<&'a str, SpotError> {
    params
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| SpotError::Validation(format!("'{key}' is required")))
}
