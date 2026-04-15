//! `spotledger wire-app` — re-apply pipeline functions and wiring for an app.
//!
//! Applies all surql/ files for the named app in the correct order:
//!   schema → shared fn:: → domain functions → registry → runner →
//!   universal nodes → wire_generic → domain wiring.surql
//!
//! Use this after manually editing a wiring.surql file to re-apply changes
//! without running a full `install-app`.

use std::path::Path;

use anyhow::{Context, Result};
use spotledger_core::config::SiteConfig;
use spotledger_db::connection::connect;
use spotledger_db::DbAdapter;
use spotledger_db::pipeline::apply_pipeline_functions;

use crate::cli::WireAppArgs;

pub async fn wire_app(args: WireAppArgs) -> Result<()> {
    let bench = args.bench.canonicalize().unwrap_or_else(|_| args.bench.clone());

    let db = connect_to_site(&bench, &args.site).await?;

    let app_root = bench.join("apps").join(&args.app).join(&args.app);
    if !app_root.exists() {
        anyhow::bail!("App root not found: {}", app_root.display());
    }

    println!("Wiring app '{}' into site '{}' …", args.app, args.site);

    let fn_count = apply_pipeline_functions(&db, &app_root)
        .await
        .context("Applying pipeline surql files")?;

    println!(
        "✓  Pipeline wired: {} fn:: registered, wiring.surql files applied.",
        fn_count
    );

    Ok(())
}

async fn connect_to_site(bench: &Path, site: &str) -> Result<DbAdapter> {
    let config_path = bench.join("sites").join(site).join("site_config.toml");
    let config_str = tokio::fs::read_to_string(&config_path)
        .await
        .with_context(|| format!("Reading {}", config_path.display()))?;
    let cfg: SiteConfig = toml::from_str(&config_str).context("Parsing site_config.toml")?;
    println!("Connecting to {} (ns={}) …", cfg.database.url, cfg.database.ns);
    let db = connect(&cfg.database).await.context("Connecting to SurrealDB")?;
    println!("Connected.");
    Ok(db)
}
