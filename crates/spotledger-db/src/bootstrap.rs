//! Framework-internal table bootstrap.
//!
//! These four tables are **not** DocTypes — they have no `DocTypeMeta` and are
//! never created by `ensure_all_schemas`.  They must be created once, up-front,
//! before anything else in the system can run.
//!
//! | Table         | Used by                                          |
//! |---------------|--------------------------------------------------|
//! | `__Auth`      | Password hashes (`auth::set_user_password`)      |
//! | `tabSessions` | Login sessions (`auth::create_session`)          |
//! | `tabSeries`   | Naming-series counters (`naming::next_name`)     |
//! | `tabSingles`  | Single-DocType key/value store                   |
//!
//! All DDL uses `IF NOT EXISTS` so calling this on an already-bootstrapped
//! site is fully idempotent.

use crate::adapter::DbAdapter;
use crate::document::upsert_doc;
use crate::error::DbError;
use spotledger_core::registry::MetaEntry;

// ── Framework DDL ─────────────────────────────────────────────────────────────

/// SurrealQL schema for the four framework-internal tables.
///
/// All statements are idempotent (`IF NOT EXISTS`). This string is compiled
/// into the binary — no file on disk is required.
const FRAMEWORK_TABLES: &str = "
-- ── __Auth ───────────────────────────────────────────────────────────────────
-- Stores password hashes keyed by (doctype, name, fieldname).
-- Intentionally not a DocType so it never appears in the Desk UI.
DEFINE TABLE IF NOT EXISTS __Auth SCHEMAFULL;
DEFINE FIELD IF NOT EXISTS doctype   ON __Auth TYPE string;
DEFINE FIELD IF NOT EXISTS name      ON __Auth TYPE string;
DEFINE FIELD IF NOT EXISTS fieldname ON __Auth TYPE string;
DEFINE FIELD IF NOT EXISTS password  ON __Auth TYPE string;
DEFINE FIELD IF NOT EXISTS encrypted ON __Auth TYPE bool   DEFAULT false;
DEFINE INDEX IF NOT EXISTS idx__auth_lookup
    ON __Auth FIELDS doctype, name, fieldname UNIQUE;

-- ── tabSessions ───────────────────────────────────────────────────────────────
-- Active (and recently expired) login sessions.
DEFINE TABLE IF NOT EXISTS tabSessions SCHEMAFULL;
DEFINE FIELD IF NOT EXISTS sid         ON tabSessions TYPE string;
DEFINE FIELD IF NOT EXISTS user        ON tabSessions TYPE string;
DEFINE FIELD IF NOT EXISTS status      ON tabSessions TYPE string   DEFAULT 'Active';
DEFINE FIELD IF NOT EXISTS lastupdate  ON tabSessions TYPE option<datetime>;
DEFINE FIELD IF NOT EXISTS ipaddress   ON tabSessions TYPE option<string>;
DEFINE FIELD IF NOT EXISTS sessiondata ON tabSessions TYPE option<string>;
DEFINE INDEX IF NOT EXISTS idx_sessions_sid
    ON tabSessions FIELDS sid UNIQUE;

-- ── tabSeries ────────────────────────────────────────────────────────────────
-- Naming-series counters (e.g. \"SINV-2024-\" → current = 42).
DEFINE TABLE IF NOT EXISTS tabSeries SCHEMAFULL;
DEFINE FIELD IF NOT EXISTS name    ON tabSeries TYPE string;
DEFINE FIELD IF NOT EXISTS current ON tabSeries TYPE int    DEFAULT 0;
DEFINE INDEX IF NOT EXISTS idx_series_name
    ON tabSeries FIELDS name UNIQUE;

-- ── tabSingles ───────────────────────────────────────────────────────────────
-- Key/value store for Single DocTypes (one row per field per doctype).
DEFINE TABLE IF NOT EXISTS tabSingles SCHEMAFULL;
DEFINE FIELD IF NOT EXISTS doctype ON tabSingles TYPE string;
DEFINE FIELD IF NOT EXISTS field   ON tabSingles TYPE string;
DEFINE FIELD IF NOT EXISTS value   ON tabSingles TYPE option<string>;
DEFINE INDEX IF NOT EXISTS idx_singles_lookup
    ON tabSingles FIELDS doctype, field UNIQUE;

-- ── tabMigration ─────────────────────────────────────────────────────────────
-- Tracks which versioned migrations have been applied to this site.
-- One row per migration; unique per site because each site is its own namespace.
DEFINE TABLE IF NOT EXISTS tabMigration SCHEMAFULL;
DEFINE FIELD IF NOT EXISTS name       ON tabMigration TYPE string;
DEFINE FIELD IF NOT EXISTS applied_at ON tabMigration TYPE datetime DEFAULT time::now();
DEFINE FIELD IF NOT EXISTS batch      ON tabMigration TYPE int DEFAULT 0;
DEFINE INDEX IF NOT EXISTS idx_migration_name
    ON tabMigration FIELDS name UNIQUE;
";

// ── Seed data ─────────────────────────────────────────────────────────────────

/// SurrealQL to upsert the minimum records required for a usable site.
///
/// Creates:
/// - `Administrator`, `System Manager`, and `All` roles in `tabRole`
/// - `System User` and `Website User` user types in `tabUserType`
/// - The `Administrator` user record in `tabUser`
///
/// Uses literal record IDs (`tabTable:Name`) so each statement is a true
/// create-or-update regardless of prior table state.  The ⟨…⟩ syntax quotes
/// record IDs that contain spaces.  All non-nullable system fields are
/// included to satisfy the SCHEMAFULL table constraints.
const SEED_RECORDS: &str = "
-- ── Roles ────────────────────────────────────────────────────────────────────
UPSERT tabRole:Administrator SET
    name        = 'Administrator',
    role_name   = 'Administrator',
    desk_access = 1,
    disabled    = 0,
    docstatus   = 0,
    idx         = 0,
    owner       = 'Administrator',
    creation    = time::now(),
    modified    = time::now(),
    modified_by = 'Administrator';

UPSERT tabRole:⟨System Manager⟩ SET
    name        = 'System Manager',
    role_name   = 'System Manager',
    desk_access = 1,
    disabled    = 0,
    docstatus   = 0,
    idx         = 0,
    owner       = 'Administrator',
    creation    = time::now(),
    modified    = time::now(),
    modified_by = 'Administrator';

UPSERT tabRole:All SET
    name        = 'All',
    role_name   = 'All',
    desk_access = 0,
    disabled    = 0,
    docstatus   = 0,
    idx         = 0,
    owner       = 'Administrator',
    creation    = time::now(),
    modified    = time::now(),
    modified_by = 'Administrator';

-- ── User Types ───────────────────────────────────────────────────────────────
UPSERT tabUserType:⟨System User⟩ SET
    name            = 'System User',
    user_type_name  = 'System User',
    docstatus       = 0,
    idx             = 0,
    owner           = 'Administrator',
    creation        = time::now(),
    modified        = time::now(),
    modified_by     = 'Administrator';

UPSERT tabUserType:⟨Website User⟩ SET
    name            = 'Website User',
    user_type_name  = 'Website User',
    docstatus       = 0,
    idx             = 0,
    owner           = 'Administrator',
    creation        = time::now(),
    modified        = time::now(),
    modified_by     = 'Administrator';

-- ── Administrator user ───────────────────────────────────────────────────────
UPSERT tabUser:Administrator SET
    name             = 'Administrator',
    email            = 'Administrator',
    first_name       = 'Administrator',
    full_name        = 'Administrator',
    user_type        = 'System User',
    enabled          = 1,
    docstatus        = 0,
    idx              = 0,
    mute_sounds      = 0,
    send_welcome_email = 0,
    roles            = [],
    permissions      = [],
    owner            = 'Administrator',
    creation         = time::now(),
    modified         = time::now(),
    modified_by      = 'Administrator';
";

// ── Public API ────────────────────────────────────────────────────────────────

/// Returns the raw SurrealQL DDL string for the four framework-internal tables.
///
/// Used by `spotledger emit` so the generated file is complete, including
/// the non-DocType tables that are not produced by `ensure_all_schemas`.
pub fn framework_tables_sql() -> &'static str {
    FRAMEWORK_TABLES
}

/// Create the four framework-internal tables in SurrealDB.
///
/// Safe to call on an existing site — every statement uses `IF NOT EXISTS`.
pub async fn run_framework_tables(adapter: &DbAdapter) -> Result<(), DbError> {
    adapter.execute(FRAMEWORK_TABLES, vec![]).await
}

/// Upsert minimum seed records (roles, user types, Administrator user).
///
/// Must be called **after** [`run_framework_tables`] and [`ensure_all_schemas`]
/// so that `tabRole`, `tabUserType`, and `tabUser` all exist.
pub async fn seed_default_records(adapter: &DbAdapter) -> Result<(), DbError> {
    adapter.execute(SEED_RECORDS, vec![]).await
}

/// Seed one `tabDocumentNamingRule` row for every compiled DocType that has a
/// non-None `autoname` in its [`DocTypeMeta`].
///
/// - Singleton DocTypes are skipped — their name is always the DocType name and
///   is resolved by the document layer before `naming::resolve_name` is called.
/// - Rows are upserted so re-running this (e.g., after an app upgrade that adds
///   new DocTypes) is fully idempotent.
/// - The `is_standard` flag is set to 1, signalling that this row was seeded
///   from compiled code.  Admins may freely edit the row; the flag is
///   informational and does not affect runtime behaviour.
///
/// Must be called **after** [`ensure_all_schemas`] so that
/// `tabDocumentNamingRule` exists.
pub async fn seed_naming_rules(adapter: &DbAdapter) -> Result<(), DbError> {
    let mut sql = String::new();

    for entry in inventory::iter::<MetaEntry>() {
        let meta = (entry.meta)();

        // Singletons: framework resolves name = doctype name, no rule needed.
        if meta.is_single {
            continue;
        }

        let autoname = match &meta.autoname {
            Some(a) if !a.is_empty() => a.clone(),
            _ => continue, // no autoname → UUID fallback, no rule row needed
        };

        let id    = surql_record_id(&meta.name);
        let dt    = escape_sq(&meta.name);
        let aname = escape_sq(&autoname);

        sql.push_str(&format!(
            "UPSERT tabDocumentNamingRule:{id} SET \
             document_type = '{dt}', \
             autoname = '{aname}', \
             is_standard = 1, \
             docstatus = 0, \
             idx = 0, \
             owner = 'Administrator', \
             creation = time::now(), \
             modified = time::now(), \
             modified_by = 'Administrator';\n"
        ));
    }

    if sql.is_empty() {
        return Ok(());
    }

    adapter.execute(&sql, vec![]).await
}

/// Seed one `tabDocType` row for every compiled DocType (framework types like
/// Currency, Country, Address, Contact, etc.).
///
/// This ensures `post_install_link_check` can find these types in `tabDocType`
/// even though they are defined in Rust code rather than JSON files.
pub async fn seed_framework_doctypes(adapter: &DbAdapter) -> Result<(), DbError> {
    for entry in inventory::iter::<MetaEntry>() {
        let meta = (entry.meta)();
        let fields = serde_json::json!({
            "module": meta.module,
            "doctype": "DocType",
            "issingle": if meta.is_single { 1 } else { 0 },
            "is_child_table": if meta.is_child { 1 } else { 0 },
            "issubmittable": if meta.is_submittable { 1 } else { 0 },
            "owner": "Administrator",
            "modified_by": "Administrator",
            "fields": [],
            "permissions": [],
        });
        if let Err(e) = upsert_doc(adapter, "DocType", &meta.name, &fields).await {
            tracing::warn!("seed_framework_doctypes: failed to seed '{}': {}", meta.name, e);
        }
    }
    Ok(())
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Produce a SurrealDB record ID segment, quoting with `⟨…⟩` when the name
/// contains characters outside `[A-Za-z0-9_]`.
fn surql_record_id(name: &str) -> String {
    if name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        name.to_string()
    } else {
        format!("⟨{}⟩", name)
    }
}

/// Escape single-quotes for embedding in SurrealQL string literals.
fn escape_sq(s: &str) -> String {
    s.replace('\'', "\\'")
}
