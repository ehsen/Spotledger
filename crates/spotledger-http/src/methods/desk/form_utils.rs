//! frappe.desk.form.* handlers (assign_to, utils, linked_with, document_follow)
//!
//! Python reference:
//!   frappe/frappe/desk/form/assign_to.py
//!   frappe/frappe/desk/form/utils.py
//!   frappe/frappe/desk/form/linked_with.py
//!   frappe/frappe/desk/form/document_follow.py
//!   frappe/frappe/desk/form/load.py (get_user_info_for_viewers)

use crate::state::SiteState;
use serde_json::{json, Value};
use spotledger_db::document::{doctype_to_table, get_list, insert_doc, upsert_doc};
use spotledger_core::error::SpotError;
use std::collections::HashMap;
use std::sync::Arc;

// ── frappe.desk.form.assign_to.add ───────────────────────────────────────────
// Python: inserts a ToDo record linking to the document.
// Return: updated assignments list
pub async fn handle_assign_to_add(
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
    let assign_to_raw = params.get("assign_to").cloned().unwrap_or(Value::Array(vec![]));
    let assign_to: Vec<String> = match assign_to_raw {
        Value::Array(arr) => arr
            .into_iter()
            .filter_map(|v| v.as_str().map(str::to_string))
            .collect(),
        Value::String(s) => serde_json::from_str::<Vec<String>>(&s).unwrap_or_default(),
        _ => vec![],
    };

    for user in &assign_to {
        let description = params
            .get("description")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let todo_name = format!("todo-{doctype}-{name}-{user}");
        let _ = insert_doc(
            &site.db,
            "ToDo",
            &json!({
                "name":           todo_name,
                "allocated_to":   user,
                "reference_type": doctype,
                "reference_name": name,
                "description":    description,
                "status":         "Open",
            }),
        )
        .await;
    }

    // Return current assignments for this document
    get_assignments(&site, &doctype, &name).await
}

// ── frappe.desk.form.assign_to.add_multiple ──────────────────────────────────
pub async fn handle_assign_to_add_multiple(
    site: Arc<SiteState>,
    params: HashMap<String, Value>,
) -> Result<Value, SpotError> {
    handle_assign_to_add(site, params).await
}

// ── frappe.desk.form.assign_to.remove ────────────────────────────────────────
// Python: marks the ToDo as Cancelled for the given user.
pub async fn handle_assign_to_remove(
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
    let assign_to = params
        .get("assign_to")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();

    if !assign_to.is_empty() && !doctype.is_empty() && !name.is_empty() {
        let table = doctype_to_table("ToDo");
        let _ = site
            .db
            .execute(
                &format!(
                    "UPDATE `{table}` SET status = 'Cancelled' \
                     WHERE reference_type = $dt AND reference_name = $n AND allocated_to = $u"
                ),
                vec![
                    ("dt".into(), doctype.clone().into()),
                    ("n".into(),  name.clone().into()),
                    ("u".into(),  assign_to.into()),
                ],
            )
            .await;
    }

    get_assignments(&site, &doctype, &name).await
}

// ── frappe.desk.form.assign_to.remove_multiple ───────────────────────────────
pub async fn handle_assign_to_remove_multiple(
    site: Arc<SiteState>,
    params: HashMap<String, Value>,
) -> Result<Value, SpotError> {
    handle_assign_to_remove(site, params).await
}

// ── frappe.desk.form.assign_to.close ─────────────────────────────────────────
// Marks the ToDo as Closed (completed).
pub async fn handle_assign_to_close(
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
    let assign_to = params
        .get("assign_to")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();

    if !assign_to.is_empty() && !doctype.is_empty() && !name.is_empty() {
        let table = doctype_to_table("ToDo");
        let _ = site
            .db
            .execute(
                &format!(
                    "UPDATE `{table}` SET status = 'Closed' \
                     WHERE reference_type = $dt AND reference_name = $n AND allocated_to = $u"
                ),
                vec![
                    ("dt".into(), doctype.clone().into()),
                    ("n".into(),  name.clone().into()),
                    ("u".into(),  assign_to.into()),
                ],
            )
            .await;
    }

    get_assignments(&site, &doctype, &name).await
}

// ── frappe.desk.form.utils.add_comment ───────────────────────────────────────
// Inserts a Communication record as a comment.
// Python: frappe.get_doc({doctype: "Communication", ...}).insert()
// Return: the saved comment object.
pub async fn handle_add_comment(
    site: Arc<SiteState>,
    params: HashMap<String, Value>,
) -> Result<Value, SpotError> {
    let reference_doctype = params
        .get("reference_doctype")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let reference_name = params
        .get("reference_name")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let content = params
        .get("content")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let comment_email = params
        .get("comment_email")
        .and_then(Value::as_str)
        .unwrap_or("Administrator")
        .to_string();
    let comment_by = params
        .get("comment_by")
        .and_then(Value::as_str)
        .unwrap_or("Administrator")
        .to_string();

    // Generate a comment name using wall-clock milliseconds
    let ts_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let comment_name = format!(
        "comment-{}-{ts_ms}",
        reference_doctype.to_lowercase().replace(' ', "-"),
    );

    let comment_doc = json!({
        "name":               comment_name,
        "doctype":            "Comment",
        "comment_type":       "Comment",
        "reference_doctype":  reference_doctype,
        "reference_name":     reference_name,
        "content":            content,
        "comment_email":      comment_email,
        "comment_by":         comment_by,
        "published":          1,
    });

    let doc = insert_doc(&site.db, "Comment", &comment_doc)
        .await
        .map_err(|e| SpotError::Db(e.to_string()))?;

    Ok(doc.as_dict())
}

// ── frappe.desk.form.utils.update_comment ───────────────────────────────────
pub async fn handle_update_comment(
    site: Arc<SiteState>,
    params: HashMap<String, Value>,
) -> Result<Value, SpotError> {
    let name = params
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let content = params.get("content").cloned().unwrap_or(Value::Null);

    if !name.is_empty() {
        let update = json!({"content": content});
        upsert_doc(&site.db, "Comment", &name, &update).await?;
    }

    Ok(Value::Null)
}

// ── frappe.desk.form.utils.update_comment_publicity ──────────────────────────
pub async fn handle_update_comment_publicity(
    site: Arc<SiteState>,
    params: HashMap<String, Value>,
) -> Result<Value, SpotError> {
    let name = params
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let published = params.get("is_visible").cloned().unwrap_or(Value::Bool(true));

    if !name.is_empty() {
        let update = json!({"published": published});
        upsert_doc(&site.db, "Comment", &name, &update).await?;
    }

    Ok(Value::Null)
}

// ── frappe.desk.form.utils.get_next ──────────────────────────────────────────
// Returns the name of the next/prev document in list order.
pub async fn handle_get_next(
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
    let prev = params
        .get("prev")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    if doctype.is_empty() {
        return Ok(Value::Null);
    }

    let table = doctype_to_table(&doctype);
    let operator = if prev { "<" } else { ">" };
    let order = if prev { "DESC" } else { "ASC" };

    let surql = format!(
        "SELECT name FROM `{table}` WHERE name {operator} $name ORDER BY name {order} LIMIT 1"
    );
    let rows = site
        .db
        .run(&surql, vec![("name".into(), name.into())])
        .await
        .map_err(SpotError::from)?;

    let next_name = rows
        .first()
        .and_then(|r| r.get("name"))
        .and_then(Value::as_str)
        .map(str::to_string);

    Ok(next_name.map(Value::String).unwrap_or(Value::Null))
}

// ── frappe.desk.form.linked_with.get ─────────────────────────────────────────
// Returns all documents that link to this document (for the LinkedWith panel).
// Python: complex cross-table search; we return a simplified version.
pub async fn handle_linked_with_get(
    _site: Arc<SiteState>,
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
        return Ok(json!({"count": 0, "groups": []}));
    }

    // Phase 3: full linked_with scan across all Link fields pointing to this doctype.
    // For now return blank — desk renders "0 linked records" which is correct.
    Ok(json!({"count": 0, "groups": []}))
}

// ── frappe.desk.form.linked_with.get_submitted_linked_docs ───────────────────
pub async fn handle_get_submitted_linked_docs(
    _site: Arc<SiteState>,
    _params: HashMap<String, Value>,
) -> Result<Value, SpotError> {
    Ok(Value::Array(vec![]))
}

// ── frappe.desk.form.linked_with.cancel_all_linked_docs ──────────────────────
pub async fn handle_cancel_all_linked_docs(
    _site: Arc<SiteState>,
    _params: HashMap<String, Value>,
) -> Result<Value, SpotError> {
    Ok(Value::Null)
}

// ── frappe.desk.form.document_follow.follow_document ─────────────────────────
pub async fn handle_follow_document(
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
    let user = "Administrator";

    if !doctype.is_empty() && !docname.is_empty() {
        let follow_name = format!("follow-{user}-{doctype}-{docname}");
        let _ = insert_doc(
            &site.db,
            "Document Follow",
            &json!({
                "name":     follow_name,
                "user":     user,
                "ref_doctype": doctype,
                "ref_docname": docname,
            }),
        )
        .await;
    }

    Ok(Value::Null)
}

// ── frappe.desk.form.document_follow.unfollow_document ──────────────────────
pub async fn handle_unfollow_document(
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
    let user = "Administrator";

    if !doctype.is_empty() && !docname.is_empty() {
        let table = doctype_to_table("Document Follow");
        let _ = site
            .db
            .execute(
                &format!(
                    "DELETE `{table}` WHERE user = $user AND ref_doctype = $dt AND ref_docname = $n"
                ),
                vec![
                    ("user".into(), user.to_owned().into()),
                    ("dt".into(),   doctype.into()),
                    ("n".into(),    docname.into()),
                ],
            )
            .await;
    }

    Ok(Value::Null)
}

// ── frappe.desk.form.document_follow.get_follow_users ────────────────────────
pub async fn handle_get_follow_users(
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

    let filter = json!({
        "ref_doctype": doctype,
        "ref_docname": docname,
    });
    let rows = get_list(
        &site.db,
        "Document Follow",
        Some(&["user"]),
        Some(&filter),
        50,
        0,
    )
    .await
    .unwrap_or_default();

    let users: Vec<Value> = rows
        .into_iter()
        .filter_map(|r| r.get("user").cloned())
        .collect();

    Ok(Value::Array(users))
}

// ── frappe.desk.form.document_follow.update_follow ───────────────────────────
pub async fn handle_update_follow(
    site: Arc<SiteState>,
    params: HashMap<String, Value>,
) -> Result<Value, SpotError> {
    let follow = params
        .get("follow")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    if follow {
        handle_follow_document(site, params).await
    } else {
        handle_unfollow_document(site, params).await
    }
}

// ── frappe.desk.form.load.get_user_info_for_viewers ──────────────────────────
// Returns user info for all users currently viewing a document (presence).
// Phase 3: This requires realtime/WebSocket state. Return empty for now.
pub async fn handle_get_user_info_for_viewers(
    _site: Arc<SiteState>,
    _params: HashMap<String, Value>,
) -> Result<Value, SpotError> {
    Ok(Value::Array(vec![]))
}

// ── frappe.desk.form.utils.remove_attach ─────────────────────────────────────
pub async fn handle_remove_attach(
    site: Arc<SiteState>,
    params: HashMap<String, Value>,
) -> Result<Value, SpotError> {
    let fid = params
        .get("fid")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    if !fid.is_empty() {
        let table = doctype_to_table("File");
        let _ = site
            .db
            .execute(
                &format!("DELETE `{table}` WHERE name = $name"),
                vec![("name".into(), fid.into())],
            )
            .await;
    }
    Ok(Value::Null)
}

// ── Shared helpers ────────────────────────────────────────────────────────────

async fn get_assignments(
    site: &Arc<SiteState>,
    doctype: &str,
    name: &str,
) -> Result<Value, SpotError> {
    let filter = json!({
        "reference_type": doctype,
        "reference_name": name,
        "status": "Open",
    });
    let rows = get_list(
        &site.db,
        "ToDo",
        Some(&["name", "allocated_to"]),
        Some(&filter),
        20,
        0,
    )
    .await
    .unwrap_or_default();

    let assignments: Vec<Value> = rows
        .into_iter()
        .map(|r| {
            let owner = r
                .get("allocated_to")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            json!({"owner": owner})
        })
        .collect();

    Ok(Value::Array(assignments))
}
