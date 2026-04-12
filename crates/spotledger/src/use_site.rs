//! `spotledger use <sitename>` — set the default site for this bench.
//!
//! Writes the site name as plain text to `<bench>/sites/currentsite`,
//! following the Frappe bench convention exactly.

use anyhow::{bail, Context};

use crate::cli::UseSiteArgs;

pub async fn use_site(args: UseSiteArgs) -> anyhow::Result<()> {
    let bench = args
        .bench
        .canonicalize()
        .unwrap_or_else(|_| args.bench.clone());

    let sites_dir = bench.join("sites");
    let site_dir = sites_dir.join(&args.sitename);

    // Verify the site actually exists before writing currentsite.
    let config_path = site_dir.join("site_config.toml");
    if !config_path.exists() {
        bail!(
            "Site '{}' not found — expected config at {}",
            args.sitename,
            config_path.display()
        );
    }

    let currentsite_path = sites_dir.join("currentsite");
    tokio::fs::write(&currentsite_path, &args.sitename)
        .await
        .with_context(|| {
            format!(
                "Failed to write default site to {}",
                currentsite_path.display()
            )
        })?;

    println!("✓  Default site set to '{}'", args.sitename);
    println!(
        "   (written to {})",
        currentsite_path.display()
    );

    Ok(())
}
