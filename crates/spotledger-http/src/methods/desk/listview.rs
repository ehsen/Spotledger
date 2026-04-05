//! frappe.desk.listview.* and frappe.desk.doctype.list_view_settings.* handlers
//!
//! Python reference:
//!   frappe/frappe/desk/listview.py
//!   frappe/frappe/desk/doctype/list_view_settings/

use crate::state::SiteState;
use serde_json::{json, Value};
use spotledger_db::document::{doctype_to_table, get_doc};
use spotledger_db::query::WhereClause;
use spotledger_core::error::SpotError;
use std::collections::HashMap;
use std::sync::Arc;

// ── frappe.desk.listview.get_list_settings ────────────────────────────────────
// Returns the List View Settings doc (column order / visible columns) for a
// doctype.  Returns null when none has been saved yet — the desk falls back to
// defaults.
// Python: frappe.get_cached_doc("List View Settings", doctype) or null.
pub async fn handle_get_list_settings(
    site: Arc<SiteState>,
    params: HashMap<String, Value>,
) -> Result<Value, SpotError> {
    let doctype = params
        .get("doctype")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    if doctype.is_empty() {
        return Ok(Value::Null);
    }

    match get_doc(&site.db, "List View Settings", &doctype).await {
        Ok(doc) => Ok(doc.as_dict()),
        Err(_) => Ok(Value::Null),
    }
}

// ── frappe.desk.doctype.list_view_settings.list_view_settings.save_listview_settings
pub async fn handle_save_listview_settings(
    site: Arc<SiteState>,
    params: HashMap<String, Value>,
) -> Result<Value, SpotError> {
    let doctype = params
        .get("doctype")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let values = params.get("values").cloned().unwrap_or(json!({}));
    if doctype.is_empty() {
        return Ok(Value::Null);
    }

    let values_map = match values {
        Value::String(s) => serde_json::from_str::<Value>(&s).unwrap_or(json!({})),
        other => other,
    };

    // Upsert the settings document
    spotledger_db::document::upsert_doc(
        &site.db,
        "List View Settings",
        &doctype,
        &values_map,
    )
    .await
    .ok();

    Ok(Value::Null)
}

// ── frappe.desk.doctype.list_view_settings.list_view_settings.get_default_listview_fields
// Returns the default visible fields for a doctype's list view.
// Python falls back to the doctype's search_fields or the first few fields.
pub async fn handle_get_default_listview_fields(
    site: Arc<SiteState>,
    params: HashMap<String, Value>,
) -> Result<Value, SpotError> {
    let doctype = params
        .get("doctype")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    if doctype.is_empty() {
        return Ok(Value::Array(vec![]));
    }

    // Check if there is a saved list view settings doc first
    if let Ok(doc) = get_doc(&site.db, "List View Settings", &doctype).await {
        if let Some(fields) = doc.as_dict().get("fields") {
            if !matches!(fields, Value::Null) {
                return Ok(fields.clone());
            }
        }
    }

    // Fallback: return search_fields from DocType meta
    if let Ok(dt_doc) = get_doc(&site.db, "DocType", &doctype).await {
        let search_fields = dt_doc
            .get_str("search_fields")
            .map(|sf| {
                sf.split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .map(|s| Value::String(s))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        if !search_fields.is_empty() {
            return Ok(Value::Array(search_fields));
        }
    }

    Ok(Value::Array(vec![]))
}

// ── frappe.desk.listview.get_group_by_count ───────────────────────────────────
// Returns count grouped by a field for the sidebar group-by panel.
// Python: frappe.get_list(doctype, filters, group_by=field, ...)
pub async fn handle_get_group_by_count(
    site: Arc<SiteState>,
    params: HashMap<String, Value>,
) -> Result<Value, SpotError> {
    let doctype = params
        .get("doctype")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let field = params
        .get("field")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let current_filters: Value = params
        .get("current_filters")
        .map(|v| match v {
            Value::String(s) => serde_json::from_str(s).unwrap_or(json!({})),
            other => other.clone(),
        })
        .unwrap_or(json!({}));

    if doctype.is_empty() || field.is_empty() {
        return Ok(Value::Array(vec![]));
    }

    let table = doctype_to_table(&doctype);
    let where_clause = WhereClause::from_filters(Some(&current_filters));

    // SurrealQL: SELECT field, count() AS count GROUP BY field
    let surql = format!(
        "SELECT `{field}` as name, count() as count FROM `{table}`{} \
         GROUP BY `{field}` ORDER BY count DESC LIMIT 50",
        where_clause.as_sql()
    );
    let bindings: Vec<(String, Value)> = where_clause.bindings().to_vec();

    let rows = match site.db.run(&surql, bindings).await {
        Ok(rows) => rows,
        Err(_)   => return Ok(Value::Array(vec![])),
    };

    Ok(Value::Array(rows))
}
