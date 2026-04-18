//! Host-based site resolution middleware + current-user extraction.
//!
//! On every request, extract the `Host` header, strip the port,
//! look up the site in `AppState`, and inject `Arc<SiteState>` into
//! request extensions. Downstream handlers extract it with
//! `Extension<Arc<SiteState>>`.
//!
//! Also resolves the current user from cookies:
//!   1. `token` cookie — verified locally as a JWT (zero DB round-trip).
//!   2. `sid` cookie — legacy session look-up in `tabSessions`.
//! Injects a `CurrentUser` extension (defaults to `"Guest"` when neither cookie is valid).

use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::sync::Arc;

use spotledger_db::jwt::{derive_signing_secret, verify_token};

use crate::state::{AppState, SiteState};

/// The currently authenticated user for this request.
/// Available via `Extension<CurrentUser>` in all route handlers.
#[derive(Clone, Debug)]
pub struct CurrentUser(pub String);

impl CurrentUser {
    pub fn name(&self) -> &str {
        &self.0
    }
}

pub async fn site_middleware(
    State(app): State<AppState>,
    mut req: Request<Body>,
    next: Next,
) -> Response {
    let hostname = req
        .headers()
        .get("host")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("")
        .split(':')
        .next()
        .unwrap_or("")
        .to_string();

    let site_opt = app.get_site(&hostname).or_else(|| {
        // Fallback: if there's exactly one site loaded, use it (dev convenience)
        if app.sites.len() == 1 {
            app.sites.iter().next().map(|r| r.value().clone())
        } else {
            None
        }
    });

    match site_opt {
        Some(site) => {
            // Resolve session → CurrentUser
            let user = resolve_current_user(&site, req.headers()).await;
            req.extensions_mut().insert(CurrentUser(user));
            req.extensions_mut().insert(site);
            next.run(req).await
        }
        None => (
            StatusCode::NOT_FOUND,
            format!("Unknown site: {hostname}"),
        )
            .into_response(),
    }
}

/// Resolve the current user from cookies.
///
/// Priority:
///   1. `token` cookie — validated as a JWT locally (no DB call).
///   2. `sid` cookie — looked up in `tabSessions` (one DB call).
///   3. Fall through → `"Guest"`.
async fn resolve_current_user(site: &Arc<SiteState>, headers: &axum::http::HeaderMap) -> String {
    // 1. JWT token: verify signature + expiry locally.
    if let Some(token) = extract_cookie(headers, "token=", 6) {
        let secret = derive_signing_secret(
            &site.config.database.db,
            &site.config.database.pass,
        );
        if let Some(user) = verify_token(&token, &secret) {
            return user;
        }
    }

    // 2. Legacy sid session.
    if let Some(sid) = extract_cookie(headers, "sid=", 4) {
        if let Ok(Some(session)) = spotledger_db::auth::get_session(&site.db, &sid).await {
            return session.user;
        }
    }

    "Guest".to_owned()
}

fn extract_sid(headers: &axum::http::HeaderMap) -> Option<String> {
    extract_cookie(headers, "sid=", 4)
}

fn extract_cookie(
    headers: &axum::http::HeaderMap,
    prefix: &str,
    prefix_len: usize,
) -> Option<String> {
    headers
        .get(axum::http::header::COOKIE)
        .and_then(|v| v.to_str().ok())
        .and_then(|cookie_str| {
            cookie_str
                .split(';')
                .map(str::trim)
                .find(|s| s.starts_with(prefix))
                .map(|s| s[prefix_len..].to_owned())
        })
        .filter(|s| !s.is_empty())
}

#[cfg(test)]
mod tests {
    use crate::state::AppState;

    #[test]
    fn app_state_register_and_get() {
        let state = AppState::new();
        assert!(state.get_site("localhost").is_none());
        assert_eq!(state.sites.len(), 0);
    }
}
