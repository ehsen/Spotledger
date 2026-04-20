//! Shared helpers for pipeline-integrated finance integration tests.
//!
//! Included via `#[path = "common/mod.rs"] mod common;` in each test file that
//! needs to exercise the full spotledger-finance domain validation pipeline.
//!
//! ## What this provides
//! - `make_pipeline_db`        — creates an isolated test DB and loads both
//!                               spotledger-core and spotledger-finance pipeline functions
//! - `seed_doctype`            — inserts a minimal tabDocType record so the runner
//!                               does not skip the pipeline for unknown doctypes
//! - DDL helpers for each table group (GL entry, JE, Fiscal Year, etc.)
//! - Data helpers (insert_valid_account, insert_open_period_2024, …)
//! - `parse_decimal`           — normalises the string/number SurrealDB decimal returns

#![allow(dead_code)]

use serde_json::json;
use spotledger_core::config::DatabaseConfig;
use spotledger_db::{document::{insert_doc, delete_doc}, DbAdapter};

// ── DB setup ──────────────────────────────────────────────────────────────────

/// Create an isolated test DB, apply the spotledger-core framework **and** the
/// spotledger-finance domain functions (including all wiring).
pub async fn make_pipeline_db(suffix: &str) -> DbAdapter {
    let url = std::env::var("SURREAL_TEST_URL")
        .unwrap_or_else(|_| "ws://127.0.0.1:8000".to_string());
    let cfg = DatabaseConfig {
        url,
        user: "root".into(),
        pass: "root".into(),
        ns: "test_ns".into(),
        db: format!("test_{suffix}"),
    };
    let db = DbAdapter::connect(&cfg).await.expect("connect to test SurrealDB");

    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../apps");
    let core_root    = root.join("spotledger-core/spotledger-core");
    let finance_root = root.join("spotledger-finance/spotledger-finance");
    spotledger_db::pipeline::apply_pipeline_functions_multi(
        &db,
        &[core_root.as_path(), finance_root.as_path()],
    )
    .await
    .expect("apply core + finance pipeline functions");

    // Define tabDocField as an empty SCHEMALESS table so fn::validate::mandatory_fields
    // can run without crashing (it queries this table but no required fields means it
    // always passes, letting domain-specific validation nodes run normally).
    db.execute(
        "DEFINE TABLE IF NOT EXISTS tabDocField SCHEMALESS;",
        vec![],
    )
    .await
    .expect("define tabDocField stub");

    db
}

/// Ensure tabDocType exists and has a record for `name`.
///
/// Without this record, `fn::pipeline::run` returns `{status:"skipped"}` and
/// the validation nodes are never reached.
pub async fn seed_doctype(db: &DbAdapter, name: &str, is_submittable: bool) {
    db.execute(
        "DEFINE TABLE IF NOT EXISTS tabDocType SCHEMALESS; \
         UPSERT type::record('tabDocType', $n) CONTENT { name: $n, issubmittable: $s };",
        vec![
            ("n".into(), json!(name)),
            ("s".into(), json!(is_submittable)),
        ],
    )
    .await
    .expect("seed tabDocType record");
}

// ── Table DDL helpers ──────────────────────────────────────────────────────────

/// Minimal tabAccount for gl_entry_validate_account and je_account_validity.
pub async fn define_account_table(db: &DbAdapter) {
    db.execute(
        "DEFINE TABLE IF NOT EXISTS tabAccount SCHEMAFULL; \
         DEFINE FIELD OVERWRITE name            ON tabAccount TYPE string; \
         DEFINE FIELD OVERWRITE account_number  ON tabAccount TYPE option<string>; \
         DEFINE FIELD OVERWRITE is_group        ON tabAccount TYPE bool DEFAULT false; \
         DEFINE FIELD OVERWRITE disabled        ON tabAccount TYPE bool DEFAULT false; \
         DEFINE FIELD OVERWRITE party_required  ON tabAccount TYPE bool DEFAULT false; \
         DEFINE FIELD OVERWRITE docstatus       ON tabAccount TYPE int DEFAULT 0; \
         DEFINE FIELD OVERWRITE owner           ON tabAccount TYPE option<string>; \
         DEFINE FIELD OVERWRITE creation        ON tabAccount TYPE option<datetime>; \
         DEFINE FIELD OVERWRITE modified        ON tabAccount TYPE option<datetime>; \
         DEFINE FIELD OVERWRITE modified_by     ON tabAccount TYPE option<string>; \
         DEFINE FIELD OVERWRITE doctype         ON tabAccount TYPE option<string>; \
         DEFINE INDEX OVERWRITE idx_acct_name ON tabAccount FIELDS name UNIQUE;",
        vec![],
    )
    .await
    .expect("define tabAccount");
}

/// Minimal tabFiscal_Year for gl_entry_derive_period and fiscal_year validation.
pub async fn define_fiscal_year_table(db: &DbAdapter) {
    db.execute(
        "DEFINE TABLE IF NOT EXISTS tabFiscal_Year SCHEMAFULL; \
         DEFINE FIELD OVERWRITE name             ON tabFiscal_Year TYPE string; \
         DEFINE FIELD OVERWRITE year_name        ON tabFiscal_Year TYPE option<string>; \
         DEFINE FIELD OVERWRITE company          ON tabFiscal_Year TYPE option<string>; \
         DEFINE FIELD OVERWRITE year_start_date  ON tabFiscal_Year TYPE option<datetime>; \
         DEFINE FIELD OVERWRITE year_end_date    ON tabFiscal_Year TYPE option<datetime>; \
         DEFINE FIELD OVERWRITE is_short_year    ON tabFiscal_Year TYPE bool DEFAULT false; \
         DEFINE FIELD OVERWRITE companies        ON tabFiscal_Year TYPE array<string> DEFAULT []; \
         DEFINE FIELD OVERWRITE docstatus        ON tabFiscal_Year TYPE int DEFAULT 0; \
         DEFINE FIELD OVERWRITE status           ON tabFiscal_Year TYPE option<string>; \
         DEFINE FIELD OVERWRITE owner            ON tabFiscal_Year TYPE option<string>; \
         DEFINE FIELD OVERWRITE creation         ON tabFiscal_Year TYPE option<datetime>; \
         DEFINE FIELD OVERWRITE modified         ON tabFiscal_Year TYPE option<datetime>; \
         DEFINE FIELD OVERWRITE modified_by      ON tabFiscal_Year TYPE option<string>; \
         DEFINE FIELD OVERWRITE doctype          ON tabFiscal_Year TYPE option<string>; \
         DEFINE INDEX OVERWRITE idx_fy_name ON tabFiscal_Year FIELDS name UNIQUE;",
        vec![],
    )
    .await
    .expect("define tabFiscal_Year");
}

/// Minimal tabAccounting_Period for period-open checks.
pub async fn define_accounting_period_table(db: &DbAdapter) {
    db.execute(
        "DEFINE TABLE IF NOT EXISTS tabAccounting_Period SCHEMAFULL; \
         DEFINE FIELD OVERWRITE name          ON tabAccounting_Period TYPE string; \
         DEFINE FIELD OVERWRITE period_name   ON tabAccounting_Period TYPE option<string>; \
         DEFINE FIELD OVERWRITE company       ON tabAccounting_Period TYPE option<string>; \
         DEFINE FIELD OVERWRITE start_date    ON tabAccounting_Period TYPE option<datetime>; \
         DEFINE FIELD OVERWRITE end_date      ON tabAccounting_Period TYPE option<datetime>; \
         DEFINE FIELD OVERWRITE fiscal_year   ON tabAccounting_Period TYPE option<string>; \
         DEFINE FIELD OVERWRITE is_closed     ON tabAccounting_Period TYPE bool DEFAULT false; \
         DEFINE FIELD OVERWRITE closed        ON tabAccounting_Period TYPE bool DEFAULT false; \
         DEFINE FIELD OVERWRITE docstatus     ON tabAccounting_Period TYPE int DEFAULT 0; \
         DEFINE FIELD OVERWRITE status        ON tabAccounting_Period TYPE option<string>; \
         DEFINE FIELD OVERWRITE owner         ON tabAccounting_Period TYPE option<string>; \
         DEFINE FIELD OVERWRITE creation      ON tabAccounting_Period TYPE option<datetime>; \
         DEFINE FIELD OVERWRITE modified      ON tabAccounting_Period TYPE option<datetime>; \
         DEFINE FIELD OVERWRITE modified_by   ON tabAccounting_Period TYPE option<string>; \
         DEFINE FIELD OVERWRITE doctype       ON tabAccounting_Period TYPE option<string>; \
         DEFINE INDEX OVERWRITE idx_ap_name ON tabAccounting_Period FIELDS name UNIQUE;",
        vec![],
    )
    .await
    .expect("define tabAccounting_Period");
}

/// Full tabGL_Entry with all fields referenced by pipeline functions.
pub async fn define_gl_entry_table(db: &DbAdapter) {
    db.execute(
        "DEFINE TABLE IF NOT EXISTS tabGL_Entry SCHEMAFULL; \
         DEFINE FIELD OVERWRITE name             ON tabGL_Entry TYPE string; \
         DEFINE FIELD OVERWRITE posting_date     ON tabGL_Entry TYPE option<datetime>; \
         DEFINE FIELD OVERWRITE account          ON tabGL_Entry TYPE option<string>; \
         DEFINE FIELD OVERWRITE account_currency ON tabGL_Entry TYPE option<string>; \
         DEFINE FIELD OVERWRITE debit            ON tabGL_Entry TYPE decimal DEFAULT 0.0; \
         DEFINE FIELD OVERWRITE credit           ON tabGL_Entry TYPE decimal DEFAULT 0.0; \
         DEFINE FIELD OVERWRITE debit_in_account_currency  ON tabGL_Entry TYPE decimal DEFAULT 0.0; \
         DEFINE FIELD OVERWRITE credit_in_account_currency ON tabGL_Entry TYPE decimal DEFAULT 0.0; \
         DEFINE FIELD OVERWRITE exchange_rate    ON tabGL_Entry TYPE option<decimal>; \
         DEFINE FIELD OVERWRITE company          ON tabGL_Entry TYPE option<string>; \
         DEFINE FIELD OVERWRITE party            ON tabGL_Entry TYPE option<string>; \
         DEFINE FIELD OVERWRITE party_role       ON tabGL_Entry TYPE option<string>; \
         DEFINE FIELD OVERWRITE cost_center      ON tabGL_Entry TYPE option<string>; \
         DEFINE FIELD OVERWRITE voucher_type     ON tabGL_Entry TYPE option<string>; \
         DEFINE FIELD OVERWRITE voucher_no       ON tabGL_Entry TYPE option<string>; \
         DEFINE FIELD OVERWRITE fiscal_year      ON tabGL_Entry TYPE option<string>; \
         DEFINE FIELD OVERWRITE accounting_period ON tabGL_Entry TYPE option<string>; \
         DEFINE FIELD OVERWRITE is_cancelled     ON tabGL_Entry TYPE bool DEFAULT false; \
         DEFINE FIELD OVERWRITE docstatus        ON tabGL_Entry TYPE int DEFAULT 0; \
         DEFINE FIELD OVERWRITE status           ON tabGL_Entry TYPE option<string>; \
         DEFINE FIELD OVERWRITE remarks          ON tabGL_Entry TYPE option<string>; \
         DEFINE FIELD OVERWRITE owner            ON tabGL_Entry TYPE option<string>; \
         DEFINE FIELD OVERWRITE creation         ON tabGL_Entry TYPE option<datetime>; \
         DEFINE FIELD OVERWRITE modified         ON tabGL_Entry TYPE option<datetime>; \
         DEFINE FIELD OVERWRITE modified_by      ON tabGL_Entry TYPE option<string>; \
         DEFINE FIELD OVERWRITE doctype          ON tabGL_Entry TYPE option<string>; \
         DEFINE INDEX OVERWRITE idx_gle_name ON tabGL_Entry FIELDS name UNIQUE; \
         DEFINE INDEX OVERWRITE idx_gle_voucher ON tabGL_Entry FIELDS voucher_type, voucher_no;",
        vec![],
    )
    .await
    .expect("define tabGL_Entry");
}

/// Full tabJournal_Entry with all fields referenced by pipeline functions, including
/// the `accounts` child-table array used by je_totals, je_has_accounts, etc.
pub async fn define_journal_entry_table(db: &DbAdapter) {
    db.execute(
        "DEFINE TABLE IF NOT EXISTS tabJournal_Entry SCHEMAFULL; \
         DEFINE FIELD OVERWRITE name          ON tabJournal_Entry TYPE string; \
         DEFINE FIELD OVERWRITE title         ON tabJournal_Entry TYPE option<string>; \
         DEFINE FIELD OVERWRITE posting_date  ON tabJournal_Entry TYPE option<datetime>; \
         DEFINE FIELD OVERWRITE company       ON tabJournal_Entry TYPE option<string>; \
         DEFINE FIELD OVERWRITE voucher_type  ON tabJournal_Entry TYPE option<string>; \
         DEFINE FIELD OVERWRITE total_debit   ON tabJournal_Entry TYPE decimal DEFAULT 0.0; \
         DEFINE FIELD OVERWRITE total_credit  ON tabJournal_Entry TYPE decimal DEFAULT 0.0; \
         DEFINE FIELD OVERWRITE difference    ON tabJournal_Entry TYPE decimal DEFAULT 0.0; \
         DEFINE FIELD OVERWRITE user_remark   ON tabJournal_Entry TYPE option<string>; \
         DEFINE FIELD OVERWRITE accounts      ON tabJournal_Entry TYPE any DEFAULT []; \
         DEFINE FIELD OVERWRITE docstatus     ON tabJournal_Entry TYPE int DEFAULT 0; \
         DEFINE FIELD OVERWRITE status        ON tabJournal_Entry TYPE option<string>; \
         DEFINE FIELD OVERWRITE owner         ON tabJournal_Entry TYPE option<string>; \
         DEFINE FIELD OVERWRITE creation      ON tabJournal_Entry TYPE option<datetime>; \
         DEFINE FIELD OVERWRITE modified      ON tabJournal_Entry TYPE option<datetime>; \
         DEFINE FIELD OVERWRITE modified_by   ON tabJournal_Entry TYPE option<string>; \
         DEFINE FIELD OVERWRITE doctype       ON tabJournal_Entry TYPE option<string>; \
         DEFINE INDEX OVERWRITE idx_je_name ON tabJournal_Entry FIELDS name UNIQUE;",
        vec![],
    )
    .await
    .expect("define tabJournal_Entry");
}

// ── Prerequisite data helpers ─────────────────────────────────────────────────

/// Insert a minimal valid, non-group, non-disabled account.
pub async fn insert_valid_account(db: &DbAdapter, name: &str) {
    delete_doc(db, "Account", name).await.ok();
    insert_doc(
        db,
        "Account",
        &json!({
            "name": name,
            "is_group": false,
            "disabled": false,
            "party_required": false,
        }),
    )
    .await
    .expect("insert valid account");
}

/// Insert a disabled account (for testing disabled-account rejection).
pub async fn insert_disabled_account(db: &DbAdapter, name: &str) {
    delete_doc(db, "Account", name).await.ok();
    insert_doc(
        db,
        "Account",
        &json!({
            "name": name,
            "is_group": false,
            "disabled": true,
            "party_required": false,
        }),
    )
    .await
    .expect("insert disabled account");
}

/// Insert a group account (for testing group-account rejection).
pub async fn insert_group_account(db: &DbAdapter, name: &str) {
    delete_doc(db, "Account", name).await.ok();
    insert_doc(
        db,
        "Account",
        &json!({
            "name": name,
            "is_group": true,
            "disabled": false,
            "party_required": false,
        }),
    )
    .await
    .expect("insert group account");
}

/// Insert a Fiscal Year and an **open** Accounting Period both covering 2024-06-15
/// for the given company.  Posting dates of 2024-06-15 will pass period-open checks.
pub async fn insert_open_period_2024(db: &DbAdapter, company: &str) {
    let fy_name = format!("FY-2024-{company}");
    let ap_name = format!("AP-Q2-2024-{company}");

    // Idempotent – ignore errors from prior runs that may have left data
    insert_doc(
        db,
        "Fiscal Year",
        &json!({
            "name": fy_name,
            "year_name": fy_name,
            "company": company,
            "year_start_date": "2024-01-01T00:00:00Z",
            "year_end_date":   "2024-12-31T23:59:59Z",
        }),
    )
    .await
    .ok();

    insert_doc(
        db,
        "Accounting Period",
        &json!({
            "name": ap_name,
            "company": company,
            "start_date": "2024-04-01T00:00:00Z",
            "end_date":   "2024-06-30T23:59:59Z",
            "is_closed":  false,
        }),
    )
    .await
    .ok();
}

/// Insert a Fiscal Year and a **closed** Accounting Period both covering 2023-06-15
/// for the given company.  Posting dates of 2023-06-15 will fail period-open checks.
pub async fn insert_closed_period_2023(db: &DbAdapter, company: &str) {
    let fy_name = format!("FY-2023-{company}");
    let ap_name = format!("AP-2023-{company}");

    insert_doc(
        db,
        "Fiscal Year",
        &json!({
            "name": fy_name,
            "year_name": fy_name,
            "company": company,
            "year_start_date": "2023-01-01T00:00:00Z",
            "year_end_date":   "2023-12-31T23:59:59Z",
        }),
    )
    .await
    .ok();

    insert_doc(
        db,
        "Accounting Period",
        &json!({
            "name": ap_name,
            "company": company,
            "start_date": "2023-01-01T00:00:00Z",
            "end_date":   "2023-12-31T23:59:59Z",
            "is_closed":  true,
        }),
    )
    .await
    .ok();
}

// ── Assertion helpers ──────────────────────────────────────────────────────────

/// Parse a decimal that SurrealDB may return as either a JSON number or a string
/// (SurrealDB v3 serialises `TYPE decimal` values as strings in some contexts).
pub fn parse_decimal(v: &serde_json::Value) -> f64 {
    v.as_f64()
        .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
        .unwrap_or(0.0)
}

/// Return the string value of a field from a document returned by `get_doc`,
/// or an empty string if the field is absent/null.
pub fn field_str<'a>(doc: &'a spotledger_core::document::Document, key: &str) -> &'a str {
    doc.fields
        .get(key)
        .and_then(|v| v.as_str())
        .unwrap_or("")
}
