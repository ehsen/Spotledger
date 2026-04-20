//! In-platform App management API handlers.
//!
//! Five methods exposed via `POST /api/method/spotledger.apps.*`:
//!
//! | Method                        | Handler              | Purpose                                |
//! |-------------------------------|----------------------|----------------------------------------|
//! | `spotledger.apps.list`        | `handle_list`        | List installed apps with counts        |
//! | `spotledger.apps.create`      | `handle_create`      | Create a new in-platform app           |
//! | `spotledger.apps.get`         | `handle_get`         | Get one app with modules + dt count    |
//! | `spotledger.apps.add_module`  | `handle_add_module`  | Add a Module Def to an app             |
//! | `spotledger.apps.list_modules`| `handle_list_modules`| List modules belonging to an app       |

use crate::state::SiteState;
use serde_json::{json, Value};
use spotledger_core::error::SpotError;
use spotledger_db::document::upsert_doc;
use std::collections::HashMap;
use std::sync::Arc;

// ── Helpers ──────────────────────────────────────────────────────────────────

fn require_str<'a>(params: &'a HashMap<String, Value>, key: &str) -> Result<&'a str, SpotError> {
    params
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| SpotError::Validation(format!("missing required param: `{key}`")))
}

/// Validate that an app name is kebab-case (lowercase ASCII, digits, hyphens).
fn validate_app_name(name: &str) -> Result<(), SpotError> {
    if name.is_empty() {
        return Err(SpotError::Validation("app name must not be empty".into()));
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    {
        return Err(SpotError::Validation(
            "app name must be kebab-case: lowercase letters, digits and hyphens only".into(),
        ));
    }
    if name.starts_with('-') || name.ends_with('-') {
        return Err(SpotError::Validation(
            "app name must not start or end with a hyphen".into(),
        ));
    }
    Ok(())
}

// ── spotledger.apps.list ──────────────────────────────────────────────────────

/// Return all installed apps enriched with module count and doctype count.
pub async fn handle_list(
    site: Arc<SiteState>,
    _params: HashMap<String, Value>,
) -> Result<Value, SpotError> {
    // Fetch all installed app rows
    let apps = site
        .db
        .run("SELECT * FROM installed_app ORDER BY name ASC", vec![])
        .await
        .map_err(|e| SpotError::Db(e.to_string()))?;

    // Enrich each app with module_count and doctype_count
    let mut result: Vec<Value> = Vec::with_capacity(apps.len());
    for mut app in apps {
        let app_name = app
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_owned();

        // Module count
        let module_rows = site
            .db
            .run(
                "SELECT count() FROM tabModule_Def WHERE app_name = $app GROUP ALL",
                vec![("app".into(), Value::String(app_name.clone()))],
            )
            .await
            .unwrap_or_default();
        let module_count = module_rows
            .into_iter()
            .next()
            .and_then(|v| v.get("count").and_then(Value::as_i64))
            .unwrap_or(0);

        // DocType count
        let dt_rows = site
            .db
            .run(
                "SELECT count() FROM tabDocType WHERE app = $app GROUP ALL",
                vec![("app".into(), Value::String(app_name.clone()))],
            )
            .await
            .unwrap_or_default();
        let doctype_count = dt_rows
            .into_iter()
            .next()
            .and_then(|v| v.get("count").and_then(Value::as_i64))
            .unwrap_or(0);

        if let Some(obj) = app.as_object_mut() {
            obj.insert("module_count".into(), Value::Number(module_count.into()));
            obj.insert("doctype_count".into(), Value::Number(doctype_count.into()));
        }
        result.push(app);
    }

    Ok(Value::Array(result))
}

// ── spotledger.apps.create ────────────────────────────────────────────────────

/// Create a new in-platform development app.
///
/// Parameters:
/// - `name`        — kebab-case app identifier (required, unique)
/// - `title`       — display title (optional, defaults to name)
/// - `description` — short description (optional)
/// - `version`     — semver string (optional, defaults to "0.1.0")
/// - `author`      — author name (optional)
/// - `modules`     — JSON array of initial module names (optional)
pub async fn handle_create(
    site: Arc<SiteState>,
    params: HashMap<String, Value>,
) -> Result<Value, SpotError> {
    let name = require_str(&params, "name")?.to_owned();
    validate_app_name(&name)?;

    // Check uniqueness
    let existing = site
        .db
        .run(
            "SELECT name FROM installed_app WHERE name = $name LIMIT 1",
            vec![("name".into(), Value::String(name.clone()))],
        )
        .await
        .map_err(|e| SpotError::Db(e.to_string()))?;
    if !existing.is_empty() {
        return Err(SpotError::Validation(format!(
            "App '{}' already exists",
            name
        )));
    }

    let title = params
        .get("title")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .unwrap_or(&name)
        .to_owned();
    let description = params
        .get("description")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_owned();
    let version = params
        .get("version")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .unwrap_or("0.1.0")
        .to_owned();
    let author = params
        .get("author")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_owned();

    let manifest = json!({
        "name":        &name,
        "title":       &title,
        "description": &description,
        "version":     &version,
        "author":      &author,
        "depends_on":  [],
    });

    let row = json!({
        "name":     &name,
        "version":  &version,
        "app_path": null,
        "manifest": &manifest,
        "status":   "development",
    });

    upsert_doc(&site.db, "installed_app", &name, &row)
        .await
        .map_err(|e| SpotError::Db(e.to_string()))?;

    // Seed initial modules if provided
    if let Some(Value::Array(modules)) = params.get("modules") {
        for (idx, m) in modules.iter().enumerate() {
            if let Some(mod_name) = m.as_str().filter(|s| !s.is_empty()) {
                let mod_row = build_module_row(mod_name, &name, idx as i64);
                let _ = upsert_doc(&site.db, "Module Def", mod_name, &mod_row).await;
            }
        }
    }

    Ok(json!({ "ok": true, "app": name }))
}

// ── spotledger.apps.get ───────────────────────────────────────────────────────

/// Return a single app record enriched with modules list and doctype count.
///
/// Parameters: `{ "name": "<app-name>" }`
pub async fn handle_get(
    site: Arc<SiteState>,
    params: HashMap<String, Value>,
) -> Result<Value, SpotError> {
    let name = require_str(&params, "name")?.to_owned();

    let rows = site
        .db
        .run(
            "SELECT * FROM installed_app WHERE name = $name LIMIT 1",
            vec![("name".into(), Value::String(name.clone()))],
        )
        .await
        .map_err(|e| SpotError::Db(e.to_string()))?;

    let app = rows.into_iter().next().ok_or_else(|| SpotError::NotFound {
        doctype: "installed_app".into(),
        name: name.clone(),
    })?;

    let modules = site
        .db
        .run(
            "SELECT * FROM tabModule_Def WHERE app_name = $app ORDER BY order ASC",
            vec![("app".into(), Value::String(name.clone()))],
        )
        .await
        .unwrap_or_default();

    let dt_count_rows = site
        .db
        .run(
            "SELECT count() FROM tabDocType WHERE app = $app GROUP ALL",
            vec![("app".into(), Value::String(name.clone()))],
        )
        .await
        .unwrap_or_default();
    let doctype_count = dt_count_rows
        .into_iter()
        .next()
        .and_then(|v| v.get("count").and_then(Value::as_i64))
        .unwrap_or(0);

    Ok(json!({
        "app":           app,
        "modules":       modules,
        "doctype_count": doctype_count,
    }))
}

// ── spotledger.apps.add_module ────────────────────────────────────────────────

/// Add a new Module Def to an existing app.
///
/// Parameters:
/// - `app_name`    — name of the installed app (required)
/// - `module_name` — display name for the new module (required)
pub async fn handle_add_module(
    site: Arc<SiteState>,
    params: HashMap<String, Value>,
) -> Result<Value, SpotError> {
    let app_name = require_str(&params, "app_name")?.to_owned();
    let module_name = require_str(&params, "module_name")?.trim().to_owned();

    if module_name.is_empty() {
        return Err(SpotError::Validation("module_name must not be empty".into()));
    }

    // Ensure the app exists
    let app_rows = site
        .db
        .run(
            "SELECT name FROM installed_app WHERE name = $app LIMIT 1",
            vec![("app".into(), Value::String(app_name.clone()))],
        )
        .await
        .map_err(|e| SpotError::Db(e.to_string()))?;
    if app_rows.is_empty() {
        return Err(SpotError::NotFound {
            doctype: "installed_app".into(),
            name: app_name.clone(),
        });
    }

    // Count existing modules for this app to determine `order`
    let count_rows = site
        .db
        .run(
            "SELECT count() FROM tabModule_Def WHERE app_name = $app GROUP ALL",
            vec![("app".into(), Value::String(app_name.clone()))],
        )
        .await
        .unwrap_or_default();
    let order = count_rows
        .into_iter()
        .next()
        .and_then(|v| v.get("count").and_then(Value::as_i64))
        .unwrap_or(0);

    let mod_row = build_module_row(&module_name, &app_name, order);
    upsert_doc(&site.db, "Module Def", &module_name, &mod_row)
        .await
        .map_err(|e| SpotError::Db(e.to_string()))?;

    Ok(json!({ "ok": true, "module": module_name, "app": app_name }))
}

// ── spotledger.apps.list_modules ─────────────────────────────────────────────

/// List all Module Def records belonging to an app.
///
/// Parameters: `{ "app_name": "<app-name>" }`
pub async fn handle_list_modules(
    site: Arc<SiteState>,
    params: HashMap<String, Value>,
) -> Result<Value, SpotError> {
    let app_name = require_str(&params, "app_name")?.to_owned();

    let modules = site
        .db
        .run(
            "SELECT * FROM tabModule_Def WHERE app_name = $app ORDER BY order ASC",
            vec![("app".into(), Value::String(app_name.clone()))],
        )
        .await
        .map_err(|e| SpotError::Db(e.to_string()))?;

    Ok(Value::Array(modules))
}

// ── Private helpers ───────────────────────────────────────────────────────────

fn build_module_row(module_name: &str, app_name: &str, order: i64) -> Value {
    json!({
        "name":        module_name,
        "doctype":     "Module Def",
        "module_name": module_name,
        "app_name":    app_name,
        "label":       module_name,
        "show_in_menu": 1,
        "order":       order,
        "owner":       "Administrator",
        "modified_by": "Administrator",
    })
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::validate_app_name;

    #[test]
    fn valid_kebab_names_are_accepted() {
        assert!(validate_app_name("my-app").is_ok());
        assert!(validate_app_name("payroll").is_ok());
        assert!(validate_app_name("my-erp-v2").is_ok());
    }

    #[test]
    fn invalid_names_are_rejected() {
        assert!(validate_app_name("").is_err()); // empty
        assert!(validate_app_name("My App").is_err()); // spaces
        assert!(validate_app_name("my_app").is_err()); // underscores
        assert!(validate_app_name("-app").is_err()); // leading hyphen
        assert!(validate_app_name("app-").is_err()); // trailing hyphen
        assert!(validate_app_name("MyApp").is_err()); // uppercase
    }
}
