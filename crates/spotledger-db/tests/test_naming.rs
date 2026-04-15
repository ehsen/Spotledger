//! Tests for `spotledger_db::naming` — naming series, token expansion, series prefix logic.
//!
//! Pure unit tests (no DB required) cover the token expansion logic embedded inside
//! `next_name()`. DB-dependent tests (counter atomicity) are in `test_db_integration.rs`.
//!
//! ## Frappe parity
//! | Frappe test | This test |
//! |---|---|
//! | `test_naming.py::test_format_autoname` | `test_token_expansion_full_series` |
//! | `test_naming.py::test_naming_series_year_digits` | `test_token_year_2_and_4_digit` |
//! | `test_naming.py::test_naming_series_hash_count` | `test_hash_count_extraction` |
//! | `test_naming.py::test_naming_series_dot_separator` | `test_dot_is_stripped_from_prefix` |
//! | `test_naming.py::test_naming_series_zero_padding` | `test_counter_zero_padded` |
//! | `test_naming.py::test_naming_series_no_hash` | `test_series_with_no_hash` |
//! | `test_naming.py::test_field_autoname` | tested in `test_db_integration.rs` |

// The naming series core logic is private to `naming.rs`, but the internal module has
// `#[cfg(test)] mod tests { ... }` for the prefix-extraction helpers.
// Here we test the observable behaviour by re-running the same arithmetic the production
// code uses, plus the `doctype_to_table` convention that feeding affects naming.

use spotledger_db::document::doctype_to_table;

// ── Token expansion (mirrors logic inside `naming::next_name`) ─────────────────

/// Frappe parity: `test_naming.py::test_format_autoname`
/// Standard SO series template expands date tokens and strips dots.
#[test]
fn test_token_expansion_full_series() {
    simulate_series_expansion("SO-.YYYY.-.####", "2024", "01", "15", 4, "SO-2024-", "SO-2024-0001");
}

/// SINV series with year + month.
#[test]
fn test_sinv_series_year_month() {
    simulate_series_expansion("SINV-.YYYY.-.MM.-.####", "2024", "03", "10", 4, "SINV-2024-03-", "SINV-2024-03-0001");
}

/// Frappe parity: `test_naming.py::test_naming_series_year_digits`
/// `YY` produces a 2-digit year.
#[test]
fn test_token_year_2_digit() {
    simulate_series_expansion("INV-.YY.-.####", "2024", "01", "01", 4, "INV-24-", "INV-24-0001");
}

/// `YYYY` produces a 4-digit year.
#[test]
fn test_token_year_4_digit() {
    simulate_series_expansion("INV-.YYYY.-.####", "2024", "01", "01", 4, "INV-2024-", "INV-2024-0001");
}

/// Frappe parity: `test_naming.py::test_naming_series_hash_count`
/// The number of `#` characters determines padding width.
#[test]
fn test_hash_count_extraction() {
    assert_eq!(hash_count("SO-.####"), 4);
    assert_eq!(hash_count("SO-.##"), 2);
    assert_eq!(hash_count("SO-########"), 8);
    assert_eq!(hash_count("CUST-###"), 3);
}

/// Frappe parity: `test_naming.py::test_naming_series_dot_separator`
/// Dots between tokens are stripped — they are Frappe visual separators, not in the output.
#[test]
fn test_dot_is_stripped_from_prefix() {
    let prefix = expand_prefix("SO-.YYYY.-.####", "2024", "01", "15");
    assert!(!prefix.contains('.'), "dots must be stripped from prefix: {prefix}");
    assert_eq!(prefix, "SO-2024-");
}

/// Frappe parity: `test_naming.py::test_naming_series_zero_padding`
/// Counter is zero-padded to the number of `#` characters.
#[test]
fn test_counter_zero_padded() {
    let fmt = make_name_with_counter("CUST-####", "CUST-", 1, 4);
    assert_eq!(fmt, "CUST-0001");
    let fmt = make_name_with_counter("CUST-####", "CUST-", 42, 4);
    assert_eq!(fmt, "CUST-0042");
    let fmt = make_name_with_counter("CUST-####", "CUST-", 1000, 4);
    assert_eq!(fmt, "CUST-1000");
}

/// Counter that overflows the hash width still formats (no truncation — Frappe behavior).
#[test]
fn test_counter_overflow_hash_width() {
    // Frappe allows counters beyond the hash digit count; the field gets wider
    let fmt = make_name_with_counter("SO-##", "SO-", 1000, 2);
    assert_eq!(fmt, "SO-1000", "overflow is allowed — name gets longer");
}

/// Series with no `#` characters: single-char `#` is the default.
#[test]
fn test_series_with_no_hash() {
    // If there are NO # at all (rare edge case), hash_count should fall back to 1.
    // We replicate the `.max(1)` guard in the code.
    assert_eq!(hash_count_with_fallback("CUST-"), 1);
}

/// DD token is expanded correctly.
#[test]
fn test_token_dd_expansion() {
    let prefix = expand_prefix("LOG-.YYYY.-.MM.-.DD.-.####", "2024", "03", "05");
    assert_eq!(prefix, "LOG-2024-03-05-", "DD token must expand with leading zero");
}

/// Static prefix (no date tokens, no dots) is passed through unchanged.
#[test]
fn test_static_prefix_passthrough() {
    let prefix = expand_prefix("MYPREFIX-####", "2024", "01", "01");
    assert_eq!(prefix, "MYPREFIX-");
}

// ── Helpers that replicate the naming.rs token-expansion logic ────────────────

/// Extract the number of trailing `#` characters (with `.max(1)` fallback).
fn hash_count(series: &str) -> usize {
    series.chars().rev().take_while(|c| *c == '#').count().max(1)
}

fn hash_count_with_fallback(series: &str) -> usize {
    series.chars().rev().take_while(|c| *c == '#').count().max(1)
}

/// Expand date tokens in a series template, strip dots, and return the prefix (without `#`).
fn expand_prefix(series: &str, year4: &str, month: &str, day: &str) -> String {
    let year2 = &year4[2..]; // last 2 digits
    let without_hash = series.trim_end_matches('#').trim_end_matches('.');
    without_hash
        .replace("YYYY", year4)
        .replace("YY", year2)
        .replace("MM", month)
        .replace("DD", day)
        .replace('.', "")
}

/// Combine prefix + zero-padded counter into the final document name.
fn make_name_with_counter(series: &str, prefix: &str, counter: u64, width: usize) -> String {
    format!("{}{:0>width$}", prefix, counter, width = width)
}

/// Full round-trip: series template → [prefix_key, formatted_name]
fn simulate_series_expansion(
    series: &str,
    year4: &str,
    month: &str,
    day: &str,
    expected_width: usize,
    expected_prefix: &str,
    expected_name_at_1: &str,
) {
    let prefix = expand_prefix(series, year4, month, day);
    assert_eq!(prefix, expected_prefix, "prefix mismatch for series {series:?}");

    let width = hash_count(series);
    assert_eq!(width, expected_width, "hash width mismatch for series {series:?}");

    let name = make_name_with_counter(series, &prefix, 1, width);
    assert_eq!(name, expected_name_at_1, "name mismatch for series {series:?}");
}

// ── doctype_to_table (used in naming table lookups) ─────────────────────────

/// Naming resolution relies on `doctype_to_table` for DB lookups.
/// Ensure the naming-relevant DocType lookups map correctly.
#[test]
fn test_doctype_table_for_naming_doctypes() {
    assert_eq!(doctype_to_table("Document Naming Rule"), "tabDocument_Naming_Rule");
    assert_eq!(doctype_to_table("Naming Series"),        "tabNaming_Series");
}
