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
}

// ── shared state ─────────────────────────────────────────────────────────────

#[derive(Clone)]
struct ProxyState {
    args: Arc<Args>,
    client: Client,
}

// ── path classification ───────────────────────────────────────────────────────

/// Returns true if the request path should go directly to Frappe without
/// being shadowed through Spotledger.  These are page/asset routes that
/// Spotledger does not serve in the proxy-test scenario — the browser gets
/// the genuine Frappe HTML shell and JS bundles.
fn is_frappe_only(path: &str) -> bool {
    let api_path = path.starts_with("/api/");
    // Everything that is NOT an /api/ route goes to Frappe only.
    // This includes /desk, /login, /app/*, /assets/*, /files/*, /favicon.ico, etc.
    !api_path
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

    let state = ProxyState {
        args: Arc::new(args.clone()),
        client: Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()?,
    };

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
        // The browser gets the real Frappe HTML + JS bundles.  We pass the
        // Host header for the Frappe site so Frappe's multi-tenancy resolves.
        let url = format!("{}{}", state.args.frappe_url, path_and_query);
        let resp = forward_full(
            &state.client,
            &method,
            &url,
            &req_headers,
            &state.args.frappe_site,
            body_bytes,
        )
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?;
        return Ok(resp);
    }

    // ── API route → Spotledger (primary) + Frappe (shadow diff) ──────────────
    let spot_url   = format!("{}{}", state.args.spotledger_url, path_and_query);
    let frappe_url = format!("{}{}", state.args.frappe_url, path_and_query);

    let (spot_result, frappe_result) = tokio::join!(
        forward_bytes(
            &state.client,
            &method,
            &spot_url,
            &req_headers,
            &state.args.spotledger_site,
            body_bytes.clone(),
        ),
        forward_bytes(
            &state.client,
            &method,
            &frappe_url,
            &req_headers,
            &state.args.frappe_site,
            body_bytes,
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
async fn forward_full(
    client: &Client,
    method: &Method,
    url: &str,
    headers: &HeaderMap,
    host_override: &str,
    body: axum::body::Bytes,
) -> Result<Response<Body>, ()> {
    let reqwest_method =
        reqwest::Method::from_bytes(method.as_str().as_bytes()).map_err(|_| ())?;

    let mut builder = client.request(reqwest_method, url);
    for (name, value) in headers {
        let n = name.as_str().to_lowercase();
        if matches!(n.as_str(), "host" | "connection" | "transfer-encoding" | "upgrade") {
            continue;
        }
        if let Ok(v) = reqwest::header::HeaderValue::from_bytes(value.as_bytes()) {
            builder = builder.header(name.as_str(), v);
        }
    }
    builder = builder.header("host", host_override);
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
async fn forward_bytes(
    client: &Client,
    method: &Method,
    url: &str,
    headers: &HeaderMap,
    host_override: &str,
    body: axum::body::Bytes,
) -> Result<(u16, HeaderMap, Vec<u8>), ()> {
    let reqwest_method =
        reqwest::Method::from_bytes(method.as_str().as_bytes()).map_err(|_| ())?;

    let mut builder = client.request(reqwest_method, url);
    for (name, value) in headers {
        let n = name.as_str().to_lowercase();
        if matches!(n.as_str(), "host" | "connection" | "transfer-encoding" | "upgrade") {
            continue;
        }
        if let Ok(v) = reqwest::header::HeaderValue::from_bytes(value.as_bytes()) {
            builder = builder.header(name.as_str(), v);
        }
    }
    builder = builder.header("host", host_override);
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
