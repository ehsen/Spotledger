//! frappe.desk.search.* handlers
//!
//! search_link / search_widget — Link-field autocomplete used on every form.
//! get_link_title               — Resolve display title for a Link field value.
//! get_names_for_mentions       — @-mention autocomplete in comments.
//!
//! Python reference:
//!   frappe/frappe/desk/search.py

use crate::state::SiteState;
use serde_json::{json, Value};
use spotledger_db::document::{get_doc, get_list};
use spotledger_core::error::SpotError;
use std::collections::HashMap;
use std::sync::Arc;

// ── frappe.desk.search.search_link ───────────────────────────────────────────
// Return: {"results": [{"value": name, "label": title, "description": ""}]}
pub async fn handle_search_link(
    site: Arc<SiteState>,
    params: HashMap<String, Value>,
) -> Result<Value, SpotError> {
    let doctype = params
        .get("doctype")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    if doctype.is_empty() {
        return Ok(json!([]));
    }

    let txt = params
        .get("txt")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let page_length = params
        .get("page_length")
        .and_then(|v| v.as_u64())
        .unwrap_or(10) as usize;

    // ── server-side cache (avoids repeated DB round-trips for same query) ──
    // Cache key includes page_length so different limit calls don't collide.
    let cache_key = (
        doctype.clone(),
        format!("{txt}:{page_length}"),
    );
    if let Some(cached) = site.search_cache.get(&cache_key).await {
        return Ok(cached);
    }

    // Determine the title field so we can return a useful label.
    let (title_field, search_fields) = resolve_search_fields(&site, &doctype).await;

    // Build a LIKE filter on name  (simple case — no custom query support yet)
    let filter = if txt.is_empty() {
        json!({})
    } else {
        json!({"name": ["like", format!("%{txt}%")]})
    };

    let mut fetch_fields = vec!["name"];
    if title_field != "name" {
        fetch_fields.push(&title_field);
    }
    for sf in &search_fields {
        if !fetch_fields.contains(&sf.as_str()) {
            fetch_fields.push(sf.as_str());
        }
    }

    let rows = get_list(
        &site.db,
        &doctype,
        Some(&fetch_fields),
        Some(&filter),
        page_length,
        0,
    )
    .await
    .unwrap_or_default();

    let results: Vec<Value> = rows
        .into_iter()
        .map(|r| {
            let name = r
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            let label = if title_field != "name" {
                r.get(&title_field)
                    .and_then(Value::as_str)
                    .filter(|s| !s.is_empty())
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| name.clone())
            } else {
                name.clone()
            };
            // Build description from extra search fields
            let description = search_fields
                .iter()
                .filter(|sf| *sf != "name" && *sf != &title_field)
                .filter_map(|sf| r.get(sf).and_then(Value::as_str))
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>()
                .join(", ");
            json!({
                "value":       name,
                "label":       label,
                "description": description,
            })
        })
        .collect();

    // Frappe returns the array directly as r.message (link.js calls results.reduce on it)
    let response = json!(results);
    // Store in cache for subsequent identical queries
    site.search_cache.insert(cache_key, response.clone()).await;
    Ok(response)
}

// search_widget is identical to search_link in its response shape
pub async fn handle_search_widget(
    site: Arc<SiteState>,
    params: HashMap<String, Value>,
) -> Result<Value, SpotError> {
    handle_search_link(site, params).await
}

// ── frappe.desk.search.get_link_title ────────────────────────────────────────
// Python: if meta.show_title_field_in_link: return db.get_value(doctype, name,
//         meta.title_field)
//         else: return docname
// Return: {"message": title_string}
pub async fn handle_get_link_title(
    site: Arc<SiteState>,
    params: HashMap<String, Value>,
) -> Result<Value, SpotError> {
    let doctype = params
        .get("doctype")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let name = params
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();

    if doctype.is_empty() || name.is_empty() {
        return Ok(Value::Null);
    }

    // Check if the DocType has show_title_field_in_link set.
    if let Ok(dt_doc) = get_doc(&site.db, "DocType", &doctype).await {
        let show_title = dt_doc
            .get_str("show_title_field_in_link")
            .map(|v| v == "1" || v == "true")
            .unwrap_or(false);
        if show_title {
            let title_field = dt_doc
                .get_str("title_field")
                .filter(|s| !s.is_empty())
                .unwrap_or("name")
                .to_string();
            if let Ok(doc) = get_doc(&site.db, &doctype, &name).await {
                let title = doc
                    .get_str(&title_field)
                    .filter(|s| !s.is_empty())
                    .unwrap_or(&name)
                    .to_string();
                return Ok(Value::String(title));
            }
        }
    }

    // Fallback: return the name itself (Frappe does the same)
    Ok(Value::String(name))
}

// ── frappe.desk.search.get_names_for_mentions ────────────────────────────────
// Used by the rich-text comment editor for @-mention autocomplete.
// Returns [{id, value, link}]  — non-admin users whose allowed_in_mentions=1.
pub async fn handle_get_names_for_mentions(
    site: Arc<SiteState>,
    params: HashMap<String, Value>,
) -> Result<Value, SpotError> {
    let search_term = params
        .get("search_term")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_lowercase();

    let filter = if search_term.is_empty() {
        json!({"allowed_in_mentions": 1, "user_type": "System User", "enabled": 1})
    } else {
        json!({
            "allowed_in_mentions": 1,
            "user_type": "System User",
            "enabled": 1,
            "full_name": ["like", format!("%{search_term}%")]
        })
    };

    let rows = get_list(
        &site.db,
        "User",
        Some(&["name", "full_name"]),
        Some(&filter),
        20,
        0,
    )
    .await
    .unwrap_or_default();

    let results: Vec<Value> = rows
        .into_iter()
        .filter(|r| {
            let n = r.get("name").and_then(Value::as_str).unwrap_or("");
            n != "Administrator" && n != "Guest"
        })
        .map(|r| {
            let id = r
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            let value = r
                .get("full_name")
                .and_then(Value::as_str)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .unwrap_or_else(|| id.clone());
            json!({
                "id":    id,
                "value": value,
                "link":  format!("/app/user/{}", urlencoded(&id)),
            })
        })
        .collect();

    Ok(Value::Array(results))
}

// ── frappe.desk.search.get_search_tags ───────────────────────────────────────
pub async fn handle_get_search_tags(
    _site: Arc<SiteState>,
    _params: HashMap<String, Value>,
) -> Result<Value, SpotError> {
    Ok(Value::Array(vec![]))
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Resolve (title_field, search_fields) for a DocType from the DocType meta.
async fn resolve_search_fields(
    site: &Arc<SiteState>,
    doctype: &str,
) -> (String, Vec<String>) {
    if let Ok(dt_doc) = get_doc(&site.db, "DocType", doctype).await {
        let title_field = dt_doc
            .get_str("title_field")
            .filter(|s| !s.is_empty())
            .unwrap_or("name")
            .to_string();
        let search_fields = dt_doc
            .get_str("search_fields")
            .map(|sf| {
                sf.split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        return (title_field, search_fields);
    }
    ("name".to_string(), vec![])
}

fn urlencoded(s: &str) -> String {
    s.replace(' ', "%20")
}
