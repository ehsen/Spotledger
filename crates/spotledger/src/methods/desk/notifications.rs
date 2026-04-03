//! frappe.desk.notifications.* and frappe.desk.doctype.notification_log.* handlers
//!
//! Python reference:
//!   frappe/frappe/desk/notifications.py
//!   frappe/frappe/desk/doctype/notification_log/notification_log.py

use crate::state::SiteState;
use serde_json::{json, Value};
use spotledger_db::connection::Db;
use spotledger_db::document::{doctype_to_table, get_list};
use spotledger_types::error::SpotError;
use std::collections::HashMap;
use std::sync::Arc;

// ── frappe.desk.notifications.get_notification_info ──────────────────────────
// Powers the notification bell badge counts in the Desk top bar.
// Python returns a dict with open_count_doctype, open_count_todo, targets, etc.
pub async fn handle_get_notification_info(
    site: Arc<SiteState>,
    _params: HashMap<String, Value>,
) -> Result<Value, SpotError> {
    // Count open ToDos for the current user (Administrator for now)
    let todo_filter = json!({"status": "Open", "allocated_to": "Administrator"});
    let todo_count = count_records(&site.db, "ToDo", Some(&todo_filter)).await;

    // Count unread Workflow Actions
    let workflow_filter = json!({"status": "Open"});
    let workflow_count = count_records(&site.db, "Workflow Action", Some(&workflow_filter)).await;

    Ok(json!({
        "open_count_doctype": {
            "Error Log":       0,
            "Communication":   0,
            "ToDo":            todo_count,
            "Event":           0,
            "Workflow Action": workflow_count,
        },
        "targets":    {},
        "conditions": {
            "Error Log": {"seen": 0},
            "Communication": {"status": "Open", "communication_type": "Communication"},
            "ToDo": "frappe.core.notifications.get_things_todo",
            "Event": "frappe.core.notifications.get_todays_events",
            "Workflow Action": {"status": "Open"},
        },
        "module_doctypes": {},
        "open_count_todo": todo_count,
        "open_count_mention": 0,
        "mentions": [],
        "energy_points": 0,
    }))
}

// ── frappe.desk.notifications.get_open_count ─────────────────────────────────
// Returns the notification open count for a specific doctype+name.
// Used by the form indicator dot.
pub async fn handle_get_open_count(
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
        return Ok(json!({"count": 0, "open_count": 0}));
    }

    // Count open todos pointing at this document
    let todo_filter = json!({
        "reference_type": doctype,
        "reference_name": name,
        "status": "Open",
    });
    let count = count_records(&site.db, "ToDo", Some(&todo_filter)).await;

    Ok(json!({"count": count, "open_count": count}))
}

// ── frappe.desk.doctype.notification_log.notification_log.get_notification_logs
// Powers the notification panel (bell dropdown).
// Returns: {"notification_logs": [...], "unread_count": N}
pub async fn handle_get_notification_logs(
    site: Arc<SiteState>,
    params: HashMap<String, Value>,
) -> Result<Value, SpotError> {
    let limit = params
        .get("limit")
        .and_then(Value::as_u64)
        .unwrap_or(5) as usize;
    let user = "Administrator"; // TODO: extract from session

    let filter = json!({"for_user": user});
    let rows = get_list(
        &site.db,
        "Notification Log",
        Some(&["name", "subject", "for_user", "document_type", "document_name",
               "from_user", "type", "read", "creation"]),
        Some(&filter),
        limit,
        0,
    )
    .await
    .unwrap_or_default();

    let unread_count = rows
        .iter()
        .filter(|r| {
            r.get("read")
                .map(|v| v == &Value::Bool(false) || v == &Value::Number(0.into()))
                .unwrap_or(true) // unread if field missing
        })
        .count() as u64;

    let logs: Vec<Value> = rows
        .into_iter()
        .map(|r| Value::Object(r.into_iter().collect()))
        .collect();

    Ok(json!({
        "notification_logs": logs,
        "unread_count":      unread_count,
    }))
}

// ── frappe.desk.doctype.notification_log.notification_log.mark_as_read ───────
pub async fn handle_mark_notification_as_read(
    site: Arc<SiteState>,
    params: HashMap<String, Value>,
) -> Result<Value, SpotError> {
    let name = params
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    if !name.is_empty() {
        let table = doctype_to_table("Notification Log");
        let _ = site
            .db
            .query(format!(
                "UPDATE `{table}` SET `read` = true WHERE name = $name"
            ))
            .bind(("name", name))
            .await;
    }
    Ok(Value::Null)
}

// ── frappe.desk.doctype.notification_log.notification_log.mark_all_as_read ───
pub async fn handle_mark_all_notifications_as_read(
    site: Arc<SiteState>,
    _params: HashMap<String, Value>,
) -> Result<Value, SpotError> {
    let user = "Administrator";
    let table = doctype_to_table("Notification Log");
    let _ = site
        .db
        .query(format!(
            "UPDATE `{table}` SET `read` = true WHERE for_user = $user"
        ))
        .bind(("user", user.to_owned()))
        .await;
    Ok(Value::Null)
}

// ── frappe.desk.doctype.notification_log.notification_log.trigger_indicator_hide
pub async fn handle_trigger_indicator_hide(
    _site: Arc<SiteState>,
    _params: HashMap<String, Value>,
) -> Result<Value, SpotError> {
    Ok(Value::Null)
}

// ── frappe.desk.doctype.notification_settings.notification_settings.set_seen_value
pub async fn handle_set_notification_seen(
    _site: Arc<SiteState>,
    _params: HashMap<String, Value>,
) -> Result<Value, SpotError> {
    Ok(Value::Null)
}

// ── Shared helper ─────────────────────────────────────────────────────────────

async fn count_records(
    db: &Db,
    doctype: &str,
    filter: Option<&Value>,
) -> u64 {
    let table = doctype_to_table(doctype);
    let (where_clause, bindings) = spotledger_db::document::build_where(filter);
    let surql = format!("SELECT count() FROM `{table}`{where_clause} GROUP ALL");
    let mut q = db.query(&surql);
    for (k, v) in bindings {
        q = q.bind((k, v));
    }
    match q.await {
        Ok(mut resp) => {
            let rows: Vec<Value> = resp.take(0).unwrap_or_default();
            rows.first()
                .and_then(|r: &Value| r.get("count"))
                .and_then(Value::as_u64)
                .unwrap_or(0)
        }
        Err(_) => 0,
    }
}
