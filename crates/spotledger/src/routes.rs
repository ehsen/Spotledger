//! Axum route handlers — Phase 2 scope:
//!   GET  /api/ping                              → health check
//!   GET  /api/resource/<doctype>                → get_list  (permission-gated)
//!   GET  /api/resource/<doctype>/<name>         → get_doc   (permission-gated)
//!   GET  /api/resource/<doctype>/<name>/<field> → get_value (permission-gated)
//!   POST /api/method/{*path}                    → method registry dispatcher

use axum::{
    extract::{Extension, Form, Path, Query},
    http::StatusCode,
    response::{IntoResponse, Json},
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;

use spotledger_db::document::{get_doc, get_list, get_value};
use spotledger_db::permissions::has_permission;
use spotledger_types::response::{DocResponse, ErrorResponse, ListResponse, MethodResponse};

use crate::methods::parse_form_params;
use crate::middleware::CurrentUser;
use crate::state::SiteState;

// ── health check ─────────────────────────────────────────────────────────────

pub async fn ping() -> impl IntoResponse {
    Json(json!({"message": "pong"}))
}

// ── get_doc ───────────────────────────────────────────────────────────────────

pub async fn resource_get(
    Extension(site): Extension<Arc<SiteState>>,
    Extension(current_user): Extension<CurrentUser>,
    Path((doctype, name)): Path<(String, String)>,
) -> impl IntoResponse {
    // Permission check: require "read"
    match has_permission(&site.db, current_user.name(), &doctype, "read").await {
        Ok(true) => {}
        Ok(false) => {
            let body = ErrorResponse::new(
                "PermissionError",
                format!("No read permission for {doctype}"),
            );
            return (StatusCode::FORBIDDEN, Json(body)).into_response();
        }
        Err(e) => {
            let body = ErrorResponse::new("InternalError", e.to_string());
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(body)).into_response();
        }
    }

    // Check doc cache first
    if let Some(cached) = site.doc_cache.get(&(doctype.clone(), name.clone())).await {
        return (StatusCode::OK, Json(DocResponse { data: cached })).into_response();
    }

    match get_doc(&site.db, &doctype, &name).await {
        Ok(doc) => {
            let _ = site
                .doc_cache
                .insert((doctype.clone(), name.clone()), doc.as_dict())
                .await;
            (
                StatusCode::OK,
                Json(DocResponse { data: doc.as_dict() }),
            )
                .into_response()
        }
        Err(e) => {
            let spot_err: spotledger_types::error::SpotError = e.into();
            let status =
                StatusCode::from_u16(spot_err.http_status()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
            let body = ErrorResponse::new(error_type(&spot_err), spot_err.to_string());
            (status, Json(body)).into_response()
        }
    }
}

// ── get_list ──────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct ListParams {
    /// JSON array string e.g. `["name","customer_name"]`
    pub fields: Option<String>,
    /// JSON object string e.g. `{"status":"Active"}`
    pub filters: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: usize,
    #[serde(default)]
    pub start: usize,
}

fn default_limit() -> usize { 20 }

pub async fn resource_list(
    Extension(site): Extension<Arc<SiteState>>,
    Extension(current_user): Extension<CurrentUser>,
    Path(doctype): Path<String>,
    Query(params): Query<ListParams>,
) -> impl IntoResponse {
    // Permission check: require "read"
    match has_permission(&site.db, current_user.name(), &doctype, "read").await {
        Ok(true) => {}
        Ok(false) => {
            let body = ErrorResponse::new(
                "PermissionError",
                format!("No read permission for {doctype}"),
            );
            return (StatusCode::FORBIDDEN, Json(body)).into_response();
        }
        Err(e) => {
            let body = ErrorResponse::new("InternalError", e.to_string());
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(body)).into_response();
        }
    }

    let fields_val: Option<Value> = params
        .fields
        .as_deref()
        .and_then(|s| serde_json::from_str(s).ok());

    let filters_val: Option<Value> = params
        .filters
        .as_deref()
        .and_then(|s| serde_json::from_str(s).ok());

    let field_strs: Vec<String> = match &fields_val {
        Some(Value::Array(arr)) => arr
            .iter()
            .filter_map(|v| v.as_str().map(str::to_string))
            .collect(),
        _ => vec![],
    };
    let field_refs: Vec<&str> = field_strs.iter().map(String::as_str).collect();

    let fields_opt = if field_refs.is_empty() {
        None
    } else {
        Some(field_refs.as_slice())
    };

    match get_list(
        &site.db,
        &doctype,
        fields_opt,
        filters_val.as_ref(),
        params.limit,
        params.start,
    )
    .await
    {
        Ok(rows) => {
            let total = rows.len();
            let data: Vec<Value> = rows
                .into_iter()
                .map(|r| serde_json::to_value(r).unwrap_or(Value::Null))
                .collect();
            (
                StatusCode::OK,
                Json(ListResponse { data, total_count: total }),
            )
                .into_response()
        }
        Err(e) => {
            let spot_err: spotledger_types::error::SpotError = e.into();
            let status =
                StatusCode::from_u16(spot_err.http_status()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
            let body = ErrorResponse::new(error_type(&spot_err), spot_err.to_string());
            (status, Json(body)).into_response()
        }
    }
}

// ── get_value ─────────────────────────────────────────────────────────────────

pub async fn resource_get_value(
    Extension(site): Extension<Arc<SiteState>>,
    Extension(current_user): Extension<CurrentUser>,
    Path((doctype, name, fieldname)): Path<(String, String, String)>,
) -> impl IntoResponse {
    // Permission check: require "read"
    match has_permission(&site.db, current_user.name(), &doctype, "read").await {
        Ok(true) => {}
        Ok(false) => {
            let body = ErrorResponse::new(
                "PermissionError",
                format!("No read permission for {doctype}"),
            );
            return (StatusCode::FORBIDDEN, Json(body)).into_response();
        }
        Err(e) => {
            let body = ErrorResponse::new("InternalError", e.to_string());
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(body)).into_response();
        }
    }

    match get_value(&site.db, &doctype, &name, &fieldname).await {
        Ok(Some(val)) => (StatusCode::OK, Json(json!({"message": val}))).into_response(),
        Ok(None) => (StatusCode::OK, Json(json!({"message": null}))).into_response(),
        Err(e) => {
            let spot_err: spotledger_types::error::SpotError = e.into();
            let status =
                StatusCode::from_u16(spot_err.http_status()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
            let body = ErrorResponse::new(error_type(&spot_err), spot_err.to_string());
            (status, Json(body)).into_response()
        }
    }
}

// ── /api/method/ dispatcher ───────────────────────────────────────────────────

/// POST /api/method/{*path}
///
/// The `path` segment is the dotted method name, e.g. `frappe.client.get_list`.
/// Parameters are accepted as `application/x-www-form-urlencoded` (standard Frappe JS client).
/// JSON-encoded parameter values (like `fields=["name","customer"]`) are automatically decoded.
pub async fn call_method(
    Extension(site): Extension<Arc<SiteState>>,
    Path(path): Path<String>,
    Form(raw_params): Form<HashMap<String, String>>,
) -> impl IntoResponse {
    // Strip leading slash that Axum may include for wildcard captures
    let method_path = path.trim_start_matches('/');

    let handler = match site.method_registry.get(method_path) {
        Some(h) => h,
        None => {
            let body = ErrorResponse::new(
                "MethodNotFoundError",
                format!("Method not found: {method_path}"),
            );
            return (StatusCode::NOT_FOUND, Json(body)).into_response();
        }
    };

    let params = parse_form_params(raw_params);

    match handler(site.clone(), params).await {
        Ok(result) => {
            let body = MethodResponse { message: result };
            (StatusCode::OK, Json(body)).into_response()
        }
        Err(e) => {
            let status = StatusCode::from_u16(e.http_status())
                .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
            let body = ErrorResponse::new(error_type(&e), e.to_string());
            (status, Json(body)).into_response()
        }
    }
}

// ── helpers ───────────────────────────────────────────────────────────────────

fn error_type(e: &spotledger_types::error::SpotError) -> &'static str {
    match e {
        spotledger_types::error::SpotError::NotFound { .. } => "DoesNotExistError",
        spotledger_types::error::SpotError::PermissionDenied(_) => "PermissionError",
        spotledger_types::error::SpotError::Validation(_) => "ValidationError",
        _ => "InternalError",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, http::Request};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    fn test_router() -> axum::Router {
        use axum::routing::get;
        axum::Router::new().route("/api/ping", get(ping))
    }

    #[tokio::test]
    async fn ping_returns_200_pong() {
        let app = test_router();
        let resp = app
            .oneshot(Request::builder().uri("/api/ping").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        let body: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body["message"], json!("pong"));
    }
}

