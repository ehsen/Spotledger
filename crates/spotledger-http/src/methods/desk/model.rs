//! frappe.model.utils.user_settings.* handlers
//! frappe.model.workflow.* handlers
//! frappe.model.rename_doc.* handlers
//!
//! Python reference:
//!   frappe/frappe/model/utils/user_settings.py
//!   frappe/frappe/model/workflow.py
//!   frappe/frappe/model/rename_doc.py

use crate::state::SiteState;
use serde_json::{json, Value};
use spotledger_db::document::{get_doc, get_list, upsert_doc};
use spotledger_core::error::SpotError;
use std::collections::HashMap;
use std::sync::Arc;

// ── frappe.model.utils.user_settings.get ─────────────────────────────────────
// Returns the saved per-user column preferences for a doctype.
// Frappe stores this in tabDefaultValue with defkey = doctype,
// defvalue = JSON string of column settings.
// Return shape: {"message": "{...json string...}"}  (string, not object!)
pub async fn handle_user_settings_get(
    site: Arc<SiteState>,
    params: HashMap<String, Value>,
) -> Result<Value, SpotError> {
    let doctype = params
        .get("doctype")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let user = "Administrator"; // TODO: extract from session

    if doctype.is_empty() {
        return Ok(Value::String("{}".into()));
    }

    // Query tabDefaultValue for the user settings
    let filter = json!({
        "defkey":    doctype,
        "parent":    user,
        "parenttype": "User",
    });
    let rows = get_list(
        &site.db,
        "DefaultValue",
        Some(&["name", "defvalue"]),
        Some(&filter),
        1,
        0,
    )
    .await
    .unwrap_or_default();

    let settings_str = rows
        .first()
        .and_then(|r| r.get("defvalue"))
        .and_then(Value::as_str)
        .unwrap_or("{}")
        .to_string();

    // Frappe returns the settings as a JSON string inside the message wrapper
    Ok(Value::String(settings_str))
}

// ── frappe.model.utils.user_settings.save ────────────────────────────────────
// Saves per-user column preferences. Returns {"user_settings": "{...json...}"}
pub async fn handle_user_settings_save(
    site: Arc<SiteState>,
    params: HashMap<String, Value>,
) -> Result<Value, SpotError> {
    let doctype = params
        .get("doctype")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let data = params
        .get("data")
        .and_then(Value::as_str)
        .unwrap_or("{}")
        .to_string();
    let user = "Administrator";

    if !doctype.is_empty() {
        // Store in tabDefaultValue as defkey = doctype
        let name = format!("{user}_{doctype}_usersettings");
        let _ = upsert_doc(
            &site.db,
            "DefaultValue",
            &name,
            &json!({
                "name": name,
                "defkey":     doctype,
                "defvalue":   data,
                "parent":     user,
                "parenttype": "User",
                "parentfield":"defaults",
            }),
        )
        .await;
    }

    Ok(json!({"user_settings": data}))
}

// ── frappe.model.workflow.get_transitions ────────────────────────────────────
// Returns list of allowed and visible workflow transitions for the given doc.
// Delegates to fn::workflow::get_transitions in SurrealDB (07_workflow.surql).
pub async fn handle_workflow_get_transitions(
    site: Arc<SiteState>,
    params: HashMap<String, Value>,
) -> Result<Value, SpotError> {
    let doc_raw = params.get("doc").cloned().unwrap_or(json!({}));
    let doc_val: Value = match &doc_raw {
        Value::String(s) => serde_json::from_str(s).unwrap_or(json!({})),
        other => other.clone(),
    };

    let doctype = doc_val
        .get("doctype")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    if doctype.is_empty() {
        return Ok(Value::Array(vec![]));
    }

    let user = params
        .get("__current_user")
        .and_then(Value::as_str)
        .unwrap_or("Administrator")
        .to_string();

    // Read current_state from the doc object (workflow_state field).
    // We first need to know the state_field name, but we pass an empty string
    // if not known — the SurrealQL fn handles the missing-state case.
    let current_state = doc_val
        .get("workflow_state")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();

    if current_state.is_empty() {
        return Ok(Value::Array(vec![]));
    }

    // Single SurrealQL call — replaces the 3-step Rust query sequence.
    let rows = site
        .db
        .run(
            "RETURN fn::workflow::get_transitions($user, $doctype, $current_state)",
            vec![
                ("user".into(), user.into()),
                ("doctype".into(), doctype.into()),
                ("current_state".into(), current_state.into()),
            ],
        )
        .await
        .unwrap_or_default();

    // `rows` is a single-element vec containing the returned array.
    let result = rows.into_iter().next().unwrap_or(Value::Array(vec![]));
    Ok(result)
}

// ── frappe.model.workflow.apply_workflow ─────────────────────────────────────
// Applies a workflow transition by name. Returns the updated doc.
// Delegates to fn::workflow::apply in SurrealDB (07_workflow.surql).
pub async fn handle_workflow_apply(
    site: Arc<SiteState>,
    params: HashMap<String, Value>,
) -> Result<Value, SpotError> {
    let doc_raw = params.get("doc").cloned().unwrap_or(json!({}));
    let doc_val: Value = match &doc_raw {
        Value::String(s) => serde_json::from_str(s).unwrap_or(json!({})),
        other => other.clone(),
    };
    let action = params
        .get("action")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();

    let doctype = doc_val
        .get("doctype")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let name = doc_val
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();

    if doctype.is_empty() || name.is_empty() || action.is_empty() {
        return Err(SpotError::Validation(
            "doc, action required for apply_workflow".into(),
        ));
    }

    let user = params
        .get("__current_user")
        .and_then(Value::as_str)
        .unwrap_or("Administrator")
        .to_string();

    // Single SurrealQL call — replaces the 5-step Rust query sequence.
    let rows = site
        .db
        .run(
            "RETURN fn::workflow::apply($user, $doctype, $doc_id, $action)",
            vec![
                ("user".into(), user.into()),
                ("doctype".into(), doctype.clone().into()),
                ("doc_id".into(), name.clone().into()),
                ("action".into(), action.into()),
            ],
        )
        .await
        .map_err(|e| SpotError::Db(e.to_string()))?;

    // Invalidate cache for the updated document.
    site.doc_cache
        .remove(&(doctype.clone(), name.clone()))
        .await;

    Ok(rows.into_iter().next().unwrap_or(json!({})))
}

// ── frappe.model.workflow.get_common_transition_actions ───────────────────────
// Returns common actions across all docs in a list (used for bulk workflow).
pub async fn handle_workflow_get_common_actions(
    _site: Arc<SiteState>,
    _params: HashMap<String, Value>,
) -> Result<Value, SpotError> {
    // Phase 3: complex multi-doc transition calculation
    Ok(Value::Array(vec![]))
}

// ── frappe.model.workflow.bulk_workflow_approval ──────────────────────────────
pub async fn handle_workflow_bulk_approval(
    _site: Arc<SiteState>,
    _params: HashMap<String, Value>,
) -> Result<Value, SpotError> {
    Ok(Value::Array(vec![]))
}

// ── frappe.model.workflow.can_cancel_document ─────────────────────────────────
// Returns whether the current user can cancel this doc.
pub async fn handle_workflow_can_cancel(
    site: Arc<SiteState>,
    params: HashMap<String, Value>,
) -> Result<Value, SpotError> {
    let doctype = params
        .get("doctype")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    // Administrator can cancel anything
    let _ = &site;
    let _ = doctype;
    Ok(Value::Bool(true))
}

// ── frappe.model.rename_doc.update_document_title ────────────────────────────
// Used by the form title bar to rename/retitle a doc.
pub async fn handle_rename_doc(
    site: Arc<SiteState>,
    params: HashMap<String, Value>,
) -> Result<Value, SpotError> {
    let doctype = params
        .get("doctype")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let docname = params
        .get("docname")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let title = params.get("title").cloned();
    let name = params.get("name").and_then(Value::as_str).map(str::to_string);

    if doctype.is_empty() || docname.is_empty() {
        return Err(SpotError::Validation("doctype and docname required".into()));
    }

    let mut update = json!({});
    if let Some(t) = title {
        // Update title field
        if let Ok(dt_doc) = get_doc(&site.db, "DocType", &doctype).await {
            let tf = dt_doc
                .get_str("title_field")
                .filter(|s| !s.is_empty())
                .unwrap_or("title")
                .to_string();
            update[tf] = t;
        }
    }

    let new_name = name.unwrap_or_else(|| docname.clone());

    if !matches!(&update, Value::Object(m) if m.is_empty()) {
        upsert_doc(&site.db, &doctype, &docname, &update).await?;
        site.doc_cache.remove(&(doctype, docname.clone())).await;
    }

    Ok(Value::String(new_name))
}
