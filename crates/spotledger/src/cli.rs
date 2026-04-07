use clap::{Parser, Subcommand};
use std::path::PathBuf;

// Re-export server types from spotledger-http
pub use spotledger_http::server::{RunMode, ServeArgs};

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
    /// Apply pending DDL schema sync and data migrations to an existing site
    Migrate(MigrateArgs),
    /// Seed all Frappe DocType definitions into an existing site's SurrealDB
    SeedDoctypes(SeedDoctypesArgs),
    /// Install a Frappe app: DocTypes + Module Defs + fixture records + Patch Log
    InstallApp(InstallAppArgs),
    /// Write full generated SurrealQL to generated/schema.surql (no DB required)
    Emit(EmitArgs),
    /// Remove orphaned tabDocField/tabDocPerm rows for deleted DocTypes
    Cleanup(CleanupArgs),
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

/// Arguments for the `migrate` subcommand.
#[derive(Debug, clap::Args, Clone)]
pub struct MigrateArgs {
    /// Hostname of the site to migrate (must already exist via new-site)
    pub site: String,

    /// Path to the bench root (default: current directory)
    #[arg(long, env = "SPOTLEDGER_BENCH", default_value = ".")]
    pub bench: PathBuf,

    /// Only print which migrations would run; do not apply any changes
    #[arg(long, default_value_t = false)]
    pub dry_run: bool,
}

#[derive(Debug, clap::Args, Clone)]
pub struct SeedDoctypesArgs {
    /// Hostname of the site to seed (must already exist via new-site)
    pub site: String,

    /// Path to the bench root (default: current directory)
    #[arg(long, env = "SPOTLEDGER_BENCH", default_value = ".")]
    pub bench: PathBuf,
}

#[derive(Debug, clap::Args, Clone)]
pub struct InstallAppArgs {
    /// Name of the app to install (e.g. frappe, erpnext)
    pub app: String,

    /// Hostname of the site to install into (must already exist via new-site)
    pub site: String,

    /// Path to the bench root (default: current directory)
    #[arg(long, env = "SPOTLEDGER_BENCH", default_value = ".")]
    pub bench: PathBuf,
}

#[derive(Debug, clap::Args, Clone)]
pub struct EmitArgs {
    /// Path to the bench root (default: current directory)
    #[arg(long, env = "SPOTLEDGER_BENCH", default_value = ".")]
    pub bench: PathBuf,
}

#[derive(Debug, clap::Args, Clone)]
pub struct CleanupArgs {
    /// Hostname of the site to clean up
    pub site: String,

    /// Path to the bench root (default: current directory)
    #[arg(long, env = "SPOTLEDGER_BENCH", default_value = ".")]
    pub bench: PathBuf,

    /// Actually delete orphaned rows (default: dry run, only prints what would be removed)
    #[arg(long, default_value_t = false)]
    pub apply: bool,
}
