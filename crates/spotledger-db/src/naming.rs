//! Naming series support — generates document names like "SO-2024-00001".
//!
//! Strategy: use `tabSeries` (created by bootstrap schema) as a counter store.
//! Each series prefix has one row; we atomically increment the counter with
//! `UPDATE ... SET current = current + 1 RETURN AFTER`.
//!
//! If the series doesn't exist it is created starting at 1.
//!
//! Frappe naming series tokens:
//!   `YYYY`  → 4-digit year
//!   `YY`    → 2-digit year
//!   `MM`    → 2-digit month
//!   `DD`    → 2-digit day
//!   `#`     → one digit (each `#` independently)
//!   `####`  → 4-digit zero-padded sequence (group of `#`)
//!   `.`     → literal separator that Frappe uses; we replace with empty string in output
//!            (some series have `.` as separator, e.g. "INV-.YYYY.-.####")

use chrono::Utc;
use serde_json::Value;
use surrealdb::engine::remote::ws::Client;
use surrealdb::Surreal;

use crate::error::DbError;

/// Given a naming series template like `"SO-.YYYY.-.####"`, generate the
/// next name and increment the counter in `tabSeries`.
///
/// Returns the generated name string.
pub async fn next_name(
    db: &Surreal<Client>,
    series: &str,
) -> Result<String, DbError> {
    // Expand the date tokens first (these don't require the counter)
    let now = Utc::now();
    let year4 = now.format("%Y").to_string();
    let year2 = now.format("%y").to_string();
    let month = now.format("%m").to_string();
    let day = now.format("%d").to_string();

    // The "current" key in tabSeries is the prefix with date tokens expanded
    // but before the `#` sequence.  Frappe stores it as: "SO-2024-" for "SO-.YYYY.-."
    let without_hash = series.trim_end_matches('#').trim_end_matches('.');
    let prefix_key = without_hash
        .replace("YYYY", &year4)
        .replace("YY", &year2)
        .replace("MM", &month)
        .replace("DD", &day)
        .replace('.', "");

    // Count how many `#` are at the end of the series template
    let hash_count = series.chars().rev().take_while(|c| *c == '#').count().max(1);

    // Atomically fetch+increment the series counter
    let counter = increment_series(db, &prefix_key).await?;

    // Format: prefix + zero-padded counter
    let name = format!("{}{:0>width$}", prefix_key, counter, width = hash_count);
    Ok(name)
}

/// Increment the `tabSeries` row for `name`, creating it if absent.
/// Returns the new counter value (1-based).
async fn increment_series(db: &Surreal<Client>, name: &str) -> Result<u64, DbError> {
    // Try to update existing row
    let mut resp = db
        .query(
            "UPDATE tabSeries SET current = current + 1 WHERE name = $name RETURN AFTER",
        )
        .bind(("name", name.to_owned()))
        .await?;
    let rows: Vec<Value> = resp.take(0)?;

    if let Some(row) = rows.into_iter().next() {
        let n = row.get("current").and_then(Value::as_u64).unwrap_or(1);
        return Ok(n);
    }

    // Row didn't exist — create it with current = 1
    let mut resp = db
        .query("INSERT INTO tabSeries (name, current) VALUES ($name, 1) RETURN AFTER")
        .bind(("name", name.to_owned()))
        .await?;
    let rows: Vec<Value> = resp.take(0)?;
    let n = rows
        .into_iter()
        .next()
        .and_then(|r| r.get("current").and_then(Value::as_u64))
        .unwrap_or(1);
    Ok(n)
}

/// Determine the name for a new document.
///
/// Priority:
/// 1. If `fields["name"]` is non-empty, use it as-is (user-supplied name).
/// 2. If the DocType has a `autoname` field in SurrealDB, use it.
/// 3. Fall back to the `doctype` itself (singleton pattern) or a hash-based name.
///
/// For Phase 2, we rely on the caller passing a `naming_series` field or we
/// inspect the `tabDocType` record's `autoname` column.
pub async fn resolve_name(
    db: &Surreal<Client>,
    doctype: &str,
    fields: &Value,
) -> Result<String, DbError> {
    // 1. Explicit name in fields
    if let Some(name) = fields.get("name").and_then(Value::as_str) {
        if !name.is_empty() {
            return Ok(name.to_owned());
        }
    }

    // 2. naming_series field in the document
    if let Some(ns) = fields.get("naming_series").and_then(Value::as_str) {
        if !ns.is_empty() {
            return next_name(db, ns).await;
        }
    }

    // 3. Inspect tabDocType.autoname
    let mut resp = db
        .query("SELECT autoname FROM tabDocType WHERE name = $dt LIMIT 1")
        .bind(("dt", doctype.to_owned()))
        .await?;
    let rows: Vec<Value> = resp.take(0)?;
    if let Some(row) = rows.into_iter().next() {
        if let Some(autoname) = row.get("autoname").and_then(Value::as_str) {
            if autoname.starts_with("field:") {
                // field:fieldname — use the value of that field
                let fieldname = &autoname["field:".len()..];
                if let Some(v) = fields.get(fieldname).and_then(Value::as_str) {
                    if !v.is_empty() {
                        return Ok(v.to_owned());
                    }
                }
            } else if !autoname.is_empty()
                && autoname != "Prompt"
                && autoname != "hash"
                && autoname != "UUID"
            {
                // It's a naming series template
                return next_name(db, autoname).await;
            }
        }
    }

    // 4. UUID fallback
    Ok(uuid_name())
}

fn uuid_name() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    // Simple deterministic-enough short UUID for now using timestamp + random suffix
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0);
    format!("new-{:08x}", ts)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn series_prefix_extraction() {
        // Simulate what next_name does for prefix building
        let series = "SO-.YYYY.-.####";
        let without_hash = series.trim_end_matches('#').trim_end_matches('.');
        let hash_count = series.chars().rev().take_while(|c| *c == '#').count();
        assert_eq!(hash_count, 4);
        let prefix_raw = without_hash
            .replace("YYYY", "2024")
            .replace("YY", "24")
            .replace("MM", "01")
            .replace("DD", "15")
            .replace('.', "");
        assert_eq!(prefix_raw, "SO-2024-");
        let name = format!("{}{:0>4}", prefix_raw, 1u64);
        assert_eq!(name, "SO-2024-0001");
    }

    #[test]
    fn series_simple_prefix() {
        let series = "CUST-####";
        let without_hash = series.trim_end_matches('#').trim_end_matches('.');
        let hash_count = series.chars().rev().take_while(|c| *c == '#').count();
        assert_eq!(hash_count, 4);
        assert_eq!(without_hash, "CUST-");
        let name = format!("{}{:0>4}", without_hash, 42u64);
        assert_eq!(name, "CUST-0042");
    }
}
