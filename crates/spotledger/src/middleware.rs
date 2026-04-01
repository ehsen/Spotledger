//! Host-based site resolution middleware.
//!
//! On every request, extract the `Host` header, strip the port,
//! look up the site in `AppState`, and inject `Arc<SiteState>` into
//! request extensions. Downstream handlers extract it with
//! `Extension<Arc<SiteState>>`.

use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::sync::Arc;

use crate::state::{AppState, SiteState};

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

    match app.get_site(&hostname) {
        Some(site) => {
            req.extensions_mut().insert(site);
            next.run(req).await
        }
        None => {
            // Fallback: if there's exactly one site loaded, use it (dev convenience)
            if app.sites.len() == 1 {
                let site: Arc<SiteState> =
                    app.sites.iter().next().unwrap().value().clone();
                req.extensions_mut().insert(site);
                next.run(req).await
            } else {
                (
                    StatusCode::NOT_FOUND,
                    format!("Unknown site: {hostname}"),
                )
                    .into_response()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::AppState;

    #[test]
    fn app_state_register_and_get() {
        // Pure unit test — no async needed for the registry itself
        // (SiteState needs a real Db to construct so we only test AppState registry)
        let state = AppState::new();
        assert!(state.get_site("localhost").is_none());
        // With one site it would fall through to the single-site fallback
        assert_eq!(state.sites.len(), 0);
    }
}
