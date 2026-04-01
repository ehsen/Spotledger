//! `spotledger new-site` — creates site directory, writes config, bootstraps SurrealDB schema.

use std::path::PathBuf;

use anyhow::{bail, Context};
use spotledger_db::connection::connect;
use spotledger_db::auth::set_user_password;
use spotledger_types::config::{AppsConfig, CacheConfig, DatabaseConfig, SiteConfig, SiteInfo};

use crate::cli::NewSiteArgs;

pub async fn new_site(args: NewSiteArgs) -> anyhow::Result<()> {
    let bench = args.bench.canonicalize()
        .unwrap_or_else(|_| args.bench.clone());

    // ── 1. Resolve paths ──────────────────────────────────────────────────────
    let site_dir = bench.join("sites").join(&args.hostname);
    let config_path = site_dir.join("site_config.toml");
    let surql_path = bench
        .join("apps")
        .join("frappe")
        .join("frappe")
        .join("database")
        .join("surrealdb")
        .join("framework_surrealdb.surql");

    if site_dir.exists() {
        bail!("Site directory already exists: {}", site_dir.display());
    }

    // ── 2. Create site directory ─────────────────────────────────────────────
    tokio::fs::create_dir_all(&site_dir)
        .await
        .with_context(|| format!("Creating site dir: {}", site_dir.display()))?;

    println!("Created {}", site_dir.display());

    // ── 3. Write site_config.toml ────────────────────────────────────────────
    let db_ns = args.db_ns.clone().unwrap_or_else(|| args.hostname.clone());
    let cfg = SiteConfig {
        site: SiteInfo {
            name: args.hostname.clone(),
            namespace: db_ns.clone(),
        },
        database: DatabaseConfig {
            url: args.db_url.clone(),
            ns: db_ns.clone(),
            db: db_ns.clone(),
            user: args.db_user.clone(),
            pass: args.db_pass.clone(),
        },
        cache: CacheConfig::default(),
        apps: AppsConfig::default(),
    };

    let toml_str = toml::to_string_pretty(&cfg)
        .context("Serialising site_config.toml")?;

    tokio::fs::write(&config_path, &toml_str)
        .await
        .with_context(|| format!("Writing {}", config_path.display()))?;

    println!("Wrote {}", config_path.display());

    // ── 4. Read bootstrap schema ─────────────────────────────────────────────
    if !surql_path.exists() {
        bail!(
            "Bootstrap schema not found: {}\n\
             Ensure the frappe app is installed at apps/frappe/",
            surql_path.display()
        );
    }

    let surql = tokio::fs::read_to_string(&surql_path)
        .await
        .with_context(|| format!("Reading {}", surql_path.display()))?;

    println!("Read {} bytes from {}", surql.len(), surql_path.display());

    // ── 5. Connect to SurrealDB and run schema ───────────────────────────────
    println!("Connecting to {} (ns={}, db={}) …", args.db_url, db_ns, db_ns);

    let db = connect(&cfg.database)
        .await
        .context("Connecting to SurrealDB")?;

    println!("Connected. Running bootstrap schema …");

    db.query(&surql)
        .await
        .context("Executing framework_surrealdb.surql")?;

    // ── 6. Seed Administrator password ───────────────────────────────────────
    println!("Seeding Administrator password …");

    set_user_password(&db, "Administrator", &args.admin_password)
        .await
        .context("Seeding Administrator password in __Auth")?;

    println!("Site '{}' created successfully.", args.hostname);
    println!(
        "\nBootstrapped tables (from framework_surrealdb.surql):\n\
         \t tabDefaultValue, tabSingles, tabSessions, __UserSettings,\n\
         \t tabSeries, __Auth, tabFile, tabDeleted_Document, __global_search,\n\
         \t tabToDo, tabUser (seeded), tabDocType, tabDocField, tabCustom_Field,\n\
         \t tabModule_Def, tabSystem_Settings, tabHas_Role, tabRole (seeded), tabDocPerm\n"
    );
    println!("Next: spotledger serve --bench {}", bench.display());

    Ok(())
}

/// Compute a default site directory for testing given a bench root.
pub fn site_dir(bench: &PathBuf, hostname: &str) -> PathBuf {
    bench.join("sites").join(hostname)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn site_dir_is_under_bench_sites() {
        let bench = PathBuf::from("/tmp/bench");
        let dir = site_dir(&bench, "mysite.localhost");
        assert_eq!(dir, PathBuf::from("/tmp/bench/sites/mysite.localhost"));
    }
}
