//! Integration tests for Spotledger — requires a live SurrealDB instance.
//!
//! ## Prerequisites
//!
//! SurrealDB must be running on `ws://127.0.0.1:8000`:
//!
//! ```sh
//! surreal start --bind 0.0.0.0:8000 --user root --pass root memory
//! ```
//!
//! Run the full suite:
//!
//! ```sh
//! cargo test --test integration -- --nocapture
//! ```
//!
//! ## What is tested
//!
//! 1. `new_site` — bootstrap a fresh site namespace
//! 2. `GET /api/ping` — health check (no Host header required)
//! 3. `POST /api/method/login` — authenticate as Administrator
//! 4. `GET /api/method/frappe.auth.get_logged_user` — session cookie returns user
//! 5. `POST /api/method/logout` — session invalidation
//! 6. `GET /api/method/frappe.auth.get_logged_user` after logout — returns Guest
//! 7. `POST /api/method/login` wrong password — 401 AuthenticationError
//! 8. `POST /api/method/login` wrong user — 401 AuthenticationError

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use spotledger_core::config::{DatabaseConfig, SiteConfig, SiteInfo, CacheConfig, AppsConfig};
use spotledger_db::{
    auth::set_user_password,
    bootstrap::{run_framework_tables, seed_default_records},
    connection::connect,
    schema::ensure_all_schemas,
};
use spotledger_http::{build_app, register_site_from_config, AppState};
use std::sync::OnceLock;
use tokio::sync::Mutex;
use tower::ServiceExt;

// ── Constants ─────────────────────────────────────────────────────────────────

const DB_URL: &str = "ws://127.0.0.1:8000";
const DB_USER: &str = "root";
const DB_PASS: &str = "root";
/// Isolated namespace so integration tests don't touch production data.
const TEST_NS: &str = "test_spotledger_integration";
const TEST_SITE: &str = "test_spotledger_integration";
const ADMIN_PASSWORD: &str = "test_admin_pass_123!";

// ── Shared setup ──────────────────────────────────────────────────────────────

/// Guards sequential access to `bootstrap_test_site` so concurrent tests do
/// not race on `REMOVE DATABASE` / `UPSERT` against the same namespace.
static BOOTSTRAP_MUTEX: OnceLock<Mutex<()>> = OnceLock::new();

fn bootstrap_mutex() -> &'static Mutex<()> {
    BOOTSTRAP_MUTEX.get_or_init(|| Mutex::new(()))
}

/// Whether the bootstrap has already completed for this test binary run.
static BOOTSTRAP_DONE: OnceLock<()> = OnceLock::new();

fn test_db_config() -> DatabaseConfig {
    DatabaseConfig {
        url: DB_URL.into(),
        ns: TEST_NS.into(),
        db: TEST_NS.into(),
        user: DB_USER.into(),
        pass: DB_PASS.into(),
    }
}

fn test_site_config() -> SiteConfig {
    SiteConfig {
        site: SiteInfo {
            name: TEST_SITE.into(),
            namespace: TEST_NS.into(),
        },
        database: test_db_config(),
        cache: CacheConfig::default(),
        apps: AppsConfig::default(),
    }
}

/// Bootstrap the test site once for all tests in this binary.
///
/// Drops the test namespace first to guarantee a clean slate, then runs the
/// full new-site sequence: framework tables → schemas → seed records →
/// Administrator password.
async fn bootstrap_test_site() {
    // Fast path: already done.
    if BOOTSTRAP_DONE.get().is_some() {
        return;
    }
    // Serialize concurrent callers so only one actually bootstraps.
    let _guard = bootstrap_mutex().lock().await;
    // Re-check after acquiring the lock (another task may have finished).
    if BOOTSTRAP_DONE.get().is_some() {
        return;
    }

    let cfg = test_db_config();
    let db = connect(&cfg).await.expect("connect to SurrealDB for bootstrap");

    // Drop the database for a clean slate, then reconnect.
    // SurrealDB v3: REMOVE DATABASE leaves the connection in a stale context.
    // We reconnect so DbAdapter::connect re-issues DEFINE NAMESPACE/DATABASE
    // and use_ns/use_db against the freshly created database.
    let _ = db
        .run(
            &format!("REMOVE DATABASE IF EXISTS `{TEST_NS}`"),
            vec![],
        )
        .await;

    // Reconnect — DbAdapter::connect now creates NS/DB before use_ns/use_db.
    let db = connect(&cfg).await.expect("reconnect after database removal");

    run_framework_tables(&db).await.expect("run_framework_tables");
    ensure_all_schemas(&db).await.expect("ensure_all_schemas");
    seed_default_records(&db).await.expect("seed_default_records");
    set_user_password(&db, "Administrator", ADMIN_PASSWORD)
        .await
        .expect("set_user_password");

    BOOTSTRAP_DONE.set(()).ok();
}

/// Build a fresh Axum [`tower::Service`] backed by the test site.
async fn build_test_app() -> axum::Router {
    bootstrap_test_site().await;

    let app_state = AppState::new();
    register_site_from_config(&app_state, test_site_config())
        .await
        .expect("register test site");

    build_app(app_state)
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Parse `Set-Cookie` headers and return the value of the named cookie.
fn extract_cookie(headers: &axum::http::HeaderMap, name: &str) -> Option<String> {
    headers
        .get_all("set-cookie")
        .iter()
        .filter_map(|v| v.to_str().ok())
        .filter_map(|line| {
            // Each line: "name=value; Path=/; ..."
            line.split(';').next().and_then(|pair| {
                let mut kv = pair.splitn(2, '=');
                let k = kv.next()?.trim();
                let v = kv.next()?.trim();
                if k == name { Some(v.to_owned()) } else { None }
            })
        })
        .next()
}

/// Collect the response body as a UTF-8 string.
async fn body_string(body: Body) -> String {
    let bytes = body.collect().await.unwrap().to_bytes();
    String::from_utf8_lossy(&bytes).into_owned()
}

// ── Tests ─────────────────────────────────────────────────────────────────────

/// GET /api/ping — health check, no Host header.
#[tokio::test]
async fn test_ping() {
    let app = build_test_app().await;

    let req = Request::builder()
        .uri("/api/ping")
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let body = body_string(resp.into_body()).await;
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(json["message"], "pong");
}

/// POST /api/method/login — correct credentials → 200 + session cookies.
#[tokio::test]
async fn test_login_success() {
    let app = build_test_app().await;

    let req = Request::builder()
        .method("POST")
        .uri("/api/method/login")
        .header("host", TEST_SITE)
        .header("content-type", "application/x-www-form-urlencoded")
        .body(Body::from(format!(
            "usr=Administrator&pwd={ADMIN_PASSWORD}"
        )))
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK, "expected 200 on login");

    // Response body contains Logged In message
    let (parts, body) = resp.into_parts();
    let text = body_string(body).await;
    let json: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(json["message"], "Logged In");
    assert_eq!(json["full_name"], "Administrator");
    assert_eq!(json["home_page"], "/desk");

    // Session cookie must be set
    let sid = extract_cookie(&parts.headers, "sid");
    assert!(sid.is_some(), "sid cookie must be set after login");
    assert!(!sid.unwrap().is_empty(), "sid must be non-empty");
}

/// POST /api/method/login with wrong password → 401.
#[tokio::test]
async fn test_login_wrong_password() {
    let app = build_test_app().await;

    let req = Request::builder()
        .method("POST")
        .uri("/api/method/login")
        .header("host", TEST_SITE)
        .header("content-type", "application/x-www-form-urlencoded")
        .body(Body::from("usr=Administrator&pwd=wrong_password"))
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    let body = body_string(resp.into_body()).await;
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(json["exc_type"], "AuthenticationError");
}

/// POST /api/method/login with unknown user → 401.
#[tokio::test]
async fn test_login_unknown_user() {
    let app = build_test_app().await;

    let req = Request::builder()
        .method("POST")
        .uri("/api/method/login")
        .header("host", TEST_SITE)
        .header("content-type", "application/x-www-form-urlencoded")
        .body(Body::from("usr=no_such_user@example.com&pwd=anything"))
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

/// GET /api/method/frappe.auth.get_logged_user with valid sid → returns username.
#[tokio::test]
async fn test_get_logged_user_after_login() {
    let app = build_test_app().await;

    // Step 1: login
    let login_req = Request::builder()
        .method("POST")
        .uri("/api/method/login")
        .header("host", TEST_SITE)
        .header("content-type", "application/x-www-form-urlencoded")
        .body(Body::from(format!(
            "usr=Administrator&pwd={ADMIN_PASSWORD}"
        )))
        .unwrap();

    let login_resp = app.clone().oneshot(login_req).await.unwrap();
    assert_eq!(login_resp.status(), StatusCode::OK);

    let sid = extract_cookie(login_resp.headers(), "sid")
        .expect("must have sid cookie after login");

    // Step 2: get_logged_user
    let glu_req = Request::builder()
        .method("GET")
        .uri("/api/method/frappe.auth.get_logged_user")
        .header("host", TEST_SITE)
        .header("cookie", format!("sid={sid}"))
        .body(Body::empty())
        .unwrap();

    let glu_resp = app.oneshot(glu_req).await.unwrap();
    assert_eq!(glu_resp.status(), StatusCode::OK);

    let body = body_string(glu_resp.into_body()).await;
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(
        json["message"], "Administrator",
        "get_logged_user must return the logged-in username"
    );
}

/// GET /api/method/frappe.auth.get_logged_user without sid → returns Guest.
#[tokio::test]
async fn test_get_logged_user_no_session() {
    let app = build_test_app().await;

    let req = Request::builder()
        .method("GET")
        .uri("/api/method/frappe.auth.get_logged_user")
        .header("host", TEST_SITE)
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let body = body_string(resp.into_body()).await;
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(json["message"], "Guest");
}

/// Full login → get_logged_user → logout → get_logged_user cycle.
#[tokio::test]
async fn test_login_logout_cycle() {
    let app = build_test_app().await;

    // Login
    let login_req = Request::builder()
        .method("POST")
        .uri("/api/method/login")
        .header("host", TEST_SITE)
        .header("content-type", "application/x-www-form-urlencoded")
        .body(Body::from(format!(
            "usr=Administrator&pwd={ADMIN_PASSWORD}"
        )))
        .unwrap();
    let login_resp = app.clone().oneshot(login_req).await.unwrap();
    assert_eq!(login_resp.status(), StatusCode::OK, "login must succeed");
    let sid = extract_cookie(login_resp.headers(), "sid").expect("sid after login");

    // Verify logged in
    let check_req = Request::builder()
        .method("GET")
        .uri("/api/method/frappe.auth.get_logged_user")
        .header("host", TEST_SITE)
        .header("cookie", format!("sid={sid}"))
        .body(Body::empty())
        .unwrap();
    let check_resp = app.clone().oneshot(check_req).await.unwrap();
    let body = body_string(check_resp.into_body()).await;
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&body).unwrap()["message"],
        "Administrator"
    );

    // Logout
    let logout_req = Request::builder()
        .method("POST")
        .uri("/api/method/logout")
        .header("host", TEST_SITE)
        .header("cookie", format!("sid={sid}"))
        .header("content-type", "application/x-www-form-urlencoded")
        .body(Body::empty())
        .unwrap();
    let logout_resp = app.clone().oneshot(logout_req).await.unwrap();
    assert_eq!(logout_resp.status(), StatusCode::OK);

    // After logout, same sid must resolve to Guest
    let after_req = Request::builder()
        .method("GET")
        .uri("/api/method/frappe.auth.get_logged_user")
        .header("host", TEST_SITE)
        .header("cookie", format!("sid={sid}"))
        .body(Body::empty())
        .unwrap();
    let after_resp = app.oneshot(after_req).await.unwrap();
    let after_body = body_string(after_resp.into_body()).await;
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&after_body).unwrap()["message"],
        "Guest",
        "After logout, expired sid must return Guest"
    );
}

/// POST /api/method/login without a valid Host header.
///
/// When exactly one site is loaded (dev mode), the middleware falls back to
/// that site, so login succeeds.  When multiple sites are loaded a 404 would
/// be returned.  This test documents the dev-convenience fallback behaviour.
#[tokio::test]
async fn test_login_no_host_header() {
    let app = build_test_app().await;

    let req = Request::builder()
        .method("POST")
        .uri("/api/method/login")
        .header("content-type", "application/x-www-form-urlencoded")
        .body(Body::from(format!(
            "usr=Administrator&pwd={ADMIN_PASSWORD}"
        )))
        .unwrap();

    // With a single site loaded the middleware falls back to it (dev convenience),
    // so the request succeeds even without a Host header.
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "single-site fallback: login must succeed without Host header"
    );
}
