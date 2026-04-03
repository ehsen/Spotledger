//! Spotledger proxy recorder.
//!
//! Route-split proxy for testing Spotledger's API compatibility against Frappe:
//!
//!   Page/asset routes  (/desk, /login, /app/*, /assets/*, /files/*, etc.)
//!     → forwarded directly to Frappe (primary, response returned as-is)
//!       The browser receives real Frappe HTML + JS bundles unchanged.
//!
//!   API routes  (/api/*)
//!     → forwarded to Spotledger (primary, response returned to browser)
//!     → forwarded to Frappe     (shadow,  response diffed + logged only)
//!       Every mismatch is written to the JSONL diff log.
//!
//! Usage:
//!   spotledger-proxy \
//!     --spotledger-url http://127.0.0.1:9100 \
//!     --frappe-url     http://127.0.0.1:8000 \
//!     --port           9200 \
//!     --site           spotledger_test \
//!     --log-file       /tmp/proxy-diffs.jsonl

use anyhow::Result;
use axum::{
    body::Body,
    extract::State,
    http::{HeaderMap, Method, Request, StatusCode},
    response::Response,
    routing::any,
    Router,
};
use clap::Parser;
use http_body_util::BodyExt;
use reqwest::Client;
use serde_json::Value;
use std::{
    fs::OpenOptions,
    io::Write,
    path::PathBuf,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

// ── CLI args ──────────────────────────────────────────────────────────────────

#[derive(Parser, Debug, Clone)]
#[command(name = "spotledger-proxy", about = "Frappe ↔ Spotledger side-by-side diff proxy")]
struct Args {
    /// Spotledger server URL (primary for API requests)
    #[arg(long, default_value = "http://127.0.0.1:9100")]
    spotledger_url: String,

    /// Real Frappe server URL (primary for page/asset requests; shadow for API diff)
    #[arg(long, default_value = "http://127.0.0.1:8000")]
    frappe_url: String,

    /// Port for this proxy to listen on
    #[arg(long, short, default_value = "9200")]
    port: u16,

    /// Host header injected into every request forwarded to Frappe.
    /// Set to the Frappe site name (e.g. spotledger_test).
    #[arg(long, default_value = "spotledger_test")]
    frappe_site: String,

    /// Host header injected into every request forwarded to Spotledger.
    /// Set to the SurrealDB-backed site name (e.g. exit-test.localhost).
    #[arg(long, default_value = "exit-test.localhost")]
    spotledger_site: String,

    /// Path to the JSONL diff log file
    #[arg(long, default_value = "/tmp/spotledger-proxy-diffs.jsonl")]
    log_file: PathBuf,

    /// When true (default) only log requests where Spotledger and Frappe differ.
    #[arg(long, default_value = "true")]
    diffs_only: bool,

    /// Frappe shadow user for independent session (avoids 403 on authenticated shadow calls).
    #[arg(long, default_value = "Administrator")]
    frappe_user: String,

    /// Frappe shadow password — must match the real Frappe site's admin password.
    #[arg(long, default_value = "")]
    frappe_password: String,
}

// ── shared state ─────────────────────────────────────────────────────────────

#[derive(Clone)]
struct ProxyState {
    args: Arc<Args>,
    client: Client,
    /// Frappe `sid` cookie obtained by logging in independently at proxy startup.
    /// Used for all shadow requests so Frappe doesn't 403 them.
    frappe_session: Arc<tokio::sync::Mutex<Option<String>>>,
}

// ── path classification ───────────────────────────────────────────────────────

/// Returns true if the request path should go directly to Frappe without
/// being shadowed through Spotledger.  These are page/asset routes that
/// Spotledger does not serve in the proxy-test scenario — the browser gets
/// the genuine Frappe HTML shell and JS bundles.
fn is_frappe_only(path: &str) -> bool {
    let bare = path.split('?').next().unwrap_or(path);
    // /api/* and /socket.io* are handled by Spotledger.
    // Everything else (/desk, /login, /app/*, /assets/*, etc.) goes to Frappe only.
    !bare.starts_with("/api/") && !bare.starts_with("/socket.io")
}

// ── main ──────────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let frappe_session = Arc::new(tokio::sync::Mutex::new(None::<String>));

    let state = ProxyState {
        args: Arc::new(args.clone()),
        client: Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()?,
        frappe_session: frappe_session.clone(),
    };

    // Establish an independent Frappe session for shadow diffing.
    // Without this every authenticated shadow call returns 403 because Frappe
    // doesn't recognise the Spotledger-issued sid.
    if !args.frappe_password.is_empty() {
        match frappe_login(
            &state.client,
            &args.frappe_url,
            &args.frappe_site,
            &args.frappe_user,
            &args.frappe_password,
        ).await {
            Some(sid) => {
                tracing::info!(sid_prefix = %&sid[..sid.len().min(8)], "Frappe shadow session established");
                *frappe_session.lock().await = Some(sid);
            }
            None => tracing::warn!("Frappe login failed — shadow diffs will show 403 for authenticated calls"),
        }
    } else {
        tracing::info!("--frappe-password not set; skipping Frappe shadow session");
    }

    let app = Router::new()
        .route("/{*path}", any(proxy_handler))
        .route("/", any(proxy_handler))
        .with_state(state);

    let addr = format!("127.0.0.1:{}", args.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!(addr = %addr, "Proxy listening");
    tracing::info!(
        spotledger = %args.spotledger_url,
        frappe     = %args.frappe_url,
        frappe_site = %args.frappe_site,
        spotledger_site = %args.spotledger_site,
        "Route split: /api/* → Spotledger(primary)+Frappe(shadow diff) | everything else → Frappe only"
    );
    tracing::info!(log = ?args.log_file, "Diff log");

    axum::serve(listener, app).await?;
    Ok(())
}

// ── proxy handler ─────────────────────────────────────────────────────────────

async fn proxy_handler(
    State(state): State<ProxyState>,
    req: Request<Body>,
) -> Result<Response<Body>, StatusCode> {
    let method = req.method().clone();
    let path_and_query = req
        .uri()
        .path_and_query()
        .map(|pq| pq.as_str())
        .unwrap_or("/")
        .to_owned();
    let req_headers = req.headers().clone();

    let body_bytes = req
        .into_body()
        .collect()
        .await
        .map_err(|_| StatusCode::BAD_REQUEST)?
        .to_bytes();

    if is_frappe_only(&path_and_query) {
        // ── Page / asset route → Frappe only ─────────────────────────────────
        // The browser gets the real Frappe HTML + JS bundles.  We inject the
        // proxy's Frappe session so Frappe serves authenticated pages instead
        // of redirecting to /login (the browser only holds a Spotledger sid
        // which Frappe doesn't recognise).
        let url = format!("{}{}", state.args.frappe_url, path_and_query);
        let frappe_sid: Option<String> = {
            let guard = state.frappe_session.lock().await;
            guard.clone()
        };
        let mut resp = forward_full(
            &state.client,
            &method,
            &url,
            &req_headers,
            &state.args.frappe_site,
            body_bytes,
            frappe_sid.as_deref(),
        )
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?;

        // For HTML page responses, inject a script that disables socket.io's
        // dev_server port-redirect so it doesn't try to connect to port 9000.
        let is_html = resp
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .map(|ct| ct.contains("text/html"))
            .unwrap_or(false);
        if is_html {
            let (parts, body) = resp.into_parts();
            if let Ok(body_bytes) = axum::body::to_bytes(body, usize::MAX).await {
                if let Ok(html) = std::str::from_utf8(&body_bytes) {
                    // Replace Frappe's dev_server=1 assignment directly so socket.io
                    // doesn't attempt to connect to port 9000.
                    let injected = html.replace(
                        "window.dev_server = 1;",
                        "window.dev_server = 0;",
                    );
                    return Ok(Response::from_parts(parts, Body::from(injected)));
                } else {
                    return Ok(Response::from_parts(parts, Body::from(body_bytes)));
                }
            } else {
                return Ok(Response::from_parts(parts, Body::empty()));
            }
        }

        return Ok(resp);
    }

    // ── socket.io → Spotledger only (no Frappe shadow) ───────────────────────
    let bare_path = path_and_query.split('?').next().unwrap_or(&path_and_query);
    if bare_path.starts_with("/socket.io") {
        let spot_url = format!("{}{}", state.args.spotledger_url, path_and_query);
        let (status, headers, body) = forward_bytes(
            &state.client,
            &method,
            &spot_url,
            &req_headers,
            &state.args.spotledger_site,
            body_bytes,
            None,
        )
        .await
        .unwrap_or((502, HeaderMap::new(), b"Bad Gateway".to_vec()));
        let mut builder = Response::builder().status(status);
        for (name, value) in &headers {
            let name_lower = name.as_str().to_lowercase();
            if name_lower != "transfer-encoding" {
                builder = builder.header(name, value);
            }
        }
        return Ok(builder.body(Body::from(body)).unwrap_or_default());
    }

    // ── API route → Spotledger (primary) + Frappe (shadow diff) ──────────────
    let spot_url   = format!("{}{}", state.args.spotledger_url, path_and_query);
    let frappe_url = format!("{}{}", state.args.frappe_url, path_and_query);

    // Use the proxy's own Frappe session for shadow calls — except for the
    // login endpoint itself (which creates a new session, no auth needed) and
    // for unauthenticated requests (no client cookie → no session injection so
    // both servers see the same auth level in the shadow diff).
    let frappe_sid: Option<String> = {
        let guard = state.frappe_session.lock().await;
        guard.clone()
    };
    let client_has_session = req_headers.contains_key(axum::http::header::COOKIE);
    let shadow_session = if path_and_query.contains("/api/method/login") || !client_has_session {
        None
    } else {
        frappe_sid.as_deref()
    };

    let (spot_result, frappe_result) = tokio::join!(
        forward_bytes(
            &state.client,
            &method,
            &spot_url,
            &req_headers,
            &state.args.spotledger_site,
            body_bytes.clone(),
            None,            // primary: pass client's own cookies unchanged
        ),
        forward_bytes(
            &state.client,
            &method,
            &frappe_url,
            &req_headers,
            &state.args.frappe_site,
            body_bytes,
            shadow_session,  // shadow: inject proxy's own Frappe session
        ),
    );

    let (spot_status, spot_headers, spot_body) =
        spot_result.unwrap_or((500, HeaderMap::new(), b"{}".to_vec()));
    let (frappe_status, _frappe_headers, frappe_body) =
        frappe_result.unwrap_or((500, HeaderMap::new(), b"{}".to_vec()));

    record_diff(
        &state.args,
        &path_and_query,
        method.as_str(),
        spot_status,
        &spot_body,
        frappe_status,
        &frappe_body,
    );

    // Build response from Spotledger's status + headers + body.
    // Crucially pass through Set-Cookie so login sessions work in the browser.
    let mut builder = Response::builder().status(spot_status);
    for (name, value) in &spot_headers {
        let name_lower = name.as_str().to_lowercase();
        if matches!(
            name_lower.as_str(),
            "connection" | "transfer-encoding" | "upgrade"
        ) {
            continue;
        }
        builder = builder.header(name, value);
    }
    let response = builder
        .body(Body::from(spot_body))
        .unwrap_or_else(|_| Response::new(Body::empty()));

    Ok(response)
}

// ── HTTP forwarding ───────────────────────────────────────────────────────────

/// Forward a request and return the complete Axum `Response` (preserves all
/// headers, content-type, status).  Used for page/asset routes.
///
/// `session_cookie`: when `Some(sid)`, the client's Cookie header is replaced
/// with `sid={sid}` so Frappe sees an authenticated session.
async fn forward_full(
    client: &Client,
    method: &Method,
    url: &str,
    headers: &HeaderMap,
    host_override: &str,
    body: axum::body::Bytes,
    session_cookie: Option<&str>,
) -> Result<Response<Body>, ()> {
    let reqwest_method =
        reqwest::Method::from_bytes(method.as_str().as_bytes()).map_err(|_| ())?;

    let mut builder = client.request(reqwest_method, url);
    for (name, value) in headers {
        let n = name.as_str().to_lowercase();
        if matches!(n.as_str(), "host" | "connection" | "transfer-encoding" | "upgrade") {
            continue;
        }
        // Drop client cookie header when injecting proxy's own session.
        if session_cookie.is_some() && n == "cookie" {
            continue;
        }
        if let Ok(v) = reqwest::header::HeaderValue::from_bytes(value.as_bytes()) {
            builder = builder.header(name.as_str(), v);
        }
    }
    builder = builder.header("host", host_override);
    if let Some(sid) = session_cookie {
        builder = builder.header("cookie", format!("sid={sid}"));
    }
    builder = builder.body(body.to_vec());

    let resp = builder.send().await.map_err(|_| ())?;
    let status = resp.status().as_u16();
    let resp_headers = resp.headers().clone();
    let body_bytes = resp.bytes().await.map_err(|_| ())?;

    let mut axum_builder = Response::builder().status(status);
    for (name, value) in &resp_headers {
        let n = name.as_str().to_lowercase();
        if matches!(n.as_str(), "connection" | "transfer-encoding" | "upgrade") {
            continue;
        }
        axum_builder = axum_builder.header(name.as_str(), value.as_bytes());
    }
    axum_builder
        .body(Body::from(body_bytes.to_vec()))
        .map_err(|_| ())
}

/// Forward a request and return `(status, response_headers, body_bytes)`.
/// Used for API routes where we need to inspect/diff the body.
///
/// `session_cookie`: when `Some(sid)`, the client's `cookie` header is stripped
/// and replaced with `sid={sid}`.  Pass `None` for the primary Spotledger path
/// so the browser's own session cookie is forwarded unchanged.
async fn forward_bytes(
    client: &Client,
    method: &Method,
    url: &str,
    headers: &HeaderMap,
    host_override: &str,
    body: axum::body::Bytes,
    session_cookie: Option<&str>,
) -> Result<(u16, HeaderMap, Vec<u8>), ()> {
    let reqwest_method =
        reqwest::Method::from_bytes(method.as_str().as_bytes()).map_err(|_| ())?;

    let mut builder = client.request(reqwest_method, url);
    for (name, value) in headers {
        let n = name.as_str().to_lowercase();
        if matches!(n.as_str(), "host" | "connection" | "transfer-encoding" | "upgrade") {
            continue;
        }
        // When injecting a session override, drop the client's Cookie header
        // entirely — we'll add our own sid below.
        if session_cookie.is_some() && n == "cookie" {
            continue;
        }
        if let Ok(v) = reqwest::header::HeaderValue::from_bytes(value.as_bytes()) {
            builder = builder.header(name.as_str(), v);
        }
    }
    builder = builder.header("host", host_override);
    if let Some(sid) = session_cookie {
        builder = builder.header("cookie", format!("sid={sid}"));
    }
    builder = builder.body(body.to_vec());

    let resp = builder.send().await.map_err(|_| ())?;
    let status = resp.status().as_u16();
    // Convert reqwest HeaderMap → axum/http HeaderMap
    let mut axum_headers = HeaderMap::new();
    for (name, value) in resp.headers() {
        if let (Ok(n), Ok(v)) = (
            axum::http::header::HeaderName::from_bytes(name.as_str().as_bytes()),
            axum::http::header::HeaderValue::from_bytes(value.as_bytes()),
        ) {
            axum_headers.append(n, v);
        }
    }
    let body_bytes = resp.bytes().await.map_err(|_| ())?.to_vec();
    Ok((status, axum_headers, body_bytes))
}

// ── diff logging ──────────────────────────────────────────────────────────────

fn record_diff(
    args: &Args,
    path: &str,
    method: &str,
    spot_status: u16,
    spot_body: &[u8],
    frappe_status: u16,
    frappe_body: &[u8],
) {
    // Parse both as JSON for structural diff; fall back to raw string comparison
    let spot_json: Value =
        serde_json::from_slice(spot_body).unwrap_or(Value::String(lossy(spot_body)));
    let frappe_json: Value =
        serde_json::from_slice(frappe_body).unwrap_or(Value::String(lossy(frappe_body)));

    let bodies_identical = spot_json == frappe_json;
    let statuses_identical = spot_status == frappe_status;

    if args.diffs_only && bodies_identical && statuses_identical {
        return;
    }

    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);

    let entry = serde_json::json!({
        "ts": ts,
        "method": method,
        "path": path,
        "spotledger": { "status": spot_status, "body": spot_json },
        "frappe":     { "status": frappe_status, "body": frappe_json },
        "identical":  bodies_identical && statuses_identical,
    });

    let line = format!("{}\n", entry);

    match OpenOptions::new()
        .create(true)
        .append(true)
        .open(&args.log_file)
    {
        Ok(mut f) => {
            let _ = f.write_all(line.as_bytes());
        }
        Err(e) => {
            tracing::error!(error = %e, path = ?args.log_file, "Failed to write diff log");
        }
    }
}

fn lossy(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

// ── Frappe shadow session ─────────────────────────────────────────────────────

/// Log in to the real Frappe server and return the `sid` cookie value.
/// Used at proxy startup so that shadow requests are authenticated.
async fn frappe_login(
    client: &Client,
    frappe_url: &str,
    frappe_site: &str,
    user: &str,
    password: &str,
) -> Option<String> {
    let url = format!("{}/api/method/login", frappe_url);
    let resp = client
        .post(&url)
        .header("host", frappe_site)
        .form(&[("usr", user), ("pwd", password)])
        .send()
        .await
        .ok()?;

    if !resp.status().is_success() {
        tracing::warn!(
            status = %resp.status(),
            url = %url,
            "Frappe shadow login returned non-2xx"
        );
        return None;
    }

    // Extract `sid` from Set-Cookie response headers.
    for value in resp.headers().get_all("set-cookie") {
        if let Ok(s) = value.to_str() {
            // Each Set-Cookie looks like: "sid=abc123; Path=/; HttpOnly; ..."
            if let Some(sid_pair) = s.split(';').next() {
                let sid_pair = sid_pair.trim();
                if let Some(sid) = sid_pair.strip_prefix("sid=") {
                    let sid = sid.trim().to_owned();
                    if !sid.is_empty() && sid != "Guest" {
                        return Some(sid);
                    }
                }
            }
        }
    }
    tracing::warn!("Frappe login succeeded but no sid cookie in response");
    None
}

// ── tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_diff_identical_skipped_when_diffs_only() {
        let args = Args {
            spotledger_url: String::new(),
            frappe_url: String::new(),
            port: 9200,
            frappe_site: "spotledger_test".into(),
            spotledger_site: "exit-test.localhost".into(),
            log_file: PathBuf::from("/dev/null"),
            diffs_only: true,
            frappe_user: "Administrator".into(),
            frappe_password: String::new(),
        };
        // Should not panic; writing to /dev/null is always fine
        record_diff(&args, "/test", "GET", 200, b"{}", 200, b"{}");
    }

    #[test]
    fn lossy_handles_valid_utf8() {
        assert_eq!(lossy(b"hello"), "hello");
    }

    #[test]
    fn lossy_handles_invalid_utf8() {
        let s = lossy(&[0xFF, 0xFE]);
        assert!(!s.is_empty());
    }
}
