use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// Spotledger — Frappe-compatible framework backed by SurrealDB
#[derive(Debug, Parser)]
#[command(name = "spotledger", version, about)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Start the HTTP server
    Serve(ServeArgs),
    /// Create a new site: write config + bootstrap SurrealDB schema
    NewSite(NewSiteArgs),
}

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
    pub bench: PathBuf,
}

#[derive(Debug, clap::Args, Clone)]
pub struct NewSiteArgs {
    /// Hostname for the new site (e.g. mysite.localhost)
    pub hostname: String,

    /// SurrealDB WebSocket URL
    #[arg(long, default_value = "ws://127.0.0.1:8001", env = "SPOTLEDGER_DB_URL")]
    pub db_url: String,

    /// SurrealDB namespace (defaults to hostname)
    #[arg(long, env = "SPOTLEDGER_DB_NS")]
    pub db_ns: Option<String>,

    /// SurrealDB username
    #[arg(long, default_value = "root", env = "SPOTLEDGER_DB_USER")]
    pub db_user: String,

    /// SurrealDB password
    #[arg(long, default_value = "root", env = "SPOTLEDGER_DB_PASS")]
    pub db_pass: String,

    /// Path to the bench root (default: current directory)
    #[arg(long, env = "SPOTLEDGER_BENCH", default_value = ".")]
    pub bench: PathBuf,

    /// Administrator password set during site creation (required to log in)
    #[arg(long, env = "SPOTLEDGER_ADMIN_PASSWORD")]
    pub admin_password: String,
}

#[derive(Debug, Clone, clap::ValueEnum)]
pub enum RunMode {
    Dev,
    Prod,
}
