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
// Return: [{"value": name, "label": title, "description": ""}]
// Mirrors frappe/desk/search.py::search_link.
//
// Search strategy (mirrors Frappe):
//   1. Always search the `name` column.
//   2. If the DocType declares a `title_field`, also search that column.
//   3. If the DocType declares `search_fields` (comma-separated), also search each.
//   4. All comparisons are case-insensitive substring (string::contains + lowercase).
//   5. When `txt` is empty, return the first `page_length` records with no filter.
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

    // Accept both `txt` (Frappe) and `query` as a defensive fallback.
    // Values may be parsed as non-strings by parse_form_params (e.g. "1200" -> Number).
    let txt = params
        .get("txt")
        .or_else(|| params.get("query"))
        .map(value_to_search_text)
        .unwrap_or_default();
    let page_length = params
        .get("page_length")
        .and_then(value_to_u64)
        .unwrap_or(10) as usize;

    tracing::debug!(
        doctype = %doctype,
        txt = %txt,
        txt_raw = ?params.get("txt"),
        query_raw = ?params.get("query"),
        page_length,
        "search_link params"
    );

    // ── server-side cache ────────────────────────────────────────────────────
    let cache_key = (doctype.clone(), format!("{txt}:{page_length}"));
    // Cache only the empty-query bootstrap list. Typed autocomplete should stay
    // fresh and must not be affected by stale intermediate values.
    if txt.is_empty() {
        if let Some(cached) = site.search_cache.get(&cache_key).await {
            return Ok(cached);
        }
    }

    // ── resolve title_field / search_fields ──────────────────────────────────
    let (title_field, search_fields) = resolve_search_fields(&site, &doctype).await;

    // ── build SELECT field list ───────────────────────────────────────────────
    // Always include `name`.  Add title_field and each search_field if distinct.
    let mut select_cols: Vec<String> = vec!["name".to_string()];
    if title_field != "name" {
        select_cols.push(format!("`{title_field}`"));
    }
    for sf in &search_fields {
        if sf != "name" && sf != &title_field {
            select_cols.push(format!("`{sf}`"));
        }
    }
    let field_clause = select_cols.join(", ");

    let table = spotledger_db::document::doctype_to_table(&doctype);

    // ── build WHERE clause ────────────────────────────────────────────────────
    // When txt is non-empty, use case-insensitive substring matching across all
    // searchable columns joined by OR.  This mirrors Frappe's behaviour and works
    // correctly in SurrealDB v3 (LIKE with % is case-sensitive and unreliable).
    let (sql, bindings) = if txt.is_empty() {
        // No filter — return first N records so the dropdown is immediately populated.
        let sql = format!("SELECT {field_clause} FROM `{table}` LIMIT {page_length}");
        (sql, vec![])
    } else {
        // Build OR conditions: one per searchable field.
        // Use an inlined, safely escaped literal for q because parameter binding
        // has been unreliable in some SurrealDB v3 runtime paths.
        let txt_lower = txt.to_lowercase();
        let q_lit = surql_string_literal(&txt_lower);

        let mut searchable: Vec<String> = vec!["name".to_string()];
        if title_field != "name" {
            searchable.push(title_field.clone());
        }
        for sf in search_fields.iter() {
            if sf != "name" && sf != &title_field {
                searchable.push(sf.clone());
            }
        }

        // `field ?? ""` — null-coalescing: returns "" if field is NONE/absent.
        // `<string>(...)` makes matching stable even if the source column is not a string.
        // string::lowercase + string::contains = case-insensitive substring match.
        let conditions: Vec<String> = searchable
            .iter()
            .map(|col| {
                format!(
                    "string::contains(string::lowercase(<string>(`{col}` ?? \"\")), {q_lit})"
                )
            })
            .collect();

        let where_clause = conditions.join(" OR ");
        let sql = format!(
            "SELECT {field_clause} FROM `{table}` WHERE {where_clause} LIMIT {page_length}"
        );
        (sql, vec![])
    };

    tracing::debug!(%sql, ?bindings, "search_link sql");

    let rows = match site.db.run(&sql, bindings).await {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!(error = %e, %sql, "search_link query failed");
            vec![]
        }
    };

    // ── map rows → result objects ─────────────────────────────────────────────
    let results: Vec<Value> = rows
        .into_iter()
        .filter_map(|r| {
            let name = r.get("name").and_then(Value::as_str)?.to_string();
            if name.is_empty() { return None; }
            let label = if title_field != "name" {
                r.get(&title_field)
                    .and_then(Value::as_str)
                    .filter(|s| !s.is_empty())
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| name.clone())
            } else {
                name.clone()
            };
            let description = search_fields
                .iter()
                .filter(|sf| *sf != "name" && *sf != &title_field)
                .filter_map(|sf| r.get(sf).and_then(Value::as_str))
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>()
                .join(", ");
            Some(json!({
                "value":       name,
                "label":       label,
                "description": description,
            }))
        })
        .collect();

    let response = json!(results);
    if txt.is_empty() {
        site.search_cache.insert(cache_key, response.clone()).await;
    }
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

fn value_to_search_text(v: &Value) -> String {
    match v {
        Value::String(s) => s.trim().to_string(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Null => String::new(),
        other => other.to_string().trim_matches('"').to_string(),
    }
}

fn value_to_u64(v: &Value) -> Option<u64> {
    match v {
        Value::Number(n) => n.as_u64(),
        Value::String(s) => s.parse::<u64>().ok(),
        _ => None,
    }
}

fn surql_string_literal(s: &str) -> String {
    let escaped = s.replace('\\', "\\\\").replace('\'', "\\'");
    format!("'{escaped}'")
}
