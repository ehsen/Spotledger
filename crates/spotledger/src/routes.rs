//! Axum route handlers — Phase 1 scope:
//!   GET  /api/ping                              → health check
//!   GET  /api/resource/<doctype>                → get_list
//!   GET  /api/resource/<doctype>/<name>         → get_doc
//!   GET  /api/resource/<doctype>/<name>/<field> → get_value

use axum::{
    extract::{Extension, Path, Query},
    http::StatusCode,
    response::{IntoResponse, Json},
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;

use spotledger_db::document::{get_doc, get_list, get_value};
use spotledger_types::response::{DocResponse, ErrorResponse, ListResponse};

use crate::state::SiteState;

// ── health check ─────────────────────────────────────────────────────────────

pub async fn ping() -> impl IntoResponse {
    Json(json!({"message": "pong"}))
}

// ── get_doc ───────────────────────────────────────────────────────────────────

pub async fn resource_get(
    Extension(site): Extension<Arc<SiteState>>,
    Path((doctype, name)): Path<(String, String)>,
) -> impl IntoResponse {
    match get_doc(&site.db, &doctype, &name).await {
        Ok(doc) => {
            // Check doc cache miss/fill
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
    Path(doctype): Path<String>,
    Query(params): Query<ListParams>,
) -> impl IntoResponse {
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
    Path((doctype, name, fieldname)): Path<(String, String, String)>,
) -> impl IntoResponse {
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
    use axum::{body::to_bytes, http::StatusCode};
    use axum_test::TestServer;

    fn test_router() -> axum::Router {
        use axum::routing::get;
        axum::Router::new().route("/api/ping", get(ping))
    }

    #[tokio::test]
    async fn ping_returns_200_pong() {
        let server = TestServer::new(test_router()).unwrap();
        let resp = server.get("/api/ping").await;
        resp.assert_status_ok();
        let body: Value = resp.json();
        assert_eq!(body["message"], json!("pong"));
    }
}
