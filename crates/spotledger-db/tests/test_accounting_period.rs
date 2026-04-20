//! Integration tests for Accounting Period — sub-periods within a Fiscal Year.
//!
//! Test 1 is a plain CRUD test (no pipeline).
//! Tests 2–4 are pipeline-integrated tests exercising the full finance domain pipeline.
//!
//! ## Running
//! ```sh
//! $env:SURREAL_TEST_URL="ws://127.0.0.1:8000"
//! cargo test -p spotledger-db --features integration --test test_accounting_period -- --test-threads=1
//! ```
//!
//! ## Test plan
//! | # | Test | Verifies |
//! |---|------|---------|
//! | 1 | `test_accounting_period_insert`             | Basic insert + fetch round-trip |
//! | 2 | `test_accounting_period_end_before_start`   | end_date before start_date → error |
//! | 3 | `test_accounting_period_within_fiscal_year` | Period dates must fall within its Fiscal Year |
//! | 4 | `test_accounting_period_no_overlap`         | Two periods for the same company cannot overlap |
//!
//! ## ERPNext parity
//! Mirrors scenarios from `erpnext/accounts/doctype/accounting_period/test_accounting_period.py`.

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
        db: format!("test_accounting_period_{suffix}"),
    };
    DbAdapter::connect(&cfg).await.expect("connect to test SurrealDB")
}

// ── 1. Basic insert round-trip (no pipeline) ─────────────────────────────────

#[tokio::test]
#[cfg_attr(not(feature = "integration"), ignore = "requires live SurrealDB")]
async fn test_accounting_period_insert() {
    let db = test_adapter("insert").await;
    common::define_accounting_period_table(&db).await;

    let doc = json!({
        "name": "Q1 2024",
        "period_name": "Q1 2024",
        "start_date": "2024-01-01T00:00:00Z",
        "end_date":   "2024-03-31T23:59:59Z",
        "fiscal_year": "2024-2025",
        "company": "Test Company",
        "is_closed": false
    });
    delete_doc(&db, "Accounting Period", "Q1 2024").await.ok();
    insert_doc(&db, "Accounting Period", &doc)
        .await
        .expect("insert Accounting Period");

    let fetched = get_doc(&db, "Accounting Period", "Q1 2024")
        .await
        .expect("get Accounting Period");
    assert_eq!(
        fetched.fields.get("fiscal_year").and_then(|v| v.as_str()),
        Some("2024-2025")
    );
    delete_doc(&db, "Accounting Period", "Q1 2024").await.ok();
}

// ── Pipeline helper ───────────────────────────────────────────────────────────

async fn setup_ap_pipeline(suffix: &str) -> DbAdapter {
    let db = common::make_pipeline_db(&format!("ap_{suffix}")).await;
    common::seed_doctype(&db, "Accounting Period", false).await;
    common::define_fiscal_year_table(&db).await;
    common::define_accounting_period_table(&db).await;
    db
}

const COMPANY: &str = "PeriodTestCo";

// ── 2. end_date before start_date → rejected ─────────────────────────────────

/// `fn::validate::accounting_period_dates` must reject a period where
/// `end_date <= start_date`.
#[tokio::test]
#[cfg_attr(not(feature = "integration"), ignore = "requires live SurrealDB")]
async fn test_accounting_period_end_before_start() {
    let db = setup_ap_pipeline("backwards").await;
    delete_doc(&db, "Accounting Period", "AP-BACKWARDS").await.ok();

    insert_doc(
        &db,
        "Accounting Period",
        &json!({
            "name":        "AP-BACKWARDS",
            "period_name": "AP-BACKWARDS",
            "company":     COMPANY,
            "start_date":  "2024-06-30T00:00:00Z", // start AFTER end
            "end_date":    "2024-01-01T00:00:00Z", // end BEFORE start
            "is_closed":   false,
        }),
    )
    .await
    .expect("insert backwards period");

    let result = run_pipeline(
        &db,
        "tabAccounting_Period",
        "AP-BACKWARDS",
        "Accounting Period",
        "save",
    )
    .await;
    assert!(
        result.is_err(),
        "Accounting Period with end before start must fail pipeline"
    );
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.to_lowercase().contains("after") || msg.to_lowercase().contains("end"),
        "error must mention date ordering; got: {msg}"
    );

    delete_doc(&db, "Accounting Period", "AP-BACKWARDS").await.ok();
}

// ── 3. Period dates must fall within an existing Fiscal Year ──────────────────

/// `fn::validate::accounting_period_in_fiscal_year` must reject a period that
/// extends beyond the boundaries of the parent Fiscal Year.
#[tokio::test]
#[cfg_attr(not(feature = "integration"), ignore = "requires live SurrealDB")]
async fn test_accounting_period_within_fiscal_year() {
    let db = setup_ap_pipeline("within_fy").await;

    // FY covers Jan-Dec 2024 only
    insert_doc(
        &db,
        "Fiscal Year",
        &json!({
            "name":            "FY-2024-WITHIN",
            "year_name":       "FY-2024-WITHIN",
            "company":         COMPANY,
            "year_start_date": "2024-01-01T00:00:00Z",
            "year_end_date":   "2024-12-31T23:59:59Z",
        }),
    )
    .await
    .ok();

    // Period that extends into 2025 (outside the FY)
    delete_doc(&db, "Accounting Period", "AP-OUTSIDE").await.ok();
    insert_doc(
        &db,
        "Accounting Period",
        &json!({
            "name":        "AP-OUTSIDE",
            "period_name": "AP-OUTSIDE",
            "company":     COMPANY,
            "start_date":  "2024-10-01T00:00:00Z",
            "end_date":    "2025-03-31T23:59:59Z", // extends past FY end!
            "is_closed":   false,
        }),
    )
    .await
    .expect("insert out-of-range period");

    let result = run_pipeline(
        &db,
        "tabAccounting_Period",
        "AP-OUTSIDE",
        "Accounting Period",
        "save",
    )
    .await;
    assert!(
        result.is_err(),
        "Accounting Period extending beyond its Fiscal Year must fail pipeline"
    );
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.to_lowercase().contains("fiscal year") || msg.to_lowercase().contains("fiscal"),
        "error must mention Fiscal Year; got: {msg}"
    );

    delete_doc(&db, "Accounting Period", "AP-OUTSIDE").await.ok();
}

// ── 4. Two accounting periods for the same company cannot overlap ────────────

/// `fn::validate::accounting_period_in_fiscal_year` also checks period-level overlap.
/// Two periods for the same company whose date ranges intersect must be rejected.
#[tokio::test]
#[cfg_attr(not(feature = "integration"), ignore = "requires live SurrealDB")]
async fn test_accounting_period_no_overlap() {
    let db = setup_ap_pipeline("no_overlap").await;

    // Insert the FY first
    insert_doc(
        &db,
        "Fiscal Year",
        &json!({
            "name":            "FY-2024-NOOVERLAP",
            "year_name":       "FY-2024-NOOVERLAP",
            "company":         COMPANY,
            "year_start_date": "2024-01-01T00:00:00Z",
            "year_end_date":   "2024-12-31T23:59:59Z",
        }),
    )
    .await
    .ok();

    // First period: Q1 2024
    delete_doc(&db, "Accounting Period", "AP-Q1-NO").await.ok();
    insert_doc(
        &db,
        "Accounting Period",
        &json!({
            "name":        "AP-Q1-NO",
            "period_name": "AP-Q1-NO",
            "company":     COMPANY,
            "start_date":  "2024-01-01T00:00:00Z",
            "end_date":    "2024-03-31T23:59:59Z",
            "is_closed":   false,
        }),
    )
    .await
    .expect("insert first period");

    let r1 = run_pipeline(
        &db,
        "tabAccounting_Period",
        "AP-Q1-NO",
        "Accounting Period",
        "save",
    )
    .await;
    assert!(
        matches!(r1, Ok(PipelineResult::Ok)),
        "first period must pass pipeline; got: {r1:?}"
    );

    // Second period that overlaps with Q1 2024 (Feb–April)
    delete_doc(&db, "Accounting Period", "AP-OVERLAP-NO").await.ok();
    insert_doc(
        &db,
        "Accounting Period",
        &json!({
            "name":        "AP-OVERLAP-NO",
            "period_name": "AP-OVERLAP-NO",
            "company":     COMPANY,
            "start_date":  "2024-02-01T00:00:00Z", // overlaps with Q1
            "end_date":    "2024-04-30T23:59:59Z",
            "is_closed":   false,
        }),
    )
    .await
    .expect("insert overlapping period");

    let r2 = run_pipeline(
        &db,
        "tabAccounting_Period",
        "AP-OVERLAP-NO",
        "Accounting Period",
        "save",
    )
    .await;
    assert!(
        r2.is_err(),
        "overlapping Accounting Period must fail pipeline"
    );
    let msg = r2.unwrap_err().to_string();
    assert!(
        msg.to_lowercase().contains("overlap"),
        "error must mention overlap; got: {msg}"
    );

    delete_doc(&db, "Accounting Period", "AP-Q1-NO").await.ok();
    delete_doc(&db, "Accounting Period", "AP-OVERLAP-NO").await.ok();
}

