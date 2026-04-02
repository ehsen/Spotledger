//! Server startup: wires together Axum router, middleware, site loading.

use axum::{
    middleware,
    routing::{get, post},
    Router,
};
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
use crate::methods::{get_logged_user_handler, getdoc_handler, getdoctype_handler, login_handler, logout_handler};
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
            post(get_logged_user_handler),
        )
        // Methods that return top-level JSON (no {message:} wrapper) — Frappe shape
        .route(
            "/api/method/frappe.desk.form.load.getdoctype",
            post(getdoctype_handler),
        )
        .route(
            "/api/method/frappe.desk.form.load.getdoc",
            post(getdoc_handler),
        )
        // Generic method dispatcher
        .route("/api/method/{*path}", post(call_method));

    let app = api_routes
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
                        let site_state = SiteState::new(cfg, db);
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
