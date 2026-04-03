//! Phase-3 stub handlers — all clearly documented as what they need to become.
//!
//! These handlers return empty/success responses that allow the Desk JS to
//! continue without errors. Each has a comment describing the real
//! implementation required in Phase 3.
//!
//! DO NOT call these "done" — they must be replaced with real implementations
//! before production use. They are separated here so the gap is visible.

use crate::state::SiteState;
use serde_json::{json, Value};
use spotledger_types::error::SpotError;
use std::collections::HashMap;
use std::sync::Arc;

type Params = HashMap<String, Value>;

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
// Phase 3: quick link card preview over a tooltip hover.
pub async fn handle_get_link_preview(
    _site: Arc<SiteState>, _p: Params,
) -> Result<Value, SpotError> {
    Ok(json!({}))
}

// ── frappe.desk.treeview.get_all_nodes ────────────────────────────────────────
// Phase 3: tree navigation for nested sets.
pub async fn handle_treeview_get_all_nodes(
    _site: Arc<SiteState>, _p: Params,
) -> Result<Value, SpotError> {
    Ok(Value::Array(vec![]))
}

// ── frappe.desk.treeview.get_children ─────────────────────────────────────────
pub async fn handle_treeview_get_children(
    _site: Arc<SiteState>, _p: Params,
) -> Result<Value, SpotError> {
    Ok(Value::Array(vec![]))
}

// ── frappe.desk.treeview.add_node ─────────────────────────────────────────────
pub async fn handle_treeview_add_node(
    _site: Arc<SiteState>, _p: Params,
) -> Result<Value, SpotError> {
    Ok(Value::Null)
}

// ── frappe.desk.doctype.bulk_update.bulk_update.submit_cancel_or_update_docs ──
pub async fn handle_bulk_update(
    _site: Arc<SiteState>, _p: Params,
) -> Result<Value, SpotError> {
    Ok(Value::Array(vec![]))
}

// ── frappe.desk.doctype.tag.tag.* ─────────────────────────────────────────────
// Phase 3: tag management backed by tabTag.
pub async fn handle_tags_get(
    _site: Arc<SiteState>, _p: Params,
) -> Result<Value, SpotError> {
    Ok(Value::Array(vec![]))
}

pub async fn handle_tags_get_list(
    _site: Arc<SiteState>, _p: Params,
) -> Result<Value, SpotError> {
    Ok(Value::Array(vec![]))
}

pub async fn handle_tags_get_for_awesomebar(
    _site: Arc<SiteState>, _p: Params,
) -> Result<Value, SpotError> {
    Ok(Value::Array(vec![]))
}

pub async fn handle_tags_get_documents(
    _site: Arc<SiteState>, _p: Params,
) -> Result<Value, SpotError> {
    Ok(Value::Array(vec![]))
}

pub async fn handle_tags_add(_site: Arc<SiteState>, _p: Params) -> Result<Value, SpotError> {
    Ok(Value::Null)
}

pub async fn handle_tags_remove(
    _site: Arc<SiteState>, _p: Params,
) -> Result<Value, SpotError> {
    Ok(Value::Null)
}

// ── frappe.desk.like.toggle_like ─────────────────────────────────────────────
pub async fn handle_toggle_like(
    _site: Arc<SiteState>, _p: Params,
) -> Result<Value, SpotError> {
    Ok(Value::Null)
}

// ── frappe.share.* ───────────────────────────────────────────────────────────
pub async fn handle_share_add(_site: Arc<SiteState>, _p: Params) -> Result<Value, SpotError> {
    Ok(Value::Null)
}

pub async fn handle_share_get_users(
    _site: Arc<SiteState>, _p: Params,
) -> Result<Value, SpotError> {
    Ok(Value::Array(vec![]))
}

pub async fn handle_share_set_permission(
    _site: Arc<SiteState>, _p: Params,
) -> Result<Value, SpotError> {
    Ok(Value::Null)
}

// ── frappe.desk.reportview.delete_items ──────────────────────────────────────
// Phase 3: bulk delete from list view.
pub async fn handle_reportview_delete_items(
    _site: Arc<SiteState>, _p: Params,
) -> Result<Value, SpotError> {
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
    _site: Arc<SiteState>, _p: Params,
) -> Result<Value, SpotError> {
    Ok(Value::Array(vec![]))
}

pub async fn handle_user_get_perm_info(
    _site: Arc<SiteState>, _p: Params,
) -> Result<Value, SpotError> {
    Ok(Value::Array(vec![]))
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
    _site: Arc<SiteState>, _p: Params,
) -> Result<Value, SpotError> {
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
    _site: Arc<SiteState>, _p: Params,
) -> Result<Value, SpotError> {
    Ok(Value::Array(vec![]))
}

pub async fn handle_global_search(
    _site: Arc<SiteState>, _p: Params,
) -> Result<Value, SpotError> {
    Ok(json!({"results": [], "more": false}))
}

pub async fn handle_nestedset_rebuild_tree(
    _site: Arc<SiteState>, _p: Params,
) -> Result<Value, SpotError> {
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
