//! Desk API method handlers.
//!
//! These are the `/api/method/frappe.desk.*` methods called by the Frappe Desk JS
//! on every page load and form open.  They must return Frappe-exact response shapes.

use super::{BoxFuture, MethodRegistry};
use crate::state::SiteState;
use serde_json::{json, Value};
use spotledger_db::document::{build_where, doctype_to_table, get_doc, get_list};
use spotledger_db::permissions::get_doc_permissions;
use spotledger_types::error::SpotError;
use std::collections::HashMap;
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
    }

    reg!("frappe.desk.form.load.getdoctype",         handle_getdoctype);
    reg!("frappe.desk.form.load.getdoc",             handle_getdoc);
    reg!("frappe.desk.reportview.get",               handle_reportview_get);
    reg!("frappe.desk.reportview.get_count",         handle_reportview_get_count);
    reg!("frappe.desk.notifications.get_notifications", handle_get_notifications);
    reg!("frappe.desk.desk.get_desk_sidebar_items",  handle_get_sidebar_items);
    reg!("frappe.utils.boot.get_boot_info",          handle_get_boot_info);
    reg!("frappe.model.db_query.get_list",           handle_model_get_list);
}

// ── frappe.desk.form.load.getdoctype ─────────────────────────────────────────
//
// Returns the DocType definition including all DocFields.
// Response shape (Frappe): {"docs": [doctype_obj], "lang_modified": null}

async fn handle_getdoctype(
    site: Arc<SiteState>,
    params: HashMap<String, Value>,
) -> Result<Value, SpotError> {
    let doctype = require_str(&params, "doctype")?;

    // Fetch the DocType record itself
    let dt_doc = get_doc(&site.db, "DocType", doctype)
        .await
        .map_err(SpotError::from)?;

    let mut dt_obj = dt_doc.as_dict();

    // Fetch all DocFields for this DocType
    let fields_filter = json!({"parent": doctype, "parenttype": "DocType"});
    let fields_rows = get_list(
        &site.db,
        "DocField",
        None,
        Some(&fields_filter),
        500,
        0,
    )
    .await
    .map_err(SpotError::from)?;

    // Attach the fields array inside the doctype object (Frappe embeds them as "fields")
    let fields_val: Vec<Value> = fields_rows
        .into_iter()
        .map(|r| serde_json::to_value(r).unwrap_or(Value::Null))
        .collect();

    if let Value::Object(ref mut map) = dt_obj {
        map.insert("fields".into(), Value::Array(fields_val));
    }

    Ok(json!({
        "docs":          [dt_obj],
        "lang_modified": null,
    }))
}

// ── frappe.desk.form.load.getdoc ─────────────────────────────────────────────
//
// Returns the document plus a minimal `docinfo` structure.
// Response shape (Frappe): {"docs": [doc], "docinfo": {...}}

async fn handle_getdoc(
    site: Arc<SiteState>,
    params: HashMap<String, Value>,
) -> Result<Value, SpotError> {
    let doctype = require_str(&params, "doctype")?;
    let name = require_str(&params, "name")?;

    // TODO: extract actual user from session; for now default to Administrator
    let user = "Administrator";

    let (doc, perms) = tokio::try_join!(
        async { get_doc(&site.db, doctype, name).await.map_err(SpotError::from) },
        async {
            get_doc_permissions(&site.db, user, doctype)
                .await
                .map_err(SpotError::from)
        }
    )?;

    let docinfo = json!({
        "attachments":     [],
        "comments":        [],
        "communications":  [],
        "assignments":     [],
        "shared_with":     [],
        "permissions":     perms.to_json(),
        "views":           [],
        "energy_point_logs": [],
        "additional_timeline_content": [],
        "milestones":      [],
        "is_document_followed": false,
        "tags":            "",
        "document_email":  null,
    });

    Ok(json!({
        "docs":    [doc.as_dict()],
        "docinfo": docinfo,
    }))
}

// ── frappe.desk.reportview.get ────────────────────────────────────────────────
//
// Powers the list view.  Returns column-value pairs in Frappe's wire format:
// {"values": [[v1,v2,...], [...]], "keys": ["name","status",...]}

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

    // Frappe reportview returns {"values": [...], "keys": [...]}
    // keys is the ordered list of field names; values is array of arrays
    let keys: Vec<String> = if field_strings.is_empty() {
        // When no fields specified return all keys from first row
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

    Ok(json!({
        "values": values,
        "keys":   keys,
    }))
}

// ── frappe.desk.reportview.get_count ─────────────────────────────────────────

async fn handle_reportview_get_count(
    site: Arc<SiteState>,
    params: HashMap<String, Value>,
) -> Result<Value, SpotError> {
    let doctype = require_str(&params, "doctype")?;
    let table = doctype_to_table(doctype);
    let filters = params.get("filters");
    let (where_clause, bindings) = build_where(filters);
    let surql = format!("SELECT count() FROM `{table}`{where_clause} GROUP ALL");
    let mut q = site.db.query(&surql);
    for (k, v) in bindings {
        q = q.bind((k, v));
    }
    let mut resp = q.await.map_err(|e| SpotError::Db(e.to_string()))?;
    let rows: Vec<Value> = resp.take(0).unwrap_or_default();
    let n = rows.first().and_then(|obj| obj.get("count")).and_then(Value::as_u64).unwrap_or(0);
    Ok(Value::Number(n.into()))
}

// ── frappe.desk.notifications.get_notifications ───────────────────────────────
//
// Returns notification counts for the current user.
// Phase 1: return zeros so the notification bell renders without errors.

async fn handle_get_notifications(
    _site: Arc<SiteState>,
    _params: HashMap<String, Value>,
) -> Result<Value, SpotError> {
    Ok(json!({
        "open_count_doctype": [],
        "open_count_todo":    0,
        "open_count_mention": 0,
        "targets":            [],
        "mentions":           [],
        "energy_points":      0,
    }))
}

// ── frappe.desk.desk.get_desk_sidebar_items ───────────────────────────────────
//
// Returns sidebar workspace items for the logged-in user.
// Phase 1: return empty list so the sidebar renders without errors.
// Phase 2: query tabWorkspace.

async fn handle_get_sidebar_items(
    _site: Arc<SiteState>,
    _params: HashMap<String, Value>,
) -> Result<Value, SpotError> {
    Ok(json!({
        "workspaces": [],
    }))
}

// ── frappe.utils.boot.get_boot_info ──────────────────────────────────────────
//
// Returns the boot object embedded in the Desk HTML.  The JS SPA reads
// `window.frappe.boot` during startup; without this the Desk won't initialise.

async fn handle_get_boot_info(
    site: Arc<SiteState>,
    params: HashMap<String, Value>,
) -> Result<Value, SpotError> {
    // Identify the logged-in user from the `user` param or default to Guest
    let user = params.get("user")
        .and_then(Value::as_str)
        .unwrap_or("Guest")
        .to_owned();

    // Look up user info (best-effort; fall back to minimal data on error)
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

    Ok(json!({
        "user":          user,
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
        "user_permissions": {},
        "can_read":       [],
        "can_create":     [],
        "can_write":      [],
        "can_cancel":     [],
        "can_delete":     [],
        "allow_print":    [],
        "allow_email":    [],
        "allow_import":   [],
        "allow_export":   [],
        "allow_reports":  [],
        "desktop_icons":  [],
        "app_list":       [],
        "navbar_settings": {
            "app_logo_url": "/assets/frappe/images/frappe-favicon.svg",
        },
        "notification_dot_count": 0,
        "sysdefaults":    {},
        "server_date":    chrono_now(),
        "time_zone":      {"user": "System", "system": "UTC"},
        "modules_by_app": {},
        "hide_modules":   [],
        "docs":           [],
    }))
}

// ── frappe.model.db_query.get_list ────────────────────────────────────────────
// Alias — same as frappe.client.get_list

async fn handle_model_get_list(
    site: Arc<SiteState>,
    params: HashMap<String, Value>,
) -> Result<Value, SpotError> {
    // Delegate entirely to the client handler logic
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

// ── helpers ───────────────────────────────────────────────────────────────────

fn require_str<'a>(params: &'a HashMap<String, Value>, key: &str) -> Result<&'a str, SpotError> {
    params
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| SpotError::Validation(format!("'{key}' is required")))
}

fn chrono_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    // Format: "YYYY-MM-DD" as a simple date string
    let days = secs / 86400;
    let y = 1970 + days / 365;
    format!("{y:04}-01-01") // rough; good enough for boot info
}

// ── Axum direct-response wrappers for methods that Frappe exposes WITHOUT
//    the {"message": ...} envelope (getdoctype, getdoc return at top-level).
// ─────────────────────────────────────────────────────────────────────────────

use axum::{
    extract::{Extension, Form},
    http::StatusCode,
    response::{IntoResponse, Json as AxumJson},
};
use spotledger_types::response::ErrorResponse;

fn axum_error_type(e: &SpotError) -> &'static str {
    match e {
        SpotError::NotFound { .. } => "DoesNotExistError",
        SpotError::PermissionDenied(_) => "PermissionError",
        SpotError::Validation(_) => "ValidationError",
        _ => "InternalError",
    }
}

/// POST /api/method/frappe.desk.form.load.getdoctype
/// Returns the result directly (no {message:} wrapper) — Frappe shape.
pub async fn getdoctype_handler(
    Extension(site): Extension<std::sync::Arc<SiteState>>,
    Form(raw): Form<HashMap<String, String>>,
) -> impl IntoResponse {
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

/// POST /api/method/frappe.desk.form.load.getdoc
/// Returns the result directly (no {message:} wrapper) — Frappe shape.
pub async fn getdoc_handler(
    Extension(site): Extension<std::sync::Arc<SiteState>>,
    Form(raw): Form<HashMap<String, String>>,
) -> impl IntoResponse {
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
