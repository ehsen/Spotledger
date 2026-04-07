//! Schema synchronisation — Hibernate-style `hbm2ddl.auto=update`.
//!
//! Every compiled DocType registers a [`MetaEntry`] via `inventory::submit!`.
//! On `spotledger migrate` (or at site startup), calling [`ensure_all_schemas`]
//! iterates all registered metas and calls [`ensure_schema`] for each one.
//!
//! ## Rules
//! - **Additive only** — fields are never dropped or renamed automatically.
//!   Removing a field from `DocTypeMeta` simply means the DB column becomes
//!   orphaned data; the framework will ignore it on read.
//! - `DEFINE TABLE … SCHEMAFULL` is idempotent (re-running is safe).
//! - `DEFINE FIELD IF NOT EXISTS …` is idempotent — won't change existing
//!   field definitions that someone altered manually.
//! - Child DocTypes get the extra `parent`/`parenttype`/`parentfield` fields.
//! - A unique index on `name` is created for every non-child table.

use spotledger_core::meta::{DocTypeMeta, FieldType};
use spotledger_core::registry::MetaEntry;

use crate::adapter::DbAdapter;
use crate::document::doctype_to_table;
use crate::error::DbError;

// ── FieldType → SurrealDB type string ────────────────────────────────────────

/// Map a [`FieldType`] to the SurrealDB column type declaration.
///
/// Returns `None` for layout-only fields (Section/Column/Tab break) which
/// are never stored in the DB.
pub fn surql_type(ft: &FieldType) -> Option<&'static str> {
    match ft {
        // Integer storage
        FieldType::Check => Some("int"),
        FieldType::Int   => Some("int"),

        // Float storage
        FieldType::Float | FieldType::Currency | FieldType::Percent | FieldType::Rating
            => Some("float"),

        // Temporal — SurrealDB native datetime for Datetime; string for Date/Time
        // so that Frappe-format strings ("2024-01-15") round-trip without conversion.
        FieldType::Datetime => Some("option<datetime>"),
        FieldType::Date     => Some("option<string>"),
        FieldType::Time     => Some("option<string>"),

        // Child table arrays — embedded JSON array of objects
        FieldType::Table | FieldType::TableMultiSelect => Some("array<object>"),

        // Arbitrary JSON blob
        FieldType::Json => Some("any"),

        // Layout-only — no DB column
        FieldType::SectionBreak | FieldType::ColumnBreak | FieldType::TabBreak => None,

        // Everything else is a nullable string
        _ => Some("option<string>"),
    }
}

// ── System fields ─────────────────────────────────────────────────────────────

/// Standard Frappe system fields present on every document table.
const SYSTEM_FIELDS: &[(&str, &str)] = &[
    ("name",        "string"),
    ("doctype",     "option<string>"),
    ("owner",       "option<string>"),
    ("creation",    "option<datetime>"),
    ("modified",    "option<datetime>"),
    ("modified_by", "option<string>"),
    ("docstatus",   "int"),
    ("idx",         "int"),
];

/// Extra fields added only to child-table documents.
const CHILD_FIELDS: &[(&str, &str)] = &[
    ("parent",      "string"),
    ("parenttype",  "string"),
    ("parentfield", "string"),
];

// ── Core DDL functions ────────────────────────────────────────────────────────

/// Idempotently ensure the SurrealDB table schema for one DocType.
///
/// Emits:
/// 1. `DEFINE TABLE IF NOT EXISTS … SCHEMAFULL`
/// 2. `DEFINE FIELD IF NOT EXISTS …` for every system field
/// 3. `DEFINE FIELD IF NOT EXISTS …` for every DocField in the meta
/// 4. `DEFINE INDEX IF NOT EXISTS … UNIQUE` on `name` (non-child tables)
///
/// Nothing is ever removed — this call is fully additive.
pub async fn ensure_schema(adapter: &DbAdapter, meta: &DocTypeMeta) -> Result<(), DbError> {
    let table    = doctype_to_table(&meta.name);
    let safe_name = meta.name.replace([' ', '-'], "_").to_lowercase();

    // 1. Table definition
    let sql = format!(
        "DEFINE TABLE IF NOT EXISTS `{table}` SCHEMAFULL \
         COMMENT 'SpotLedger DocType: {name}';",
        name = meta.name
    );
    adapter.execute(&sql, vec![]).await?;
    tracing::debug!(doctype = %meta.name, "DEFINE TABLE done");

    // 2. System fields
    for (fieldname, ty) in SYSTEM_FIELDS {
        define_field(adapter, &table, fieldname, ty).await?;
    }

    // 3. Child-table extra fields
    if meta.is_child {
        for (fieldname, ty) in CHILD_FIELDS {
            define_field(adapter, &table, fieldname, ty).await?;
        }
    }

    // 4. DocType-declared fields
    for df in &meta.fields {
        if df.fieldtype.is_layout() {
            continue;
        }
        let Some(ty) = surql_type(&df.fieldtype) else { continue };

        // Nullable override: if not_nullable AND has a default, use non-option type
        let effective_type = if df.not_nullable && !ty.starts_with("option<") {
            ty.to_owned()
        } else if df.not_nullable && ty.starts_with("option<") {
            // strip the option<> wrapper
            ty[7..ty.len() - 1].to_owned()
        } else {
            ty.to_owned()
        };

        let mut sql = format!(
            "DEFINE FIELD IF NOT EXISTS `{fn}` ON TABLE `{table}` TYPE {ty}",
            r#fn = df.fieldname,
            ty = effective_type,
        );

        // Attach DEFAULT for not-nullable fields that have a default value
        if let Some(ref dv) = df.default_value {
            match df.fieldtype {
                FieldType::Int | FieldType::Check => {
                    sql.push_str(&format!(" DEFAULT {dv}"));
                }
                FieldType::Float | FieldType::Currency | FieldType::Percent | FieldType::Rating => {
                    sql.push_str(&format!(" DEFAULT {dv}"));
                }
                _ => {
                    // String/text — quote the value
                    let escaped = dv.replace('\'', "\\'");
                    sql.push_str(&format!(" DEFAULT '{escaped}'"));
                }
            }
        }

        sql.push(';');
        adapter.execute(&sql, vec![]).await?;
    }

    // 5. Unique index on `name` (not needed for child tables — parent+idx is the key)
    if !meta.is_child {
        let sql = format!(
            "DEFINE INDEX IF NOT EXISTS idx_{safe_name}_name \
             ON TABLE `{table}` FIELDS name UNIQUE;"
        );
        adapter.execute(&sql, vec![]).await?;
    }

    // 6. Extra indexes: unique fields and standard-filter fields
    for df in &meta.fields {
        if df.fieldtype.is_layout() { continue; }

        if df.unique {
            let idx_name = format!("idx_{safe_name}_{fn}_uq", r#fn = df.fieldname);
            let sql = format!(
                "DEFINE INDEX IF NOT EXISTS `{idx_name}` \
                 ON TABLE `{table}` FIELDS `{fn}` UNIQUE;",
                r#fn = df.fieldname
            );
            adapter.execute(&sql, vec![]).await?;
        }

        if df.in_standard_filter {
            let idx_name = format!("idx_{safe_name}_{fn}", r#fn = df.fieldname);
            let sql = format!(
                "DEFINE INDEX IF NOT EXISTS `{idx_name}` \
                 ON TABLE `{table}` FIELDS `{fn}`;",
                r#fn = df.fieldname
            );
            adapter.execute(&sql, vec![]).await?;
        }
    }

    tracing::info!(doctype = %meta.name, table = %table, "Schema synced");
    Ok(())
}

/// Helper — emit one `DEFINE FIELD IF NOT EXISTS` statement.
async fn define_field(
    adapter: &DbAdapter,
    table: &str,
    fieldname: &str,
    ty: &str,
) -> Result<(), DbError> {
    let sql = format!(
        "DEFINE FIELD IF NOT EXISTS `{fieldname}` ON TABLE `{table}` TYPE {ty};"
    );
    adapter.execute(&sql, vec![]).await
}

// ── Public entry point ────────────────────────────────────────────────────────

/// Run schema sync for **all** compiled DocTypes registered via `inventory`.
///
/// Call this once during `spotledger migrate` or at site startup before
/// accepting any requests.
pub async fn ensure_all_schemas(adapter: &DbAdapter) -> Result<(), DbError> {
    let entries: Vec<_> = inventory::iter::<MetaEntry>.into_iter().collect();

    if entries.is_empty() {
        tracing::warn!("ensure_all_schemas: no MetaEntry registrations found");
        return Ok(());
    }

    tracing::info!(count = entries.len(), "Syncing schemas for compiled DocTypes");

    let mut errors = 0usize;
    for entry in entries {
        let meta = (entry.meta)();
        if let Err(e) = ensure_schema(adapter, &meta).await {
            tracing::error!(doctype = %entry.name, error = %e, "Schema sync failed");
            errors += 1;
        }
    }

    if errors > 0 {
        return Err(DbError::Other(format!(
            "Schema sync failed for {errors} DocType(s)"
        )));
    }

    Ok(())
}

/// Returns the set of DocType names currently registered in the compiled inventory.
///
/// Used by `spotledger cleanup` to determine which `tabDocField` / `tabDocPerm`
/// rows are orphaned (their parent no longer exists in code).
pub fn compiled_doctype_names() -> std::collections::HashSet<String> {
    inventory::iter::<MetaEntry>
        .into_iter()
        .map(|e| e.name.to_owned())
        .collect()
}

// ── Emit (dry-run generator) ──────────────────────────────────────────────────

/// Generate the SurrealQL that `ensure_schema` would execute for `meta`,
/// without touching the database.
///
/// Used by `spotledger emit` to produce a reviewable `generated/schema.surql`.
pub fn emit_schema_sql(meta: &DocTypeMeta) -> String {
    let table     = crate::document::doctype_to_table(&meta.name);
    let safe_name = meta.name.replace([' ', '-'], "_").to_lowercase();
    let mut out   = String::new();

    out.push_str(&format!(
        "-- ── {name} ──\n\
         DEFINE TABLE IF NOT EXISTS `{table}` SCHEMAFULL \
         COMMENT 'SpotLedger DocType: {name}';\n",
        name = meta.name,
    ));

    for (fieldname, ty) in SYSTEM_FIELDS {
        out.push_str(&format!(
            "DEFINE FIELD IF NOT EXISTS `{fieldname}` ON TABLE `{table}` TYPE {ty};\n"
        ));
    }

    if meta.is_child {
        for (fieldname, ty) in CHILD_FIELDS {
            out.push_str(&format!(
                "DEFINE FIELD IF NOT EXISTS `{fieldname}` ON TABLE `{table}` TYPE {ty};\n"
            ));
        }
    }

    for df in &meta.fields {
        if df.fieldtype.is_layout() {
            continue;
        }
        let Some(ty) = surql_type(&df.fieldtype) else { continue };

        let effective_type = if df.not_nullable && ty.starts_with("option<") {
            ty[7..ty.len() - 1].to_owned()
        } else {
            ty.to_owned()
        };

        let mut line = format!(
            "DEFINE FIELD IF NOT EXISTS `{fn}` ON TABLE `{table}` TYPE {ty}",
            r#fn = df.fieldname,
            ty = effective_type,
        );

        if let Some(ref dv) = df.default_value {
            match df.fieldtype {
                FieldType::Int
                | FieldType::Check
                | FieldType::Float
                | FieldType::Currency
                | FieldType::Percent
                | FieldType::Rating => {
                    line.push_str(&format!(" DEFAULT {dv}"));
                }
                _ => {
                    let escaped = dv.replace('\'', "\\'");
                    line.push_str(&format!(" DEFAULT '{escaped}'"));
                }
            }
        }

        line.push(';');
        line.push('\n');
        out.push_str(&line);
    }

    if !meta.is_child {
        out.push_str(&format!(
            "DEFINE INDEX IF NOT EXISTS idx_{safe_name}_name \
             ON TABLE `{table}` FIELDS name UNIQUE;\n"
        ));
    }

    for df in &meta.fields {
        if df.fieldtype.is_layout() { continue; }
        if df.unique {
            let idx_name = format!("idx_{safe_name}_{fn}_uq", r#fn = df.fieldname);
            out.push_str(&format!(
                "DEFINE INDEX IF NOT EXISTS `{idx_name}` \
                 ON TABLE `{table}` FIELDS `{fn}` UNIQUE;\n",
                r#fn = df.fieldname,
            ));
        }
        if df.in_standard_filter {
            let idx_name = format!("idx_{safe_name}_{fn}", r#fn = df.fieldname);
            out.push_str(&format!(
                "DEFINE INDEX IF NOT EXISTS `{idx_name}` \
                 ON TABLE `{table}` FIELDS `{fn}`;\n",
                r#fn = df.fieldname,
            ));
        }
    }

    out
}

/// Generate the full SurrealQL for **all** compiled DocTypes concatenated,
/// preceded by the framework-tables DDL from `bootstrap.rs`.
///
/// Returns `(sql_string, doctype_count)`.
pub fn emit_all_schemas_sql() -> (String, usize) {
    let entries: Vec<_> = inventory::iter::<MetaEntry>.into_iter().collect();
    let count = entries.len();

    let header = format!(
        "-- Generated by `spotledger emit`\n\
         -- Source of truth: compiled Rust DocType definitions\n\
         -- DO NOT EDIT — re-run `spotledger emit` to regenerate\n\
         -- DocTypes: {count}\n\n"
    );

    let body: String = entries
        .iter()
        .map(|e| emit_schema_sql(&(e.meta)()))
        .collect::<Vec<_>>()
        .join("\n");

    (header + &body, count)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use spotledger_core::meta::FieldType;

    #[test]
    fn surql_type_mapping() {
        assert_eq!(surql_type(&FieldType::Check),    Some("int"));
        assert_eq!(surql_type(&FieldType::Int),      Some("int"));
        assert_eq!(surql_type(&FieldType::Float),    Some("float"));
        assert_eq!(surql_type(&FieldType::Currency), Some("float"));
        assert_eq!(surql_type(&FieldType::Datetime), Some("option<datetime>"));
        assert_eq!(surql_type(&FieldType::Date),     Some("option<string>"));
        assert_eq!(surql_type(&FieldType::Table),    Some("array<object>"));
        assert_eq!(surql_type(&FieldType::Json),     Some("any"));
        assert_eq!(surql_type(&FieldType::Data),     Some("option<string>"));
        assert_eq!(surql_type(&FieldType::SectionBreak), None);
        assert_eq!(surql_type(&FieldType::ColumnBreak),  None);
    }
}
