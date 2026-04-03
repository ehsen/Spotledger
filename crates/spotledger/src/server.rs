//! Server startup: wires together Axum router, middleware, site loading.

use axum::{
    extract::Query,
    http::{Method, StatusCode},
    middleware,
    response::IntoResponse,
    routing::{any, get, post},
    Router,
};
use std::collections::HashMap;
use std::path::Path;
use tower_http::{
    compression::CompressionLayer,
    cors::CorsLayer,
    services::ServeDir,
    trace::TraceLayer,
};
use tracing_subscriber::EnvFilter;

use crate::cli::{RunMode, ServeArgs};
use crate::middleware::site_middleware;
use crate::methods::{get_logged_user_handler, getdoc_handler, getdoctype_handler, getpage_handler, login_handler, logout_handler};
use crate::pages::{app_wildcard, desk_page, login_page, root_handler};
use crate::routes::{call_method, ping, resource_get, resource_get_value, resource_list};
use crate::state::{AppState, SiteState};
use spotledger_db::connection::connect;
use spotledger_types::config::SiteConfig;

pub async fn serve(args: ServeArgs) -> anyhow::Result<()> {
    // ── logging ──────────────────────────────────────────────────────────────
    let log_level = match args.mode {
        RunMode::Dev => "debug",
        RunMode::Prod => "warn",
    };
    let log_format = match args.mode {
        RunMode::Dev => "pretty",
        RunMode::Prod => "json",
    };
    init_logging(log_level, log_format);

    // ── load sites ───────────────────────────────────────────────────────────
    let app_state = AppState::new();
    let sites_dir = args.bench.join("sites");

    load_sites(&app_state, &sites_dir).await?;

    if app_state.sites.is_empty() {
        tracing::warn!("No sites loaded — check sites/ directory for site_config.toml files");
    }

    // ── static assets: sites/assets/ → /assets/** ────────────────────────────
    let assets_dir = sites_dir.join("assets");

    // ── router ───────────────────────────────────────────────────────────────
    let api_routes = Router::new()
        .route("/api/ping", get(ping))
        // Page routes
        .route("/",          get(root_handler))
        .route("/desk",      get(desk_page))
        .route("/login",     get(login_page))
        .route("/app/{*path}", get(app_wildcard))
        // REST resource API
        .route("/api/resource/{doctype}", get(resource_list))
        .route("/api/resource/{doctype}/{name}", get(resource_get))
        .route(
            "/api/resource/{doctype}/{name}/{fieldname}",
            get(resource_get_value),
        )
        // Auth methods (dedicated routes — must precede catch-all)
        .route("/api/method/login",  post(login_handler))
        .route("/api/method/logout", post(logout_handler))
        .route(
            "/api/method/frappe.auth.get_logged_user",
            get(get_logged_user_handler).post(get_logged_user_handler),
        )
        // Methods that return top-level JSON (no {message:} wrapper) — Frappe shape
        // Registered for both GET and POST: Frappe JS uses GET with query params
        .route(
            "/api/method/frappe.desk.form.load.getdoctype",
            get(getdoctype_handler).post(getdoctype_handler),
        )
        .route(
            "/api/method/frappe.desk.form.load.getdoc",
            get(getdoc_handler).post(getdoc_handler),
        )
        .route(
            "/api/method/frappe.desk.desk_page.getpage",
            get(getpage_handler).post(getpage_handler),
        )
        // Generic method dispatcher — GET and POST both supported
        .route("/api/method/{*path}", get(call_method).post(call_method));

    // Socket.io stub — Frappe Desk JS requires socket.io for realtime features.
    // We implement the Engine.io v4 polling handshake so the client connects
    // successfully (no 404 errors) and then silently receives NOOP packets.
    // Real realtime (WebSocket pushes) is Phase 3 work.
    let app = Router::new()
        .route("/socket.io/", any(socketio_handler))
        .route("/socket.io", any(socketio_handler))
        .merge(api_routes)
        .layer(middleware::from_fn_with_state(
            app_state.clone(),
            site_middleware,
        ))
        .layer(CompressionLayer::new())
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(app_state);

    // Nest static file serving OUTSIDE the layered stack (no site middleware needed)
    let app = if assets_dir.exists() {
        let serve_assets = ServeDir::new(&assets_dir);
        Router::new()
            .nest_service("/assets", serve_assets)
            .merge(app)
    } else {
        tracing::warn!(path = ?assets_dir, "assets directory not found; /assets/* will 404");
        app
    };

    // ── bind ─────────────────────────────────────────────────────────────────
    let bind_addr = args
        .bind
        .unwrap_or_else(|| match args.mode {
            RunMode::Dev => "127.0.0.1:8000".into(),
            RunMode::Prod => "0.0.0.0:8000".into(),
        });

    let listener = tokio::net::TcpListener::bind(&bind_addr).await?;
    tracing::info!(addr = %bind_addr, mode = ?args.mode, "Spotledger listening");

    axum::serve(listener, app).await?;

    Ok(())
}

async fn load_sites(state: &AppState, sites_dir: &Path) -> anyhow::Result<()> {
    if !sites_dir.exists() {
        return Ok(());
    }

    // Load assets.json once — shared by all sites (assets are bench-global).
    let assets_json = load_assets_json(sites_dir).await;

    let mut read_dir = tokio::fs::read_dir(sites_dir).await?;
    while let Some(entry) = read_dir.next_entry().await? {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let cfg_path = path.join("site_config.toml");
        if !cfg_path.exists() {
            continue;
        }

        match SiteConfig::from_file(&cfg_path) {
            Ok(cfg) => {
                let hostname = cfg.site.name.clone();
                tracing::info!(site = %hostname, "Loading site");
                match connect(&cfg.database).await {
                    Ok(db) => {
                        let site_state = SiteState::new(cfg, db, assets_json.clone());
                        state.register(hostname.clone(), site_state);
                        tracing::info!(site = %hostname, "Site ready");
                    }
                    Err(e) => {
                        tracing::error!(site = %hostname, error = %e, "Failed to connect to SurrealDB");
                    }
                }
            }
            Err(e) => {
                tracing::warn!(path = ?cfg_path, error = %e, "Skipping site");
            }
        }
    }

    Ok(())
}

/// Read and parse `sites/assets/assets.json`.
/// On any error (missing file, parse failure) returns an empty object so the
/// server still starts; bundle 404s will appear in the browser console but
/// nothing will crash.
async fn load_assets_json(sites_dir: &Path) -> serde_json::Value {
    let path = sites_dir.join("assets").join("assets.json");
    let contents = match tokio::fs::read_to_string(&path).await {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!(path = ?path, error = %e, "assets.json not found — bundle URLs will be empty");
            return serde_json::Value::Object(Default::default());
        }
    };
    match serde_json::from_str::<serde_json::Value>(&contents) {
        Ok(v) => {
            tracing::info!(path = ?path, "Loaded assets.json");
            v
        }
        Err(e) => {
            tracing::warn!(path = ?path, error = %e, "Failed to parse assets.json");
            serde_json::Value::Object(Default::default())
        }
    }
}

fn init_logging(level: &str, format: &str) {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(level));

    if format == "json" {
        tracing_subscriber::fmt()
            .json()
            .with_env_filter(filter)
            .init();
    } else {
        tracing_subscriber::fmt()
            .pretty()
            .with_env_filter(filter)
            .init();
    }
}

/// Minimal Engine.io v4 / socket.io polling stub.
///
/// The Frappe Desk JS connects via socket.io for realtime features (live
/// notifications, form collaboration). Until we implement a real WebSocket
/// push server (Phase 3) this stub:
///   • Completes the polling handshake so the client doesn't flood logs with 404s.
///   • Returns NOOP packets on subsequent polls so the client stays idle.
///   • POST requests (outbound events from the browser) return 200 silently.
async fn socketio_handler(
    method: Method,
    Query(params): Query<HashMap<String, String>>,
) -> impl IntoResponse {
    let has_sid = params.contains_key("sid");

    if method == Method::POST {
        // Browser sending events — silently accept, return "ok" per EIO v4 spec.
        return (
            StatusCode::OK,
            [("Content-Type", "text/plain; charset=UTF-8"),
             ("Cache-Control", "no-cache, no-store"),
             ("Access-Control-Allow-Origin", "*")],
            "ok",
        ).into_response();
    }

    if !has_sid {
        // Initial handshake — return OPEN packet with a static session ID so
        // the client believes it has connected.  Then immediately append
        // socket.io CONNECT (40) to namespace "/" so the socket.io layer on
        // top considers the connection established without waiting for a server
        // push.
        let body = concat!(
            r#"0{"sid":"spotledger-realtime","upgrades":[],"pingInterval":25000,"pingTimeout":20000,"maxPayload":1000000}"#,
            "\x1e",  // EIO4 packet separator
            "40",    // socket.io CONNECT to namespace /
        );
        return (
            StatusCode::OK,
            [("Content-Type", "text/plain; charset=UTF-8"),
             ("Cache-Control", "no-cache, no-store"),
             ("Access-Control-Allow-Origin", "*")],
            body,
        ).into_response();
    }

    // Subsequent polls — return NOOP (6) so the client keeps long-polling
    // without triggering any error callbacks.
    (
        StatusCode::OK,
        [("Content-Type", "text/plain; charset=UTF-8"),
         ("Cache-Control", "no-cache, no-store"),
         ("Access-Control-Allow-Origin", "*")],
        "6",
    ).into_response()
}
