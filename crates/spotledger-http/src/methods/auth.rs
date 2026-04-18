//! Authentication Axum route handlers.
//!
//! These are dedicated Axum handlers (not entries in the MethodRegistry) because
//! they require direct access to request cookies and must set response cookies.
//!
//! Routes registered in server.rs (before the catch-all /api/method/{*path}):
//!   POST /api/method/login                      → login_handler
//!   POST /api/method/logout                     → logout_handler
//!   POST /api/method/frappe.auth.get_logged_user → get_logged_user_handler

use axum::{
    extract::{Extension, Form},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Json, Response},
};
use std::collections::HashMap;
use std::sync::Arc;

use spotledger_db::auth::{
    create_session, expire_session, get_password_hash, get_session, lookup_user, verify_password,
};
use spotledger_db::jwt::{derive_signing_secret, issue_token, verify_token};
use spotledger_core::response::{ErrorResponse, MethodResponse};

use crate::state::SiteState;

// ── login ─────────────────────────────────────────────────────────────────────

/// POST /api/method/login
///
/// Form params: `usr` (username or email), `pwd` (password)
///
/// Success response (200):
/// ```json
/// {"message": "Logged In", "home_page": "/desk", "full_name": "..."}
/// ```
/// Sets cookies: `sid` (HttpOnly), `user_id`, `full_name`, `system_user`
///
/// Failure response (401):
/// ```json
/// {"exc_type": "AuthenticationError", "message": "Incorrect User or Password"}
/// ```
pub async fn login_handler(
    Extension(site): Extension<Arc<SiteState>>,
    headers: HeaderMap,
    Form(params): Form<HashMap<String, String>>,
) -> Response {
    let usr = match params.get("usr").filter(|s| !s.is_empty()) {
        Some(u) => u.clone(),
        None => {
            return auth_error("usr is required").into_response();
        }
    };
    let pwd = match params.get("pwd").filter(|s| !s.is_empty()) {
        Some(p) => p.clone(),
        None => {
            return auth_error("pwd is required").into_response();
        }
    };

    // 1. Look up user (by name or email)
    let user_info = match lookup_user(&site.db, &usr).await {
        Ok(u) => u,
        Err(e) => {
            tracing::debug!(usr = %usr, err = %e, "login: user not found");
            return auth_error("Incorrect User or Password").into_response();
        }
    };

    if !user_info.enabled {
        tracing::debug!(usr = %usr, "login: user disabled");
        return auth_error("User disabled or missing").into_response();
    }

    // 2. Get password hash from __Auth
    let hash = match get_password_hash(&site.db, &user_info.name).await {
        Ok(h) => h,
        Err(e) => {
            tracing::debug!(usr = %usr, err = %e, "login: password hash not found");
            return auth_error("Incorrect User or Password").into_response();
        }
    };

    // 3. Verify password (pbkdf2-sha256 or argon2)
    if !verify_password(&hash, &pwd) {
        tracing::debug!(usr = %usr, hash_prefix = %&hash[..20.min(hash.len())], "login: password mismatch");
        return auth_error("Incorrect User or Password").into_response();
    }

    // 4. Derive client IP
    let client_ip = headers
        .get("X-Forwarded-For")
        .or_else(|| headers.get("X-Real-IP"))
        .and_then(|v| v.to_str().ok())
        .map(|s| s.split(',').next().unwrap_or(s).trim().to_owned());

    // 5. Create session
    let sid = match create_session(&site.db, &user_info.name, client_ip.as_deref()).await {
        Ok(s) => s,
        Err(e) => {
            let body = ErrorResponse::new("InternalError", e.to_string());
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(body)).into_response();
        }
    };

    // 6. Build full name
    let full_name = user_info
        .full_name
        .clone()
        .or_else(|| {
            let parts: Vec<_> = [
                user_info.first_name.as_deref(),
                user_info.last_name.as_deref(),
            ]
            .into_iter()
            .flatten()
            .collect();
            if parts.is_empty() {
                None
            } else {
                Some(parts.join(" "))
            }
        })
        .unwrap_or_else(|| user_info.name.clone());

    let is_system_user = user_info
        .user_type
        .as_deref()
        .map(|t| t != "Website User")
        .unwrap_or(true);

    let body = serde_json::json!({
        "message": "Logged In",
        "home_page": "/desk",
        "full_name": full_name,
    });

    // 7. Set cookies
    let jwt_secret = derive_signing_secret(&site.config.database.db, &site.config.database.pass);
    let jwt = issue_token(&user_info.name, &jwt_secret).unwrap_or_default();

    let system_user_val = if is_system_user { "yes" } else { "no" };
    let full_name_encoded = urlencodelight(&full_name);
    let path = "Path=/; SameSite=Lax";
    let cookies: &[String] = &[
        format!("sid={sid}; HttpOnly; {path}"),
        // Stateless JWT token — verified in middleware without a DB lookup.
        format!("token={jwt}; HttpOnly; {path}"),
        format!("user_id={}; {path}", user_info.name),
        format!("full_name={full_name_encoded}; {path}"),
        format!("system_user={system_user_val}; {path}"),
    ];

    let mut resp = axum::http::Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/json");
    for c in cookies {
        resp = resp.header(header::SET_COOKIE, c.as_str());
    }
    resp.body(axum::body::Body::from(serde_json::to_string(&body).unwrap()))
        .unwrap()
}

// ── logout ────────────────────────────────────────────────────────────────────

/// POST /api/method/logout
///
/// Reads `sid` from the `Cookie` header, expires the session record in
/// `tabSessions`, and clears auth cookies via `Set-Cookie: ...; Max-Age=0`.
///
/// Always succeeds (200) even if no valid session is found.
pub async fn logout_handler(
    Extension(site): Extension<Arc<SiteState>>,
    headers: HeaderMap,
    Form(_params): Form<HashMap<String, String>>,
) -> Response {
    if let Some(sid) = extract_sid_cookie(&headers) {
        // Best-effort: ignore errors (session may already be expired)
        let _ = expire_session(&site.db, &sid).await;
    }

    let body = serde_json::json!({"message": "success"});
    let expired = "Thu, 01 Jan 1970 00:00:00 GMT";
    let cookies: &[String] = &[
        format!("sid=; Expires={expired}; HttpOnly; Path=/; SameSite=Lax"),
        format!("token=; Expires={expired}; HttpOnly; Path=/; SameSite=Lax"),
        format!("user_id=; Expires={expired}; Path=/; SameSite=Lax"),
        format!("full_name=; Expires={expired}; Path=/; SameSite=Lax"),
        format!("system_user=; Expires={expired}; Path=/; SameSite=Lax"),
    ];

    let mut resp = axum::http::Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/json");
    for c in cookies {
        resp = resp.header(header::SET_COOKIE, c.as_str());
    }
    resp.body(axum::body::Body::from(serde_json::to_string(&body).unwrap()))
        .unwrap()
}

// ── get_logged_user ───────────────────────────────────────────────────────────

/// POST /api/method/frappe.auth.get_logged_user
///
/// Reads `sid` from the `Cookie` header, looks up the active session, and
/// returns the username.
///
/// Response: `{"message": "Administrator"}` (or `"Guest"` if not logged in)
pub async fn get_logged_user_handler(
    Extension(site): Extension<Arc<SiteState>>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let user = {
        // 1. Try the stateless JWT token cookie first (no DB call).
        let jwt_user = extract_token_cookie(&headers).and_then(|token| {
            let secret = derive_signing_secret(
                &site.config.database.db,
                &site.config.database.pass,
            );
            verify_token(&token, &secret)
        });

        if let Some(u) = jwt_user {
            u
        } else {
            // 2. Fall back to the sid session cookie (DB lookup).
            match extract_sid_cookie(&headers) {
                Some(sid) => match get_session(&site.db, &sid).await {
                    Ok(Some(session)) => session.user,
                    _ => "Guest".to_owned(),
                },
                None => "Guest".to_owned(),
            }
        }
    };

    let body = MethodResponse {
        message: serde_json::json!(user),
    };
    (StatusCode::OK, Json(body))
}

// ── helpers ───────────────────────────────────────────────────────────────────

/// Extract the `sid` value from the `Cookie` request header.
pub fn extract_sid_cookie(headers: &HeaderMap) -> Option<String> {
    headers
        .get(header::COOKIE)
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

/// Extract the `token` (JWT) value from the `Cookie` request header.
pub fn extract_token_cookie(headers: &HeaderMap) -> Option<String> {
    headers
        .get(header::COOKIE)
        .and_then(|v| v.to_str().ok())
        .and_then(|cookie_str| {
            cookie_str
                .split(';')
                .map(str::trim)
                .find(|s| s.starts_with("token="))
                .map(|s| s[6..].to_owned())
        })
        .filter(|s| !s.is_empty())
}

fn auth_error(message: &str) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::UNAUTHORIZED,
        Json(ErrorResponse::new("AuthenticationError", message)),
    )
}

/// Minimal percent-encoding: encode space and non-ASCII chars in cookie values.
/// Full URL encoding is not required here — only characters that break the
/// `Set-Cookie` header value need to be encoded.
fn urlencodelight(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            ' ' => "%20".to_owned(),
            ';' => "%3B".to_owned(),
            ',' => "%2C".to_owned(),
            _ => c.to_string(),
        })
        .collect()
}

// ── tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    fn make_headers_with_cookie(cookie: &str) -> HeaderMap {
        let mut h = HeaderMap::new();
        h.insert(header::COOKIE, HeaderValue::from_str(cookie).unwrap());
        h
    }

    #[test]
    fn extract_sid_cookie_finds_sid() {
        let h = make_headers_with_cookie("sid=abc123; user_id=admin");
        assert_eq!(extract_sid_cookie(&h), Some("abc123".to_owned()));
    }

    #[test]
    fn extract_sid_cookie_no_sid_returns_none() {
        let h = make_headers_with_cookie("user_id=admin; full_name=Admin");
        assert_eq!(extract_sid_cookie(&h), None);
    }

    #[test]
    fn extract_sid_cookie_empty_sid_returns_none() {
        let h = make_headers_with_cookie("sid=; user_id=admin");
        assert_eq!(extract_sid_cookie(&h), None);
    }

    #[test]
    fn extract_sid_cookie_single_cookie() {
        let h = make_headers_with_cookie("sid=deadbeef00112233445566778899aabb");
        assert_eq!(
            extract_sid_cookie(&h),
            Some("deadbeef00112233445566778899aabb".to_owned())
        );
    }

    #[test]
    fn urlencodelight_encodes_spaces() {
        assert_eq!(urlencodelight("John Doe"), "John%20Doe");
        assert_eq!(urlencodelight("Admin"), "Admin");
        assert_eq!(urlencodelight(""), "");
    }
}
