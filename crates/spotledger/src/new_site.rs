//! `spotledger new-site` — creates a site directory, writes config, and
//! bootstraps SurrealDB entirely from compiled Rust code.
//!
//! ## Bootstrap order
//!
//! 1. Create `sites/<hostname>/` directory on disk.
//! 2. Write `site_config.toml`.
//! 3. Connect to SurrealDB.
//! 4. **Framework tables** — `__Auth`, `tabSessions`, `tabSeries`, `tabSingles`.
//!    These are internal tables with no `DocTypeMeta`; they only exist here.
//! 5. **DocType schemas** — every DocType registered via `inventory::submit!`
//!    is materialised as a `tab<Name>` table with `DEFINE TABLE / DEFINE FIELD`.
//!    Adding a new field in Rust re-runs this on the next `new-site` or migrate
//!    and the `IF NOT EXISTS` guards make it fully additive.
//! 6. **Seed records** — default Roles, UserTypes, and the Administrator user.
//! 7. **Administrator password** written to `__Auth`.
//!
//! No external `.surql` files are read. The binary is the sole source of truth.

use anyhow::{bail, Context};
use spotledger_db::bootstrap::{run_framework_tables, seed_default_records, seed_framework_doctypes, seed_naming_rules};
use spotledger_db::migrations::{current_batch, run_pending_migrations};
use spotledger_db::auth::set_user_password;
use spotledger_db::connection::connect;
use spotledger_db::schema::ensure_all_schemas;
use spotledger_core::config::{AppsConfig, CacheConfig, DatabaseConfig, SiteConfig, SiteInfo};

use crate::cli::NewSiteArgs;
use crate::install_app::install_app_into_db;

pub async fn new_site(args: NewSiteArgs) -> anyhow::Result<()> {
    let bench = args.bench.canonicalize()
        .unwrap_or_else(|_| args.bench.clone());

    // ── 1. Guard: site must not already exist ─────────────────────────────────
    let site_dir    = bench.join("sites").join(&args.hostname);
    let config_path = site_dir.join("site_config.toml");

    if site_dir.exists() {
        bail!("Site directory already exists: {}", site_dir.display());
    }

    // ── 2. Create site directory ──────────────────────────────────────────────
    tokio::fs::create_dir_all(&site_dir)
        .await
        .with_context(|| format!("Creating {}", site_dir.display()))?;

    println!("[1/6] Created {}", site_dir.display());

    // ── 3. Write site_config.toml ─────────────────────────────────────────────
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

    tokio::fs::write(&config_path, toml_str)
        .await
        .with_context(|| format!("Writing {}", config_path.display()))?;

    println!("[2/6] Wrote {}", config_path.display());

    // ── 4. Connect to SurrealDB ───────────────────────────────────────────────
    println!("[3/6] Connecting to {} (ns={}, db={}) …", args.db_url, db_ns, db_ns);

    let db = connect(&cfg.database)
        .await
        .context("Connecting to SurrealDB")?;

    println!("      Connected.");

    // ── 5. Framework-internal tables ─────────────────────────────────────────
    //  __Auth · tabSessions · tabSeries · tabSingles
    //  These are not DocTypes; they live only in bootstrap.rs.
    println!("[4/6] Creating framework tables (__Auth, tabSessions, tabSeries, tabSingles) …");

    run_framework_tables(&db)
        .await
        .context("Creating framework-internal tables")?;

    // ── 6. DocType schemas from inventory ────────────────────────────────────
    //  Every `inventory::submit!(MetaEntry { … })` in any linked crate is
    //  iterated here and emits:
    //    DEFINE TABLE IF NOT EXISTS `tab<Name>` SCHEMAFULL;
    //    DEFINE FIELD IF NOT EXISTS `<field>` ON TABLE `tab<Name>` TYPE …;
    //  Adding a new DocType or field in Rust re-runs this and the IF NOT EXISTS
    //  guards make it additive-only — no manual migration files needed.
    println!("[5/6] Syncing DocType schemas from compiled inventory …");

    ensure_all_schemas(&db)
        .await
        .context("Syncing DocType schemas")?;

    // ── 7. Seed default records + naming rules + Administrator password ────────
    println!("[6/6] Seeding default roles, user types, Administrator user …");

    seed_default_records(&db)
        .await
        .context("Seeding default records")?;

    seed_naming_rules(&db)
        .await
        .context("Seeding DocumentNamingRule rows")?;

    seed_framework_doctypes(&db)
        .await
        .context("Seeding tabDocType rows for framework types")?;

    run_pending_migrations(&db, current_batch())
        .await
        .context("Running data migrations")?;

    set_user_password(&db, "Administrator", &args.admin_password)
        .await
        .context("Setting Administrator password")?;

    // ── Auto-install spotledger-core if present ───────────────────────────────
    let core_app_dir = bench.join("apps").join("spotledger-core");
    if core_app_dir.exists() {
        println!("\nAuto-installing spotledger-core …");
        match install_app_into_db(&db, &bench, "spotledger-core", None).await {
            Ok(()) => println!("✓  spotledger-core installed."),
            Err(e) => eprintln!("WARN: spotledger-core auto-install failed (non-fatal): {:?}", e),
        }
    }

    // ── Done ─────────────────────────────────────────────────────────────────
    println!();
    println!("Site '{}' created successfully.", args.hostname);
    println!("  DB namespace : {db_ns}");
    println!("  Config       : {}", config_path.display());
    println!();
    println!("Next: spotledger serve --bench {}", bench.display());

    Ok(())
}
