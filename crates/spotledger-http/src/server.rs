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
    trace::TraceLayer,
};
use tracing_subscriber::EnvFilter;

use crate::middleware::site_middleware;
use crate::methods::{get_logged_user_handler, getdoc_handler, getdoctype_handler, getpage_handler, login_handler, logout_handler};
use crate::routes::{call_method, ping, resource_get, resource_get_value, resource_list, resource_create, resource_update};
use crate::state::{AppState, SiteState};
use spotledger_db::connection::connect;
use spotledger_db::migrations::{current_batch, run_pending_migrations};
use spotledger_db::schema::ensure_all_schemas;

use spotledger_core::config::SiteConfig;

// ── Configuration types ───────────────────────────────────────────────────────

/// Run mode: determines logging level and bind behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum RunMode {
    Dev,
    Prod,
}

/// Server startup arguments.
#[derive(Debug, clap::Args, Clone)]
pub struct ServeArgs {
    /// Run mode: dev or prod
    #[arg(long, default_value = "dev", env = "SPOTLEDGER_MODE")]
    pub mode: RunMode,

    /// Override bind address (e.g. 0.0.0.0:8000)
    #[arg(long, env = "SPOTLEDGER_BIND")]
    pub bind: Option<String>,

    /// Path to the bench root (default: current directory)
    #[arg(long, env = "SPOTLEDGER_BENCH", default_value = ".")]
    pub bench: std::path::PathBuf,
}

// ── Server entry point ────────────────────────────────────────────────────────

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

    // ── router ───────────────────────────────────────────────────────────────
    let api_routes = Router::new()
        // REST resource API
        .route("/api/resource/{doctype}", get(resource_list).post(resource_create))
        .route("/api/resource/{doctype}/{name}", get(resource_get).put(resource_update))
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

    let app = Router::new()
        .merge(api_routes)
        .layer(middleware::from_fn_with_state(
            app_state.clone(),
            site_middleware,
        ))
        .layer(CompressionLayer::new())
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(app_state);

    // /api/ping is served OUTSIDE the site middleware
    // so healthcheck calls without a Host header still work.
    let app = Router::new()
        .route("/api/ping", get(ping))
        .merge(app);

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

/// Build the Axum router without binding a socket.
///
/// Intended for use in integration tests.  The caller must populate
/// `app_state` with at least one site before calling this function.
pub fn build_app(app_state: AppState) -> axum::Router {
    use axum::routing::{get, post, put};

    let api_routes = axum::Router::new()
        .route("/api/resource/{doctype}", get(resource_list).post(resource_create))
        .route("/api/resource/{doctype}/{name}", get(resource_get).put(resource_update))
        .route(
            "/api/resource/{doctype}/{name}/{fieldname}",
            get(resource_get_value),
        )
        .route("/api/method/login",  post(login_handler))
        .route("/api/method/logout", post(logout_handler))
        .route(
            "/api/method/frappe.auth.get_logged_user",
            get(get_logged_user_handler).post(get_logged_user_handler),
        )
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
        .route("/api/method/{*path}", get(call_method).post(call_method));

    let app = axum::Router::new()
        .merge(api_routes)
        .layer(axum::middleware::from_fn_with_state(
            app_state.clone(),
            site_middleware,
        ))
        .layer(CorsLayer::permissive())
        .with_state(app_state);

    axum::Router::new()
        .route("/api/ping", get(ping))
        .merge(app)
}

/// Register a site directly into an [`AppState`] from a config + adapter.
///
/// Convenience for integration tests that construct state without touching disk.
pub async fn register_site_from_config(
    app_state: &AppState,
    cfg: spotledger_core::config::SiteConfig,
) -> anyhow::Result<()> {
    let hostname = cfg.site.name.clone();
    let db = connect(&cfg.database).await?;
    let site_state = SiteState::new(cfg, db);
    app_state.register(hostname, site_state);
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
                        // Auto-sync: apply any new DEFINE TABLE / DEFINE FIELD statements
                        // from compiled inventory before accepting requests.  All DDL uses
                        // IF NOT EXISTS so this is safe and fast on an already-current site.
                        if let Err(e) = ensure_all_schemas(&db).await {
                            tracing::error!(site = %hostname, error = %e, "Schema sync failed at startup");
                        } else {
                            tracing::info!(site = %hostname, "Schema sync complete");
                        }
                        match run_pending_migrations(&db, current_batch()).await {
                            Ok(0)  => tracing::debug!(site = %hostname, "No pending migrations"),
                            Ok(n)  => tracing::info!(site = %hostname, count = n, "Migrations applied"),
                            Err(e) => tracing::error!(site = %hostname, error = %e, "Migration failed at startup"),
                        }
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
