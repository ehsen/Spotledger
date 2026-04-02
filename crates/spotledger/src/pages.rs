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

    // Bundle hashes are stable between Frappe builds; update these when running
    // `bench build` and the hash changes.
    let desk_js  = "/assets/frappe/dist/js/desk.bundle.CJXF32XD.js";
    let desk_css = "/assets/frappe/dist/css/desk.bundle.JYCYQNVB.css";

    let is_system_user = user != "Guest";
    let boot_json = serde_json::json!({
        // ── user identity ────────────────────────────────────────────────────
        "user": &user,
        "user_info": {
            &user: {
                "name":      &user,
                "full_name": &full_name,
                "image":     "",
                "email":     "",
                "enabled":   1,
                "user_type": if is_system_user { "System User" } else { "Website User" },
            }
        },
        // desk.js reads user.name, user.email, user.roles, user.defaults
        // as a sub-object, not as top-level keys.
        // Frappe embeds: bootinfo.user = {name, email, roles, defaults, ...}
        // This differs from bootinfo.user_info (display map keyed by username).

        // ── locale / meta ────────────────────────────────────────────────────
        "lang":             "en",
        "__messages":       {},
        "metadata_version": "1",   // desk caches meta at this version; bump to force refresh

        // ── navigation ───────────────────────────────────────────────────────
        "home_page":        "Workspaces",
        "is_system_user":   is_system_user,

        // page_info: dict of allowed page/module names — Desk uses to build routes.
        // Empty dict is safe; routing falls back to default Workspaces home.
        "page_info": {},

        // workspaces: {pages: [...]} — Desk sidebar.
        // Empty pages list renders an empty sidebar without a JS error.
        "workspaces": { "pages": [] },

        // ── system defaults ──────────────────────────────────────────────────
        "sysdefaults": {
            "setup_complete":    "1",
            "date_format":       "dd-mm-yyyy",
            "time_format":       "HH:mm:ss",
            "float_precision":   "3",
            "currency_precision":"2",
            "number_format":     "#,###.##",
            "first_day_of_the_week": "Sunday",
        },

        // ── modules (drives sidebar grouping) ────────────────────────────────
        "modules":          {},
        "module_list":      [],
        "single_types":     [],
        "nested_set_doctypes": [],

        // ── permissions (populated guest-safe defaults) ───────────────────────
        "user_permissions": {},
        "can_read": [], "can_create": [], "can_write": [],
        "can_cancel": [], "can_delete": [],
        "allow_print": [], "allow_email": [], "allow_import": [],
        "allow_export": [], "allow_reports": [],

        // ── desktop / navbar ─────────────────────────────────────────────────
        "desktop_icons": [],
        "navbar_settings": {
            "app_logo_url": "/assets/frappe/images/frappe-favicon.svg",
            "brand_html":   "",
            "help_links":   [],
            "notifications_viewall_by_type": [],
        },
        "notification_dot_count": 0,
        "notification_settings": {
            "enable_notifications": 0,
        },

        // ── misc ─────────────────────────────────────────────────────────────
        "versions":          {},
        "docs":              [],
        "change_log":        [],
        "notes":             [],
        "onboarding_tours":  [],
        "print_css":         "",
        "timezone_info":     "",
        "time_zone": {
            "user":   "System",
            "system": "UTC",
        },
        "server_date": current_date(),
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

/// Return today's date as `YYYY-MM-DD` using only std::time (no chrono dep).
fn current_date() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    // Compute civil calendar date from Unix timestamp (Greg. calendar, no leap seconds).
    let days = (secs / 86400) as u32;
    // Algorithm from http://howardhinnant.github.io/date_algorithms.html
    let z = days + 719468;
    let era = z / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{:04}-{:02}-{:02}", y, m, d)
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
