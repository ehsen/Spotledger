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
    /// Generate Rust DocType source from a Frappe-compatible JSON file
    Generate(GenerateArgs),
    /// Set the default site for this bench (writes sites/currentsite)
    Use(UseSiteArgs),
    /// Start all backend components (reads default site from sites/currentsite)
    Start(StartArgs),
    /// Apply pipeline wiring.surql files to a site without a full reinstall
    WireApp(WireAppArgs),
    /// Scaffold a new DB-native app directory under apps/
    NewApp(NewAppArgs),
    /// Add a module directory to an existing DB-native app
    NewModule(NewModuleArgs),
    /// Scaffold a new DocType JSON file in a DB-native app
    NewDoctype(NewDoctypeArgs),
    /// Export installed DocTypes for an app back to filesystem JSON
    ExportApp(ExportAppArgs),
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

    /// App version string (recorded in the schema_change_log and app graph node)
    #[arg(long)]
    pub version: Option<String>,

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

#[derive(Debug, clap::Args, Clone)]
pub struct GenerateArgs {
    /// Path to the Frappe-compatible DocType JSON file
    #[arg(long, short)]
    pub from: PathBuf,

    /// Output file path for the generated Rust source (default: print to stdout)
    #[arg(long, short)]
    pub out: Option<PathBuf>,
}

/// Arguments for the `use` subcommand.
#[derive(Debug, clap::Args, Clone)]
pub struct UseSiteArgs {
    /// Name of the site to set as default (e.g. hello_graph)
    pub sitename: String,

    /// Path to the bench root (default: current directory)
    #[arg(long, env = "SPOTLEDGER_BENCH", default_value = ".")]
    pub bench: PathBuf,
}

/// Arguments for the `start` subcommand.
#[derive(Debug, clap::Args, Clone)]
pub struct StartArgs {
    /// Run mode: dev or prod
    #[arg(long, env = "SPOTLEDGER_MODE")]
    pub mode: Option<RunMode>,

    /// Override bind address (e.g. 0.0.0.0:8000)
    #[arg(long, env = "SPOTLEDGER_BIND")]
    pub bind: Option<String>,

    /// Path to the bench root (default: current directory)
    #[arg(long, env = "SPOTLEDGER_BENCH", default_value = ".")]
    pub bench: PathBuf,
}

#[derive(Debug, clap::Args, Clone)]
pub struct WireAppArgs {
    /// Name of the app whose wiring.surql files to apply (e.g. erpnext)
    pub app: String,

    /// Hostname of the site to wire into (must already exist via new-site)
    pub site: String,

    /// Path to the bench root (default: current directory)
    #[arg(long, env = "SPOTLEDGER_BENCH", default_value = ".")]
    pub bench: PathBuf,
}

#[derive(Debug, clap::Args, Clone)]
pub struct NewAppArgs {
    /// Name of the new app (e.g. my-app, erpnext-extensions)
    pub app: String,

    /// Human-readable title for the app
    #[arg(long)]
    pub title: Option<String>,

    /// App version string
    #[arg(long, default_value = "0.1.0")]
    pub version: String,

    /// Comma-separated list of initial module names
    #[arg(long)]
    pub modules: Option<String>,

    /// Path to the bench root (default: current directory)
    #[arg(long, env = "SPOTLEDGER_BENCH", default_value = ".")]
    pub bench: PathBuf,
}

#[derive(Debug, clap::Args, Clone)]
pub struct NewModuleArgs {
    /// Name of the app to add a module to
    pub app: String,

    /// Name of the new module (e.g. Accounting, HR)
    pub module: String,

    /// Path to the bench root (default: current directory)
    #[arg(long, env = "SPOTLEDGER_BENCH", default_value = ".")]
    pub bench: PathBuf,
}

#[derive(Debug, clap::Args, Clone)]
pub struct NewDoctypeArgs {
    /// Name of the app to add the DocType to
    pub app: String,

    /// Module inside the app (e.g. Contacts)
    pub module: String,

    /// DocType name (e.g. "Contact", "Journal Entry")
    pub name: String,

    /// Path to the bench root (default: current directory)
    #[arg(long, env = "SPOTLEDGER_BENCH", default_value = ".")]
    pub bench: PathBuf,
}

#[derive(Debug, clap::Args, Clone)]
pub struct ExportAppArgs {
    /// Name of the app to export
    pub app: String,

    /// Hostname of the site to export from
    pub site: String,

    /// Path to the bench root (default: current directory)
    #[arg(long, env = "SPOTLEDGER_BENCH", default_value = ".")]
    pub bench: PathBuf,
}
