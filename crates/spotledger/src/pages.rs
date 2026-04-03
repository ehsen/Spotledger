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
use spotledger_db::document::{get_doc, get_list};
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

/// GET `/login` — serve Frappe's login page (website CSS + login bundle CSS + inline login.js).
pub async fn login_page(
    Extension(site): Extension<Arc<SiteState>>,
) -> impl IntoResponse {
    let a = &site.assets_json;
    let resolve = |key: &str| -> &str {
        a.get(key).and_then(|v| v.as_str()).unwrap_or("")
    };
    let website_css  = resolve("website.bundle.css");
    let login_css    = resolve("login.bundle.css");
    let frappe_web_js = resolve("frappe-web.bundle.js");
    let assets_json_str = serde_json::to_string(&site.assets_json)
        .unwrap_or_else(|_| "{}".to_string());
    let html = format!(r##"<!DOCTYPE html>
<!-- Built on Spotledger (Frappe-compatible). -->
<html lang="en" dir="ltr">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1, shrink-to-fit=no">
  <meta name="generator" content="frappe">
  <title>Login</title>
  <link rel="shortcut icon" href="/assets/frappe/images/frappe-favicon.svg" type="image/x-icon">
  <link type="text/css" rel="stylesheet" href="{website_css}">
  <link type="text/css" rel="stylesheet" href="{login_css}">
  <script>
    window.frappe = {{}};
    window._version_number = "1";
    frappe.ready_events = [];
    frappe.ready = function(fn) {{ frappe.ready_events.push(fn); }};
    window.dev_server = 0;
    window.socketio_port = 9000;
    window.show_language_picker = false;
  </script>
</head>
{LOGIN_BODY_HTML}
<script type="text/javascript" src="{frappe_web_js}"></script>
<script>
frappe.boot = {{"lang":"en","sysdefaults":{{"float_precision":3,"date_format":"dd-mm-yyyy","time_format":"HH:mm:ss","first_day_of_the_week":"Sunday","number_format":"#,###.##"}},"time_zone":{{"system":"UTC","user":"System"}},"assets_json":{assets_json_str}}};
frappe.csrf_token = "None";
</script>
{LOGIN_SCRIPT_HTML}
</body>
</html>"##);
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
        html,
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

    // Resolve bundle paths from the bench's assets.json (loaded at server startup).
    // Falls back to empty string if a key is missing — the browser will 404 gracefully.
    let a = &site.assets_json;
    let resolve = |key: &str| -> String {
        a.get(key).and_then(|v| v.as_str()).unwrap_or("").to_owned()
    };

    let libs_js      = resolve("libs.bundle.js");
    let desk_js      = resolve("desk.bundle.js");
    let list_js      = resolve("list.bundle.js");
    let form_js      = resolve("form.bundle.js");
    let controls_js  = resolve("controls.bundle.js");
    let report_js    = resolve("report.bundle.js");
    let telemetry_js = resolve("telemetry.bundle.js");
    let billing_js   = resolve("billing.bundle.js");
    let desk_css     = resolve("desk.bundle.css");
    let report_css   = resolve("report.bundle.css");

    let is_system_user = user != "Guest";
    let user_type = if is_system_user { "System User" } else { "Website User" };

    // ── workspaces from DB ────────────────────────────────────────────────────
    let workspace_pages = {
        let fields = &[
            "name", "title", "label", "public", "icon", "module", "app",
            "type", "parent_page", "for_user", "sequence_id", "is_hidden",
            "restrict_to_domain", "content", "indicator_color",
            "external_link", "link_to", "link_type",
        ];
        let filter = serde_json::json!({"public": 1, "is_hidden": 0, "for_user": ""});
        match get_list(&site.db, "Workspace", Some(fields), Some(&filter), 100, 0).await {
            Ok(rows) => {
                let mut pages: Vec<serde_json::Value> = rows
                    .into_iter()
                    .filter(|r| r.get("title").and_then(|v| v.as_str()).unwrap_or("") != "Welcome Workspace")
                    .map(|r| serde_json::Value::Object(r.into_iter().collect()))
                    .collect();
                pages.sort_by(|a, b| {
                    let sa = a.get("sequence_id").and_then(serde_json::Value::as_f64).unwrap_or(999.0);
                    let sb = b.get("sequence_id").and_then(serde_json::Value::as_f64).unwrap_or(999.0);
                    sa.partial_cmp(&sb).unwrap_or(std::cmp::Ordering::Equal)
                });
                serde_json::Value::Array(pages)
            }
            Err(_) => serde_json::Value::Array(vec![]),
        }
    };

    // workspace_sidebar_item: map keyed by workspace title (lowercase)
    let sidebar_item: serde_json::Map<String, serde_json::Value> =
        if let serde_json::Value::Array(ref pages) = workspace_pages {
            pages.iter().filter_map(|p| {
                let title = p.get("title").and_then(serde_json::Value::as_str)?;
                let key = title.to_lowercase();
                Some((key, serde_json::json!({
                    "label": title,
                    "items": [],
                    "app": p.get("app").cloned().unwrap_or(serde_json::Value::Null),
                    "module": p.get("module").cloned().unwrap_or(serde_json::Value::Null),
                })))
            }).collect()
        } else {
            serde_json::Map::new()
        };

    let mut boot_json = serde_json::json!({
        // ── user identity ────────────────────────────────────────────────────
        // desk.js `set_globals()` does:
        //   frappe.session.user      = frappe.boot.user.name
        //   frappe.user_roles        = frappe.boot.user.roles
        //   frappe.user_defaults     = frappe.boot.user.defaults
        //   frappe.boot.user.last_selected_values = {}   ← crashes if user is a string
        // So user MUST be an object, not a string.
        "user": {
            "name":          &user,
            "full_name":     &full_name,
            "first_name":    &full_name,
            "last_name":     serde_json::Value::Null,
            "email":         "",
            "user_type":     user_type,
            "desk_theme":    "Automatic",
            "language":      serde_json::Value::Null,
            "roles":         ["System Manager", "Administrator", "All"],
            "defaults":      {
                "language": "en",
                "date_format": "dd-mm-yyyy",
                "time_format": "HH:mm:ss",
                "float_precision": "3",
                "number_format": "#,###.##",
                "first_day_of_the_week": "Sunday",
                "desktop:home_page": "workspace",
                "setup_complete": "1",
            },
            "can_read":      [],
            "can_write":     [],
            "can_create":    [],
            "can_delete":    [],
            "can_cancel":    [],
            "can_submit":    [],
            "can_get_report":[],
            "allow_modules": [],
            "all_read":      [],
            "can_search":    [],
            "in_create":     [],
            "can_export":    [],
            "can_import":    [],
            "can_print":     [],
            "can_email":     [],
            "can_select":    [],
            "all_reports":   {},
            "onboarding_status": {},
            "default_workspace": serde_json::Value::Null,
            "mute_sounds": 0,
            "send_me_a_copy": 0,
            "document_follow_notify": 0,
            "show_absolute_datetime_in_timeline": 0,
            "code_editor_type": "vscode",
        },
        // user_info: map keyed by username — Frappe uses "fullname" (not "full_name")
        "user_info": {
            &user: {
                "name":      &user,
                "fullname":  &full_name,
                "image":     serde_json::Value::Null,
                "email":     "",
                "time_zone": "System",
            }
        },

        // ── locale / meta ────────────────────────────────────────────────────
        "lang":             "en",
        "lang_dict":        {},
        "__messages":       {},
        "metadata_version": "1",   // desk caches meta at this version; bump to force refresh

        // ── desk theme & settings ─────────────────────────────────────────────
        "desk_theme":       "Automatic",
        "desk_settings": {
            "search_bar": 1, "notifications": 1, "list_sidebar": 1,
            "bulk_actions": 1, "view_switcher": 1, "form_sidebar": 1,
            "timeline": 1, "dashboard": 1,
        },
        "desktop_icon_style": "Subtle",
        "desktop_icon_urls":  {},

        // ── navigation ───────────────────────────────────────────────────────
        "home_page":        "Workspaces",
        "is_system_user":   is_system_user,
        "setup_complete":   true,
        "setup_wizard_completed_apps": ["frappe"],
        "sitename":         "exit-test.localhost",

        // ── socketio ─────────────────────────────────────────────────────────
        "socketio_port":    9000,
        "disable_async":    serde_json::Value::Null,
        "file_watcher_port": 6787,
        "from_cache":       0,
        "read_only":        false,

        // ── page / module info ────────────────────────────────────────────────
        "page_info": {},
        "module_app": {},
        "module_wise_workspaces": {},

        // workspaces: used by the workspaces page renderer
        "workspaces": {
            "pages": workspace_pages,
            "has_access": true,
            "has_create_access": true,
            "workspace_setup_completed": 1,
        },

        // workspace_sidebar_item: dict keyed by sidebar title (lowercase).
        "workspace_sidebar_item": serde_json::Value::Object(sidebar_item),

        // app data
        "app_name_style": "Default",
        "app_data": [],
        "apps_data": { "apps": [], "is_desk_apps": 1, "default_path": "" },
        "app_logo_url": "/assets/frappe/images/frappe-framework-logo.svg",
        "is_fc_site": false,
        "show_app_icons_as_folder": 0,
        "show_external_link_warning": "Never",

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
        "treeviews":        [],
        "calendars":        [],
        "translated_doctypes": [],
        "doctype_layouts":  [],
        "link_title_doctypes": [],
        "link_preview_doctypes": [],
        "additional_filters_config": {},
        "active_domains":   [],
        "all_domains":      [],

        // ── permissions ───────────────────────────────────────────────────────
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
            "settings_dropdown": [
                { "item_label": "User Settings", "item_type": "Action", "action": "frappe.ui.toolbar.route_to_user()", "is_standard": 1, "hidden": 0 },
                { "item_label": "Log out",       "item_type": "Action", "action": "frappe.app.logout()",             "is_standard": 1, "hidden": 0 }
            ],
            "help_dropdown": [
                { "item_label": "About",              "item_type": "Action", "action": "frappe.ui.toolbar.show_about()",     "is_standard": 1, "hidden": 0 },
                { "item_label": "Keyboard Shortcuts", "item_type": "Action", "action": "frappe.ui.toolbar.show_shortcuts(event)", "is_standard": 1, "hidden": 0 }
            ],
        },
        "notification_settings": {
            "enable_notifications": 0,
        },
        "frequently_visited_links": [],
        "success_action": [],
        "dashboards": [],
        "changelog_feed": [],
        "email_accounts": [],
        "letter_heads": {},
        "marketplace_apps": [],
        "max_file_size": 26214400,
        "enable_address_autocompletion": 0,
        "sms_gateway_enabled": false,
        "has_app_updates": false,
        "home_folder": "Home",
        "subscription_conf": serde_json::Value::Null,
        "error_report_email": serde_json::Value::Null,
        "telemetry_site_age": 1,

        // ── misc ─────────────────────────────────────────────────────────────
        "versions":          { "frappe": "16.0.0-dev" },
        "docs":              [],
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

    // Insert assets_json after building the base boot object to avoid hitting
    // the serde_json::json!() macro recursion limit with a 47-key nested map.
    if let Some(obj) = boot_json.as_object_mut() {
        obj.insert("assets_json".to_string(), site.assets_json.clone());
    }

    let boot_str = serde_json::to_string(&boot_json).unwrap_or_else(|_| "{}".to_string());

    let html = format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>Spotledger</title>
  <link rel="shortcut icon" href="/assets/frappe/images/frappe-favicon.svg">
  <link type="text/css" rel="stylesheet" href="{desk_css}">
  <link type="text/css" rel="stylesheet" href="{report_css}">
</head>
<body>
  <div class="main-section">
    <header></header>
    <div id="body"></div>
    <footer></footer>
  </div>
  <div id="all-symbols" style="display:none"></div>
  <script type="text/javascript">fetch(`/assets/frappe/icons/lucide/icons.svg?v=1`,{{credentials:"same-origin"}}).then(r=>r.text()).then(svg=>{{let c=document.getElementById("all-symbols");c.insertAdjacentHTML("beforeend",svg);}});</script>
  <script type="text/javascript">fetch(`/assets/frappe/icons/timeless/icons.svg?v=1`,{{credentials:"same-origin"}}).then(r=>r.text()).then(svg=>{{let c=document.getElementById("all-symbols");c.insertAdjacentHTML("beforeend",svg);}});</script>
  <script type="text/javascript">fetch(`/assets/frappe/icons/espresso/icons.svg?v=1`,{{credentials:"same-origin"}}).then(r=>r.text()).then(svg=>{{let c=document.getElementById("all-symbols");c.insertAdjacentHTML("beforeend",svg);}});</script>
  <script>
    window._version_number = "1";
    window.app = true;
    window.dev_server = false;
    if(!window.frappe) window.frappe = {{}};
    frappe.boot = {boot_str};
    frappe._messages = {{}};
    frappe.csrf_token = "spotledger-no-csrf";
  </script>
  <script type="text/javascript" src="{libs_js}"></script>
  <script type="text/javascript" src="{desk_js}"></script>
  <script type="text/javascript" src="{list_js}"></script>
  <script type="text/javascript" src="{form_js}"></script>
  <script type="text/javascript" src="{controls_js}"></script>
  <script type="text/javascript" src="{report_js}"></script>
  <script type="text/javascript" src="{telemetry_js}"></script>
  <script type="text/javascript" src="{billing_js}"></script>
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

// ── Login HTML parts (bundle URLs injected at runtime from assets.json) ──────

const LOGIN_BODY_HTML: &str = r##"<body frappe-session-status="logged-out" data-path="login" class="">

<div id="all-symbols" style="display:none"></div>
<script type="text/javascript">fetch(`/assets/frappe/icons/lucide/icons.svg?v=1`,{credentials:"same-origin"}).then(r=>r.text()).then(svg=>{let c=document.getElementById("all-symbols");c.insertAdjacentHTML("beforeend",svg);});</script>
<script type="text/javascript">fetch(`/assets/frappe/icons/timeless/icons.svg?v=1`,{credentials:"same-origin"}).then(r=>r.text()).then(svg=>{let c=document.getElementById("all-symbols");c.insertAdjacentHTML("beforeend",svg);});</script>
<script type="text/javascript">fetch(`/assets/frappe/icons/espresso/icons.svg?v=1`,{credentials:"same-origin"}).then(r=>r.text()).then(svg=>{let c=document.getElementById("all-symbols");c.insertAdjacentHTML("beforeend",svg);});</script>

<div id="page-login" data-path="login">
<div class="page-content-wrapper">
  <div class="page-breadcrumbs"></div>
  <main class="container">
    <div class="page-header-wrapper"><div class="page-header"></div></div>
    <div class="page_content">

<div>
  <noscript>
    <div class="text-center my-5">
      <h4>Javascript is disabled on your browser</h4>
      <p class="text-muted">You need to enable JavaScript for your app to work.</p>
    </div>
  </noscript>
  <section class='for-login'>

    <div class="page-card-head">
      <img class="app-logo" src="/assets/frappe/images/frappe-framework-logo.svg">
      <h4>Login to Frappe</h4>
    </div>

    <div class="login-content page-card">
      <form class="form-signin form-login" role="form">
        <div class="page-card-body">
          <div class="form-group">
            <label class="form-label sr-only" for="login_email">Email</label>
            <div class="email-field">
              <input type="text" id="login_email" class="form-control"
                placeholder="jane@example.com" required autofocus autocomplete="username">
              <svg class="field-icon email-icon" width="16" height="16" viewBox="0 0 16 16" fill="none" xmlns="http://www.w3.org/2000/svg">
                <use class="es-lock" href="#es-line-email"></use>
              </svg>
            </div>
          </div>
          <div class="form-group">
            <label class="form-label sr-only" for="login_password">Password</label>
            <div class="password-field">
              <input type="password" id="login_password" class="form-control"
                placeholder="•••••" autocomplete="current-password" required>
              <svg class="field-icon password-icon" width="16" height="16" viewBox="0 0 16 16" fill="none" xmlns="http://www.w3.org/2000/svg">
                <use class="es-lock" href="#es-line-lock"></use>
              </svg>
              <span toggle="#login_password" class="toggle-password text-muted">Show</span>
            </div>
          </div>
          <p class="forgot-password-message"><a href="#forgot">Forgot Password?</a></p>
        </div>
        <div class="page-card-actions">
          <button class="btn btn-sm btn-primary btn-block btn-login" type="submit">Login</button>
        </div>
      </form>
    </div>

  </section>
</div>

    </div>
  </main>
</div>
</div>"##;

const LOGIN_SCRIPT_HTML: &str = r##"<script>
// login.js
window.disable_signup = true;
window.show_footer_on_login = false;
window.login = {};
window.verify = {};
login.bind_events = function () {
  $(window).on("hashchange", function () { login.route(); });
  $(".form-login").on("submit", function (event) {
    event.preventDefault();
    var args = {};
    args.cmd = "login";
    args.usr = ($("#login_email").val() || "").trim();
    args.pwd = $("#login_password").val();
    if (!args.usr || !args.pwd) {
      frappe.msgprint(__("Please enter both email and password"));
      return false;
    }
    login.call(args);
    return false;
  });
  $(".toggle-password").on("click", function () {
    var input = $($(this).attr("toggle"));
    if (input.attr("type") === "password") {
      input.attr("type", "text");
      $(this).text("Hide");
    } else {
      input.attr("type", "password");
      $(this).text("Show");
    }
  });
};
login.route = function () {
  var route = window.location.hash;
  if (!route) { login.show_login_modal(); return; }
  route = route.replace("#", "");
  if (route === "forgot") {
    login.show_forgot_password();
  } else {
    login.show_login_modal();
  }
};
login.show_login_modal = function () {
  $(".for-login").toggle(true);
  $(".for-forgot-password").toggle(false);
  $(".for-email-login").toggle(false);
};
login.show_forgot_password = function () {
  $(".for-login").toggle(false);
  $(".for-forgot-password").toggle(true);
};
login.call = function (args) {
  return $.ajax({
    type: "POST",
    url: "/api/method/login",
    data: { usr: args.usr, pwd: args.pwd },
    success: function (data) {
      if (data.message === "Logged In") {
        login.login_again(data);
      } else if (data.message === "No App") {
        window.location.href = "/";
      }
    },
    error: function (xhr) {
      var msg = (xhr.responseJSON && xhr.responseJSON.message) || "Invalid credentials";
      if (msg === "User disabled or missing") {
        frappe.msgprint(__("Invalid login. Try again."));
      } else {
        frappe.msgprint(__(msg));
      }
    }
  });
};
login.login_again = function (data) {
  var home = data && data.home_page ? data.home_page : "/desk";
  window.location.href = home;
};
frappe.ready(function () {
  login.bind_events();
  login.route();
});
</script>"##;
