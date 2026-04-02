//! Spotledger proxy recorder.
//!
//! Forwards every HTTP request to **both** Spotledger and the real Frappe
//! instance simultaneously, returns the Spotledger response to the caller,
//! and logs a JSON diff of any response body differences to a file.
//!
//! Usage:
//!   spotledger-proxy \
//!     --spotledger-url http://127.0.0.1:8000 \
//!     --frappe-url     http://127.0.0.1:8080 \
//!     --port           9000 \
//!     --log-file       /tmp/proxy-diffs.jsonl

use anyhow::Result;
use axum::{
    body::Body,
    extract::State,
    http::{Method, Request, StatusCode},
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
#[command(name = "spotledger-proxy", about = "Side-by-side proxy recorder")]
struct Args {
    /// Spotledger server URL (primary — its response is returned to caller)
    #[arg(long, default_value = "http://127.0.0.1:8000")]
    spotledger_url: String,

    /// Real Frappe server URL (shadow — used only for diffing)
    #[arg(long, default_value = "http://127.0.0.1:8080")]
    frappe_url: String,

    /// Port to listen on
    #[arg(long, short, default_value = "9000")]
    port: u16,

    /// Path to the JSONL diff log file
    #[arg(long, default_value = "/tmp/spotledger-proxy-diffs.jsonl")]
    log_file: PathBuf,

    /// Only log when response bodies differ (skip identical responses)
    #[arg(long, default_value = "true")]
    diffs_only: bool,
}

// ── shared state ─────────────────────────────────────────────────────────────

#[derive(Clone)]
struct ProxyState {
    args: Arc<Args>,
    client: Client,
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
    tracing::info!(spotledger = %args.spotledger_url, frappe = %args.frappe_url, "Targets");

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
    let headers = req.headers().clone();

    // Consume body once; we'll reuse the bytes for both requests.
    let body_bytes = req
        .into_body()
        .collect()
        .await
        .map_err(|_| StatusCode::BAD_REQUEST)?
        .to_bytes();

    // ── send to both backends in parallel ────────────────────────────────────
    let spot_url = format!("{}{}", state.args.spotledger_url, path_and_query);
    let frappe_url = format!("{}{}", state.args.frappe_url, path_and_query);

    let (spot_result, frappe_result) = tokio::join!(
        forward(
            &state.client,
            &method,
            &spot_url,
            &headers,
            body_bytes.clone(),
        ),
        forward(
            &state.client,
            &method,
            &frappe_url,
            &headers,
            body_bytes,
        ),
    );

    // ── record diff ──────────────────────────────────────────────────────────
    let (spot_status, spot_body) = spot_result.unwrap_or((500, b"{}".to_vec()));
    let (frappe_status, frappe_body) = frappe_result.unwrap_or((500, b"{}".to_vec()));

    record_diff(
        &state.args,
        &path_and_query,
        method.as_str(),
        spot_status,
        &spot_body,
        frappe_status,
        &frappe_body,
    );

    // Return primary (Spotledger) response to caller
    let response = Response::builder()
        .status(spot_status)
        .header("content-type", "application/json")
        .body(Body::from(spot_body))
        .unwrap_or_else(|_| Response::new(Body::empty()));

    Ok(response)
}

// ── HTTP forwarding ───────────────────────────────────────────────────────────

async fn forward(
    client: &Client,
    method: &Method,
    url: &str,
    headers: &axum::http::HeaderMap,
    body: axum::body::Bytes,
) -> Result<(u16, Vec<u8>), ()> {
    let reqwest_method =
        reqwest::Method::from_bytes(method.as_str().as_bytes()).map_err(|_| ())?;

    let mut builder = client.request(reqwest_method, url);
    for (name, value) in headers {
        // Skip hop-by-hop headers
        let name_lower = name.as_str().to_lowercase();
        if matches!(
            name_lower.as_str(),
            "host" | "connection" | "transfer-encoding" | "upgrade"
        ) {
            continue;
        }
        if let Ok(v) = reqwest::header::HeaderValue::from_bytes(value.as_bytes()) {
            builder = builder.header(name.as_str(), v);
        }
    }
    builder = builder.body(body.to_vec());

    let resp = builder.send().await.map_err(|_| ())?;
    let status = resp.status().as_u16();
    let body_bytes = resp.bytes().await.map_err(|_| ())?.to_vec();

    Ok((status, body_bytes))
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
            port: 9000,
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
