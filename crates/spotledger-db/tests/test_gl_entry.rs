//! Integration tests for GL Entry — the atomic double-entry bookkeeping record.
//!
//! Tests 1–2 are plain CRUD tests that run without the finance pipeline.
//! Tests 3–7 are pipeline-integrated tests: they load the full spotledger-finance
//! domain functions into an isolated SurrealDB and exercise `fn::pipeline::run`.
//!
//! ## Running
//! ```sh
//! $env:SURREAL_TEST_URL="ws://127.0.0.1:8000"
//! cargo test -p spotledger-db --features integration --test test_gl_entry -- --test-threads=1
//! ```
//!
//! ## Test plan
//! | # | Test | Verifies |
//! |---|------|---------|
//! | 1 | `test_gl_entry_insert`                      | Basic insert + fetch round-trip (no pipeline) |
//! | 2 | `test_gl_entry_debit_only_valid`            | debit=100 credit=0 with valid account + open period passes |
//! | 3 | `test_gl_entry_debit_credit_exclusive`      | debit AND credit both non-zero → pipeline error |
//! | 4 | `test_gl_entry_both_zero_rejected`          | debit=0, credit=0 → pipeline error |
//! | 5 | `test_gl_entry_disabled_account_rejected`   | disabled account → pipeline error |
//! | 6 | `test_gl_entry_posting_date_in_closed_period` | closed Accounting Period → pipeline error |
//! | 7 | `test_gl_entry_cancel_reverses_amounts`     | cancel pipeline swaps debit/credit, sets is_cancelled |
//!
//! ## ERPNext parity
//! Mirrors scenarios from `erpnext/accounts/doctype/gl_entry/test_gl_entry.py`:
//! - `test_round_off_entry` (amounts mutual exclusivity)
//! - `test_outstanding_amount` (valid debit-only entry)
//! - `test_cancel_gl_entry` (cancel reverses)

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
        db: format!("test_gl_entry_{suffix}"),
    };
    DbAdapter::connect(&cfg).await.expect("connect to test SurrealDB")
}

// ── 1. Basic insert round-trip (no pipeline) ─────────────────────────────────

#[tokio::test]
#[cfg_attr(not(feature = "integration"), ignore = "requires live SurrealDB")]
async fn test_gl_entry_insert() {
    let db = test_adapter("insert").await;
    common::define_gl_entry_table(&db).await;

    let doc = json!({
        "name": "ACC-GLE-2024-0001",
        "posting_date": "2024-06-15T00:00:00Z",
        "account": "1000 - Cash - COMP",
        "account_currency": "USD",
        "debit": 1000.00,
        "credit": 0.00,
        "debit_in_account_currency": 1000.00,
        "credit_in_account_currency": 0.00,
        "docstatus": 0,
        "voucher_type": "Journal Entry",
        "voucher_no": "JE-2024-0001"
    });
    delete_doc(&db, "GL Entry", "ACC-GLE-2024-0001").await.ok();
    insert_doc(&db, "GL Entry", &doc).await.expect("insert GL Entry");

    let fetched = get_doc(&db, "GL Entry", "ACC-GLE-2024-0001")
        .await
        .expect("get GL Entry");
    let debit = common::parse_decimal(fetched.fields.get("debit").unwrap_or(&json!(0)));
    assert!(
        (debit - 1000.0).abs() < 0.001,
        "debit must be 1000.00, got {debit}"
    );
    delete_doc(&db, "GL Entry", "ACC-GLE-2024-0001").await.ok();
}

// ── Pipeline test helpers ─────────────────────────────────────────────────────

/// Build a fresh pipeline DB, define all tables, seed "GL Entry" doctype,
/// and insert prerequisite account + period records.
/// Returns (db, company_name).
async fn setup_gl_pipeline(suffix: &str, company: &str) -> DbAdapter {
    let db = common::make_pipeline_db(&format!("gle_{suffix}")).await;
    common::seed_doctype(&db, "GL Entry", true).await;
    common::define_account_table(&db).await;
    common::define_fiscal_year_table(&db).await;
    common::define_accounting_period_table(&db).await;
    common::define_gl_entry_table(&db).await;
    common::insert_valid_account(&db, "Cash").await;
    common::insert_valid_account(&db, "Revenue").await;
    common::insert_open_period_2024(&db, company).await;
    db
}

const COMPANY: &str = "TestCo";
const POSTING_DATE: &str = "2024-06-15T00:00:00Z";
const OLD_POSTING_DATE: &str = "2023-06-15T00:00:00Z"; // inside closed 2023 period

// ── 2. Valid debit-only entry passes the save pipeline ────────────────────────

/// A well-formed GL entry (debit=100, credit=0, valid account, open period) must
/// complete the full save pipeline successfully.
#[tokio::test]
#[cfg_attr(not(feature = "integration"), ignore = "requires live SurrealDB")]
async fn test_gl_entry_debit_only_valid() {
    let db = setup_gl_pipeline("valid_debit", COMPANY).await;
    delete_doc(&db, "GL Entry", "GLE-VALID").await.ok();

    insert_doc(
        &db,
        "GL Entry",
        &json!({
            "name":         "GLE-VALID",
            "account":      "Cash",
            "company":      COMPANY,
            "posting_date": POSTING_DATE,
            "debit":        100.0,
            "credit":       0.0,
            "debit_in_account_currency":  100.0,
            "credit_in_account_currency": 0.0,
            "docstatus":    0,
        }),
    )
    .await
    .expect("insert GL entry");

    let result = run_pipeline(&db, "tabGL_Entry", "GLE-VALID", "GL Entry", "save").await;
    assert!(
        matches!(result, Ok(PipelineResult::Ok)),
        "valid GL entry must pass save pipeline; got: {result:?}"
    );

    // Compute stage should have derived fiscal_year and accounting_period
    let fetched = get_doc(&db, "GL Entry", "GLE-VALID").await.unwrap();
    let fy = common::field_str(&fetched, "fiscal_year");
    assert!(
        fy.contains("2024"),
        "fiscal_year must be derived for 2024 posting date; got: {fy}"
    );

    delete_doc(&db, "GL Entry", "GLE-VALID").await.ok();
}

// ── 3. Both debit and credit non-zero → rejected ───────────────────────────────

/// ERPNext parity: `test_round_off_entry`.
/// `gl_entry_validate_amounts` must reject an entry where both debit and credit
/// are non-zero — that is not valid double-entry bookkeeping.
#[tokio::test]
#[cfg_attr(not(feature = "integration"), ignore = "requires live SurrealDB")]
async fn test_gl_entry_debit_credit_exclusive() {
    let db = setup_gl_pipeline("exclusive", COMPANY).await;
    delete_doc(&db, "GL Entry", "GLE-BOTH").await.ok();

    insert_doc(
        &db,
        "GL Entry",
        &json!({
            "name":         "GLE-BOTH",
            "account":      "Cash",
            "company":      COMPANY,
            "posting_date": POSTING_DATE,
            "debit":        100.0,
            "credit":       50.0,  // <- also non-zero: invalid
            "debit_in_account_currency":  100.0,
            "credit_in_account_currency": 50.0,
            "docstatus":    0,
        }),
    )
    .await
    .expect("insert GL entry");

    let result = run_pipeline(&db, "tabGL_Entry", "GLE-BOTH", "GL Entry", "save").await;
    assert!(result.is_err(), "both debit+credit non-zero must fail pipeline");
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.to_lowercase().contains("both") || msg.to_lowercase().contains("cannot"),
        "error must mention mutual exclusivity; got: {msg}"
    );

    delete_doc(&db, "GL Entry", "GLE-BOTH").await.ok();
}

// ── 4. Both debit and credit zero → rejected ──────────────────────────────────

/// A GL entry with no monetary movement is meaningless and must be rejected.
#[tokio::test]
#[cfg_attr(not(feature = "integration"), ignore = "requires live SurrealDB")]
async fn test_gl_entry_both_zero_rejected() {
    let db = setup_gl_pipeline("both_zero", COMPANY).await;
    delete_doc(&db, "GL Entry", "GLE-ZERO").await.ok();

    insert_doc(
        &db,
        "GL Entry",
        &json!({
            "name":         "GLE-ZERO",
            "account":      "Cash",
            "company":      COMPANY,
            "posting_date": POSTING_DATE,
            "debit":        0.0,
            "credit":       0.0,  // <- both zero: invalid
            "docstatus":    0,
        }),
    )
    .await
    .expect("insert GL entry");

    let result = run_pipeline(&db, "tabGL_Entry", "GLE-ZERO", "GL Entry", "save").await;
    assert!(result.is_err(), "both debit and credit zero must fail pipeline");
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.to_lowercase().contains("non-zero") || msg.to_lowercase().contains("zero"),
        "error must mention non-zero requirement; got: {msg}"
    );

    delete_doc(&db, "GL Entry", "GLE-ZERO").await.ok();
}

// ── 5. Disabled account → rejected ────────────────────────────────────────────

/// gl_entry_validate_account must reject a GL entry that references a disabled account.
#[tokio::test]
#[cfg_attr(not(feature = "integration"), ignore = "requires live SurrealDB")]
async fn test_gl_entry_disabled_account_rejected() {
    let db = setup_gl_pipeline("disabled_acct", COMPANY).await;
    common::insert_disabled_account(&db, "Disabled-Account").await;
    delete_doc(&db, "GL Entry", "GLE-DISABLED").await.ok();

    insert_doc(
        &db,
        "GL Entry",
        &json!({
            "name":         "GLE-DISABLED",
            "account":      "Disabled-Account",
            "company":      COMPANY,
            "posting_date": POSTING_DATE,
            "debit":        100.0,
            "credit":       0.0,
            "docstatus":    0,
        }),
    )
    .await
    .expect("insert GL entry with disabled account");

    let result = run_pipeline(&db, "tabGL_Entry", "GLE-DISABLED", "GL Entry", "save").await;
    assert!(
        result.is_err(),
        "GL entry with disabled account must fail pipeline"
    );
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.to_lowercase().contains("disabled"),
        "error must mention 'disabled'; got: {msg}"
    );

    delete_doc(&db, "GL Entry", "GLE-DISABLED").await.ok();
}

// ── 6. Closed Accounting Period → rejected ────────────────────────────────────

/// ERPNext parity: period-closure check in `test_accounting_period.py`.
/// Posting a GL entry into a closed period must be rejected by the pipeline.
#[tokio::test]
#[cfg_attr(not(feature = "integration"), ignore = "requires live SurrealDB")]
async fn test_gl_entry_posting_date_in_closed_period() {
    let db = setup_gl_pipeline("closed_period", COMPANY).await;
    // Also insert a closed FY + AP covering 2023
    common::insert_closed_period_2023(&db, COMPANY).await;
    delete_doc(&db, "GL Entry", "GLE-CLOSED").await.ok();

    insert_doc(
        &db,
        "GL Entry",
        &json!({
            "name":         "GLE-CLOSED",
            "account":      "Cash",
            "company":      COMPANY,
            "posting_date": OLD_POSTING_DATE, // inside closed 2023 period
            "debit":        100.0,
            "credit":       0.0,
            "docstatus":    0,
        }),
    )
    .await
    .expect("insert GL entry in closed period");

    let result = run_pipeline(&db, "tabGL_Entry", "GLE-CLOSED", "GL Entry", "save").await;
    assert!(
        result.is_err(),
        "posting into a closed period must fail pipeline"
    );
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.to_lowercase().contains("closed") || msg.to_lowercase().contains("period"),
        "error must mention closed period; got: {msg}"
    );

    delete_doc(&db, "GL Entry", "GLE-CLOSED").await.ok();
}

// ── 7. Cancel pipeline reverses debit/credit and sets is_cancelled ────────────

/// ERPNext parity: `test_cancel_gl_entry` in `test_gl_entry.py`.
/// Running the cancel pipeline on a submitted GL entry must swap debit↔credit
/// and set `is_cancelled = true` via `fn::on_cancel::gl_entry_reverse`.
#[tokio::test]
#[cfg_attr(not(feature = "integration"), ignore = "requires live SurrealDB")]
async fn test_gl_entry_cancel_reverses_amounts() {
    let db = setup_gl_pipeline("cancel_rev", COMPANY).await;
    delete_doc(&db, "GL Entry", "GLE-CANCEL").await.ok();

    // Insert a submitted GL entry (docstatus=1, debit=100, credit=0)
    insert_doc(
        &db,
        "GL Entry",
        &json!({
            "name":                         "GLE-CANCEL",
            "account":                      "Cash",
            "company":                      COMPANY,
            "posting_date":                 POSTING_DATE,
            "debit":                        100.0,
            "credit":                       0.0,
            "debit_in_account_currency":    100.0,
            "credit_in_account_currency":   0.0,
            "docstatus":                    1,   // submitted
            "is_cancelled":                 false,
        }),
    )
    .await
    .expect("insert submitted GL entry");

    let result =
        run_pipeline(&db, "tabGL_Entry", "GLE-CANCEL", "GL Entry", "cancel").await;
    assert!(
        matches!(result, Ok(PipelineResult::Ok)),
        "cancel pipeline must succeed on submitted GL entry; got: {result:?}"
    );

    let fetched = get_doc(&db, "GL Entry", "GLE-CANCEL")
        .await
        .expect("fetch cancelled GL entry");

    let debit  = common::parse_decimal(fetched.fields.get("debit").unwrap_or(&json!(0)));
    let credit = common::parse_decimal(fetched.fields.get("credit").unwrap_or(&json!(0)));
    let is_cancelled = fetched.fields.get("is_cancelled")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let docstatus = fetched.fields.get("docstatus")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);

    assert!(
        (debit - 0.0).abs() < 0.001,
        "after cancel: debit must be 0 (was credit 0); got {debit}"
    );
    assert!(
        (credit - 100.0).abs() < 0.001,
        "after cancel: credit must be 100 (was debit 100); got {credit}"
    );
    assert!(is_cancelled, "after cancel: is_cancelled must be true");
    assert_eq!(docstatus, 2, "after cancel: docstatus must be 2");

    delete_doc(&db, "GL Entry", "GLE-CANCEL").await.ok();
}

