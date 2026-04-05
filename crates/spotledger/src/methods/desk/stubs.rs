//! Method handlers that are fully implemented using SurrealDB fn:: calls,
//! plus stubs for infrastructure concerns (email, PDF, calendar) that need
//! Phase E before they can be real.

use crate::state::SiteState;
use serde_json::{json, Value};
use spotledger_core::error::SpotError;
use std::collections::HashMap;
use std::sync::Arc;

type Params = HashMap<String, Value>;

// ── helper: call an arbitrary SQL statement that returns an array ────────────
async fn surql_arr(
    site: &Arc<SiteState>,
    surql: &str,
    bindings: Vec<(&'static str, Value)>,
) -> Result<Value, SpotError> {
    let b: Vec<(String, Value)> = bindings.into_iter().map(|(k, v)| (k.to_string(), v)).collect();
    let rows = site.db.run(surql, b).await.map_err(SpotError::from)?;
    Ok(Value::Array(rows))
}

// ── helper: call an arbitrary SQL statement that returns a single value ───────
async fn surql_one(
    site: &Arc<SiteState>,
    surql: &str,
    bindings: Vec<(&'static str, Value)>,
) -> Result<Value, SpotError> {
    let b: Vec<(String, Value)> = bindings.into_iter().map(|(k, v)| (k.to_string(), v)).collect();
    let mut rows = site.db.run(surql, b).await.map_err(SpotError::from)?;
    Ok(rows.pop().unwrap_or(Value::Null))
}

// ── helper: call an arbitrary SQL statement with no meaningful return ─────────
async fn surql_exec(
    site: &Arc<SiteState>,
    surql: &str,
    bindings: Vec<(&'static str, Value)>,
) -> Result<(), SpotError> {
    let b: Vec<(String, Value)> = bindings.into_iter().map(|(k, v)| (k.to_string(), v)).collect();
    site.db.execute(surql, b).await.map_err(SpotError::from)
}

// ── frappe.desk.calendar.get_events ──────────────────────────────────────────
// Phase 3: Query tabEvent with date range filter, return list of events.
pub async fn handle_calendar_get_events(
    _site: Arc<SiteState>, _p: Params,
) -> Result<Value, SpotError> {
    Ok(Value::Array(vec![]))
}

// ── frappe.desk.calendar.update_event ────────────────────────────────────────
// Phase 3: Update event start/end times from drag-drop.
pub async fn handle_calendar_update_event(
    _site: Arc<SiteState>, _p: Params,
) -> Result<Value, SpotError> {
    Ok(Value::Null)
}

// ── frappe.desk.doctype.dashboard_chart.dashboard_chart.get ──────────────────
// Phase 3: Execute the chart query and return labels + datasets.
pub async fn handle_dashboard_chart_get(
    _site: Arc<SiteState>, _p: Params,
) -> Result<Value, SpotError> {
    Ok(json!({"datasets": [], "labels": []}))
}

// ── frappe.desk.doctype.dashboard_chart.dashboard_chart.get_charts_for_user ──
pub async fn handle_get_charts_for_user(
    _site: Arc<SiteState>, _p: Params,
) -> Result<Value, SpotError> {
    Ok(Value::Array(vec![]))
}

// ── frappe.desk.doctype.dashboard_chart.dashboard_chart.create_dashboard_chart
pub async fn handle_create_dashboard_chart(
    _site: Arc<SiteState>, _p: Params,
) -> Result<Value, SpotError> {
    Ok(Value::Null)
}

// ── frappe.desk.doctype.dashboard_chart_source.dashboard_chart_source.get_config
pub async fn handle_dashboard_source_get_config(
    _site: Arc<SiteState>, _p: Params,
) -> Result<Value, SpotError> {
    Ok(json!({}))
}

// ── frappe.desk.doctype.dashboard_settings.dashboard_settings.create_dashboard_settings
pub async fn handle_create_dashboard_settings(
    _site: Arc<SiteState>, _p: Params,
) -> Result<Value, SpotError> {
    Ok(Value::Null)
}

// ── frappe.desk.doctype.dashboard_settings.dashboard_settings.save_chart_config
pub async fn handle_save_chart_config(
    _site: Arc<SiteState>, _p: Params,
) -> Result<Value, SpotError> {
    Ok(Value::Null)
}

// ── frappe.desk.doctype.event.event.get_events ────────────────────────────────
pub async fn handle_event_get_events(
    _site: Arc<SiteState>, _p: Params,
) -> Result<Value, SpotError> {
    Ok(Value::Array(vec![]))
}

// ── frappe.desk.doctype.kanban_board.kanban_board.get_kanban_boards ───────────
// Phase 3: Query tabKanban Board for this doctype.
pub async fn handle_get_kanban_boards(
    _site: Arc<SiteState>, _p: Params,
) -> Result<Value, SpotError> {
    Ok(Value::Array(vec![]))
}

// ── frappe.desk.doctype.kanban_board.kanban_board.quick_kanban_board ──────────
pub async fn handle_quick_kanban_board(
    _site: Arc<SiteState>, _p: Params,
) -> Result<Value, SpotError> {
    Ok(json!({}))
}

// ── frappe.desk.doctype.kanban_board.kanban_board.save_settings ───────────────
pub async fn handle_kanban_save_settings(
    _site: Arc<SiteState>, _p: Params,
) -> Result<Value, SpotError> {
    Ok(Value::Null)
}

// ── frappe.desk.doctype.number_card.number_card.get_result ────────────────────
// Phase 3: Execute the number card query.
pub async fn handle_number_card_get_result(
    _site: Arc<SiteState>, _p: Params,
) -> Result<Value, SpotError> {
    Ok(Value::Null)
}

// ── frappe.desk.doctype.number_card.number_card.get_cards_for_user ────────────
pub async fn handle_get_cards_for_user(
    _site: Arc<SiteState>, _p: Params,
) -> Result<Value, SpotError> {
    Ok(Value::Array(vec![]))
}

// ── frappe.desk.doctype.number_card.number_card.create_number_card ────────────
pub async fn handle_create_number_card(
    _site: Arc<SiteState>, _p: Params,
) -> Result<Value, SpotError> {
    Ok(Value::Null)
}

// ── frappe.desk.doctype.number_card.number_card.get_percentage_difference ─────
pub async fn handle_number_card_get_percentage(
    _site: Arc<SiteState>, _p: Params,
) -> Result<Value, SpotError> {
    Ok(Value::Null)
}

// ── frappe.desk.doctype.onboarding_step.onboarding_step.get_onboarding_steps ──
// Phase 3: query tabOnboarding Step.
pub async fn handle_get_onboarding_steps(
    _site: Arc<SiteState>, _p: Params,
) -> Result<Value, SpotError> {
    Ok(Value::Array(vec![]))
}

// ── frappe.desk.desktop.update_onboarding_step ────────────────────────────────
pub async fn handle_update_onboarding_step(
    _site: Arc<SiteState>, _p: Params,
) -> Result<Value, SpotError> {
    Ok(Value::Null)
}

// ── frappe.desk.doctype.system_console.system_console.execute_code ────────────
// Intentionally unimplemented — security risk; only for internal dev use.
pub async fn handle_execute_code(
    _site: Arc<SiteState>, _p: Params,
) -> Result<Value, SpotError> {
    Err(SpotError::PermissionDenied("system_console not available".into()))
}

// ── frappe.desk.doctype.custom_html_block.custom_html_block.get_custom_blocks_for_user
pub async fn handle_get_custom_blocks_for_user(
    _site: Arc<SiteState>, _p: Params,
) -> Result<Value, SpotError> {
    Ok(Value::Array(vec![]))
}

// ── frappe.desk.doctype.sidebar_item_group.sidebar_item_group.get_reports ─────
pub async fn handle_get_sidebar_report_items(
    _site: Arc<SiteState>, _p: Params,
) -> Result<Value, SpotError> {
    Ok(Value::Array(vec![]))
}

// ── frappe.desk.query_report.run ─────────────────────────────────────────────
// Phase 3: Execute query reports (Script Report / Query Report).
pub async fn handle_query_report_run(
    _site: Arc<SiteState>, _p: Params,
) -> Result<Value, SpotError> {
    Ok(json!({"result": [], "columns": [], "message": null, "chart": null}))
}

// ── frappe.desk.link_preview.get_preview_data ────────────────────────────────
// Returns the title field + image of a linked document for the tooltip preview.
pub async fn handle_get_link_preview(
    site: Arc<SiteState>, p: Params,
) -> Result<Value, SpotError> {
    let doctype = p.get("doctype").and_then(Value::as_str).unwrap_or("").to_string();
    let name    = p.get("docname").and_then(Value::as_str).unwrap_or("").to_string();
    if doctype.is_empty() || name.is_empty() {
        return Ok(json!({ "preview_html": "" }));
    }
    // Fetch title_field from the DocType meta
    let title_field = surql_one(&site,
        "RETURN fn::get_title_field($dt)",
        vec![("dt", Value::String(doctype.clone()))],
    ).await.unwrap_or(Value::Null);
    let tf = title_field.as_str().unwrap_or("name").to_string();
    // Fetch the doc's title and image
    let doc = spotledger_db::document::get_doc(&site.db, &doctype, &name)
        .await
        .ok();
    let title = doc.as_ref()
        .and_then(|d| d.get_str(&tf))
        .or_else(|| doc.as_ref().and_then(|d| d.get_str("name")))
        .unwrap_or(&name)
        .to_string();
    Ok(json!({
        "preview_html": format!("<div class=\"preview-title\">{title}</div>"),
        "name": name,
        "doctype": doctype,
        "title": title,
    }))
}

// ── frappe.desk.treeview.get_all_nodes ────────────────────────────────────────
pub async fn handle_treeview_get_all_nodes(
    site: Arc<SiteState>, p: Params,
) -> Result<Value, SpotError> {
    let doctype = p.get("doctype").and_then(Value::as_str).unwrap_or("").to_string();
    if doctype.is_empty() { return Ok(json!([])); }
    let table = spotledger_db::document::doctype_to_table(&doctype);
    let rows = site.db
        .run(
            &format!("SELECT name, parent_node, lft, rgt FROM `{table}` ORDER BY lft"),
            vec![],
        )
        .await
        .map_err(SpotError::from)?;
    Ok(Value::Array(rows))
}

// ── frappe.desk.treeview.get_children ─────────────────────────────────────────
pub async fn handle_treeview_get_children(
    site: Arc<SiteState>, p: Params,
) -> Result<Value, SpotError> {
    let doctype = p.get("doctype").and_then(Value::as_str).unwrap_or("").to_string();
    let parent  = p.get("parent").and_then(Value::as_str).unwrap_or("").to_string();
    surql_arr(&site,
        "RETURN fn::tree_children($dt, $parent)",
        vec![("dt", Value::String(doctype)), ("parent", Value::String(parent))],
    ).await
}

// ── frappe.desk.treeview.add_node ─────────────────────────────────────────────
pub async fn handle_treeview_add_node(
    site: Arc<SiteState>, p: Params,
) -> Result<Value, SpotError> {
    let doctype = p.get("doctype").and_then(Value::as_str).unwrap_or("").to_string();
    let name    = p.get("name").and_then(Value::as_str).unwrap_or("").to_string();
    let parent  = p.get("parent_node").and_then(Value::as_str).unwrap_or("").to_string();
    if doctype.is_empty() || name.is_empty() {
        return Err(SpotError::Validation("doctype and name required for add_node".into()));
    }
    let table = spotledger_db::document::doctype_to_table(&doctype);
    // Insert the node first
    let _ = site.db
        .execute(
            &format!(
                "INSERT INTO `{table}` {{ name: $name, parent_node: $parent, lft: 0, rgt: 0, \
                 creation: time::now(), modified: time::now(), modified_by: 'Administrator', \
                 owner: 'Administrator', docstatus: 0 }}"
            ),
            vec![
                ("name".into(),   name.into()),
                ("parent".into(), parent.clone().into()),
            ],
        )
        .await;
    // Rebuild nested set from root
    let root = if parent.is_empty() { "root".to_string() } else { parent };
    surql_exec(&site,
        "fn::rebuild_tree($dt, $root, 0)",
        vec![("dt", Value::String(doctype)), ("root", Value::String(root))],
    ).await.ok();
    Ok(Value::Null)
}

// ── frappe.desk.doctype.bulk_update.bulk_update.submit_cancel_or_update_docs ──
pub async fn handle_bulk_update(
    _site: Arc<SiteState>, _p: Params,
) -> Result<Value, SpotError> {
    Ok(Value::Array(vec![]))
}

// ── frappe.desk.doctype.tag.tag.* ─────────────────────────────────────────────
// Real: backed by fn::get_tags / fn::add_tag / fn::remove_tag in SurrealDB
pub async fn handle_tags_get(
    site: Arc<SiteState>, p: Params,
) -> Result<Value, SpotError> {
    let doctype = p.get("doctype").and_then(Value::as_str).unwrap_or("").to_string();
    let name    = p.get("name").and_then(Value::as_str).unwrap_or("").to_string();
    if doctype.is_empty() || name.is_empty() { return Ok(json!([])); }
    surql_arr(&site,
        "RETURN fn::get_tags($dt, $dn)",
        vec![("dt", Value::String(doctype)), ("dn", Value::String(name))],
    ).await
}

pub async fn handle_tags_get_list(
    site: Arc<SiteState>, p: Params,
) -> Result<Value, SpotError> {
    let doctype = p.get("doctype").and_then(Value::as_str).map(str::to_string);
    let query   = p.get("query").and_then(Value::as_str).map(str::to_string);
    surql_arr(&site,
        "RETURN fn::get_tag_list($dt, $q)",
        vec![
            ("dt", doctype.map(Value::String).unwrap_or(Value::Null)),
            ("q",  query.map(Value::String).unwrap_or(Value::Null)),
        ],
    ).await
}

pub async fn handle_tags_get_for_awesomebar(
    site: Arc<SiteState>, p: Params,
) -> Result<Value, SpotError> {
    // Awesomebar uses txt as the query parameter
    let query = p.get("txt").and_then(Value::as_str).map(str::to_string);
    surql_arr(&site,
        "RETURN fn::get_tag_list(NONE, $q)",
        vec![("q", query.map(Value::String).unwrap_or(Value::Null))],
    ).await
}

pub async fn handle_tags_get_documents(
    site: Arc<SiteState>, p: Params,
) -> Result<Value, SpotError> {
    let tag     = p.get("tag").and_then(Value::as_str).unwrap_or("").to_string();
    let doctype = p.get("document_type").and_then(Value::as_str).map(str::to_string);
    surql_arr(&site,
        "RETURN fn::get_tagged_docs($tag, $dt)",
        vec![
            ("tag", Value::String(tag)),
            ("dt",  doctype.map(Value::String).unwrap_or(Value::Null)),
        ],
    ).await
}

pub async fn handle_tags_add(site: Arc<SiteState>, p: Params) -> Result<Value, SpotError> {
    let doctype = p.get("doctype").and_then(Value::as_str).unwrap_or("").to_string();
    let name    = p.get("name").and_then(Value::as_str).unwrap_or("").to_string();
    // tag may come as a JSON array or single string
    let tags: Vec<String> = match p.get("tag") {
        Some(Value::Array(arr)) => arr.iter().filter_map(|v| v.as_str().map(str::to_string)).collect(),
        Some(Value::String(s)) => serde_json::from_str::<Vec<String>>(s)
            .unwrap_or_else(|_| vec![s.clone()]),
        _ => vec![],
    };
    for tag in &tags {
        surql_exec(&site,
            "fn::add_tag($dt, $dn, $tag)",
            vec![
                ("dt",  Value::String(doctype.clone())),
                ("dn",  Value::String(name.clone())),
                ("tag", Value::String(tag.clone())),
            ],
        ).await.ok();
    }
    // Return updated tags list
    surql_arr(&site,
        "RETURN fn::get_tags($dt, $dn)",
        vec![("dt", Value::String(doctype)), ("dn", Value::String(name))],
    ).await
}

pub async fn handle_tags_remove(
    site: Arc<SiteState>, p: Params,
) -> Result<Value, SpotError> {
    let doctype = p.get("doctype").and_then(Value::as_str).unwrap_or("").to_string();
    let name    = p.get("name").and_then(Value::as_str).unwrap_or("").to_string();
    let tag     = p.get("tag").and_then(Value::as_str).unwrap_or("").to_string();
    surql_exec(&site,
        "fn::remove_tag($dt, $dn, $tag)",
        vec![
            ("dt",  Value::String(doctype.clone())),
            ("dn",  Value::String(name.clone())),
            ("tag", Value::String(tag)),
        ],
    ).await.ok();
    surql_arr(&site,
        "RETURN fn::get_tags($dt, $dn)",
        vec![("dt", Value::String(doctype)), ("dn", Value::String(name))],
    ).await
}

// ── frappe.desk.like.toggle_like ─────────────────────────────────────────────
pub async fn handle_toggle_like(
    site: Arc<SiteState>, p: Params,
) -> Result<Value, SpotError> {
    let doctype = p.get("doctype").and_then(Value::as_str).unwrap_or("").to_string();
    let name    = p.get("name").and_then(Value::as_str).unwrap_or("").to_string();
    // liked: "Yes" / true / 1
    let liked_raw = p.get("liked").cloned().unwrap_or(Value::Bool(false));
    let liked = match &liked_raw {
        Value::Bool(b) => *b,
        Value::String(s) => s == "Yes" || s == "1" || s == "true",
        Value::Number(n) => n.as_i64().unwrap_or(0) != 0,
        _ => false,
    };
    // user from session; hardcode Administrator for now
    let user = "Administrator".to_string();
    surql_arr(&site,
        "RETURN fn::toggle_like($user, $dt, $dn, $liked)",
        vec![
            ("user",  Value::String(user)),
            ("dt",    Value::String(doctype)),
            ("dn",    Value::String(name)),
            ("liked", Value::Bool(liked)),
        ],
    ).await
}

// ── frappe.share.* ───────────────────────────────────────────────────────────
pub async fn handle_share_add(site: Arc<SiteState>, p: Params) -> Result<Value, SpotError> {
    let doctype    = p.get("doctype").and_then(Value::as_str).unwrap_or("").to_string();
    let name       = p.get("name").and_then(Value::as_str).unwrap_or("").to_string();
    let share_with = p.get("user").and_then(Value::as_str).unwrap_or("").to_string();
    let owner      = "Administrator".to_string();
    let read       = bool_param(&p, "read");
    let write      = bool_param(&p, "write");
    let share_perm = bool_param(&p, "share");
    surql_one(&site,
        "RETURN fn::share_add($dt, $dn, $owner, $share_with, $read, $write, $share)",
        vec![
            ("dt",         Value::String(doctype)),
            ("dn",         Value::String(name)),
            ("owner",      Value::String(owner)),
            ("share_with", Value::String(share_with)),
            ("read",       Value::Bool(read)),
            ("write",      Value::Bool(write)),
            ("share",      Value::Bool(share_perm)),
        ],
    ).await
}

pub async fn handle_share_get_users(
    site: Arc<SiteState>, p: Params,
) -> Result<Value, SpotError> {
    let doctype = p.get("doctype").and_then(Value::as_str).unwrap_or("").to_string();
    let name    = p.get("name").and_then(Value::as_str).unwrap_or("").to_string();
    surql_arr(&site,
        "RETURN fn::share_get_users($dt, $dn)",
        vec![("dt", Value::String(doctype)), ("dn", Value::String(name))],
    ).await
}

pub async fn handle_share_set_permission(
    site: Arc<SiteState>, p: Params,
) -> Result<Value, SpotError> {
    let doctype    = p.get("doctype").and_then(Value::as_str).unwrap_or("").to_string();
    let name       = p.get("name").and_then(Value::as_str).unwrap_or("").to_string();
    let share_with = p.get("user").and_then(Value::as_str).unwrap_or("").to_string();
    let owner      = "Administrator".to_string();
    let read       = bool_param(&p, "read");
    let write      = bool_param(&p, "write");
    let share_perm = bool_param(&p, "share");
    // Re-upsert with the new permission flags
    surql_one(&site,
        "RETURN fn::share_add($dt, $dn, $owner, $share_with, $read, $write, $share)",
        vec![
            ("dt",         Value::String(doctype)),
            ("dn",         Value::String(name)),
            ("owner",      Value::String(owner)),
            ("share_with", Value::String(share_with)),
            ("read",       Value::Bool(read)),
            ("write",      Value::Bool(write)),
            ("share",      Value::Bool(share_perm)),
        ],
    ).await
}

// ── private helper: coerce param to bool ─────────────────────────────────────
fn bool_param(p: &Params, key: &str) -> bool {
    match p.get(key) {
        Some(Value::Bool(b)) => *b,
        Some(Value::Number(n)) => n.as_i64().unwrap_or(0) != 0,
        Some(Value::String(s)) => s == "1" || s == "true" || s == "Yes",
        _ => false,
    }
}

// ── frappe.desk.reportview.delete_items ──────────────────────────────────────
pub async fn handle_reportview_delete_items(
    site: Arc<SiteState>, p: Params,
) -> Result<Value, SpotError> {
    let doctype = p.get("doctype").and_then(Value::as_str).unwrap_or("").to_string();
    let items_raw = p.get("items").cloned().unwrap_or(Value::Array(vec![]));
    let items: Vec<String> = match items_raw {
        Value::Array(arr) => arr.into_iter().filter_map(|v| v.as_str().map(str::to_string)).collect(),
        Value::String(s)  => serde_json::from_str::<Vec<String>>(&s).unwrap_or_default(),
        _ => vec![],
    };
    if doctype.is_empty() || items.is_empty() {
        return Ok(Value::Null);
    }
    let table = spotledger_db::document::doctype_to_table(&doctype);
    for item in &items {
        let _ = site.db
            .execute(
                &format!("DELETE `{table}` WHERE name = $name"),
                vec![("name".into(), item.clone().into())],
            )
            .await;
    }
    Ok(Value::Null)
}

// ── frappe.desk.reportview.get_sidebar_stats ─────────────────────────────────
pub async fn handle_reportview_get_sidebar_stats(
    _site: Arc<SiteState>, _p: Params,
) -> Result<Value, SpotError> {
    Ok(Value::Array(vec![]))
}

// ── frappe.core.doctype.user.user.* ──────────────────────────────────────────
pub async fn handle_user_get_all_roles(
    site: Arc<SiteState>, p: Params,
) -> Result<Value, SpotError> {
    let user = p.get("user").and_then(Value::as_str).unwrap_or("Administrator").to_string();
    surql_arr(&site,
        "RETURN fn::get_user_roles($user)",
        vec![("user", Value::String(user))],
    ).await
}

pub async fn handle_user_get_perm_info(
    site: Arc<SiteState>, p: Params,
) -> Result<Value, SpotError> {
    let doctype = p.get("doctype").and_then(Value::as_str).unwrap_or("").to_string();
    let user    = p.get("user").and_then(Value::as_str).unwrap_or("Administrator").to_string();
    if doctype.is_empty() { return Ok(json!([])); }
    surql_one(&site,
        "RETURN fn::get_doc_permissions($user, $dt, NONE)",
        vec![("user", Value::String(user)), ("dt", Value::String(doctype))],
    ).await
}

pub async fn handle_user_switch_theme(
    _site: Arc<SiteState>, _p: Params,
) -> Result<Value, SpotError> {
    Ok(Value::Null)
}

pub async fn handle_user_test_password_strength(
    _site: Arc<SiteState>, p: Params,
) -> Result<Value, SpotError> {
    // Very basic strength check without calling zxcvbn
    let pwd = p.get("password").and_then(Value::as_str).unwrap_or("");
    let score = match pwd.len() {
        0..=5 => 0,
        6..=7 => 1,
        8..=9 => 2,
        10..=11 => 3,
        _ => 4,
    };
    Ok(json!({
        "score": score,
        "feedback": {"warning": "", "suggestions": []},
        "crack_time": score * 1000,
        "crack_time_display": "unknown",
    }))
}

pub async fn handle_user_verify_password(
    _site: Arc<SiteState>, _p: Params,
) -> Result<Value, SpotError> {
    Ok(Value::Null)
}

pub async fn handle_user_get_email_awaiting(
    _site: Arc<SiteState>, _p: Params,
) -> Result<Value, SpotError> {
    Ok(Value::Array(vec![]))
}

// ── frappe.automation.* ──────────────────────────────────────────────────────
pub async fn handle_automation_bulk_apply(
    _site: Arc<SiteState>, _p: Params,
) -> Result<Value, SpotError> {
    Ok(Value::Null)
}

pub async fn handle_automation_create_reminder(
    _site: Arc<SiteState>, _p: Params,
) -> Result<Value, SpotError> {
    Ok(Value::Null)
}

// ── frappe.core.api.file.* ───────────────────────────────────────────────────
pub async fn handle_file_get_attached_images(
    _site: Arc<SiteState>, _p: Params,
) -> Result<Value, SpotError> {
    Ok(Value::Array(vec![]))
}

pub async fn handle_file_create_new_folder(
    _site: Arc<SiteState>, _p: Params,
) -> Result<Value, SpotError> {
    Ok(Value::Null)
}

pub async fn handle_file_move_file(
    _site: Arc<SiteState>, _p: Params,
) -> Result<Value, SpotError> {
    Ok(Value::Null)
}

pub async fn handle_file_unzip_file(
    _site: Arc<SiteState>, _p: Params,
) -> Result<Value, SpotError> {
    Ok(Value::Null)
}

pub async fn handle_file_add_attachments(
    _site: Arc<SiteState>, _p: Params,
) -> Result<Value, SpotError> {
    Ok(Value::Null)
}

// ── frappe.core.doctype.access_log.access_log.make_access_log ────────────────
pub async fn handle_make_access_log(
    site: Arc<SiteState>, p: Params,
) -> Result<Value, SpotError> {
    let doctype = p.get("reference_doctype").and_then(Value::as_str).unwrap_or("").to_string();
    let name    = p.get("reference_name").and_then(Value::as_str).unwrap_or("").to_string();
    let method  = p.get("method").and_then(Value::as_str).unwrap_or("view").to_string();
    let user    = "Administrator".to_string();
    surql_exec(&site,
        "fn::make_access_log($dt, $dn, $user, $method)",
        vec![
            ("dt",     Value::String(doctype)),
            ("dn",     Value::String(name)),
            ("user",   Value::String(user)),
            ("method", Value::String(method)),
        ],
    ).await.ok();
    Ok(Value::Null)
}

// ── frappe.core.doctype.communication.email.make ─────────────────────────────
// Phase 3: send email.
pub async fn handle_email_make(
    _site: Arc<SiteState>, _p: Params,
) -> Result<Value, SpotError> {
    Ok(Value::Null)
}

// ── frappe.email.* ───────────────────────────────────────────────────────────
pub async fn handle_email_set_password(
    _site: Arc<SiteState>, _p: Params,
) -> Result<Value, SpotError> {
    Ok(Value::Null)
}

pub async fn handle_email_get_template(
    _site: Arc<SiteState>, _p: Params,
) -> Result<Value, SpotError> {
    Ok(json!({}))
}

pub async fn handle_email_get_html(
    _site: Arc<SiteState>, _p: Params,
) -> Result<Value, SpotError> {
    Ok(Value::Null)
}

pub async fn handle_email_get_contact_list(
    _site: Arc<SiteState>, _p: Params,
) -> Result<Value, SpotError> {
    Ok(Value::Array(vec![]))
}

// ── frappe.utils.* ───────────────────────────────────────────────────────────
pub async fn handle_change_log_show_popup(
    _site: Arc<SiteState>, _p: Params,
) -> Result<Value, SpotError> {
    Ok(Value::Null)
}

pub async fn handle_change_log_update_last_known(
    _site: Arc<SiteState>, _p: Params,
) -> Result<Value, SpotError> {
    Ok(Value::Null)
}

pub async fn handle_diff_get_version_diff(
    _site: Arc<SiteState>, _p: Params,
) -> Result<Value, SpotError> {
    Ok(Value::Null)
}

pub async fn handle_diff_version_query(
    site: Arc<SiteState>, p: Params,
) -> Result<Value, SpotError> {
    let doctype = p.get("doctype").and_then(Value::as_str).unwrap_or("").to_string();
    let name    = p.get("name").and_then(Value::as_str).unwrap_or("").to_string();
    if doctype.is_empty() || name.is_empty() { return Ok(json!([])); }
    surql_arr(&site,
        "SELECT name, creation, owner, data FROM tabVersion \
         WHERE ref_doctype = $dt AND docname = $dn \
         ORDER BY creation DESC LIMIT 50",
        vec![("dt", Value::String(doctype)), ("dn", Value::String(name))],
    ).await
}

pub async fn handle_global_search(
    site: Arc<SiteState>, p: Params,
) -> Result<Value, SpotError> {
    let query = p.get("text").or_else(|| p.get("q")).or_else(|| p.get("search"))
        .and_then(Value::as_str).unwrap_or("").to_string();
    if query.is_empty() {
        return Ok(json!({"results": [], "more": false}));
    }
    let limit = p.get("limit").and_then(Value::as_i64).unwrap_or(20) as i64;
    let results = surql_arr(&site,
        "RETURN fn::global_search($q, NONE, $lim)",
        vec![
            ("q",   Value::String(query)),
            ("lim", Value::Number(limit.into())),
        ],
    ).await.unwrap_or(Value::Array(vec![]));
    Ok(json!({"results": results, "more": false}))
}

pub async fn handle_nestedset_rebuild_tree(
    site: Arc<SiteState>, p: Params,
) -> Result<Value, SpotError> {
    let doctype = p.get("doctype").and_then(Value::as_str).unwrap_or("").to_string();
    let parent  = p.get("parent").and_then(Value::as_str).unwrap_or("root").to_string();
    surql_exec(&site,
        "RETURN fn::rebuild_tree($dt, $parent, 0)",
        vec![("dt", Value::String(doctype)), ("parent", Value::String(parent))],
    ).await.ok();
    Ok(Value::Null)
}

pub async fn handle_print_download_multi_pdf(
    _site: Arc<SiteState>, _p: Params,
) -> Result<Value, SpotError> {
    Ok(Value::Null)
}

// ── frappe.model.mapper.make_mapped_doc ──────────────────────────────────────
// Phase 3: doc-to-doc mapping (e.g. Purchase Order → Purchase Invoice).
pub async fn handle_make_mapped_doc(
    _site: Arc<SiteState>, _p: Params,
) -> Result<Value, SpotError> {
    Ok(Value::Null)
}

// ── frappe.geo.country_info.get_country_timezone_info ────────────────────────
pub async fn handle_get_country_timezone_info(
    _site: Arc<SiteState>, _p: Params,
) -> Result<Value, SpotError> {
    Ok(json!({}))
}

// ── frappe.integrations.doctype.geolocation_settings.*.autocomplete ──────────
pub async fn handle_geolocation_autocomplete(
    _site: Arc<SiteState>, _p: Params,
) -> Result<Value, SpotError> {
    Ok(Value::Array(vec![]))
}

// ── frappe.contacts.doctype.address.address.get_address_display ──────────────
pub async fn handle_get_address_display(
    _site: Arc<SiteState>, _p: Params,
) -> Result<Value, SpotError> {
    Ok(Value::Null)
}

// ── frappe.core.doctype.submission_queue.submission_queue.get_latest_submissions
pub async fn handle_get_latest_submissions(
    _site: Arc<SiteState>, _p: Params,
) -> Result<Value, SpotError> {
    Ok(Value::Array(vec![]))
}

// ── frappe.core.doctype.server_script.server_script.get_autocompletion_items ──
pub async fn handle_server_script_autocompletion(
    _site: Arc<SiteState>, _p: Params,
) -> Result<Value, SpotError> {
    Ok(Value::Array(vec![]))
}

// ── frappe.core.doctype.doctype.doctype.get_row_size_utilization ──────────────
pub async fn handle_get_row_size_utilization(
    _site: Arc<SiteState>, _p: Params,
) -> Result<Value, SpotError> {
    Ok(json!({"used": 0, "total": 100}))
}

// ── frappe.core.report.permitted_documents_for_user.*.query_doctypes ──────────
pub async fn handle_query_permitted_doctypes(
    _site: Arc<SiteState>, _p: Params,
) -> Result<Value, SpotError> {
    Ok(Value::Array(vec![]))
}

// ── frappe.desk.doctype.note.note.* ──────────────────────────────────────────
pub async fn handle_note_mark_seen(
    _site: Arc<SiteState>, _p: Params,
) -> Result<Value, SpotError> {
    Ok(Value::Null)
}

pub async fn handle_note_reset(
    _site: Arc<SiteState>, _p: Params,
) -> Result<Value, SpotError> {
    Ok(Value::Null)
}

// ── frappe.website.doctype.web_page_view.web_page_view.get_page_view_count ────
pub async fn handle_get_page_view_count(
    _site: Arc<SiteState>, _p: Params,
) -> Result<Value, SpotError> {
    Ok(Value::Number(0.into()))
}

// ── frappe.desk.doctype.workspace_sidebar.workspace_sidebar.add_sidebar_items ─
pub async fn handle_workspace_add_sidebar_items(
    _site: Arc<SiteState>, _p: Params,
) -> Result<Value, SpotError> {
    Ok(Value::Null)
}

// ── frappe.core.doctype.session_default_settings.*.set_session_default_values ─
pub async fn handle_set_session_defaults(
    _site: Arc<SiteState>, _p: Params,
) -> Result<Value, SpotError> {
    Ok(Value::Null)
}
