//! Host-based site resolution middleware + current-user extraction.
//!
//! On every request, extract the `Host` header, strip the port,
//! look up the site in `AppState`, and inject `Arc<SiteState>` into
//! request extensions. Downstream handlers extract it with
//! `Extension<Arc<SiteState>>`.
//!
//! Also extracts the `sid` cookie, resolves the session, and injects
//! a `CurrentUser` extension (defaults to `"Guest"` if no valid session).

use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::sync::Arc;

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

/// Resolve the current user from the `sid` cookie.
/// Returns `"Guest"` if the session is missing, expired, or invalid.
async fn resolve_current_user(site: &Arc<SiteState>, headers: &axum::http::HeaderMap) -> String {
    let sid = match extract_sid(headers) {
        Some(s) => s,
        None => return "Guest".to_owned(),
    };
    match spotledger_db::auth::get_session(&site.db, &sid).await {
        Ok(Some(session)) => session.user,
        _ => "Guest".to_owned(),
    }
}

fn extract_sid(headers: &axum::http::HeaderMap) -> Option<String> {
    headers
        .get(axum::http::header::COOKIE)
        .and_then(|v| v.to_str().ok())
        .and_then(|cookie_str| {
            cookie_str
                .split(';')
                .map(str::trim)
                .find(|s| s.starts_with("sid="))
                .map(|s| s[4..].to_owned())
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
