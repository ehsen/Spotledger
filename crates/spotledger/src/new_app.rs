//! Scaffolding commands for DB-native apps.
//!
//! - `new-app`     — create `apps/{app}/app.json` and inner directory structure
//! - `new-module`  — add a module subdirectory to an existing app
//! - `new-doctype` — scaffold a minimal DocType JSON file
//! - `export-app`  — query SurrealDB and write DocType JSONs back to disk

use anyhow::{bail, Context, Result};
use serde_json::json;
use std::path::{Path, PathBuf};

use crate::cli::{ExportAppArgs, NewAppArgs, NewDoctypeArgs, NewModuleArgs};

// ── new-app ───────────────────────────────────────────────────────────────────

pub async fn new_app(args: NewAppArgs) -> Result<()> {
    let bench = args.bench.canonicalize().unwrap_or(args.bench.clone());
    let app_dir = bench.join("apps").join(&args.app);

    if app_dir.exists() {
        bail!("App directory already exists: {}", app_dir.display());
    }

    let title = args.title.clone().unwrap_or_else(|| {
        // "my-cool-app" → "My Cool App"
        args.app
            .split('-')
            .map(|w| {
                let mut chars = w.chars();
                match chars.next() {
                    None => String::new(),
                    Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
    });

    let modules: Vec<String> = args
        .modules
        .as_deref()
        .unwrap_or("")
        .split(',')
        .map(|s| s.trim().to_owned())
        .filter(|s| !s.is_empty())
        .collect();

    let app_root = app_dir.join(&args.app);
    tokio::fs::create_dir_all(&app_root)
        .await
        .with_context(|| format!("Creating {}", app_root.display()))?;

    // Write app.json in the outer apps/{app}/ directory
    let manifest = json!({
        "name":         args.app,
        "title":        title,
        "version":      args.version,
        "modules":      modules,
        "auto_install": false,
        "depends_on":   [],
    });
    let manifest_path = app_dir.join("app.json");
    tokio::fs::write(&manifest_path, serde_json::to_string_pretty(&manifest)?)
        .await
        .with_context(|| format!("Writing {}", manifest_path.display()))?;

    // Write modules.txt in the inner app root
    let modules_txt = app_root.join("modules.txt");
    let module_lines = manifest["modules"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap_or(""))
        .collect::<Vec<_>>()
        .join("\n");
    tokio::fs::write(&modules_txt, module_lines.clone())
        .await
        .with_context(|| format!("Writing {}", modules_txt.display()))?;

    // Create module subdirectories
    for module in manifest["modules"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|v| v.as_str())
    {
        let module_dir = app_root.join(module.to_lowercase().replace(' ', "_"));
        tokio::fs::create_dir_all(&module_dir).await?;
    }

    println!("✓  Created app '{}' at {}", args.app, app_dir.display());
    println!("   Title   : {}", title);
    println!("   Version : {}", args.version);
    if !manifest["modules"].as_array().unwrap().is_empty() {
        println!("   Modules : {}", module_lines.replace('\n', ", "));
    }
    println!();
    println!(
        "Next: spotledger install-app {} --site <site> --bench {}",
        args.app,
        bench.display()
    );
    Ok(())
}

// ── new-module ────────────────────────────────────────────────────────────────

pub async fn new_module(args: NewModuleArgs) -> Result<()> {
    let bench = args.bench.canonicalize().unwrap_or(args.bench.clone());
    let app_root = bench.join("apps").join(&args.app).join(&args.app);

    if !app_root.exists() {
        bail!("App root not found: {}", app_root.display());
    }

    let module_dir = app_root.join(args.module.to_lowercase().replace(' ', "_"));
    if module_dir.exists() {
        bail!("Module directory already exists: {}", module_dir.display());
    }
    tokio::fs::create_dir_all(&module_dir).await?;

    // Append to modules.txt
    let modules_txt = app_root.join("modules.txt");
    let current = if modules_txt.exists() {
        tokio::fs::read_to_string(&modules_txt).await?
    } else {
        String::new()
    };
    let updated = if current.trim().is_empty() {
        args.module.clone()
    } else {
        format!("{}\n{}", current.trim_end(), args.module)
    };
    tokio::fs::write(&modules_txt, updated).await?;

    println!(
        "✓  Added module '{}' to app '{}' at {}",
        args.module,
        args.app,
        module_dir.display()
    );
    Ok(())
}

// ── new-doctype ───────────────────────────────────────────────────────────────

pub async fn new_doctype(args: NewDoctypeArgs) -> Result<()> {
    let bench = args.bench.canonicalize().unwrap_or(args.bench.clone());
    let app_root = bench.join("apps").join(&args.app).join(&args.app);

    if !app_root.exists() {
        bail!("App root not found: {}", app_root.display());
    }

    let snake_name = args.name.to_lowercase().replace(' ', "_");
    let module_slug = args.module.to_lowercase().replace(' ', "_");
    let dt_dir = app_root
        .join(&module_slug)
        .join("doctype")
        .join(&snake_name);

    if dt_dir.exists() {
        bail!("DocType directory already exists: {}", dt_dir.display());
    }
    tokio::fs::create_dir_all(&dt_dir).await?;

    let scaffold = json!({
        "doctype": "DocType",
        "name": args.name,
        "module": args.module,
        "field_order": ["name_field"],
        "fields": [
            {
                "fieldname": "name_field",
                "fieldtype": "Data",
                "label": "Name",
                "reqd": 1,
                "in_list_view": 1,
                "idx": 1
            }
        ],
        "permissions": [
            {
                "role": "System Manager",
                "read": 1,
                "write": 1,
                "create": 1,
                "delete": 1,
                "report": 1
            }
        ],
        "sort_field": "creation",
        "sort_order": "DESC"
    });

    let json_path = dt_dir.join(format!("{}.json", snake_name));
    tokio::fs::write(&json_path, serde_json::to_string_pretty(&scaffold)?)
        .await
        .with_context(|| format!("Writing {}", json_path.display()))?;

    println!(
        "✓  Created DocType '{}' (module: {}) at {}",
        args.name,
        args.module,
        json_path.display()
    );
    println!("   Edit {} to add fields and permissions.", json_path.display());
    Ok(())
}

// ── export-app ────────────────────────────────────────────────────────────────

pub async fn export_app(args: ExportAppArgs) -> Result<()> {
    use spotledger_core::config::SiteConfig;
    use spotledger_db::connection::connect;

    let bench = args.bench.canonicalize().unwrap_or(args.bench.clone());
    let config_path = bench.join("sites").join(&args.site).join("site_config.toml");
    let config_str = tokio::fs::read_to_string(&config_path)
        .await
        .with_context(|| format!("Reading {}", config_path.display()))?;
    let cfg: SiteConfig =
        toml::from_str(&config_str).context("Parsing site_config.toml")?;
    let db = connect(&cfg.database)
        .await
        .context("Connecting to SurrealDB")?;

    // Find all DocTypes belonging to modules that are declared in this app's modules.txt
    let app_root = bench.join("apps").join(&args.app).join(&args.app);
    let modules_txt = app_root.join("modules.txt");
    if !modules_txt.exists() {
        bail!(
            "modules.txt not found for app '{}': {}",
            args.app,
            modules_txt.display()
        );
    }
    let modules_raw = tokio::fs::read_to_string(&modules_txt).await?;
    let modules: Vec<&str> = modules_raw
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect();

    if modules.is_empty() {
        bail!("No modules found in {}", modules_txt.display());
    }

    println!(
        "Exporting DocTypes for app '{}' (modules: {}) …",
        args.app,
        modules.join(", ")
    );

    let mut exported = 0usize;
    for module in &modules {
        let query = format!(
            "SELECT * FROM tabDocType WHERE module = '{}' ORDER BY name ASC",
            module.replace('\'', "\\'")
        );
        let rows = db.run(&query, vec![]).await.context("Querying tabDocType")?;

        for row in rows {
            let name = match row.get("name").and_then(|v| v.as_str()) {
                Some(n) => n.to_owned(),
                None => continue,
            };

            let snake_name = name.to_lowercase().replace(' ', "_");
            let module_slug = module.to_lowercase().replace(' ', "_");
            let dt_dir = app_root
                .join(&module_slug)
                .join("doctype")
                .join(&snake_name);
            tokio::fs::create_dir_all(&dt_dir).await?;

            // Fetch fields for this DocType
            let field_query = format!(
                "SELECT * FROM tabDocField WHERE parent = '{}' ORDER BY idx ASC",
                name.replace('\'', "\\'")
            );
            let fields = db.run(&field_query, vec![]).await.unwrap_or_default();

            // Build field_order from fields
            let field_order: Vec<String> = fields
                .iter()
                .filter_map(|f| f.get("fieldname").and_then(|v| v.as_str()).map(|s| s.to_owned()))
                .collect();

            let doc = json!({
                "doctype": "DocType",
                "name": name,
                "module": module,
                "field_order": field_order,
                "fields": fields,
                "permissions": [],
                "sort_field": "creation",
                "sort_order": "DESC"
            });

            let json_path = dt_dir.join(format!("{}.json", snake_name));
            tokio::fs::write(&json_path, serde_json::to_string_pretty(&doc)?)
                .await
                .with_context(|| format!("Writing {}", json_path.display()))?;

            println!("  wrote {}", json_path.display());
            exported += 1;
        }
    }

    println!("\n✓  Exported {} DocType(s) for app '{}'.", exported, args.app);
    Ok(())
}
