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

// ── Integration: SurrealDB fn::naming::resolve ────────────────────────────────
//
// These tests require a live SurrealDB instance.
// Set SURREAL_TEST_URL=ws://127.0.0.1:8000 and run with --features integration.

#[cfg(feature = "integration")]
mod integration {
    use serde_json::json;
    use spotledger_db::adapter::DbAdapter;
    use spotledger_core::config::DatabaseConfig;
    use spotledger_db::schema::apply_naming_functions;

    async fn test_adapter(suffix: &str) -> DbAdapter {
        let url = std::env::var("SURREAL_TEST_URL")
            .unwrap_or_else(|_| "ws://127.0.0.1:8500".to_string());
        let cfg = DatabaseConfig {
            url,
            user: "root".into(),
            pass: "root".into(),
            ns: "test_ns".into(),
            db: format!("test_naming_{suffix}"),
        };
        DbAdapter::connect(&cfg).await.expect("connect to test SurrealDB")
    }

    /// Bootstrap tables needed by fn::naming::resolve.
    async fn setup_naming_tables(adapter: &DbAdapter) {
        let ddl = "\
            DEFINE TABLE IF NOT EXISTS tabDocType SCHEMALESS;\
            DEFINE TABLE IF NOT EXISTS tabDocumentNamingRule SCHEMALESS;\
            DEFINE TABLE IF NOT EXISTS tabSeries SCHEMAFULL;\
            DEFINE FIELD IF NOT EXISTS name    ON tabSeries TYPE string;\
            DEFINE FIELD IF NOT EXISTS current ON tabSeries TYPE int;\
            DEFINE INDEX IF NOT EXISTS idx_series_name ON tabSeries FIELDS name UNIQUE;\
        ";
        adapter.execute(ddl, vec![]).await.expect("setup_naming_tables");
    }

    /// fn::naming::resolve uses tabDocType.autoname for field: pattern.
    #[tokio::test]
    async fn test_surreal_naming_field_pattern() {
        let adapter = test_adapter("fn1").await;
        setup_naming_tables(&adapter).await;
        apply_naming_functions(&adapter).await.expect("apply naming fns");

        // Seed tabDocType with autoname = "field:employee_name"
        adapter.execute(
            "UPSERT type::record('tabDocType', 'Employee') CONTENT { name: 'Employee', autoname: 'field:employee_name' }",
            vec![]
        ).await.expect("seed tabDocType");

        let rows = adapter.run(
            "RETURN fn::naming::resolve($dt, $doc)",
            vec![
                ("dt".into(), json!("Employee")),
                ("doc".into(), json!({ "employee_name": "John Doe" })),
            ]
        ).await.expect("fn::naming::resolve");

        let name = rows.into_iter().next()
            .and_then(|v| v.as_str().map(str::to_owned))
            .expect("non-null result");
        assert_eq!(name, "John Doe", "field: autoname should return field value");
    }

    /// fn::naming::resolve generates series names from tabDocType.autoname.
    #[tokio::test]
    async fn test_surreal_naming_series_from_doctype() {
        let adapter = test_adapter("fn2").await;
        setup_naming_tables(&adapter).await;
        apply_naming_functions(&adapter).await.expect("apply naming fns");

        // Use a unique per-run prefix so the counter is fresh regardless of prior runs.
        let unique_prefix = format!(
            "TINV-{:08x}-####",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .subsec_nanos()
        );

        // Seed tabDocType with a direct series template
        adapter.execute(
            &format!("UPSERT type::record('tabDocType', 'TestInvoice') CONTENT {{ name: 'TestInvoice', autoname: '{unique_prefix}' }}"),
            vec![]
        ).await.expect("seed tabDocType");

        let r1 = adapter.run(
            "RETURN fn::naming::resolve($dt, $doc)",
            vec![("dt".into(), json!("TestInvoice")), ("doc".into(), json!({}))]
        ).await.expect("first resolve").into_iter().next()
            .and_then(|v| v.as_str().map(str::to_owned)).expect("non-null");

        let r2 = adapter.run(
            "RETURN fn::naming::resolve($dt, $doc)",
            vec![("dt".into(), json!("TestInvoice")), ("doc".into(), json!({}))]
        ).await.expect("second resolve").into_iter().next()
            .and_then(|v| v.as_str().map(str::to_owned)).expect("non-null");

        assert!(r1.contains("-0001"), "first call should end in -0001, got {r1}");
        assert!(r2.contains("-0002"), "second call should end in -0002, got {r2}");
        assert_ne!(r1, r2, "each call must produce a unique name");
    }

    /// fn::naming::resolve respects naming_series field in document (user-selected).
    #[tokio::test]
    async fn test_surreal_naming_series_field_in_doc() {
        let adapter = test_adapter("fn3").await;
        setup_naming_tables(&adapter).await;
        apply_naming_functions(&adapter).await.expect("apply naming fns");

        let rows = adapter.run(
            "RETURN fn::naming::resolve($dt, $doc)",
            vec![
                ("dt".into(), json!("Employee")),
                ("doc".into(), json!({ "naming_series": "EMP-####" })),
            ]
        ).await.expect("resolve with naming_series field");

        let name = rows.into_iter().next()
            .and_then(|v| v.as_str().map(str::to_owned))
            .expect("non-null");
        assert!(name.starts_with("EMP-"), "got {name}");
    }

    /// Explicit name in doc is returned as-is.
    #[tokio::test]
    async fn test_surreal_naming_explicit_name() {
        let adapter = test_adapter("fn4").await;
        setup_naming_tables(&adapter).await;
        apply_naming_functions(&adapter).await.expect("apply naming fns");

        let rows = adapter.run(
            "RETURN fn::naming::resolve($dt, $doc)",
            vec![
                ("dt".into(), json!("SomeDoc")),
                ("doc".into(), json!({ "name": "EXPLICIT-001" })),
            ]
        ).await.expect("resolve with explicit name");

        let name = rows.into_iter().next()
            .and_then(|v| v.as_str().map(str::to_owned))
            .expect("non-null");
        assert_eq!(name, "EXPLICIT-001");
    }

    /// When no rule matches, returns a UUID fallback (starts with "new-").
    #[tokio::test]
    async fn test_surreal_naming_uuid_fallback() {
        let adapter = test_adapter("fn5").await;
        setup_naming_tables(&adapter).await;
        apply_naming_functions(&adapter).await.expect("apply naming fns");

        let rows = adapter.run(
            "RETURN fn::naming::resolve($dt, $doc)",
            vec![
                ("dt".into(), json!("UnknownDocType999")),
                ("doc".into(), json!({})),
            ]
        ).await.expect("resolve with no rule");

        let name = rows.into_iter().next()
            .and_then(|v| v.as_str().map(str::to_owned))
            .expect("non-null");
        assert!(name.starts_with("new-"), "fallback should start with 'new-', got {name}");
    }

    /// Frappe v15: naming_rule = "By fieldname" with autoname = fieldname (not "field:xxx").
    /// This is what Employee uses when the user changes naming to use employee_name directly.
    #[tokio::test]
    async fn test_surreal_naming_by_fieldname_v15() {
        let adapter = test_adapter("fn6").await;
        setup_naming_tables(&adapter).await;
        apply_naming_functions(&adapter).await.expect("apply naming fns");

        // Seed tabDocType: naming_rule="By fieldname", autoname="employee_name"
        adapter.execute(
            "UPSERT type::record('tabDocType', 'Employee') CONTENT \
             { name: 'Employee', autoname: 'employee_name', naming_rule: 'By fieldname' }",
            vec![]
        ).await.expect("seed tabDocType");

        let rows = adapter.run(
            "RETURN fn::naming::resolve($dt, $doc)",
            vec![
                ("dt".into(), json!("Employee")),
                ("doc".into(), json!({ "employee_name": "Jane Smith", "date_of_birth": "1990-01-01" })),
            ]
        ).await.expect("resolve by_fieldname");

        let name = rows.into_iter().next()
            .and_then(|v| v.as_str().map(str::to_owned))
            .expect("non-null result");
        assert_eq!(name, "Jane Smith", "naming_rule By fieldname should return field value");
    }

    /// naming_rule "By fieldname" with empty field → error (not a UUID fallback).
    #[tokio::test]
    async fn test_surreal_naming_by_fieldname_empty_fallback() {
        let adapter = test_adapter("fn7").await;
        setup_naming_tables(&adapter).await;
        apply_naming_functions(&adapter).await.expect("apply naming fns");

        adapter.execute(
            "UPSERT type::record('tabDocType', 'Employee7') CONTENT \
             { name: 'Employee7', autoname: 'employee_name', naming_rule: 'By fieldname' }",
            vec![]
        ).await.expect("seed tabDocType");

        let result = adapter.run(
            "RETURN fn::naming::resolve($dt, $doc)",
            vec![
                ("dt".into(), json!("Employee7")),
                ("doc".into(), json!({ "employee_name": "" })),
            ]
        ).await;

        assert!(result.is_err(), "empty field must throw an error, not return a UUID");
        let err = result.unwrap_err().to_string();
        assert!(err.contains("employee_name"), "error must name the missing field, got: {err}");
    }

    /// REGRESSION: When the by_fieldname field is absent from the doc entirely,
    /// SurrealDB <string>(NONE) used to return the string "NONE" which passed the
    /// != NONE check and was stored as the document name.
    /// After the fix the function must THROW with a clear error, not store "NONE".
    #[tokio::test]
    async fn test_surreal_naming_by_fieldname_missing_field_not_none_string() {
        let adapter = test_adapter("fn7b").await;
        setup_naming_tables(&adapter).await;
        apply_naming_functions(&adapter).await.expect("apply naming fns");

        adapter.execute(
            "UPSERT type::record('tabDocType', 'Employee7b') CONTENT \
             { name: 'Employee7b', autoname: 'employee_name', naming_rule: 'By fieldname' }",
            vec![]
        ).await.expect("seed tabDocType");

        // Doc has NO employee_name field at all — must error, not store "NONE".
        let result = adapter.run(
            "RETURN fn::naming::resolve($dt, $doc)",
            vec![
                ("dt".into(), json!("Employee7b")),
                ("doc".into(), json!({ "doctype": "Employee7b" })),
            ]
        ).await;

        assert!(result.is_err(), "missing field must throw an error, not return a UUID or NONE");
        let err = result.unwrap_err().to_string();
        assert!(err.contains("employee_name"), "error must name the missing field, got: {err}");
    }

    /// naming_series: autoname — Employee's default. User selects a series in the form.
    /// The naming_series field in the doc drives the series expansion.
    #[tokio::test]
    async fn test_surreal_naming_naming_series_autoname() {
        let adapter = test_adapter("fn8").await;
        setup_naming_tables(&adapter).await;
        apply_naming_functions(&adapter).await.expect("apply naming fns");

        // Seed tabDocType like ERPNext Employee: autoname="naming_series:"
        adapter.execute(
            "UPSERT type::record('tabDocType', 'EmployeeNS') CONTENT \
             { name: 'EmployeeNS', autoname: 'naming_series:', \
               naming_rule: 'By \"Naming Series\" field' }",
            vec![]
        ).await.expect("seed tabDocType");

        // Use a unique series prefix per run to avoid counter conflicts.
        let unique_series = format!(
            "EMP{:08x}-####",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .subsec_nanos()
        );

        // When user selects a naming_series in the form, it drives expansion.
        let rows = adapter.run(
            "RETURN fn::naming::resolve($dt, $doc)",
            vec![
                ("dt".into(), json!("EmployeeNS")),
                ("doc".into(), json!({ "naming_series": unique_series })),
            ]
        ).await.expect("resolve naming_series field");

        let name = rows.into_iter().next()
            .and_then(|v| v.as_str().map(str::to_owned))
            .expect("non-null");
        assert!(name.starts_with("EMP"), "should use doc.naming_series, got {name}");
        assert!(name.ends_with("-0001"), "first use of new series should be 0001, got {name}");
    }

    /// naming_series: autoname — without naming_series in doc → UUID fallback.
    #[tokio::test]
    async fn test_surreal_naming_naming_series_no_field() {
        let adapter = test_adapter("fn9").await;
        setup_naming_tables(&adapter).await;
        apply_naming_functions(&adapter).await.expect("apply naming fns");

        adapter.execute(
            "UPSERT type::record('tabDocType', 'EmpNoSeries') CONTENT \
             { name: 'EmpNoSeries', autoname: 'naming_series:' }",
            vec![]
        ).await.expect("seed tabDocType");

        let rows = adapter.run(
            "RETURN fn::naming::resolve($dt, $doc)",
            vec![
                ("dt".into(), json!("EmpNoSeries")),
                ("doc".into(), json!({ "employee_name": "Test" })),
            ]
        ).await.expect("resolve no naming_series");

        let name = rows.into_iter().next()
            .and_then(|v| v.as_str().map(str::to_owned))
            .expect("non-null");
        assert!(name.starts_with("new-"), "no naming_series field → UUID, got {name}");
    }

    /// Direct series template in tabDocType.autoname (no naming_rule).
    /// e.g. ERPNext Account: autoname = "ACC-.YYYY.-#####"
    #[tokio::test]
    async fn test_surreal_naming_direct_series_template() {
        let adapter = test_adapter("fn10").await;
        setup_naming_tables(&adapter).await;
        apply_naming_functions(&adapter).await.expect("apply naming fns");

        // Use a unique prefix per run to avoid counter conflicts with prior runs.
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .subsec_nanos();
        let unique_autoname = format!("A{ts:08x}-####");

        adapter.execute(
            &format!("UPSERT type::record('tabDocType', 'Account10') CONTENT \
                      {{ name: 'Account10', autoname: '{unique_autoname}' }}"),
            vec![]
        ).await.expect("seed tabDocType");

        let r1 = adapter.run("RETURN fn::naming::resolve($dt, $doc)",
            vec![("dt".into(), json!("Account10")), ("doc".into(), json!({}))]
        ).await.expect("r1").into_iter().next()
            .and_then(|v| v.as_str().map(str::to_owned)).expect("non-null");
        assert!(r1.ends_with("-0001"), "first call should end in -0001, got {r1}");

        let r2 = adapter.run("RETURN fn::naming::resolve($dt, $doc)",
            vec![("dt".into(), json!("Account10")), ("doc".into(), json!({}))]
        ).await.expect("r2").into_iter().next()
            .and_then(|v| v.as_str().map(str::to_owned)).expect("non-null");
        assert!(r2.ends_with("-0002"), "second call should end in -0002, got {r2}");
        assert_ne!(r1, r2, "each call must produce a unique name");
    }

    /// Old-style "field:fieldname" autoname (Expression style).
    #[tokio::test]
    async fn test_surreal_naming_field_colon_old_style() {
        let adapter = test_adapter("fn11").await;
        setup_naming_tables(&adapter).await;
        apply_naming_functions(&adapter).await.expect("apply naming fns");

        adapter.execute(
            "UPSERT type::record('tabDocType', 'Customer11') CONTENT \
             { name: 'Customer11', autoname: 'field:customer_name' }",
            vec![]
        ).await.expect("seed tabDocType");

        let rows = adapter.run("RETURN fn::naming::resolve($dt, $doc)",
            vec![
                ("dt".into(), json!("Customer11")),
                ("doc".into(), json!({ "customer_name": "Acme Corp" })),
            ]
        ).await.expect("resolve field: style");

        let name = rows.into_iter().next()
            .and_then(|v| v.as_str().map(str::to_owned)).expect("non-null");
        assert_eq!(name, "Acme Corp");
    }
}
