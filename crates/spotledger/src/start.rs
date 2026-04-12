//! `spotledger start` — start all backend components.
//!
//! Reads the default site from `<bench>/sites/currentsite` (written by
//! `spotledger use <sitename>`) and starts the HTTP server.  Equivalent to
//! `spotledger serve --bench .` but prints a friendlier startup message and
//! does not require repeating the bench path on every invocation.
//!
//! The database (SurrealDB) is a separate process not managed by this command.

use anyhow::Context;
use spotledger_http::server::{RunMode, ServeArgs};

use crate::cli::StartArgs;

pub async fn start(args: StartArgs) -> anyhow::Result<()> {
    let bench = args
        .bench
        .canonicalize()
        .unwrap_or_else(|_| args.bench.clone());

    // Read the default site from sites/currentsite if present.
    let currentsite_path = bench.join("sites").join("currentsite");
    if currentsite_path.exists() {
        let site = tokio::fs::read_to_string(&currentsite_path)
            .await
            .with_context(|| format!("Failed to read {}", currentsite_path.display()))?;
        let site = site.trim();
        if !site.is_empty() {
            println!("Using default site: {site}");
        }
    } else {
        println!(
            "Note: no default site set — run `spotledger use <sitename>` to set one."
        );
        println!("      Loading all sites found in {}", bench.join("sites").display());
    }

    let serve_args = ServeArgs {
        mode: args.mode.unwrap_or(RunMode::Dev),
        bind: args.bind,
        bench,
    };

    spotledger_http::server::serve(serve_args).await
}
