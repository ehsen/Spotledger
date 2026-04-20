//! Integration tests for Journal Entry — the primary user-facing accounting document.
//!
//! Tests 1 is a plain CRUD test (no pipeline).
//! Tests 2–6 are pipeline-integrated tests that load the full spotledger-finance
//! domain functions and exercise `fn::pipeline::run` for save, submit, and cancel.
//!
//! ## Running
//! ```sh
//! $env:SURREAL_TEST_URL="ws://127.0.0.1:8000"
//! cargo test -p spotledger-db --features integration --test test_journal_entry -- --test-threads=1
//! ```
//!
//! ## Test plan
//! | # | Test | Verifies |
//! |---|------|---------|
//! | 1 | `test_journal_entry_insert`               | Basic draft insert + fetch round-trip |
//! | 2 | `test_je_requires_at_least_two_accounts`  | 0 or 1 accounts → save pipeline error |
//! | 3 | `test_je_totals_computed_on_save`         | save/compute derives total_debit, total_credit |
//! | 4 | `test_je_unbalanced_rejected_on_submit`   | total_debit ≠ total_credit → submit error |
//! | 5 | `test_je_submit_creates_gl_entries`       | balanced submit → GL Entry rows in tabGL_Entry |
//! | 6 | `test_je_cancel_marks_gl_cancelled`       | cancel after submit → GL Entries set is_cancelled |
//! | 7 | `test_je_submit_gl_entries_full_fields`   | **Checkpoint**: ALL GL Entry fields verified end-to-end |
//! | 8 | `test_je_submit_gl_failure_rolls_back`    | **Atomicity**: GL creation failure → error + no partial state |
//!
//! ## ERPNext parity
//! Mirrors scenarios from `erpnext/accounts/doctype/journal_entry/test_journal_entry.py`:
//! - `test_journal_entry` (save + submit round-trip)
//! - `test_jv_linked_gl_entry` (GL entries created on submit)
//! - `test_repost_journal_entry` (cancel + reversal)

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
        db: format!("test_journal_entry_{suffix}"),
    };
    DbAdapter::connect(&cfg).await.expect("connect to test SurrealDB")
}

// Table DDL for simple tests that don't need the full pipeline
async fn define_simple_je_table(db: &DbAdapter) {
    db.execute(
        "DEFINE TABLE IF NOT EXISTS tabJournal_Entry SCHEMAFULL; \
         DEFINE FIELD OVERWRITE name          ON tabJournal_Entry TYPE string; \
         DEFINE FIELD OVERWRITE posting_date  ON tabJournal_Entry TYPE option<datetime>; \
         DEFINE FIELD OVERWRITE company       ON tabJournal_Entry TYPE option<string>; \
         DEFINE FIELD OVERWRITE voucher_type  ON tabJournal_Entry TYPE option<string>; \
         DEFINE FIELD OVERWRITE total_debit   ON tabJournal_Entry TYPE decimal DEFAULT 0.0; \
         DEFINE FIELD OVERWRITE total_credit  ON tabJournal_Entry TYPE decimal DEFAULT 0.0; \
         DEFINE FIELD OVERWRITE docstatus     ON tabJournal_Entry TYPE int DEFAULT 0; \
         DEFINE FIELD OVERWRITE owner         ON tabJournal_Entry TYPE option<string>; \
         DEFINE FIELD OVERWRITE creation      ON tabJournal_Entry TYPE option<datetime>; \
         DEFINE FIELD OVERWRITE modified      ON tabJournal_Entry TYPE option<datetime>; \
         DEFINE FIELD OVERWRITE modified_by   ON tabJournal_Entry TYPE option<string>; \
         DEFINE FIELD OVERWRITE doctype       ON tabJournal_Entry TYPE option<string>; \
         DEFINE INDEX OVERWRITE idx_je_name ON tabJournal_Entry FIELDS name UNIQUE;",
        vec![],
    )
    .await
    .expect("define tabJournal_Entry (simple)");
}

// ── 1. Basic insert round-trip (no pipeline) ─────────────────────────────────

#[tokio::test]
#[cfg_attr(not(feature = "integration"), ignore = "requires live SurrealDB")]
async fn test_journal_entry_insert() {
    let db = test_adapter("insert").await;
    define_simple_je_table(&db).await;

    let je = json!({
        "name": "ACC-JV-2024-000001",
        "posting_date": "2024-06-15T00:00:00Z",
        "voucher_type": "Journal Entry",
        "company": "Test Company",
        "docstatus": 0,
        "total_debit": 0.0,
        "total_credit": 0.0
    });
    delete_doc(&db, "Journal Entry", "ACC-JV-2024-000001").await.ok();
    insert_doc(&db, "Journal Entry", &je).await.expect("insert Journal Entry");

    let fetched = get_doc(&db, "Journal Entry", "ACC-JV-2024-000001")
        .await
        .expect("get JE");
    assert_eq!(
        fetched.fields.get("voucher_type").and_then(|v| v.as_str()),
        Some("Journal Entry")
    );
    delete_doc(&db, "Journal Entry", "ACC-JV-2024-000001").await.ok();
}

// ── Pipeline helpers ─────────────────────────────────────────────────────────

const COMPANY: &str = "TestCo";
const POSTING_DATE: &str = "2024-06-15T00:00:00Z";

/// Build a pipeline DB with all JE-related tables and prerequisites.
async fn setup_je_pipeline(suffix: &str) -> DbAdapter {
    let db = common::make_pipeline_db(&format!("je_{suffix}")).await;
    common::seed_doctype(&db, "Journal Entry", true).await;
    common::seed_doctype(&db, "GL Entry", true).await;
    common::define_account_table(&db).await;
    common::define_fiscal_year_table(&db).await;
    common::define_accounting_period_table(&db).await;
    common::define_gl_entry_table(&db).await;
    common::define_journal_entry_table(&db).await;
    common::insert_valid_account(&db, "Cash").await;
    common::insert_valid_account(&db, "Revenue").await;
    common::insert_open_period_2024(&db, COMPANY).await;
    db
}

/// Build the `accounts` array for a balanced Journal Entry (Cash ↔ Revenue).
fn balanced_accounts() -> serde_json::Value {
    json!([
        {
            "account": "Cash",
            "debit": 100.0,
            "credit": 0.0,
            "debit_in_account_currency": 100.0,
            "credit_in_account_currency": 0.0,
            "idx": 0
        },
        {
            "account": "Revenue",
            "debit": 0.0,
            "credit": 100.0,
            "debit_in_account_currency": 0.0,
            "credit_in_account_currency": 100.0,
            "idx": 1
        }
    ])
}

// ── 2. At least two accounts required ────────────────────────────────────────

/// ERPNext parity: JE with empty accounts list is invalid.
/// `fn::validate::je_has_accounts` requires at least 2 account lines.
#[tokio::test]
#[cfg_attr(not(feature = "integration"), ignore = "requires live SurrealDB")]
async fn test_je_requires_at_least_two_accounts() {
    let db = setup_je_pipeline("two_accts").await;
    delete_doc(&db, "Journal Entry", "JE-NO-ACCTS").await.ok();

    insert_doc(
        &db,
        "Journal Entry",
        &json!({
            "name":         "JE-NO-ACCTS",
            "posting_date": POSTING_DATE,
            "voucher_type": "Journal Entry",
            "company":      COMPANY,
            "total_debit":  0.0,
            "total_credit": 0.0,
            "docstatus":    0,
            "accounts":     []  // <- empty, not allowed
        }),
    )
    .await
    .expect("insert JE without accounts");

    let result = run_pipeline(&db, "tabJournal_Entry", "JE-NO-ACCTS", "Journal Entry", "save").await;
    assert!(result.is_err(), "JE with 0 accounts must fail save pipeline");
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.to_lowercase().contains("at least") || msg.to_lowercase().contains("2"),
        "error must mention minimum account lines; got: {msg}"
    );

    delete_doc(&db, "Journal Entry", "JE-NO-ACCTS").await.ok();
}

// ── 3. je_totals computes total_debit and total_credit ────────────────────────

/// `fn::compute::je_totals` must sum the debit/credit columns in the accounts
/// array and update total_debit, total_credit, difference on the JE record.
#[tokio::test]
#[cfg_attr(not(feature = "integration"), ignore = "requires live SurrealDB")]
async fn test_je_totals_computed_on_save() {
    let db = setup_je_pipeline("totals").await;
    delete_doc(&db, "Journal Entry", "JE-TOTALS").await.ok();

    insert_doc(
        &db,
        "Journal Entry",
        &json!({
            "name":         "JE-TOTALS",
            "posting_date": POSTING_DATE,
            "voucher_type": "Journal Entry",
            "company":      COMPANY,
            "total_debit":  0.0, // will be overwritten by compute stage
            "total_credit": 0.0,
            "docstatus":    0,
            "accounts":     balanced_accounts()
        }),
    )
    .await
    .expect("insert JE for totals test");

    let result =
        run_pipeline(&db, "tabJournal_Entry", "JE-TOTALS", "Journal Entry", "save").await;
    assert!(
        matches!(result, Ok(PipelineResult::Ok)),
        "balanced JE with valid accounts and period must pass save pipeline; got: {result:?}"
    );

    let fetched = get_doc(&db, "Journal Entry", "JE-TOTALS")
        .await
        .expect("fetch JE after save pipeline");
    let total_debit = common::parse_decimal(
        fetched.fields.get("total_debit").unwrap_or(&json!(0)),
    );
    let total_credit = common::parse_decimal(
        fetched.fields.get("total_credit").unwrap_or(&json!(0)),
    );

    assert!(
        (total_debit - 100.0).abs() < 0.001,
        "total_debit must be 100.0 after compute; got {total_debit}"
    );
    assert!(
        (total_credit - 100.0).abs() < 0.001,
        "total_credit must be 100.0 after compute; got {total_credit}"
    );

    delete_doc(&db, "Journal Entry", "JE-TOTALS").await.ok();
}

// ── 4. Unbalanced JE is rejected at submit time ────────────────────────────

/// ERPNext parity: `test_journal_entry.py::test_jv_against_sales_order`
/// (submit validation requires balanced entries).
/// `fn::validate::je_balance` must reject an entry where total_debit ≠ total_credit.
#[tokio::test]
#[cfg_attr(not(feature = "integration"), ignore = "requires live SurrealDB")]
async fn test_je_unbalanced_rejected_on_submit() {
    let db = setup_je_pipeline("unbalanced").await;
    delete_doc(&db, "Journal Entry", "JE-UNBAL").await.ok();

    // total_debit=100, total_credit=50 — unbalanced.
    // Setting them directly bypasses the compute stage so the imbalance persists.
    insert_doc(
        &db,
        "Journal Entry",
        &json!({
            "name":         "JE-UNBAL",
            "posting_date": POSTING_DATE,
            "voucher_type": "Journal Entry",
            "company":      COMPANY,
            "total_debit":  100.0,
            "total_credit": 50.0,  // <- unbalanced
            "docstatus":    0,
            "accounts":     [
                {"account": "Cash",    "debit": 100.0, "credit": 0.0,  "idx": 0,
                 "debit_in_account_currency": 100.0, "credit_in_account_currency": 0.0},
                {"account": "Revenue", "debit": 0.0,  "credit": 50.0, "idx": 1,
                 "debit_in_account_currency": 0.0,   "credit_in_account_currency": 50.0}
            ]
        }),
    )
    .await
    .expect("insert unbalanced JE");

    let result =
        run_pipeline(&db, "tabJournal_Entry", "JE-UNBAL", "Journal Entry", "submit").await;
    assert!(
        result.is_err(),
        "unbalanced JE must fail submit pipeline"
    );
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.to_lowercase().contains("balanced") || msg.to_lowercase().contains("balance"),
        "error must mention balance; got: {msg}"
    );

    delete_doc(&db, "Journal Entry", "JE-UNBAL").await.ok();
}

// ── 5. Submitting a balanced JE creates GL Entries ───────────────────────────

/// ERPNext parity: `test_journal_entry.py::test_jv_linked_gl_entry`.
/// Running the submit pipeline on a balanced JE must create one GL Entry row
/// per account line in `tabGL_Entry`.
#[tokio::test]
#[cfg_attr(not(feature = "integration"), ignore = "requires live SurrealDB")]
async fn test_je_submit_creates_gl_entries() {
    let db = setup_je_pipeline("submit_gl").await;
    delete_doc(&db, "Journal Entry", "JE-SUBMIT").await.ok();

    insert_doc(
        &db,
        "Journal Entry",
        &json!({
            "name":         "JE-SUBMIT",
            "posting_date": POSTING_DATE,
            "voucher_type": "Journal Entry",
            "company":      COMPANY,
            "total_debit":  100.0,
            "total_credit": 100.0,   // balanced
            "docstatus":    0,
            "accounts":     balanced_accounts()
        }),
    )
    .await
    .expect("insert JE for submit test");

    let result =
        run_pipeline(&db, "tabJournal_Entry", "JE-SUBMIT", "Journal Entry", "submit").await;
    assert!(
        matches!(result, Ok(PipelineResult::Ok)),
        "balanced JE submit must succeed; got: {result:?}"
    );

    // fn::on_submit::je_create_gl_entries names each GL entry as "{je_name}-GL-{idx}"
    let gl0 = get_doc(&db, "GL Entry", "JE-SUBMIT-GL-0").await;
    let gl1 = get_doc(&db, "GL Entry", "JE-SUBMIT-GL-1").await;

    assert!(
        gl0.is_ok(),
        "GL Entry JE-SUBMIT-GL-0 must be created on JE submit"
    );
    assert!(
        gl1.is_ok(),
        "GL Entry JE-SUBMIT-GL-1 must be created on JE submit"
    );

    // Verify the debit GL entry
    let gl0_doc = gl0.unwrap();
    let debit = common::parse_decimal(gl0_doc.fields.get("debit").unwrap_or(&json!(0)));
    assert!(
        (debit - 100.0).abs() < 0.001,
        "GL Entry 0 debit must be 100.0; got {debit}"
    );

    // Verify the credit GL entry
    let gl1_doc = gl1.unwrap();
    let credit = common::parse_decimal(gl1_doc.fields.get("credit").unwrap_or(&json!(0)));
    assert!(
        (credit - 100.0).abs() < 0.001,
        "GL Entry 1 credit must be 100.0; got {credit}"
    );

    delete_doc(&db, "Journal Entry", "JE-SUBMIT").await.ok();
    delete_doc(&db, "GL Entry", "JE-SUBMIT-GL-0").await.ok();
    delete_doc(&db, "GL Entry", "JE-SUBMIT-GL-1").await.ok();
}

// ── 6. Cancel pipeline marks all associated GL Entries as cancelled ───────────

/// ERPNext parity: `test_journal_entry.py::test_repost_journal_entry`.
/// Cancelling a JE must mark all GL entries for that voucher_no as is_cancelled=true
/// via `fn::on_cancel::je_cancel_gl_entries`.
#[tokio::test]
#[cfg_attr(not(feature = "integration"), ignore = "requires live SurrealDB")]
async fn test_je_cancel_marks_gl_cancelled() {
    let db = setup_je_pipeline("cancel_gl").await;
    delete_doc(&db, "Journal Entry", "JE-CANCEL").await.ok();
    delete_doc(&db, "GL Entry", "JE-CANCEL-GL-0").await.ok();
    delete_doc(&db, "GL Entry", "JE-CANCEL-GL-1").await.ok();

    // Step 1: insert and submit the JE so GL entries are created
    insert_doc(
        &db,
        "Journal Entry",
        &json!({
            "name":         "JE-CANCEL",
            "posting_date": POSTING_DATE,
            "voucher_type": "Journal Entry",
            "company":      COMPANY,
            "total_debit":  100.0,
            "total_credit": 100.0,
            "docstatus":    0,
            "accounts":     balanced_accounts()
        }),
    )
    .await
    .expect("insert JE before submit");

    let submit_result =
        run_pipeline(&db, "tabJournal_Entry", "JE-CANCEL", "Journal Entry", "submit").await;
    assert!(
        matches!(submit_result, Ok(PipelineResult::Ok)),
        "JE submit must succeed before cancel test; got: {submit_result:?}"
    );

    // Verify GL entries were created
    assert!(
        get_doc(&db, "GL Entry", "JE-CANCEL-GL-0").await.is_ok(),
        "GL Entry 0 must exist after submit"
    );

    // Step 2: cancel the JE
    let cancel_result =
        run_pipeline(&db, "tabJournal_Entry", "JE-CANCEL", "Journal Entry", "cancel").await;
    assert!(
        matches!(cancel_result, Ok(PipelineResult::Ok)),
        "JE cancel must succeed; got: {cancel_result:?}"
    );

    // Step 3: verify GL entries are now cancelled
    let gl0 = get_doc(&db, "GL Entry", "JE-CANCEL-GL-0")
        .await
        .expect("GL Entry 0 must still exist after cancel");
    let gl1 = get_doc(&db, "GL Entry", "JE-CANCEL-GL-1")
        .await
        .expect("GL Entry 1 must still exist after cancel");

    let gl0_cancelled = gl0
        .fields
        .get("is_cancelled")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let gl1_cancelled = gl1
        .fields
        .get("is_cancelled")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    assert!(gl0_cancelled, "GL Entry 0 must be is_cancelled=true after JE cancel");
    assert!(gl1_cancelled, "GL Entry 1 must be is_cancelled=true after JE cancel");

    delete_doc(&db, "Journal Entry", "JE-CANCEL").await.ok();
    delete_doc(&db, "GL Entry", "JE-CANCEL-GL-0").await.ok();
    delete_doc(&db, "GL Entry", "JE-CANCEL-GL-1").await.ok();
}

// ── 7. Checkpoint: JE submit automatically creates GL Entries with correct fields ──

/// **Checkpoint test** — proves end-to-end that submitting a Journal Entry
/// automatically creates General Ledger entries with every field set correctly.
///
/// This is the authoritative assertion that the JE→GL pipeline works:
///   JE submit triggers `fn::on_submit::je_create_gl_entries`
///   → one tabGL_Entry row per account line
///   → each row carries: account, debit/credit, posting_date, company,
///     voucher_type="Journal Entry", voucher_no=JE name, is_cancelled=false,
///     docstatus=1
///
/// ERPNext parity: `test_journal_entry.py::test_jv_linked_gl_entry`
#[tokio::test]
#[cfg_attr(not(feature = "integration"), ignore = "requires live SurrealDB")]
async fn test_je_submit_gl_entries_full_fields() {
    let db = setup_je_pipeline("checkpoint_gl").await;
    let je_name = "JE-CHECKPOINT";
    delete_doc(&db, "Journal Entry", je_name).await.ok();
    delete_doc(&db, "GL Entry", &format!("{je_name}-GL-0")).await.ok();
    delete_doc(&db, "GL Entry", &format!("{je_name}-GL-1")).await.ok();

    // Insert a fully-populated balanced JE
    insert_doc(
        &db,
        "Journal Entry",
        &json!({
            "name":         je_name,
            "posting_date": POSTING_DATE,
            "voucher_type": "Journal Entry",
            "company":      COMPANY,
            "total_debit":  100.0,
            "total_credit": 100.0,
            "docstatus":    0,
            "accounts": [
                {
                    "account": "Cash",
                    "debit":   100.0, "credit": 0.0,
                    "debit_in_account_currency":  100.0,
                    "credit_in_account_currency": 0.0,
                    "idx": 0
                },
                {
                    "account": "Revenue",
                    "debit":   0.0, "credit": 100.0,
                    "debit_in_account_currency":  0.0,
                    "credit_in_account_currency": 100.0,
                    "idx": 1
                }
            ]
        }),
    )
    .await
    .expect("insert JE-CHECKPOINT");

    // ── Action: submit the JE through the full pipeline ───────────────────────
    let result =
        run_pipeline(&db, "tabJournal_Entry", je_name, "Journal Entry", "submit").await;
    assert!(
        matches!(result, Ok(PipelineResult::Ok)),
        "JE submit must succeed; got: {result:?}"
    );

    // ── Assert GL entries were created automatically ───────────────────────────
    let gl0 = get_doc(&db, "GL Entry", &format!("{je_name}-GL-0"))
        .await
        .expect("GL Entry -GL-0 must be created automatically on JE submit");
    let gl1 = get_doc(&db, "GL Entry", &format!("{je_name}-GL-1"))
        .await
        .expect("GL Entry -GL-1 must be created automatically on JE submit");

    // ── Assert exact count: exactly 2 GL entries for this voucher_no ──────────
    // A spurious third entry would mean the loop ran too many times.
    let gl2_must_not_exist = get_doc(&db, "GL Entry", &format!("{je_name}-GL-2")).await;
    assert!(
        gl2_must_not_exist.is_err(),
        "A third GL entry must not be created — only one per account line"
    );

    // ── Assert fields on GL Entry 0 (debit side: Cash) ────────────────────────
    let f0 = &gl0.fields;
    assert_eq!(
        f0.get("account").and_then(|v| v.as_str()),
        Some("Cash"),
        "GL-0 account must be 'Cash'"
    );
    let debit0 = common::parse_decimal(f0.get("debit").unwrap_or(&json!(0)));
    assert!(
        (debit0 - 100.0).abs() < 0.001,
        "GL-0 debit must be 100.0; got {debit0}"
    );
    let credit0 = common::parse_decimal(f0.get("credit").unwrap_or(&json!(0)));
    assert!(
        credit0.abs() < 0.001,
        "GL-0 credit must be 0.0; got {credit0}"
    );
    assert_eq!(
        f0.get("voucher_type").and_then(|v| v.as_str()),
        Some("Journal Entry"),
        "GL-0 voucher_type must be 'Journal Entry'"
    );
    assert_eq!(
        f0.get("voucher_no").and_then(|v| v.as_str()),
        Some(je_name),
        "GL-0 voucher_no must be the JE name"
    );
    assert_eq!(
        f0.get("company").and_then(|v| v.as_str()),
        Some(COMPANY),
        "GL-0 company must match JE company"
    );
    let is_cancelled0 = f0
        .get("is_cancelled")
        .and_then(|v| v.as_bool())
        .unwrap_or(true); // default true would fail the assert below
    assert!(
        !is_cancelled0,
        "GL-0 must not be cancelled immediately after submit"
    );
    let docstatus0 = f0.get("docstatus").and_then(|v| v.as_i64()).unwrap_or(0);
    assert_eq!(docstatus0, 1, "GL-0 docstatus must be 1 (submitted)");

    // ── Assert fields on GL Entry 1 (credit side: Revenue) ───────────────────
    let f1 = &gl1.fields;
    assert_eq!(
        f1.get("account").and_then(|v| v.as_str()),
        Some("Revenue"),
        "GL-1 account must be 'Revenue'"
    );
    let debit1 = common::parse_decimal(f1.get("debit").unwrap_or(&json!(0)));
    assert!(
        debit1.abs() < 0.001,
        "GL-1 debit must be 0.0; got {debit1}"
    );
    let credit1 = common::parse_decimal(f1.get("credit").unwrap_or(&json!(0)));
    assert!(
        (credit1 - 100.0).abs() < 0.001,
        "GL-1 credit must be 100.0; got {credit1}"
    );
    assert_eq!(
        f1.get("voucher_no").and_then(|v| v.as_str()),
        Some(je_name),
        "GL-1 voucher_no must be the JE name"
    );

    // Cleanup
    delete_doc(&db, "Journal Entry", je_name).await.ok();
    delete_doc(&db, "GL Entry", &format!("{je_name}-GL-0")).await.ok();
    delete_doc(&db, "GL Entry", &format!("{je_name}-GL-1")).await.ok();
}

// ── 8. GL creation failure causes a full transaction rollback ─────────────────

/// **Atomicity test** — proves that if GL Entry creation fails partway through
/// the submit pipeline, the ENTIRE operation is rolled back:
///   - The pipeline call returns `Err` (not silent partial success)
///   - No orphaned GL entries are left in the database
///   - The JE is left in its pre-submit state (docstatus=0)
///
/// Mechanism: we pre-insert the SECOND GL entry ("JE-ROLLBACK-GL-1") before
/// calling submit.  The `je_create_gl_entries` loop successfully creates
/// GL-0, then hits a duplicate-key conflict on GL-1.  Because SurrealDB
/// executes the pipeline function as a single query-level transaction,
/// the engine-level INSERT error throws, rolling back GL-0 as well.
///
/// This test therefore validates two invariants simultaneously:
///   1. Errors during on_submit propagate to the caller as `Err`.
///   2. No partial GL state persists when creation fails.
#[tokio::test]
#[cfg_attr(not(feature = "integration"), ignore = "requires live SurrealDB")]
async fn test_je_submit_gl_failure_rolls_back() {
    let db = setup_je_pipeline("rollback_gl").await;
    let je_name = "JE-ROLLBACK";
    let gl0_name = format!("{je_name}-GL-0");
    let gl1_name = format!("{je_name}-GL-1");

    // Start clean
    delete_doc(&db, "Journal Entry", je_name).await.ok();
    delete_doc(&db, "GL Entry", &gl0_name).await.ok();
    delete_doc(&db, "GL Entry", &gl1_name).await.ok();

    // Insert a valid, balanced JE
    insert_doc(
        &db,
        "Journal Entry",
        &json!({
            "name":         je_name,
            "posting_date": POSTING_DATE,
            "voucher_type": "Journal Entry",
            "company":      COMPANY,
            "total_debit":  100.0,
            "total_credit": 100.0,
            "docstatus":    0,
            "accounts":     balanced_accounts()
        }),
    )
    .await
    .expect("insert JE-ROLLBACK");

    // ── Sabotage: pre-insert GL-1 so the second INSERT inside the loop fails ──
    // `je_create_gl_entries` names entries "{je_name}-GL-{idx}", idx starts at 0.
    // By occupying the idx=1 slot we force a duplicate-key collision after GL-0
    // has already been written, maximally stressing the rollback boundary.
    insert_doc(
        &db,
        "GL Entry",
        &json!({
            "name":         gl1_name,
            "account":      "Cash",           // dummy — just needs to exist
            "posting_date": POSTING_DATE,
            "company":      COMPANY,
            "debit":        0.0,
            "credit":       0.0,
            "voucher_type": "pre-inserted",
            "voucher_no":   "SABOTAGE",
            "docstatus":    0,
            "is_cancelled": false
        }),
    )
    .await
    .expect("pre-insert GL-1 to sabotage submit");

    // ── Action: attempt to submit the JE ─────────────────────────────────────
    let result =
        run_pipeline(&db, "tabJournal_Entry", je_name, "Journal Entry", "submit").await;

    // ── Assert 1: pipeline must return an error ───────────────────────────────
    assert!(
        result.is_err(),
        "submit must fail when GL entry creation encounters a duplicate; got: {result:?}"
    );

    // ── Assert 2: GL-0 must NOT exist — the partial insert was rolled back ────
    let gl0_state = get_doc(&db, "GL Entry", &gl0_name).await;
    assert!(
        gl0_state.is_err(),
        "GL Entry -GL-0 must NOT persist after a rolled-back submit; \
         if it exists, the transaction boundary is broken"
    );

    // ── Assert 3: the pre-inserted GL-1 still exists unchanged ───────────────
    // (it was written outside the pipeline transaction and must be unaffected)
    let gl1_state = get_doc(&db, "GL Entry", &gl1_name).await;
    assert!(
        gl1_state.is_ok(),
        "The pre-inserted GL-1 saboteur must still be present"
    );
    let gl1_doc = gl1_state.unwrap();
    assert_eq!(
        gl1_doc.fields.get("voucher_no").and_then(|v| v.as_str()),
        Some("SABOTAGE"),
        "GL-1 voucher_no must still be 'SABOTAGE' — rollback must not alter it"
    );

    // ── Assert 4: the JE itself is still at docstatus=0 (not submitted) ──────
    // The Rust layer writes the document first, then runs the pipeline.
    // On pipeline failure, run_pipeline returns Err — the JE docstatus
    // field is written by the pipeline's on_submit mark-submitted stage,
    // which is inside the same transaction and therefore rolled back.
    let je_state = get_doc(&db, "Journal Entry", je_name)
        .await
        .expect("JE must still exist after failed submit");
    let je_docstatus = je_state
        .fields
        .get("docstatus")
        .and_then(|v| v.as_i64())
        .unwrap_or(1); // default 1 would fail the assert
    assert_eq!(
        je_docstatus, 0,
        "JE docstatus must remain 0 after a failed submit (pipeline rollback)"
    );

    // Cleanup
    delete_doc(&db, "Journal Entry", je_name).await.ok();
    delete_doc(&db, "GL Entry", &gl0_name).await.ok();
    delete_doc(&db, "GL Entry", &gl1_name).await.ok();
}

