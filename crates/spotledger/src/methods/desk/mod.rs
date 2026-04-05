//! Desk API method handlers.
//!
//! Organised into sub-modules by domain area. This file contains the core
//! form/document/boot handlers and wires everything into the method registry.

pub mod form_utils;
pub mod listview;
pub mod model;
pub mod notifications;
pub mod search;
pub mod stubs;

use super::{BoxFuture, MethodRegistry};
use crate::state::SiteState;
use axum::extract::Query as AxumQuery;
use serde_json::{json, Value};
use spotledger_db::document::{get_doc, get_list, get_value};
use spotledger_db::DbAdapter;
use spotledger_db::permissions::get_doc_permissions;
use spotledger_core::document::Document;
use spotledger_core::error::SpotError;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

pub fn register_desk_methods(registry: &Arc<MethodRegistry>) {
    macro_rules! reg {
        ($path:expr, $fn:ident) => {
            registry.register(
                $path,
                Arc::new(|site: Arc<SiteState>, params: HashMap<String, Value>| -> BoxFuture {
                    Box::pin(async move { $fn(site, params).await })
                }),
            );
        };
        // cross-module handler: $mod::$fn
        ($path:expr, $mod:ident :: $fn:ident) => {
            registry.register(
                $path,
                Arc::new(|site: Arc<SiteState>, params: HashMap<String, Value>| -> BoxFuture {
                    Box::pin(async move { $mod::$fn(site, params).await })
                }),
            );
        };
    }

    // ── Core form / doc load ──────────────────────────────────────────────
    reg!("frappe.desk.form.load.getdoctype",  handle_getdoctype);
    reg!("frappe.desk.form.load.getdoc",      handle_getdoc);
    reg!("frappe.desk.form.load.get_docinfo", handle_get_docinfo);
    reg!("frappe.desk.form.load.get_communications", handle_get_communications);
    reg!("frappe.desk.form.load.get_user_info_for_viewers",
         form_utils::handle_get_user_info_for_viewers);

    // ── Form save / cancel / discard ──────────────────────────────────────
    reg!("frappe.desk.form.save.savedocs", handle_savedocs);
    reg!("savedocs",                       handle_savedocs); // short alias used by browser
    reg!("frappe.desk.form.save.cancel",   handle_form_cancel);
    reg!("frappe.desk.form.save.discard",  handle_noop_ok);

    // ── Form utils (comments, next, attach) ───────────────────────────────
    reg!("frappe.desk.form.utils.add_comment",              form_utils::handle_add_comment);
    reg!("frappe.desk.form.utils.update_comment",           form_utils::handle_update_comment);
    reg!("frappe.desk.form.utils.update_comment_publicity", form_utils::handle_update_comment_publicity);
    reg!("frappe.desk.form.utils.get_next",                 form_utils::handle_get_next);
    reg!("frappe.desk.form.utils.remove_attach",            form_utils::handle_remove_attach);

    // ── Assignments ───────────────────────────────────────────────────────
    reg!("frappe.desk.form.assign_to.add",            form_utils::handle_assign_to_add);
    reg!("frappe.desk.form.assign_to.add_multiple",   form_utils::handle_assign_to_add_multiple);
    reg!("frappe.desk.form.assign_to.remove",         form_utils::handle_assign_to_remove);
    reg!("frappe.desk.form.assign_to.remove_multiple",form_utils::handle_assign_to_remove_multiple);
    reg!("frappe.desk.form.assign_to.close",          form_utils::handle_assign_to_close);

    // ── Linked documents ──────────────────────────────────────────────────
    reg!("frappe.desk.form.linked_with.get",                  form_utils::handle_linked_with_get);
    reg!("frappe.desk.form.linked_with.get_submitted_linked_docs", form_utils::handle_get_submitted_linked_docs);
    reg!("frappe.desk.form.linked_with.cancel_all_linked_docs",    form_utils::handle_cancel_all_linked_docs);

    // ── Document follow ───────────────────────────────────────────────────
    reg!("frappe.desk.form.document_follow.follow_document",   form_utils::handle_follow_document);
    reg!("frappe.desk.form.document_follow.unfollow_document", form_utils::handle_unfollow_document);
    reg!("frappe.desk.form.document_follow.get_follow_users",  form_utils::handle_get_follow_users);
    reg!("frappe.desk.form.document_follow.update_follow",     form_utils::handle_update_follow);

    // ── List view / reportview ────────────────────────────────────────────
    reg!("frappe.desk.reportview.get",                  handle_reportview_get);
    reg!("frappe.desk.reportview.get_count",            handle_reportview_get_count);
    reg!("frappe.desk.reportview.get_list",             handle_reportview_get_list);
    reg!("frappe.desk.reportview.delete_items",         stubs::handle_reportview_delete_items);
    reg!("frappe.desk.reportview.get_sidebar_stats",    stubs::handle_reportview_get_sidebar_stats);

    // ── List view settings ────────────────────────────────────────────────
    reg!("frappe.desk.listview.get_list_settings",
         listview::handle_get_list_settings);
    reg!("frappe.desk.listview.get_group_by_count",
         listview::handle_get_group_by_count);
    reg!("frappe.desk.doctype.list_view_settings.list_view_settings.save_listview_settings",
         listview::handle_save_listview_settings);
    reg!("frappe.desk.doctype.list_view_settings.list_view_settings.get_default_listview_fields",
         listview::handle_get_default_listview_fields);

    // ── Search ────────────────────────────────────────────────────────────
    reg!("frappe.desk.search.search_link",          search::handle_search_link);
    reg!("frappe.desk.search.search_widget",        search::handle_search_widget);
    reg!("frappe.desk.search.get_link_title",       search::handle_get_link_title);
    reg!("frappe.desk.search.get_names_for_mentions", search::handle_get_names_for_mentions);
    reg!("frappe.desk.search.get_search_tags",      search::handle_get_search_tags);
    reg!("frappe.utils.global_search.search",       stubs::handle_global_search);

    // ── Notifications ─────────────────────────────────────────────────────
    reg!("frappe.desk.notifications.get_notification_info",
         notifications::handle_get_notification_info);
    reg!("frappe.desk.notifications.get_open_count",
         notifications::handle_get_open_count);
    // Legacy path used in some versions
    reg!("frappe.desk.notifications.get_notifications",
         notifications::handle_get_notification_info);
    reg!("frappe.desk.doctype.notification_log.notification_log.get_notification_logs",
         notifications::handle_get_notification_logs);
    reg!("frappe.desk.doctype.notification_log.notification_log.mark_as_read",
         notifications::handle_mark_notification_as_read);
    reg!("frappe.desk.doctype.notification_log.notification_log.mark_all_as_read",
         notifications::handle_mark_all_notifications_as_read);
    reg!("frappe.desk.doctype.notification_log.notification_log.trigger_indicator_hide",
         notifications::handle_trigger_indicator_hide);
    reg!("frappe.desk.doctype.notification_settings.notification_settings.set_seen_value",
         notifications::handle_set_notification_seen);

    // ── User settings (model.utils) ───────────────────────────────────────
    reg!("frappe.model.utils.user_settings.get",  model::handle_user_settings_get);
    reg!("frappe.model.utils.user_settings.save", model::handle_user_settings_save);

    // ── Workflow (model.workflow) ─────────────────────────────────────────
    reg!("frappe.model.workflow.get_transitions",           model::handle_workflow_get_transitions);
    reg!("frappe.model.workflow.apply_workflow",            model::handle_workflow_apply);
    reg!("frappe.model.workflow.get_common_transition_actions", model::handle_workflow_get_common_actions);
    reg!("frappe.model.workflow.bulk_workflow_approval",    model::handle_workflow_bulk_approval);
    reg!("frappe.model.workflow.can_cancel_document",       model::handle_workflow_can_cancel);

    // ── Rename doc ────────────────────────────────────────────────────────
    reg!("frappe.model.rename_doc.update_document_title", model::handle_rename_doc);

    // ── Sidebar / desktop / workspace ─────────────────────────────────────
    reg!("frappe.desk.desk.get_desk_sidebar_items",      handle_get_sidebar_items);
    reg!("frappe.desk.desktop.get_workspace_sidebar_items", handle_get_workspace_sidebar_items);
    reg!("frappe.desk.desktop.get_desktop_page",         handle_get_desktop_page);
    reg!("frappe.desk.desktop.get_desktop_page_data",    handle_get_desktop_page);
    reg!("frappe.desk.desktop.get_installed_apps",       handle_get_installed_apps);
    reg!("frappe.desk.desktop.update_onboarding_step",   stubs::handle_update_onboarding_step);
    reg!("frappe.desk.desk_page.getpage",                handle_get_desk_page);
    reg!("frappe.desk.doctype.workspace.workspace.get_workspaces", handle_get_workspaces);
    reg!("frappe.desk.doctype.workspace.workspace.new_page",       handle_noop_ok);
    reg!("frappe.desk.doctype.workspace.workspace.save_page",      handle_noop_ok);
    reg!("frappe.desk.doctype.workspace_sidebar.workspace_sidebar.add_sidebar_items",
         stubs::handle_workspace_add_sidebar_items);

    // ── Boot info ─────────────────────────────────────────────────────────
    reg!("frappe.utils.boot.get_boot_info", handle_get_boot_info);

    // ── Route history ─────────────────────────────────────────────────────
    reg!("frappe.desk.doctype.route_history.route_history.deferred_insert", handle_noop_ok);

    // ── Tags ──────────────────────────────────────────────────────────────
    reg!("frappe.desk.doctype.tag.tag.get_tags",                    stubs::handle_tags_get);
    reg!("frappe.desk.doctype.tag.tag.get_tags_list",               stubs::handle_tags_get_list);
    reg!("frappe.desk.doctype.tag.tag.get_tags_list_for_awesomebar",stubs::handle_tags_get_for_awesomebar);
    reg!("frappe.desk.doctype.tag.tag.get_documents_for_tag",       stubs::handle_tags_get_documents);
    reg!("frappe.desk.doctype.tag.tag.add_tag",                     stubs::handle_tags_add);
    reg!("frappe.desk.doctype.tag.tag.add_tags",                    stubs::handle_tags_add);
    reg!("frappe.desk.doctype.tag.tag.remove_tag",                  stubs::handle_tags_remove);

    // ── Like / share ──────────────────────────────────────────────────────
    reg!("frappe.desk.like.toggle_like",  stubs::handle_toggle_like);
    reg!("frappe.share.add",              stubs::handle_share_add);
    reg!("frappe.share.get_users",        stubs::handle_share_get_users);
    reg!("frappe.share.set_permission",   stubs::handle_share_set_permission);

    // ── Link preview ──────────────────────────────────────────────────────
    reg!("frappe.desk.link_preview.get_preview_data", stubs::handle_get_link_preview);

    // ── Query report ──────────────────────────────────────────────────────
    reg!("frappe.desk.query_report.run", stubs::handle_query_report_run);

    // ── Kanban ────────────────────────────────────────────────────────────
    reg!("frappe.desk.doctype.kanban_board.kanban_board.get_kanban_boards",  stubs::handle_get_kanban_boards);
    reg!("frappe.desk.doctype.kanban_board.kanban_board.quick_kanban_board", stubs::handle_quick_kanban_board);
    reg!("frappe.desk.doctype.kanban_board.kanban_board.save_settings",      stubs::handle_kanban_save_settings);

    // ── Dashboard / number cards ──────────────────────────────────────────
    reg!("frappe.desk.doctype.dashboard_chart.dashboard_chart.get",              stubs::handle_dashboard_chart_get);
    reg!("frappe.desk.doctype.dashboard_chart.dashboard_chart.get_charts_for_user", stubs::handle_get_charts_for_user);
    reg!("frappe.desk.doctype.dashboard_chart.dashboard_chart.create_dashboard_chart", stubs::handle_create_dashboard_chart);
    reg!("frappe.desk.doctype.dashboard_chart_source.dashboard_chart_source.get_config", stubs::handle_dashboard_source_get_config);
    reg!("frappe.desk.doctype.dashboard_settings.dashboard_settings.create_dashboard_settings", stubs::handle_create_dashboard_settings);
    reg!("frappe.desk.doctype.dashboard_settings.dashboard_settings.save_chart_config", stubs::handle_save_chart_config);
    reg!("frappe.desk.doctype.number_card.number_card.get_result",            stubs::handle_number_card_get_result);
    reg!("frappe.desk.doctype.number_card.number_card.get_cards_for_user",    stubs::handle_get_cards_for_user);
    reg!("frappe.desk.doctype.number_card.number_card.create_number_card",    stubs::handle_create_number_card);
    reg!("frappe.desk.doctype.number_card.number_card.get_percentage_difference", stubs::handle_number_card_get_percentage);

    // ── Onboarding ────────────────────────────────────────────────────────
    reg!("frappe.desk.doctype.onboarding_step.onboarding_step.get_onboarding_steps",
         stubs::handle_get_onboarding_steps);

    // ── Calendar ──────────────────────────────────────────────────────────
    reg!("frappe.desk.calendar.get_events",          stubs::handle_calendar_get_events);
    reg!("frappe.desk.calendar.update_event",        stubs::handle_calendar_update_event);
    reg!("frappe.desk.doctype.event.event.get_events", stubs::handle_event_get_events);

    // ── Tree view ─────────────────────────────────────────────────────────
    reg!("frappe.desk.treeview.get_all_nodes", stubs::handle_treeview_get_all_nodes);
    reg!("frappe.desk.treeview.get_children",  stubs::handle_treeview_get_children);
    reg!("frappe.desk.treeview.add_node",      stubs::handle_treeview_add_node);

    // ── Note ──────────────────────────────────────────────────────────────
    reg!("frappe.desk.doctype.note.note.mark_as_seen",  stubs::handle_note_mark_seen);
    reg!("frappe.desk.doctype.note.note.reset_notes",   stubs::handle_note_reset);

    // ── Bulk update ───────────────────────────────────────────────────────
    reg!("frappe.desk.doctype.bulk_update.bulk_update.submit_cancel_or_update_docs",
         stubs::handle_bulk_update);

    // ── Sidebar item group / reports ──────────────────────────────────────
    reg!("frappe.desk.doctype.sidebar_item_group.sidebar_item_group.get_reports",
         stubs::handle_get_sidebar_report_items);

    // ── Custom HTML block ─────────────────────────────────────────────────
    reg!("frappe.desk.doctype.custom_html_block.custom_html_block.get_custom_blocks_for_user",
         stubs::handle_get_custom_blocks_for_user);

    // ── System console (intentionally blocked) ────────────────────────────
    reg!("frappe.desk.doctype.system_console.system_console.execute_code",
         stubs::handle_execute_code);

    // ── Client extras ─────────────────────────────────────────────────────
    reg!("frappe.client.get_single_value",    handle_get_single_value);
    reg!("frappe.client.get_doc_permissions", handle_get_doc_permissions_stub);
    reg!("frappe.model.db_query.get_list",    handle_model_get_list);

    // ── User permissions ──────────────────────────────────────────────────
    reg!("frappe.core.doctype.user_permission.user_permission.get_user_permissions",
         handle_get_user_permissions);

    // ── Core user ─────────────────────────────────────────────────────────
    reg!("frappe.core.doctype.user.user.get_all_roles",          stubs::handle_user_get_all_roles);
    reg!("frappe.core.doctype.user.user.get_perm_info",          stubs::handle_user_get_perm_info);
    reg!("frappe.core.doctype.user.user.switch_theme",           stubs::handle_user_switch_theme);
    reg!("frappe.core.doctype.user.user.test_password_strength", stubs::handle_user_test_password_strength);
    reg!("frappe.core.doctype.user.user.verify_password",        stubs::handle_user_verify_password);
    reg!("frappe.core.doctype.user.user.get_email_awaiting",     stubs::handle_user_get_email_awaiting);

    // ── Core file ─────────────────────────────────────────────────────────
    reg!("frappe.core.api.file.get_attached_images",  stubs::handle_file_get_attached_images);
    reg!("frappe.core.api.file.create_new_folder",    stubs::handle_file_create_new_folder);
    reg!("frappe.core.api.file.move_file",            stubs::handle_file_move_file);
    reg!("frappe.core.api.file.unzip_file",           stubs::handle_file_unzip_file);
    reg!("frappe.utils.file_manager.add_attachments", stubs::handle_file_add_attachments);

    // ── Core access log ───────────────────────────────────────────────────
    reg!("frappe.core.doctype.access_log.access_log.make_access_log",
         stubs::handle_make_access_log);

    // ── Communication / email ─────────────────────────────────────────────
    reg!("frappe.core.doctype.communication.email.make", stubs::handle_email_make);
    reg!("frappe.email.doctype.email_account.email_account.set_email_password",
         stubs::handle_email_set_password);
    reg!("frappe.email.doctype.email_template.email_template.get_email_template",
         stubs::handle_email_get_template);
    reg!("frappe.email.email_body.get_email_html",  stubs::handle_email_get_html);
    reg!("frappe.email.get_contact_list",           stubs::handle_email_get_contact_list);

    // ── Session defaults ──────────────────────────────────────────────────
    reg!("frappe.core.doctype.session_default_settings.session_default_settings.get_session_default_values",
         handle_get_session_defaults);
    reg!("frappe.core.doctype.session_default_settings.session_default_settings.set_session_default_values",
         stubs::handle_set_session_defaults);

    // ── Submission queue ──────────────────────────────────────────────────
    reg!("frappe.core.doctype.submission_queue.submission_queue.get_latest_submissions",
         stubs::handle_get_latest_submissions);

    // ── Server script autocompletion ──────────────────────────────────────
    reg!("frappe.core.doctype.server_script.server_script.get_autocompletion_items",
         stubs::handle_server_script_autocompletion);

    // ── Row size utilization ──────────────────────────────────────────────
    reg!("frappe.core.doctype.doctype.doctype.get_row_size_utilization",
         stubs::handle_get_row_size_utilization);

    // ── Permitted documents for user ──────────────────────────────────────
    reg!("frappe.core.report.permitted_documents_for_user.permitted_documents_for_user.query_doctypes",
         stubs::handle_query_permitted_doctypes);

    // ── Change log ────────────────────────────────────────────────────────
    reg!("frappe.utils.change_log.get_versions",               handle_get_versions);
    reg!("frappe.utils.change_log.show_update_popup",          stubs::handle_change_log_show_popup);
    reg!("frappe.utils.change_log.update_last_known_versions", stubs::handle_change_log_update_last_known);

    // ── Diff / version ────────────────────────────────────────────────────
    reg!("frappe.utils.diff.get_version_diff", stubs::handle_diff_get_version_diff);
    reg!("frappe.utils.diff.version_query",    stubs::handle_diff_version_query);

    // ── Print ─────────────────────────────────────────────────────────────
    reg!("frappe.utils.print_format.download_multi_pdf_async", stubs::handle_print_download_multi_pdf);

    // ── Nestedset ─────────────────────────────────────────────────────────
    reg!("frappe.utils.nestedset.rebuild_tree", stubs::handle_nestedset_rebuild_tree);

    // ── Automation ────────────────────────────────────────────────────────
    reg!("frappe.automation.doctype.assignment_rule.assignment_rule.bulk_apply",
         stubs::handle_automation_bulk_apply);
    reg!("frappe.automation.doctype.auto_repeat.auto_repeat.make_auto_repeat",
         handle_noop_ok);
    reg!("frappe.automation.doctype.reminder.reminder.create_new_reminder",
         stubs::handle_automation_create_reminder);

    // ── Mapper ────────────────────────────────────────────────────────────
    reg!("frappe.model.mapper.make_mapped_doc", stubs::handle_make_mapped_doc);

    // ── Geo / integrations ────────────────────────────────────────────────
    reg!("frappe.geo.country_info.get_country_timezone_info",
         stubs::handle_get_country_timezone_info);
    reg!("frappe.integrations.doctype.geolocation_settings.geolocation_settings.autocomplete",
         stubs::handle_geolocation_autocomplete);

    // ── Contacts ──────────────────────────────────────────────────────────
    reg!("frappe.contacts.doctype.address.address.get_address_display",
         stubs::handle_get_address_display);

    // ── Website ───────────────────────────────────────────────────────────
    reg!("frappe.website.doctype.web_page_view.web_page_view.get_page_view_count",
         stubs::handle_get_page_view_count);

    // ── Translate ─────────────────────────────────────────────────────────
    reg!("frappe.translate.update_translations_for_source", handle_noop_ok);
}

// ── frappe.desk.form.load.getdoctype ─────────────────────────────────────────

async fn append_doctype_children(db: &DbAdapter, dt_name: &str, doc_obj: &mut Value) {
    let child_filter = json!({"parent": dt_name, "parenttype": "DocType"});

    // fields
    let fields_val: Vec<Value> = get_list(db, "DocField", None, Some(&child_filter), 500, 0)
        .await.unwrap_or_default().into_iter()
        .map(|r| serde_json::to_value(r).unwrap_or(Value::Null)).collect();

    // permissions (DocPerm) — critical: creates rights_without_if_owner Set in perm.js
    let perms_val: Vec<Value> = get_list(db, "DocPerm", None, Some(&child_filter), 100, 0)
        .await.unwrap_or_default().into_iter()
        .map(|r| serde_json::to_value(r).unwrap_or(Value::Null)).collect();

    // states (DocState) — critical: indicator.js reads .states.length
    let states_val: Vec<Value> = get_list(db, "DocState", None, Some(&child_filter), 50, 0)
        .await.unwrap_or_default().into_iter()
        .map(|r| serde_json::to_value(r).unwrap_or(Value::Null)).collect();

    if let Value::Object(ref mut map) = doc_obj {
        map.insert("fields".into(), Value::Array(fields_val));

        // If there are no DocPerm records seeded, inject a System Manager catch-all
        // so get_role_permissions always creates rights_without_if_owner Set.
        if !perms_val.is_empty() {
            map.insert("permissions".into(), Value::Array(perms_val));
        } else {
            map.insert("permissions".into(), json!([
                {"role": "System Manager", "permlevel": 0, "read": 1, "write": 1,
                 "create": 1, "delete": 1, "submit": 0, "cancel": 0, "amend": 0,
                 "report": 1, "export": 1, "import": 1, "share": 1,
                 "print": 1, "email": 1, "if_owner": 0},
                {"role": "Administrator", "permlevel": 0, "read": 1, "write": 1,
                 "create": 1, "delete": 1, "submit": 0, "cancel": 0, "amend": 0,
                 "report": 1, "export": 1, "import": 1, "share": 1,
                 "print": 1, "email": 1, "if_owner": 0}
            ]));
        }
        map.insert("states".into(), Value::Array(states_val));
        map.entry("links".to_string()).or_insert(Value::Array(vec![]));
        map.entry("actions".to_string()).or_insert(Value::Array(vec![]));
    }
}

async fn handle_getdoctype(
    site: Arc<SiteState>,
    params: HashMap<String, Value>,
) -> Result<Value, SpotError> {
    let doctype = require_str(&params, "doctype")?;

    let dt_doc = get_doc(&site.db, "DocType", doctype)
        .await
        .map_err(SpotError::from)?;

    let mut dt_obj = dt_doc.as_dict();
    append_doctype_children(&site.db, doctype, &mut dt_obj).await;

    // Collect child doctype names from Table / Table MultiSelect fields
    let child_dt_names: Vec<String> = if let Value::Array(ref fields) = dt_obj.get("fields").cloned().unwrap_or(Value::Array(vec![])) {
        fields.iter()
            .filter(|f| {
                let ft = f.get("fieldtype").and_then(Value::as_str).unwrap_or("");
                ft == "Table" || ft == "Table MultiSelect"
            })
            .filter_map(|f| f.get("options").and_then(Value::as_str).map(str::to_string))
            .filter(|s| !s.is_empty())
            .collect()
    } else {
        vec![]
    };

    // Build docs: main doctype + flat child doctypes (one level, no recursion)
    let mut docs = vec![dt_obj];
    for child_name in child_dt_names {
        if let Ok(child_doc_raw) = get_doc(&site.db, "DocType", &child_name).await {
            let mut child_obj = child_doc_raw.as_dict();
            append_doctype_children(&site.db, &child_name, &mut child_obj).await;
            docs.push(child_obj);
        }
    }

    Ok(json!({
        "docs":          docs,
        "lang_modified": null,
    }))
}

// ── frappe.desk.form.load.getdoc ─────────────────────────────────────────────

async fn handle_getdoc(
    site: Arc<SiteState>,
    params: HashMap<String, Value>,
) -> Result<Value, SpotError> {
    let doctype = require_str(&params, "doctype")?;
    let name = require_str(&params, "name")?;

    let user = params.get("__current_user").and_then(Value::as_str).unwrap_or("Administrator");

    let doc_result = get_doc(&site.db, doctype, name).await;

    // For Single doctypes, a missing record is normal — return an empty default doc.
    let doc = match doc_result {
        Ok(d) => d,
        Err(spotledger_db::error::DbError::NotFound { .. }) => {
            // Check if this is a Single doctype
            let is_single = get_doc(&site.db, "DocType", doctype).await
                .ok()
                .and_then(|dt| dt.get_value("issingle"))
                .and_then(|v| match &v {
                    Value::Number(n) => n.as_i64().map(|n| n != 0),
                    Value::Bool(b) => Some(*b),
                    _ => None,
                })
                .unwrap_or(false);

            if is_single {
                // Return an empty default doc (name == doctype for Singles)
                let mut empty = spotledger_core::document::Document::default();
                empty.doctype = doctype.to_string();
                empty.name = doctype.to_string();
                empty.set("__islocal", json!(1));
                empty.set("__unsaved", json!(1));
                empty
            } else {
                return Err(SpotError::NotFound {
                    doctype: doctype.to_string(),
                    name: name.to_string(),
                });
            }
        }
        Err(e) => return Err(SpotError::from(e)),
    };

    let perms = get_doc_permissions(&site.db, user, doctype)
        .await
        .map_err(SpotError::from)?;

    let link_titles = build_link_titles(&site.db, doctype, &doc).await;

    let mut owner_users: Vec<String> = vec![];
    if let Some(owner) = doc.get_str("owner") { owner_users.push(owner.to_owned()); }
    if let Some(mb) = doc.get_str("modified_by") { owner_users.push(mb.to_owned()); }
    owner_users.dedup();
    let docinfo_user_info = build_user_info(&site.db, &owner_users).await;

    // B22: mark_as_seen + fetch live docinfo from SurrealDB fn:: calls
    let _ = site.db
        .execute(
            "fn::mark_as_seen($user, $dt, $dn)",
            vec![
                ("user".into(), user.to_string().into()),
                ("dt".into(),   doctype.to_string().into()),
                ("dn".into(),   name.to_string().into()),
            ],
        )
        .await;

    // Fetch comments, versions, tags, shares, assignments via fn::
    let comments: Vec<Value> = site.db
        .run(
            "RETURN fn::get_communications($dt, $dn, NONE)",
            vec![
                ("dt".into(), doctype.to_string().into()),
                ("dn".into(), name.to_string().into()),
            ],
        )
        .await
        .unwrap_or_default();

    let versions: Vec<Value> = site.db
        .run(
            "SELECT name, creation, owner, data FROM tabVersion \
             WHERE ref_doctype = $dt AND docname = $dn ORDER BY creation DESC LIMIT 20",
            vec![
                ("dt".into(), doctype.to_string().into()),
                ("dn".into(), name.to_string().into()),
            ],
        )
        .await
        .unwrap_or_default();

    let tags_rows: Vec<Value> = site.db
        .run(
            "RETURN fn::get_tags($dt, $dn)",
            vec![
                ("dt".into(), doctype.to_string().into()),
                ("dn".into(), name.to_string().into()),
            ],
        )
        .await
        .unwrap_or_default();
    let tags_str = tags_rows.iter()
        .filter_map(|v| v.get("tag").and_then(Value::as_str))
        .collect::<Vec<_>>().join(",");

    let shared: Vec<Value> = site.db
        .run(
            "RETURN fn::share_get_users($dt, $dn)",
            vec![
                ("dt".into(), doctype.to_string().into()),
                ("dn".into(), name.to_string().into()),
            ],
        )
        .await
        .unwrap_or_default();

    let assignments: Vec<Value> = site.db
        .run(
            "RETURN fn::get_assignments($dt, $dn)",
            vec![
                ("dt".into(), doctype.to_string().into()),
                ("dn".into(), name.to_string().into()),
            ],
        )
        .await
        .unwrap_or_default();

    let followers: Vec<Value> = site.db
        .run(
            "RETURN fn::get_followers($dt, $dn)",
            vec![
                ("dt".into(), doctype.to_string().into()),
                ("dn".into(), name.to_string().into()),
            ],
        )
        .await
        .unwrap_or_default();
    let is_followed = followers.iter().any(|f| {
        f.get("user").and_then(Value::as_str).map(|u| u == user).unwrap_or(false)
    });

    let docinfo = json!({
        "doctype":         doctype,
        "name":            name,
        "attachments":     [],
        "comments":        comments,
        "communications":  [],
        "automated_messages": [],
        "assignments":     assignments,
        "shared":          shared,
        "views":           [],
        "energy_point_logs": [],
        "additional_timeline_content": [],
        "milestones":      [],
        "versions":        versions,
        "is_document_followed": is_followed,
        "tags":            tags_str,
        "document_email":  null,
        "share_logs":      [],
        "assignment_logs": [],
        "attachment_logs": [],
        "info_logs":       [],
        "like_logs":       [],
        "workflow_logs":   [],
        "permissions":     perms.to_json(),
        "user_info":       docinfo_user_info,
    });

    Ok(json!({
        "docs":         [doc.as_dict()],
        "docinfo":      docinfo,
        "_link_titles": link_titles,
    }))
}

// ── frappe.desk.form.load.get_docinfo ────────────────────────────────────────

async fn handle_get_docinfo(
    site: Arc<SiteState>,
    params: HashMap<String, Value>,
) -> Result<Value, SpotError> {
    let doctype = params.get("doctype").and_then(Value::as_str).unwrap_or("").to_string();
    let name = params.get("name").and_then(Value::as_str).unwrap_or("").to_string();

    let permissions = if !doctype.is_empty() {
        get_doc_permissions(&site.db, "Administrator", &doctype).await
            .map(|p| p.to_json())
            .unwrap_or_else(|_| json!({"read": 1, "write": 1}))
    } else {
        json!({"read": 1, "write": 1})
    };

    Ok(json!({
        "doctype": doctype,
        "name": name,
        "attachments": [],
        "comments": [],
        "communications": [],
        "automated_messages": [],
        "versions": [],
        "assignments": [],
        "permissions": permissions,
        "shared": [],
        "views": [],
        "additional_timeline_content": [],
        "energy_point_logs": [],
        "milestones": [],
        "is_document_followed": false,
        "tags": "",
        "share_logs": [],
        "assignment_logs": [],
        "attachment_logs": [],
        "info_logs": [],
        "like_logs": [],
        "workflow_logs": [],
        "workflow_logs": [],
        "info_logs": [],
        "assignment_logs": [],
        "attachment_logs": [],
        "like_logs": [],
        "document_timeline_content": [],
        "user_info": {},
        "workflow_action_buttons": [],
        "custom_perm_types": [],
        "document_email": null,
    }))
}

// ── frappe.desk.form.load.get_communications ──────────────────────────────────

async fn handle_get_communications(
    _site: Arc<SiteState>,
    _params: HashMap<String, Value>,
) -> Result<Value, SpotError> {
    Ok(Value::Array(vec![]))
}

// ── frappe.desk.reportview.get ────────────────────────────────────────────────

async fn handle_reportview_get(
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
    let limit = params.get("page_length").or_else(|| params.get("limit"))
        .and_then(Value::as_u64).map(|n| n as usize).unwrap_or(20);
    let start = params.get("start").and_then(Value::as_u64).map(|n| n as usize).unwrap_or(0);

    let rows = get_list(&site.db, doctype, fields_opt, filters, limit, start)
        .await
        .map_err(SpotError::from)?;

    let keys: Vec<String> = if field_strings.is_empty() {
        rows.first()
            .map(|r| r.keys().cloned().collect())
            .unwrap_or_default()
    } else {
        let mut k = field_strings.clone();
        if !k.contains(&"name".to_string()) {
            k.insert(0, "name".to_string());
        }
        k
    };

    let values: Vec<Value> = rows
        .iter()
        .map(|row| {
            Value::Array(
                keys.iter()
                    .map(|k| row.get(k).cloned().unwrap_or(Value::Null))
                    .collect(),
            )
        })
        .collect();

    let mut assign_users: Vec<String> = vec![];
    if keys.contains(&"_assign".to_string()) {
        let mut seen: HashSet<String> = HashSet::new();
        for row in &rows {
            if let Some(Value::String(assign_str)) = row.get("_assign") {
                if let Ok(Value::Array(arr)) = serde_json::from_str(assign_str) {
                    for u in arr {
                        if let Value::String(uname) = u {
                            if seen.insert(uname.clone()) {
                                assign_users.push(uname);
                            }
                        }
                    }
                }
            }
        }
    }
    let user_info = build_user_info(&site.db, &assign_users).await;

    Ok(json!({
        "values":    values,
        "keys":      keys,
        "user_info": user_info,
    }))
}

// ── frappe.desk.reportview.get_list ──────────────────────────────────────────
// frappe.db.get_list() resolves r.message directly as an array of objects.
// Unlike reportview.get (which uses {keys:[], values:[]} column format), this
// must return a plain JSON array.

async fn handle_reportview_get_list(
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
        _ => vec!["name".to_string()],
    };
    let field_refs: Vec<&str> = field_strings.iter().map(String::as_str).collect();
    let fields_opt = if field_refs.is_empty() { None } else { Some(field_refs.as_slice()) };

    let filters = params.get("filters").or_else(|| params.get("filter_list"));
    let limit = params.get("limit").or_else(|| params.get("page_length"))
        .and_then(Value::as_u64).map(|n| n as usize).unwrap_or(20);
    let start = params.get("start").and_then(Value::as_u64).map(|n| n as usize).unwrap_or(0);

    let rows = get_list(&site.db, doctype, fields_opt, filters, limit, start)
        .await
        .map_err(SpotError::from)?;

    let values: Vec<Value> = rows.into_iter()
        .map(|r| Value::Object(r.into_iter().collect()))
        .collect();
    Ok(Value::Array(values))
}

// ── frappe.desk.reportview.get_count ─────────────────────────────────────────
async fn handle_reportview_get_count(
    site: Arc<SiteState>,
    params: HashMap<String, Value>,
) -> Result<Value, SpotError> {
    let doctype = require_str(&params, "doctype")?;
    let filters = params.get("filters");
    let count = spotledger_db::document::get_count(&site.db, doctype, filters)
        .await
        .map_err(SpotError::from)?;
    Ok(Value::Number(count.into()))
}

// ── frappe.desk.form.save.savedocs ───────────────────────────────────────────

async fn handle_savedocs(
    site: Arc<SiteState>,
    params: HashMap<String, Value>,
) -> Result<Value, SpotError> {
    // Frappe's request.js JSON.stringify's the doc object before sending,
    // so it arrives as a JSON string in the form body. parse_form_params then
    // further parses it into a Value::Object. Accept both cases.
    let doc_val: Value = match params.get("doc") {
        Some(Value::Object(o)) => Value::Object(o.clone()),
        Some(Value::String(s)) => serde_json::from_str(s)
            .map_err(|e| SpotError::Validation(format!("invalid doc JSON: {e}")))?,
        _ => return Err(SpotError::Validation("doc param required".into())),
    };

    let action = params.get("action").and_then(Value::as_str).unwrap_or("Save");
    let doctype = doc_val.get("doctype").and_then(Value::as_str)
        .ok_or_else(|| SpotError::Validation("doctype missing in doc".into()))?
        .to_string();

    // The browser's original local name (e.g. "new-todo-abc123xyz").
    // Captured before resolving to a final saved name.
    let browser_local_name = doc_val.get("name")
        .and_then(Value::as_str)
        .filter(|s| s.starts_with("new-"))
        .map(str::to_string);

    // __newname = user typed name (Prompt autoname), present when the form shows a name field.
    let explicit_name = doc_val.get("__newname")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .map(str::to_string);

    let is_new = explicit_name.is_none()
        && (browser_local_name.is_some()
            || doc_val.get("name").and_then(Value::as_str).map(|n| n.is_empty()).unwrap_or(true)
            || doc_val.get("__islocal").and_then(Value::as_i64).unwrap_or(0) == 1);

    // The browser's local name returned as `localname` so sync.js rename_after_save
    // can update locals[] from the temp name to the real saved name.
    let local_name = if is_new { browser_local_name.clone() } else { None };

    // Resolve final name following Frappe's autoname rules:
    //   1. Prompt / __newname  → explicit_name
    //   2. field:fieldname     → value of that field in the doc
    //   3. hash / empty / else → generate a random name
    let saved_name: String = if let Some(n) = explicit_name {
        // autoname = "Prompt" or user supplied __newname
        n
    } else if is_new {
        // Look up autoname from tabDocType
        let autoname = get_doc(&site.db, "DocType", &doctype).await
            .ok()
            .and_then(|dt| dt.get_str("autoname").map(str::to_string));

        match autoname.as_deref() {
            Some(an) if an.starts_with("field:") => {
                // autoname = "field:fieldname" → use that field's value as the name
                let fieldname = &an["field:".len()..];
                doc_val.get(fieldname)
                    .and_then(Value::as_str)
                    .filter(|s| !s.is_empty())
                    .map(str::to_string)
                    .ok_or_else(|| SpotError::Validation(
                        format!("autoname field '{fieldname}' is empty or missing")
                    ))?
            }
            Some(an) if an.starts_with("format:") => {
                // autoname = "format:{fieldname}-..." — simple single-field case
                // Full Jinja format strings not yet supported; fall back to random.
                let _ = an;
                use std::time::{SystemTime, UNIX_EPOCH};
                let ts = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().subsec_nanos();
                format!("{}-{:08x}", doctype.to_lowercase().replace(' ', "-"), ts)
            }
            _ => {
                // hash, None, or anything else → random
                use std::time::{SystemTime, UNIX_EPOCH};
                let ts = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().subsec_nanos();
                format!("{}-{:08x}", doctype.to_lowercase().replace(' ', "-"), ts)
            }
        }
    } else {
        // Existing doc — name from doc itself
        doc_val.get("name")
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .ok_or_else(|| SpotError::Validation("name missing for existing doc".into()))?
    };

    // Build a clean parent doc: strip UI-only meta fields and child table arrays.
    // Child table rows (identified by having a parentfield key, or being arrays of objects)
    // are saved separately to their own SurrealDB tables.
    let mut parent_fields = serde_json::Map::new();
    let mut child_rows: Vec<Value> = vec![];  // collect all child table rows

    if let Value::Object(ref map) = doc_val {
        for (k, v) in map {
            match k.as_str() {
                // Strip UI-only fields
                "__islocal" | "__unsaved" | "__newname" | "__run_link_triggers"
                | "__last_sync_on" | "__dirty" | "__checked" | "__doctype_fields" => {}
                _ => {
                    // Arrays are always child table rows in Frappe's data model.
                    // Frappe never stores child table arrays on the parent record —
                    // they live in their own tables (tabDocField, tabDocPerm, etc.).
                    // Collect non-empty arrays as child rows; skip empty arrays entirely.
                    if let Value::Array(arr) = v {
                        for row in arr {
                            if row.as_object().is_some() {
                                child_rows.push(row.clone());
                            }
                        }
                        // Never include array fields in the parent record
                        continue;
                    }
                    parent_fields.insert(k.clone(), v.clone());
                }
            }
        }
    }
    parent_fields.insert("name".into(), Value::String(saved_name.clone()));

    // Inject standard Frappe tracking fields if not already present
    let current_user = params.get("__current_user").and_then(Value::as_str).unwrap_or("Administrator");
    parent_fields.entry("owner".to_string()).or_insert_with(|| Value::String(current_user.to_string()));
    parent_fields.entry("modified_by".to_string()).or_insert_with(|| Value::String(current_user.to_string()));
    parent_fields.entry("docstatus".to_string()).or_insert_with(|| Value::Number(0.into()));

    let parent_val = Value::Object(parent_fields);

    match action {
        "Save" | "Update" => {
            if is_new {
                spotledger_db::document::insert_doc(&site.db, &doctype, &parent_val).await
                    .map_err(|e| SpotError::Db(e.to_string()))?;
            } else {
                spotledger_db::document::upsert_doc(&site.db, &doctype, &saved_name, &parent_val).await
                    .map_err(|e| SpotError::Db(e.to_string()))?;
                site.doc_cache.remove(&(doctype.clone(), saved_name.clone())).await;
            }
        }
        "Submit" => {
            spotledger_db::document::submit_doc(&site.db, &doctype, &saved_name).await
                .map_err(|e| SpotError::Db(e.to_string()))?;
            site.doc_cache.remove(&(doctype.clone(), saved_name.clone())).await;
        }
        "Cancel" => {
            spotledger_db::document::cancel_doc(&site.db, &doctype, &saved_name).await
                .map_err(|e| SpotError::Db(e.to_string()))?;
            site.doc_cache.remove(&(doctype.clone(), saved_name.clone())).await;
        }
        _ => {}
    }

    // Save child table rows to their own tables
    for child in &child_rows {
        if let Value::Object(row_map) = child {
            let child_doctype = row_map.get("doctype").and_then(Value::as_str).unwrap_or("");
            if child_doctype.is_empty() { continue; }
            let child_name = row_map.get("name").and_then(Value::as_str).unwrap_or("").to_string();
            let mut row_val = child.clone();
            // Inject parent linkage
            if let Some(obj) = row_val.as_object_mut() {
                obj.insert("parent".into(), Value::String(saved_name.clone()));
                obj.insert("parenttype".into(), Value::String(doctype.clone()));
                obj.remove("__islocal");
                obj.remove("__unsaved");
            }
            let is_child_new = child_name.is_empty() || child_name.starts_with("new-")
                || row_map.get("__islocal").and_then(Value::as_i64).unwrap_or(0) == 1;
            if is_child_new {
                // Generate name for child row if missing
                if let Some(obj) = row_val.as_object_mut() {
                    if obj.get("name").and_then(Value::as_str).map(|n| n.is_empty() || n.starts_with("new-")).unwrap_or(true) {
                        use std::time::{SystemTime, UNIX_EPOCH};
                        let ts = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().subsec_nanos();
                        let n = format!("{}-{:08x}", child_doctype.to_lowercase().replace(' ', "-"), ts);
                        obj.insert("name".into(), Value::String(n));
                    }
                }
                let _ = spotledger_db::document::insert_doc(&site.db, child_doctype, &row_val).await;
            } else {
                let _ = spotledger_db::document::upsert_doc(&site.db, child_doctype, &child_name, &row_val).await;
            }
        }
    }

    // Re-fetch the saved parent doc to return the post-save state
    let saved_doc = get_doc(&site.db, &doctype, &saved_name).await
        .map_err(|e| SpotError::Db(e.to_string()))?;

    // Build the response doc dict; for new docs inject `localname` so that
    // sync.js rename_after_save can update locals[] from the browser's temp name.
    let mut doc_dict = saved_doc.as_dict();
    if let (Some(ln), Value::Object(ref mut map)) = (local_name, &mut doc_dict) {
        map.insert("localname".to_string(), Value::String(ln));
    }

    let user = params.get("__current_user").and_then(Value::as_str).unwrap_or("Administrator");
    let perms = get_doc_permissions(&site.db, user, &doctype).await
        .map(|p| p.to_json())
        .unwrap_or_else(|_| json!({"read": 1, "write": 1, "create": 1}));

    // Return in Frappe's send_updated_docs format (top-level docs + docinfo)
    Ok(json!({
        "docs": [doc_dict],
        "docinfo": {
            "doctype": &doctype,
            "name": &saved_name,
            "attachments": [],
            "comments": [],
            "communications": [],
            "automated_messages": [],
            "assignments": [],
            "shared": [],
            "views": [],
            "additional_timeline_content": [],
            "milestones": [],
            "versions": [],
            "is_document_followed": false,
            "tags": "",
            "document_email": null,
            "share_logs": [],
            "assignment_logs": [],
            "attachment_logs": [],
            "info_logs": [],
            "like_logs": [],
            "workflow_logs": [],
            "permissions": perms,
            "user_info": {},
        }
    }))
}

// ── frappe.desk.form.save.cancel ─────────────────────────────────────────────

async fn handle_form_cancel(
    site: Arc<SiteState>,
    params: HashMap<String, Value>,
) -> Result<Value, SpotError> {
    let doctype = require_str(&params, "doctype")?.to_string();
    let name = require_str(&params, "name")?.to_string();
    spotledger_db::document::cancel_doc(&site.db, &doctype, &name).await
        .map_err(|e| SpotError::Db(e.to_string()))?;
    site.doc_cache.remove(&(doctype.clone(), name.clone())).await;
    Ok(json!({"docname": name}))
}

// ── frappe.desk.desk.get_desk_sidebar_items ───────────────────────────────────

async fn handle_get_sidebar_items(
    site: Arc<SiteState>,
    _params: HashMap<String, Value>,
) -> Result<Value, SpotError> {
    let pages = query_workspace_pages(&site.db).await;
    Ok(json!({
        "workspaces": {
            "pages": pages,
            "has_access": true,
            "has_create_access": true,
            "workspace_setup_completed": 1,
        },
    }))
}

// ── frappe.desk.desktop.get_workspace_sidebar_items ──────────────────────────
// Frappe returns the same workspace list as boot but in a different shape.

async fn handle_get_workspace_sidebar_items(
    site: Arc<SiteState>,
    params: HashMap<String, Value>,
) -> Result<Value, SpotError> {
    // Determine if user has access (Administrator / System Manager can edit workspaces)
    let user = params.get("__current_user")
        .and_then(Value::as_str)
        .unwrap_or("Guest");
    let has_access = user == "Administrator" || user != "Guest";

    let pages = query_workspace_pages(&site.db).await;
    // Frappe shape: {"workspace_setup_completed": 1, "pages": [...], "has_access": bool, "has_create_access": bool}
    Ok(json!({
        "workspace_setup_completed": 1,
        "pages": pages,
        "has_access": has_access,
        "has_create_access": has_access,
    }))
}

/// Shared helper — queries tabWorkspace and returns pages array.
pub(crate) async fn query_workspace_pages(db: &DbAdapter) -> Value {
    let fields = &[
        "name", "title", "label", "public", "icon", "module", "app",
        "type", "parent_page", "for_user", "sequence_id", "is_hidden",
        "restrict_to_domain", "content",
    ];
    let filter = json!({"public": 1, "is_hidden": 0, "for_user": ""});
    match get_list(db, "Workspace", Some(fields), Some(&filter), 100, 0).await {
        Ok(rows) => {
            let mut pages: Vec<Value> = rows
                .into_iter()
                .filter(|r| {
                    r.get("title").and_then(|v| v.as_str()).unwrap_or("") != "Welcome Workspace"
                })
                .map(|r| Value::Object(r.into_iter().collect()))
                .collect();
            pages.sort_by(|a, b| {
                let sa = a.get("sequence_id").and_then(Value::as_f64).unwrap_or(999.0);
                let sb = b.get("sequence_id").and_then(Value::as_f64).unwrap_or(999.0);
                sa.partial_cmp(&sb).unwrap_or(std::cmp::Ordering::Equal)
            });
            Value::Array(pages)
        }
        Err(_) => Value::Array(vec![]),
    }
}

// ── frappe.utils.boot.get_boot_info ──────────────────────────────────────────

/// Build the `user` boot object expected at `frappe.boot.user` by the Desk JS.
/// Shape mirrors `frappe.utils.user.UserPermissions.load_user()`.
async fn build_boot_user(db: &DbAdapter, user: &str) -> Value {
    // Get all doctypes for can_* permission lists
    let all_doctypes: Vec<String> = match get_list(db, "DocType", Some(&["name"]), None, 5000, 0).await {
        Ok(rows) => rows.into_iter()
            .filter_map(|r| r.get("name").and_then(|v| v.as_str()).map(String::from))
            .collect(),
        Err(_) => vec![],
    };

    // For Administrator — full access to everything
    let (roles, can_read, can_write, can_create, can_delete, can_submit, can_cancel) =
        if user == "Administrator" {
            (
                vec!["Administrator".to_string(), "System Manager".to_string(), "All".to_string()],
                all_doctypes.clone(),
                all_doctypes.clone(),
                all_doctypes.clone(),
                all_doctypes.clone(),
                vec![] as Vec<String>,
                vec![] as Vec<String>,
            )
        } else {
            // Fetch roles from tabHasRole
            let role_filter = json!({"parent": user, "parenttype": "User"});
            let roles_rows = get_list(db, "Has Role", Some(&["role"]), Some(&role_filter), 100, 0)
                .await.unwrap_or_default();
            let roles: Vec<String> = roles_rows.into_iter()
                .filter_map(|r| r.get("role").and_then(|v| v.as_str()).map(String::from))
                .collect();
            // Simple read list based on DocPerm for user's roles
            let can_read = all_doctypes.clone();
            (roles, can_read, vec![], vec![], vec![], vec![], vec![])
        };

    json!({
        "name":              user,
        "email":             "",
        "full_name":         user,
        "roles":             roles,
        "defaults":          {},
        "can_read":          can_read,
        "can_write":         can_write,
        "can_create":        can_create,
        "can_delete":        can_delete,
        "can_submit":        can_submit,
        "can_cancel":        can_cancel,
        "can_select":        [],
        "can_search":        [],
        "can_export":        [],
        "can_import":        [],
        "can_print":         [],
        "can_email":         [],
        "can_get_report":    [],
        "in_create":         [],
        "all_read":          [],
        "allow_modules":     [],
        "all_reports":       {},
        "onboarding_status": null,
        "default_workspace": null,
    })
}

async fn handle_get_boot_info(
    site: Arc<SiteState>,
    params: HashMap<String, Value>,
) -> Result<Value, SpotError> {
    // Prefer the session-resolved user (injected by call_method as __current_user),
    // fall back to explicit "user" param, then Guest.
    let user = params.get("__current_user")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty() && *s != "Guest")
        .or_else(|| params.get("user").and_then(Value::as_str))
        .unwrap_or("Guest")
        .to_owned();

    let (full_name, user_type) = if user != "Guest" {
        match get_doc(&site.db, "User", &user).await {
            Ok(doc) => (
                doc.get_str("full_name").unwrap_or(&user).to_owned(),
                doc.get_str("user_type").unwrap_or("System User").to_owned(),
            ),
            Err(_) => (user.clone(), "System User".to_owned()),
        }
    } else {
        ("Guest".to_owned(), "Website User".to_owned())
    };

    let is_system_user = user_type != "Website User";
    let workspace_pages = query_workspace_pages(&site.db).await;
    let boot_user = build_boot_user(&site.db, &user).await;

    let sidebar_item: serde_json::Map<String, Value> = if let Value::Array(ref pages) = workspace_pages {
        pages.iter().filter_map(|p| {
            let title = p.get("title").and_then(Value::as_str)?;
            let key = title.to_lowercase();
            Some((key, json!({
                "label": title,
                "items": [],
                "app": p.get("app").cloned().unwrap_or(Value::Null),
                "module": p.get("module").cloned().unwrap_or(Value::Null),
            })))
        }).collect()
    } else {
        serde_json::Map::new()
    };

    Ok(json!({
        "user":          boot_user,
        "user_info": {
            &user: {
                "name":      &user,
                "full_name": &full_name,
                "image":     "",
            }
        },
        "lang":           "en",
        "__messages":     {},
        "home_page":      "Workspaces",
        "user_type":      &user_type,
        "is_system_user": is_system_user,
        "disable_async":  1,
        "user_permissions": {},
        "desktop_icons":  [],
        "app_list":       [],
        "navbar_settings": {
            "app_logo_url": "/assets/frappe/images/frappe-favicon.svg",
        },
        "notification_dot_count": 0,
        "sysdefaults":    {},
        "server_date":    chrono_now(),
        "time_zone":      {"user": "UTC", "system": "UTC"},
        "modules_by_app": {},
        "hide_modules":   [],
        "docs":           [],
        "workspaces": {
            "pages": workspace_pages,
            "has_access": true,
            "has_create_access": true,
            "workspace_setup_completed": 1,
        },
        "workspace_sidebar_item": serde_json::Value::Object(sidebar_item),
    }))
}

// ── frappe.desk.desktop.get_desktop_page ─────────────────────────────────────

async fn handle_get_desktop_page(
    site: Arc<SiteState>,
    params: HashMap<String, Value>,
) -> Result<Value, SpotError> {
    // `page` param is a JSON string like `{"name":"Build","title":"Build","public":1}`
    let workspace_name: String = match params.get("page") {
        Some(Value::String(s)) => {
            // Try to parse as JSON object to get the name
            if let Ok(Value::Object(obj)) = serde_json::from_str::<Value>(s) {
                obj.get("name").and_then(Value::as_str).unwrap_or(s).to_owned()
            } else {
                s.clone()
            }
        }
        Some(Value::Object(obj)) => {
            obj.get("name").and_then(Value::as_str).unwrap_or("").to_owned()
        }
        _ => return Ok(json!({})),
    };

    if workspace_name.is_empty() {
        return Ok(json!({}));
    }

    // Fetch workspace record to get content
    let filter = json!({"name": workspace_name});
    let fields = &["name", "title", "content", "icon", "module", "app", "public"];
    let rows = get_list(&site.db, "Workspace", Some(fields), Some(&filter), 1, 0)
        .await.unwrap_or_default();

    if rows.is_empty() {
        return Ok(json!({}));
    }

    let content = rows[0].get("content").cloned().unwrap_or(Value::Null);
    let content_val: Value = match &content {
        Value::String(s) => serde_json::from_str(s).unwrap_or(Value::Array(vec![])),
        other => other.clone(),
    };

    // Fetch Workspace Shortcut child records
    let shortcut_filter = json!({"parent": workspace_name});
    let shortcut_fields = &["name", "type", "link_to", "label", "icon", "color",
                             "url", "doc_view", "report_ref_doctype", "restrict_to_domain",
                             "stats_filter", "format", "idx", "parent", "parenttype", "parentfield"];
    let shortcut_rows = get_list(&site.db, "Workspace Shortcut", Some(shortcut_fields),
        Some(&shortcut_filter), 100, 0).await.unwrap_or_default();

    // Fetch Workspace Link child records (cards)
    let link_filter = json!({"parent": workspace_name});
    let link_fields = &["name", "type", "label", "icon", "description", "hidden",
                         "link_type", "link_to", "report_ref_doctype", "dependencies",
                         "only_for", "onboard", "is_query_report", "idx",
                         "parent", "parenttype", "parentfield"];
    let link_rows = get_list(&site.db, "Workspace Link", Some(link_fields),
        Some(&link_filter), 500, 0).await.unwrap_or_default();

    // Decorate shortcuts with doctype field and convert to Value
    let shortcuts: Vec<Value> = shortcut_rows.into_iter().map(|mut r| {
        r.entry("doctype".to_string()).or_insert_with(|| json!("Workspace Shortcut"));
        Value::Object(r.into_iter().collect())
    }).collect();

    // Group links into cards: a Card Break item starts a new card group,
    // subsequent Link items belong to that card until the next Card Break.
    let mut cards: Vec<Value> = vec![];
    let mut current_card: Option<serde_json::Map<String, Value>> = None;
    let mut current_links: Vec<Value> = vec![];

    for mut row in link_rows {
        let row_type = row.get("type").and_then(Value::as_str).unwrap_or("").to_string();
        if row_type == "Card Break" {
            // Save previous card if any
            if let Some(mut card) = current_card.take() {
                card.insert("links".to_string(), Value::Array(current_links));
                cards.push(Value::Object(card));
                current_links = vec![];
            }
            row.entry("doctype".to_string()).or_insert_with(|| json!("Workspace Link"));
            current_card = Some(row.into_iter().collect());
        } else {
            row.entry("doctype".to_string()).or_insert_with(|| json!("Workspace Link"));
            current_links.push(Value::Object(row.into_iter().collect()));
        }
    }
    // Flush last card
    if let Some(mut card) = current_card.take() {
        card.insert("links".to_string(), Value::Array(current_links));
        cards.push(Value::Object(card));
    }

    // Return in the shape workspace.js expects at data.message
    Ok(json!({
        "charts":       {"items": []},
        "shortcuts":    {"items": shortcuts},
        "cards":        {"items": cards},
        "onboardings":  [],
        "quick_lists":  {"items": []},
        "number_cards": {"items": []},
        "custom_blocks":{"items": []},
        "content":      content_val,
    }))
}

// ── frappe.desk.desktop.get_installed_apps ───────────────────────────────────

async fn handle_get_installed_apps(
    _site: Arc<SiteState>,
    _params: HashMap<String, Value>,
) -> Result<Value, SpotError> {
    Ok(json!([{"app_name": "frappe", "app_title": "Frappe"}]))
}

// ── frappe.desk.desk_page.getpage ────────────────────────────────────────────

async fn handle_get_desk_page(
    site: Arc<SiteState>,
    params: HashMap<String, Value>,
) -> Result<Value, SpotError> {
    let name = params.get("name").and_then(Value::as_str).unwrap_or("").to_string();
    if name.is_empty() {
        return Ok(json!({"docs": []}));
    }
    match get_doc(&site.db, "Page", &name).await {
        Ok(doc) => Ok(json!({"docs": [doc.as_dict()]})),
        // Return a minimal dynamic-page stub — browser won't crash and won't cache it
        Err(_) => Ok(json!({
            "docs": [{
                "name":          &name,
                "doctype":       "Page",
                "title":         &name,
                "script":        null,
                "style":         null,
                "_dynamic_page": 1,
            }]
        })),
    }
}

/// Public Axum handler for `frappe.desk.desk_page.getpage`.
/// Returns `{docs: [...]}` directly — no `{message: ...}` wrapper — matching
/// the Frappe Python `frappe.response.docs.append(doc)` convention.
pub async fn getpage_handler(
    Extension(site): Extension<Arc<SiteState>>,
    AxumQuery(query): AxumQuery<HashMap<String, String>>,
    body: axum::body::Bytes,
) -> impl IntoResponse {
    let mut raw: HashMap<String, String> = serde_urlencoded::from_bytes(&body).unwrap_or_default();
    raw.extend(query);
    let params = super::parse_form_params(raw);
    match handle_get_desk_page(site, params).await {
        Ok(val) => (StatusCode::OK, AxumJson(val)).into_response(),
        Err(e) => {
            let status = StatusCode::from_u16(e.http_status())
                .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
            (status, AxumJson(ErrorResponse::new(axum_error_type(&e), e.to_string()))).into_response()
        }
    }
}

// ── frappe.desk.doctype.workspace.workspace.get_workspaces ───────────────────

async fn handle_get_workspaces(
    site: Arc<SiteState>,
    _params: HashMap<String, Value>,
) -> Result<Value, SpotError> {
    let fields = &["name", "title", "label", "public", "icon", "module", "app",
                   "type", "parent_page", "for_user", "sequence_id", "is_hidden"];
    let filter = json!({"public": 1, "is_hidden": 0});
    let rows = get_list(&site.db, "Workspace", Some(fields), Some(&filter), 100, 0)
        .await.unwrap_or_default();
    let workspaces: Vec<Value> = rows.into_iter()
        .map(|r| Value::Object(r.into_iter().collect()))
        .collect();
    Ok(json!({"public_workspaces": workspaces, "private_workspaces": []}))
}

// ── frappe.model.db_query.get_list ────────────────────────────────────────────

async fn handle_model_get_list(
    site: Arc<SiteState>,
    params: HashMap<String, Value>,
) -> Result<Value, SpotError> {
    let doctype = require_str(&params, "doctype")?;

    let field_strings: Vec<String> = match params.get("fields") {
        Some(Value::Array(arr)) => arr
            .iter()
            .filter_map(|v| v.as_str().map(str::to_string))
            .collect(),
        _ => vec![],
    };
    let field_refs: Vec<&str> = field_strings.iter().map(String::as_str).collect();
    let fields_opt = if field_refs.is_empty() { None } else { Some(field_refs.as_slice()) };

    let filters = params.get("filters");
    let limit = params.get("limit").and_then(Value::as_u64).map(|n| n as usize).unwrap_or(20);
    let start = params.get("start").or_else(|| params.get("limit_start"))
        .and_then(Value::as_u64).map(|n| n as usize).unwrap_or(0);

    let rows = get_list(&site.db, doctype, fields_opt, filters, limit, start)
        .await
        .map_err(SpotError::from)?;

    Ok(serde_json::to_value(rows).unwrap_or(Value::Array(vec![])))
}

// ── frappe.client.get_single_value ───────────────────────────────────────────

async fn handle_get_single_value(
    site: Arc<SiteState>,
    params: HashMap<String, Value>,
) -> Result<Value, SpotError> {
    let doctype = require_str(&params, "doctype")?.to_string();
    let field = require_str(&params, "field")?.to_string();
    match get_doc(&site.db, &doctype, &doctype).await {
        Ok(doc) => Ok(doc.as_dict().get(&field).cloned().unwrap_or(Value::Null)),
        Err(_) => Ok(Value::Null),
    }
}

// ── frappe.client.get_doc_permissions ────────────────────────────────────────

async fn handle_get_doc_permissions_stub(
    site: Arc<SiteState>,
    params: HashMap<String, Value>,
) -> Result<Value, SpotError> {
    let doctype = params.get("doctype").and_then(Value::as_str).unwrap_or("").to_string();
    if doctype.is_empty() {
        return Ok(json!({"read": 1, "write": 1, "create": 1, "delete": 1, "submit": 0, "cancel": 0}));
    }
    let perms = get_doc_permissions(&site.db, "Administrator", &doctype).await
        .map(|p| p.to_json())
        .unwrap_or_else(|_| json!({"read": 1, "write": 1, "create": 1, "delete": 1}));
    Ok(perms)
}

// ── frappe.core.doctype.user_permission.user_permission.get_user_permissions ──

async fn handle_get_user_permissions(
    _site: Arc<SiteState>,
    _params: HashMap<String, Value>,
) -> Result<Value, SpotError> {
    Ok(json!({}))
}

// ── frappe.core.doctype.session_default_settings.*.get_session_default_values ─

async fn handle_get_session_defaults(
    _site: Arc<SiteState>,
    _params: HashMap<String, Value>,
) -> Result<Value, SpotError> {
    Ok(json!({"session_defaults": []}))
}

// ── frappe.utils.change_log.get_versions ─────────────────────────────────────

async fn handle_get_versions(
    _site: Arc<SiteState>,
    _params: HashMap<String, Value>,
) -> Result<Value, SpotError> {
    Ok(json!({
        "frappe": {
            "title": "Frappe Framework",
            "version": "16.0.0",
        }
    }))
}

// ── Catch-all no-ops ─────────────────────────────────────────────────────────

async fn handle_noop_ok(
    _site: Arc<SiteState>,
    _params: HashMap<String, Value>,
) -> Result<Value, SpotError> {
    Ok(Value::Null)
}

// ── Helper functions ──────────────────────────────────────────────────────────

pub(crate) fn require_str<'a>(
    params: &'a HashMap<String, Value>,
    key: &str,
) -> Result<&'a str, SpotError> {
    params
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| SpotError::Validation(format!("'{key}' is required")))
}

pub(crate) fn chrono_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let days = secs / 86400;
    let y = 1970 + days / 365;
    format!("{y:04}-01-01")
}

/// For every Link field in the document, resolve the title if the linked DocType
/// has `show_title_field_in_link = 1`. Returns `{"LinkedDocType::value": "title", ...}`.
async fn build_link_titles(db: &DbAdapter, doctype: &str, doc: &Document) -> Value {
    let filter = json!({"parent": doctype, "parenttype": "DocType", "fieldtype": "Link"});
    let link_fields = match get_list(db, "DocField", Some(&["fieldname", "options"]), Some(&filter), 200, 0).await {
        Ok(rows) => rows,
        Err(_) => return json!({}),
    };

    let mut titles = serde_json::Map::new();

    for field_row in &link_fields {
        let fieldname = match field_row.get("fieldname").and_then(Value::as_str) {
            Some(f) if !f.is_empty() => f,
            _ => continue,
        };
        let linked_doctype = match field_row.get("options").and_then(Value::as_str) {
            Some(o) if !o.is_empty() => o.to_owned(),
            _ => continue,
        };
        let field_value = match doc.get_str(fieldname) {
            Some(v) if !v.is_empty() => v.to_owned(),
            _ => continue,
        };

        let dt_doc = match get_doc(db, "DocType", &linked_doctype).await {
            Ok(d) => d,
            Err(_) => continue,
        };

        let show_in_link = dt_doc.get_value("show_title_field_in_link")
            .map(|v| match v {
                Value::Bool(b) => b,
                Value::Number(n) => n.as_i64().unwrap_or(0) != 0,
                _ => false,
            })
            .unwrap_or(false);

        if !show_in_link { continue; }

        let title_field = match dt_doc.get_str("title_field") {
            Some(f) if !f.is_empty() => f.to_owned(),
            _ => continue,
        };

        if let Ok(Some(title_val)) = get_value(db, &linked_doctype, &field_value, &title_field).await {
            let title_str = match &title_val {
                Value::String(s) => s.clone(),
                other => other.to_string(),
            };
            let key = format!("{}::{}", linked_doctype, field_value);
            titles.insert(key, Value::String(title_str));
        }
    }

    Value::Object(titles)
}

/// Fetch User records and return Frappe user_info shape.
pub(crate) async fn build_user_info(db: &DbAdapter, users: &[String]) -> Value {
    if users.is_empty() {
        return json!({});
    }

    let mut info_map = serde_json::Map::new();

    for username in users {
        if username.is_empty() { continue; }
        if let Ok(user_doc) = get_doc(db, "User", username).await {
            let full_name = user_doc.get_str("full_name").unwrap_or(username).to_owned();
            let image = user_doc.get_str("user_image").unwrap_or("").to_owned();
            let email = user_doc.get_str("email").unwrap_or("").to_owned();
            let time_zone = user_doc.get_str("time_zone").unwrap_or("").to_owned();
            info_map.insert(username.clone(), json!({
                "fullname":  full_name,
                "image":     image,
                "name":      username,
                "email":     email,
                "time_zone": time_zone,
            }));
        }
    }

    Value::Object(info_map)
}

// ── Axum handler wrappers ─────────────────────────────────────────────────────
//
// getdoctype and getdoc return responses WITHOUT the {"message": ...} envelope.
// They need dedicated Axum handlers that bypass the standard MethodResponse wrapper.

use axum::{
    extract::Extension,
    http::StatusCode,
    response::{IntoResponse, Json as AxumJson},
};
use spotledger_core::response::ErrorResponse;

fn axum_error_type(e: &SpotError) -> &'static str {
    match e {
        SpotError::NotFound { .. } => "DoesNotExistError",
        SpotError::PermissionDenied(_) => "PermissionError",
        SpotError::Validation(_) => "ValidationError",
        _ => "InternalError",
    }
}

pub async fn getdoctype_handler(
    Extension(site): Extension<Arc<SiteState>>,
    AxumQuery(query): AxumQuery<HashMap<String, String>>,
    body: axum::body::Bytes,
) -> impl IntoResponse {
    let mut raw: HashMap<String, String> = serde_urlencoded::from_bytes(&body).unwrap_or_default();
    raw.extend(query);
    let params = super::parse_form_params(raw);
    match handle_getdoctype(site, params).await {
        Ok(val) => (StatusCode::OK, AxumJson(val)).into_response(),
        Err(e) => {
            let status = StatusCode::from_u16(e.http_status())
                .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
            (status, AxumJson(ErrorResponse::new(axum_error_type(&e), e.to_string()))).into_response()
        }
    }
}

pub async fn getdoc_handler(
    Extension(site): Extension<Arc<SiteState>>,
    AxumQuery(query): AxumQuery<HashMap<String, String>>,
    body: axum::body::Bytes,
) -> impl IntoResponse {
    let mut raw: HashMap<String, String> = serde_urlencoded::from_bytes(&body).unwrap_or_default();
    raw.extend(query);
    let params = super::parse_form_params(raw);
    match handle_getdoc(site, params).await {
        Ok(val) => (StatusCode::OK, AxumJson(val)).into_response(),
        Err(e) => {
            let status = StatusCode::from_u16(e.http_status())
                .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
            (status, AxumJson(ErrorResponse::new(axum_error_type(&e), e.to_string()))).into_response()
        }
    }
}
