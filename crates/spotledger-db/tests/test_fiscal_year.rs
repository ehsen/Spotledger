//! Integration tests for Fiscal Year — the annual accounting period container.
//!
//! Test 1 is a plain CRUD test (no pipeline).
//! Tests 2–4 are pipeline-integrated tests that load spotledger-finance functions.
//!
//! ## Running
//! ```sh
//! $env:SURREAL_TEST_URL="ws://127.0.0.1:8000"
//! cargo test -p spotledger-db --features integration --test test_fiscal_year -- --test-threads=1
//! ```
//!
//! ## Test plan
//! | # | Test | Verifies |
//! |---|------|---------|
//! | 1 | `test_fiscal_year_insert`          | Basic insert + fetch round-trip |
//! | 2 | `test_fiscal_year_date_order`       | year_end_date must be after year_start_date |
//! | 3 | `test_fiscal_year_overlap_rejected` | Two Fiscal Years for same company cannot overlap |
//! | 4 | `test_fiscal_year_naming`           | Auto-derived name from start/end years |
//!
//! ## ERPNext parity
//! Mirrors `erpnext/accounts/doctype/fiscal_year/test_fiscal_year.py`.

#![cfg(feature = "integration")]

#[path = "common/mod.rs"]
mod common;

use serde_json::json;
use spotledger_core::config::DatabaseConfig;
use spotledger_db::{
    document::{delete_doc, get_doc, insert_doc},
    pipeline::{run_pipeline, PipelineResult},
    DbAdapter,
};

// ── Simple (no-pipeline) test adapter ────────────────────────────────────────

async fn test_adapter(suffix: &str) -> DbAdapter {
    let url = std::env::var("SURREAL_TEST_URL")
        .unwrap_or_else(|_| "ws://127.0.0.1:8000".to_string());
    let cfg = DatabaseConfig {
        url,
        user: "root".into(),
        pass: "root".into(),
        ns: "test_ns".into(),
        db: format!("test_fiscal_year_{suffix}"),
    };
    DbAdapter::connect(&cfg).await.expect("connect to test SurrealDB")
}

// ── 1. Basic insert round-trip (no pipeline) ─────────────────────────────────

#[tokio::test]
#[cfg_attr(not(feature = "integration"), ignore = "requires live SurrealDB")]
async fn test_fiscal_year_insert() {
    let db = test_adapter("insert").await;
    common::define_fiscal_year_table(&db).await;

    let doc = json!({
        "name": "2024-2025",
        "year_name": "2024-2025",
        "year_start_date": "2024-01-01T00:00:00Z",
        "year_end_date":   "2024-12-31T23:59:59Z"
    });
    delete_doc(&db, "Fiscal Year", "2024-2025").await.ok();
    insert_doc(&db, "Fiscal Year", &doc).await.expect("insert Fiscal Year");

    let fetched = get_doc(&db, "Fiscal Year", "2024-2025")
        .await
        .expect("get Fiscal Year");
    assert!(
        fetched.fields.contains_key("year_start_date"),
        "year_start_date must be present"
    );
    delete_doc(&db, "Fiscal Year", "2024-2025").await.ok();
}

// ── Pipeline helper ───────────────────────────────────────────────────────────

async fn setup_fy_pipeline(suffix: &str) -> DbAdapter {
    let db = common::make_pipeline_db(&format!("fy_{suffix}")).await;
    common::seed_doctype(&db, "Fiscal Year", false).await;
    common::define_fiscal_year_table(&db).await;
    db
}

// ── 2. year_end_date must be after year_start_date ────────────────────────────

/// ERPNext parity: fiscal year with end before start must be rejected.
/// `fn::validate::fiscal_year_dates` enforces `year_end_date > year_start_date`.
#[tokio::test]
#[cfg_attr(not(feature = "integration"), ignore = "requires live SurrealDB")]
async fn test_fiscal_year_date_order() {
    let db = setup_fy_pipeline("date_order").await;
    delete_doc(&db, "Fiscal Year", "FY-BACKWARDS").await.ok();

    insert_doc(
        &db,
        "Fiscal Year",
        &json!({
            "name":            "FY-BACKWARDS",
            "year_name":       "FY-BACKWARDS",
            "year_start_date": "2024-12-31T00:00:00Z",  // start AFTER end
            "year_end_date":   "2024-01-01T00:00:00Z",  // end BEFORE start
        }),
    )
    .await
    .expect("insert backwards FY");

    let result =
        run_pipeline(&db, "tabFiscal_Year", "FY-BACKWARDS", "Fiscal Year", "save").await;
    assert!(
        result.is_err(),
        "FY with end before start must fail save pipeline"
    );
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.to_lowercase().contains("after") || msg.to_lowercase().contains("end"),
        "error must mention date ordering; got: {msg}"
    );

    delete_doc(&db, "Fiscal Year", "FY-BACKWARDS").await.ok();
}

// ── 3. Overlapping Fiscal Years for the same company → rejected ──────────────

/// ERPNext parity: two fiscal years with the same company and overlapping date ranges
/// must not coexist.  `fn::validate::fiscal_year_no_overlap` detects this.
#[tokio::test]
#[cfg_attr(not(feature = "integration"), ignore = "requires live SurrealDB")]
async fn test_fiscal_year_overlap_rejected() {
    let db = setup_fy_pipeline("overlap").await;
    let company = "OverlapCo";

    // Clean up any leftovers from previous runs before starting
    delete_doc(&db, "Fiscal Year", "FY-2023").await.ok();
    delete_doc(&db, "Fiscal Year", "FY-OVERLAP").await.ok();

    // Insert first fiscal year: 2023-01-01 to 2023-12-31
    insert_doc(
        &db,
        "Fiscal Year",
        &json!({
            "name":            "FY-2023",
            "year_name":       "FY-2023",
            "company":         company,
            "year_start_date": "2023-01-01T00:00:00Z",
            "year_end_date":   "2023-12-31T23:59:59Z",
        }),
    )
    .await
    .expect("insert first FY");

    // First FY must pass validation
    let r1 = run_pipeline(&db, "tabFiscal_Year", "FY-2023", "Fiscal Year", "save").await;
    assert!(
        matches!(r1, Ok(PipelineResult::Ok)),
        "first FY must pass pipeline; got: {r1:?}"
    );

    // Insert second fiscal year that overlaps: 2023-07-01 to 2024-06-30
    delete_doc(&db, "Fiscal Year", "FY-OVERLAP").await.ok();
    insert_doc(
        &db,
        "Fiscal Year",
        &json!({
            "name":            "FY-OVERLAP",
            "year_name":       "FY-OVERLAP",
            "company":         company,
            "year_start_date": "2023-07-01T00:00:00Z", // overlaps with FY-2023
            "year_end_date":   "2024-06-30T23:59:59Z",
        }),
    )
    .await
    .expect("insert overlapping FY");

    let r2 =
        run_pipeline(&db, "tabFiscal_Year", "FY-OVERLAP", "Fiscal Year", "save").await;
    assert!(
        r2.is_err(),
        "overlapping FY for same company must fail save pipeline"
    );
    let msg = r2.unwrap_err().to_string();
    assert!(
        msg.to_lowercase().contains("overlap") || msg.to_lowercase().contains("overlaps"),
        "error must mention overlap; got: {msg}"
    );

    delete_doc(&db, "Fiscal Year", "FY-2023").await.ok();
    delete_doc(&db, "Fiscal Year", "FY-OVERLAP").await.ok();
}

// ── 4. Naming ─────────────────────────────────────────────────────────────────

/// Naming convention for Fiscal Year is defined in the doctype JSON (autoname).
/// This test verifies the naming convention at the infrastructure level without
/// requiring a full `install-app` (which is covered by the naming integration tests).
///
/// NOTE: Full naming-by-field-from-dates requires `fn::naming::resolve` to be
/// configured with the finance app's naming rules, which in turn requires
/// `install-app spotledger-finance` to have run.  This is tested separately in
/// the naming integration tests (`test_naming.rs`).
#[tokio::test]
#[cfg_attr(not(feature = "integration"), ignore = "requires live SurrealDB")]
async fn test_fiscal_year_naming() {
    // Verify the basic insert still assigns the name we provide (field-based naming
    // is exercised fully in `test_naming.rs::test_surreal_naming_by_fieldname_*`).
    let db = setup_fy_pipeline("naming").await;
    delete_doc(&db, "Fiscal Year", "2025-2026").await.ok();

    insert_doc(
        &db,
        "Fiscal Year",
        &json!({
            "name":            "2025-2026",
            "year_name":       "2025-2026",
            "year_start_date": "2025-01-01T00:00:00Z",
            "year_end_date":   "2025-12-31T23:59:59Z",
        }),
    )
    .await
    .expect("insert named FY");

    let fetched = get_doc(&db, "Fiscal Year", "2025-2026")
        .await
        .expect("get named FY");
    assert_eq!(fetched.name, "2025-2026", "document name must match");

    delete_doc(&db, "Fiscal Year", "2025-2026").await.ok();
}

