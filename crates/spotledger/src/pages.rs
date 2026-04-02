//! Page routes: `/`, `/desk`, `/login`, `/app/**`
//!
//! Frappe Desk is a JavaScript SPA.  The server serves a minimal HTML shell
//! that loads the compiled bundles from `/assets/` and injects `frappe.boot`
//! so the JS can initialise without a round-trip.
//!
//! The assets are read from the bench's `sites/assets/assets.json` file to
//! resolve hashed bundle filenames.

use axum::{
    extract::Extension,
    http::{header, StatusCode},
    response::{IntoResponse, Redirect, Response},
};
use std::sync::Arc;

use spotledger_db::auth::get_session;
use spotledger_db::document::get_doc;
use crate::state::SiteState;

// ── / ─────────────────────────────────────────────────────────────────────────

/// GET `/` — redirect to `/desk` if a valid session cookie is present,
/// otherwise redirect to `/login`.
pub async fn root_handler(
    Extension(site): Extension<Arc<SiteState>>,
    headers: axum::http::HeaderMap,
) -> Response {
    let logged_in = is_logged_in(&site, &headers).await;
    if logged_in {
        Redirect::to("/desk").into_response()
    } else {
        Redirect::to("/login").into_response()
    }
}

// ── /login ────────────────────────────────────────────────────────────────────

/// GET `/login` — serve the login HTML page.
pub async fn login_page() -> impl IntoResponse {
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
        LOGIN_HTML,
    )
}

// ── /desk ─────────────────────────────────────────────────────────────────────

/// GET `/desk` — serve the Desk SPA shell with minimal `frappe.boot` embedded.
pub async fn desk_page(
    Extension(site): Extension<Arc<SiteState>>,
    headers: axum::http::HeaderMap,
) -> Response {
    // Extract user for boot data
    let (user, full_name) = boot_user(&site, &headers).await;

    let desk_js  = "/assets/frappe/dist/js/desk.bundle.CJXF32XD.js";
    let desk_css = "/assets/frappe/dist/css/desk.bundle.U6TWVR73.css";

    let boot_json = serde_json::json!({
        "user":          &user,
        "user_info": {
            &user: {
                "name":      &user,
                "full_name": &full_name,
                "image":     "",
            }
        },
        "lang":              "en",
        "__messages":        {},
        "home_page":         "Workspaces",
        "is_system_user":    user != "Guest",
        "user_permissions":  {},
        "can_read": [], "can_create": [], "can_write": [],
        "can_cancel": [], "can_delete": [],
        "desktop_icons": [], "app_list": [],
        "navbar_settings": {
            "app_logo_url": "/assets/frappe/images/frappe-favicon.svg",
        },
        "notification_dot_count": 0,
        "sysdefaults": {},
        "docs": [],
    });

    let boot_str = serde_json::to_string(&boot_json).unwrap_or_else(|_| "{}".to_string());

    let html = format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>Spotledger</title>
  <link rel="shortcut icon" href="/assets/frappe/images/frappe-favicon.svg">
  <link rel="stylesheet" href="{desk_css}">
</head>
<body>
  <div class="main-section">
    <header></header>
    <div id="body"></div>
    <footer></footer>
  </div>
  <div id="all-symbols" style="display:none"></div>
  <script>
    window._version_number = "1";
    window.app = true;
    window.dev_server = false;
    if(!window.frappe) window.frappe = {{}};
    frappe.boot = {boot_str};
    frappe._messages = {{}};
    frappe.csrf_token = "spotledger-no-csrf";
  </script>
  <script src="{desk_js}"></script>
</body>
</html>"#
    );

    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
        html,
    )
        .into_response()
}

/// GET `/app/**` — SPA deep-link; serve the same Desk shell.
pub async fn app_wildcard(
    Extension(site): Extension<Arc<SiteState>>,
    headers: axum::http::HeaderMap,
) -> Response {
    desk_page(Extension(site), headers).await
}

// ── helpers ───────────────────────────────────────────────────────────────────

async fn is_logged_in(site: &SiteState, headers: &axum::http::HeaderMap) -> bool {
    if let Some(sid) = extract_sid(headers) {
        return get_session(&site.db, &sid).await.ok().flatten().is_some();
    }
    false
}

async fn boot_user(site: &SiteState, headers: &axum::http::HeaderMap) -> (String, String) {
    let sid = match extract_sid(headers) {
        Some(s) => s,
        None => return ("Guest".into(), "Guest".into()),
    };
    let session = match get_session(&site.db, &sid).await.ok().flatten() {
        Some(s) => s,
        None => return ("Guest".into(), "Guest".into()),
    };
    let user = session.user.clone();
    let full_name = match get_doc(&site.db, "User", &user).await {
        Ok(doc) => doc.get_str("full_name").unwrap_or(&user).to_owned(),
        Err(_) => user.clone(),
    };
    (user, full_name)
}

fn extract_sid(headers: &axum::http::HeaderMap) -> Option<String> {
    headers
        .get(header::COOKIE)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| {
            s.split(';')
                .map(str::trim)
                .find(|p| p.starts_with("sid="))
                .map(|p| p[4..].to_owned())
        })
        .filter(|s| !s.is_empty())
}

// ── Login HTML (self-contained, no template engine needed) ────────────────────

const LOGIN_HTML: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>Login — Spotledger</title>
  <link rel="shortcut icon" href="/assets/frappe/images/frappe-favicon.svg">
  <style>
    *{box-sizing:border-box;margin:0;padding:0}
    body{font-family:-apple-system,BlinkMacSystemFont,"Segoe UI",Roboto,sans-serif;
         background:#f0f4f8;display:flex;align-items:center;justify-content:center;
         min-height:100vh}
    .card{background:#fff;border-radius:8px;box-shadow:0 2px 16px rgba(0,0,0,.12);
          padding:2rem;width:100%;max-width:400px}
    h2{margin-bottom:1.5rem;font-size:1.5rem;color:#171717;text-align:center}
    label{display:block;margin-bottom:.25rem;font-size:.875rem;color:#374151}
    input{width:100%;padding:.625rem .75rem;border:1px solid #d1d5db;border-radius:4px;
          font-size:1rem;margin-bottom:1rem;outline:none;transition:border-color .15s}
    input:focus{border-color:#0070f3}
    button{width:100%;padding:.75rem;background:#0070f3;color:#fff;border:none;
           border-radius:4px;font-size:1rem;cursor:pointer;transition:background .15s}
    button:hover{background:#005ce6}
    #msg{margin-top:1rem;font-size:.875rem;text-align:center;color:#dc2626}
  </style>
</head>
<body>
  <div class="card">
    <h2>Spotledger</h2>
    <form id="login-form">
      <label for="usr">User / Email</label>
      <input id="usr" name="usr" type="text" autocomplete="username" autofocus required>
      <label for="pwd">Password</label>
      <input id="pwd" name="pwd" type="password" autocomplete="current-password" required>
      <button type="submit">Log In</button>
    </form>
    <div id="msg"></div>
  </div>
  <script>
    document.getElementById('login-form').addEventListener('submit', async function(e) {
      e.preventDefault();
      const body = new URLSearchParams({
        usr: document.getElementById('usr').value,
        pwd: document.getElementById('pwd').value,
      });
      const res = await fetch('/api/method/login', {method:'POST', body});
      if (res.ok) {
        window.location.href = '/desk';
      } else {
        const d = await res.json().catch(()=>({}));
        document.getElementById('msg').textContent = d.message || 'Login failed';
      }
    });
  </script>
</body>
</html>"#;
