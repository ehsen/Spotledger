//! Integration tests for Payment — the core payment document in SpotLedger Finance.
//!
//! ## Running
//! ```sh
//! $env:SURREAL_TEST_URL="ws://127.0.0.1:8000"
//! cargo test -p spotledger-db --features integration --test test_payment -- --test-threads=1
//! ```
//!
//! ## Test plan
//! | # | Test | Verifies |
//! |---|------|---------|
//! | 1 | `test_payment_insert`                   | Basic insert + fetch round-trip |
//! | 2 | `test_payment_type_direction_mismatch`  | Incoming payment with Expense Payment type → error |
//! | 3 | `test_payment_party_role_required`      | Party Payment without party_role → error |
//! | 4 | `test_payment_internal_transfer_same_account` | from_account = to_account → error |
//! | 5 | `test_payment_casual_party_not_on_party_payment` | casual_party on Party Payment → error |

#![cfg(feature = "integration")]

#[path = "common/mod.rs"]
mod common;

use serde_json::json;
use spotledger_core::config::DatabaseConfig;
use spotledger_db::{
    document::{delete_doc, insert_doc},
    pipeline::run_pipeline,
    DbAdapter,
};

// ── Setup ─────────────────────────────────────────────────────────────────────

async fn setup_payment_pipeline(suffix: &str) -> DbAdapter {
    let db = common::make_pipeline_db(&format!("payment_{suffix}")).await;
    common::seed_doctype(&db, "Payment", true).await;  // submittable

    db.execute(
        "DEFINE TABLE IF NOT EXISTS tabPayment SCHEMAFULL; \
         DEFINE FIELD OVERWRITE name                 ON tabPayment TYPE string; \
         DEFINE FIELD OVERWRITE payment_type         ON tabPayment TYPE option<string>; \
         DEFINE FIELD OVERWRITE payment_direction    ON tabPayment TYPE option<string>; \
         DEFINE FIELD OVERWRITE party                ON tabPayment TYPE option<string>; \
         DEFINE FIELD OVERWRITE party_role           ON tabPayment TYPE option<string>; \
         DEFINE FIELD OVERWRITE casual_party         ON tabPayment TYPE option<string>; \
         DEFINE FIELD OVERWRITE paid_from            ON tabPayment TYPE option<string>; \
         DEFINE FIELD OVERWRITE paid_into            ON tabPayment TYPE option<string>; \
         DEFINE FIELD OVERWRITE from_account         ON tabPayment TYPE option<string>; \
         DEFINE FIELD OVERWRITE to_account           ON tabPayment TYPE option<string>; \
         DEFINE FIELD OVERWRITE amount               ON tabPayment TYPE decimal DEFAULT 0.0; \
         DEFINE FIELD OVERWRITE expense_income_lines ON tabPayment TYPE any DEFAULT []; \
         DEFINE FIELD OVERWRITE against_invoices     ON tabPayment TYPE any DEFAULT []; \
         DEFINE FIELD OVERWRITE posting_date         ON tabPayment TYPE option<datetime>; \
         DEFINE FIELD OVERWRITE company              ON tabPayment TYPE option<string>; \
         DEFINE FIELD OVERWRITE docstatus            ON tabPayment TYPE int DEFAULT 0; \
         DEFINE FIELD OVERWRITE owner                ON tabPayment TYPE option<string>; \
         DEFINE FIELD OVERWRITE creation             ON tabPayment TYPE option<datetime>; \
         DEFINE FIELD OVERWRITE modified             ON tabPayment TYPE option<datetime>; \
         DEFINE FIELD OVERWRITE modified_by          ON tabPayment TYPE option<string>; \
         DEFINE FIELD OVERWRITE doctype              ON tabPayment TYPE option<string>; \
         DEFINE INDEX OVERWRITE idx_pmt_name ON tabPayment FIELDS name UNIQUE;",
        vec![],
    )
    .await
    .expect("define tabPayment");

    // Account table — needed by payment_paid_accounts and payment_transfer_accounts
    db.execute(
        "DEFINE TABLE IF NOT EXISTS tabAccount SCHEMAFULL; \
         DEFINE FIELD OVERWRITE name             ON tabAccount TYPE string; \
         DEFINE FIELD OVERWRITE account_category ON tabAccount TYPE option<string>; \
         DEFINE FIELD OVERWRITE fs_type          ON tabAccount TYPE option<string>; \
         DEFINE FIELD OVERWRITE is_group         ON tabAccount TYPE bool DEFAULT false; \
         DEFINE FIELD OVERWRITE disabled         ON tabAccount TYPE bool DEFAULT false; \
         DEFINE FIELD OVERWRITE docstatus        ON tabAccount TYPE int DEFAULT 0; \
         DEFINE FIELD OVERWRITE owner            ON tabAccount TYPE option<string>; \
         DEFINE FIELD OVERWRITE creation         ON tabAccount TYPE option<datetime>; \
         DEFINE FIELD OVERWRITE modified         ON tabAccount TYPE option<datetime>; \
         DEFINE FIELD OVERWRITE modified_by      ON tabAccount TYPE option<string>; \
         DEFINE FIELD OVERWRITE doctype          ON tabAccount TYPE option<string>; \
         DEFINE INDEX OVERWRITE idx_pmt_acct ON tabAccount FIELDS name UNIQUE;",
        vec![],
    )
    .await
    .expect("define tabAccount for payment tests");

    // tabCasual_Party stub — needed by casual_party_scope validator
    db.execute(
        "DEFINE TABLE IF NOT EXISTS tabCasual_Party SCHEMALESS;",
        vec![],
    )
    .await
    .expect("define tabCasual_Party");

    db
}

/// Insert a Cash and Bank account for payment tests.
async fn insert_cash_account(db: &DbAdapter, name: &str) {
    delete_doc(db, "Account", name).await.ok();
    insert_doc(
        db,
        "Account",
        &json!({
            "name":             name,
            "account_category": "Cash and Bank",
            "fs_type":          "Asset",
            "is_group":         false,
            "disabled":         false
        }),
    )
    .await
    .expect("insert cash account");
}

/// Insert an Expense account for payment tests.
async fn insert_expense_account(db: &DbAdapter, name: &str) {
    delete_doc(db, "Account", name).await.ok();
    insert_doc(
        db,
        "Account",
        &json!({
            "name":             name,
            "account_category": "Expense",
            "fs_type":          "Expense",
            "is_group":         false,
            "disabled":         false
        }),
    )
    .await
    .expect("insert expense account");
}

// ── 1. Plain CRUD ─────────────────────────────────────────────────────────────

#[tokio::test]
#[cfg_attr(not(feature = "integration"), ignore = "requires live SurrealDB")]
async fn test_payment_insert() {
    let url = std::env::var("SURREAL_TEST_URL")
        .unwrap_or_else(|_| "ws://127.0.0.1:8000".to_string());
    let cfg = DatabaseConfig {
        url, user: "root".into(), pass: "root".into(),
        ns: "test_ns".into(), db: "test_payment_crud".into(),
    };
    let db = DbAdapter::connect(&cfg).await.expect("connect");
    db.execute(
        "DEFINE TABLE IF NOT EXISTS tabPayment SCHEMAFULL; \
         DEFINE FIELD OVERWRITE name              ON tabPayment TYPE string; \
         DEFINE FIELD OVERWRITE payment_type      ON tabPayment TYPE option<string>; \
         DEFINE FIELD OVERWRITE payment_direction ON tabPayment TYPE option<string>; \
         DEFINE FIELD OVERWRITE amount            ON tabPayment TYPE decimal DEFAULT 0.0; \
         DEFINE FIELD OVERWRITE docstatus         ON tabPayment TYPE int DEFAULT 0; \
         DEFINE FIELD OVERWRITE owner             ON tabPayment TYPE option<string>; \
         DEFINE FIELD OVERWRITE creation          ON tabPayment TYPE option<datetime>; \
         DEFINE FIELD OVERWRITE modified          ON tabPayment TYPE option<datetime>; \
         DEFINE FIELD OVERWRITE modified_by       ON tabPayment TYPE option<string>; \
         DEFINE FIELD OVERWRITE doctype           ON tabPayment TYPE option<string>; \
         DEFINE INDEX OVERWRITE idx_pmt_crud ON tabPayment FIELDS name UNIQUE;",
        vec![],
    )
    .await
    .expect("DDL");

    delete_doc(&db, "Payment", "PMT-CRUD-001").await.ok();
    insert_doc(&db, "Payment", &json!({
        "name":              "PMT-CRUD-001",
        "payment_type":      "Expense Payment",
        "payment_direction": "Outgoing",
        "amount":            500.0
    }))
    .await
    .expect("insert Payment");

    let fetched = db
        .run(
            "SELECT name, payment_type, amount FROM tabPayment WHERE name = 'PMT-CRUD-001' LIMIT 1",
            vec![],
        )
        .await
        .expect("fetch payment");
    assert!(!fetched.is_empty(), "payment must exist after insert");
    delete_doc(&db, "Payment", "PMT-CRUD-001").await.ok();
}

// ── 2. Incoming direction with Expense Payment type is rejected ───────────────

/// `fn::validate::payment_type_direction` — "Expense Payment" is only valid for
/// Outgoing direction; pairing it with "Incoming" must be rejected.
#[tokio::test]
#[cfg_attr(not(feature = "integration"), ignore = "requires live SurrealDB")]
async fn test_payment_type_direction_mismatch() {
    let db = setup_payment_pipeline("type_dir").await;
    delete_doc(&db, "Payment", "PMT-WRONG-DIR").await.ok();

    insert_doc(
        &db,
        "Payment",
        &json!({
            "name":              "PMT-WRONG-DIR",
            "payment_type":      "Expense Payment",  // only valid for Outgoing
            "payment_direction": "Incoming",          // <-- mismatch
            "amount":            200.0,
            "docstatus":         0
        }),
    )
    .await
    .expect("insert mismatched payment");

    let result = run_pipeline(&db, "tabPayment", "PMT-WRONG-DIR", "Payment", "save").await;
    assert!(result.is_err(), "direction/type mismatch must fail; got: {result:?}");
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.to_lowercase().contains("direction") || msg.to_lowercase().contains("type"),
        "error must mention direction or type; got: {msg}"
    );

    delete_doc(&db, "Payment", "PMT-WRONG-DIR").await.ok();
}

// ── 3. Party Payment without party_role is rejected ───────────────────────────

/// `fn::validate::payment_party_role` — Party Payment and Party Receipt require
/// a non-empty party_role.
#[tokio::test]
#[cfg_attr(not(feature = "integration"), ignore = "requires live SurrealDB")]
async fn test_payment_party_role_required() {
    let db = setup_payment_pipeline("party_role").await;
    delete_doc(&db, "Payment", "PMT-NO-ROLE").await.ok();

    insert_doc(
        &db,
        "Payment",
        &json!({
            "name":              "PMT-NO-ROLE",
            "payment_type":      "Party Payment",
            "payment_direction": "Outgoing",
            // party_role intentionally omitted
            "amount":            300.0,
            "docstatus":         0
        }),
    )
    .await
    .expect("insert party payment without role");

    let result = run_pipeline(&db, "tabPayment", "PMT-NO-ROLE", "Payment", "save").await;
    assert!(result.is_err(), "missing party_role must fail; got: {result:?}");
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.to_lowercase().contains("party role") || msg.to_lowercase().contains("role"),
        "error must mention party role; got: {msg}"
    );

    delete_doc(&db, "Payment", "PMT-NO-ROLE").await.ok();
}

// ── 4. Internal Transfer with same from and to account is rejected ────────────

/// `fn::validate::payment_transfer_accounts` — from_account cannot equal to_account.
#[tokio::test]
#[cfg_attr(not(feature = "integration"), ignore = "requires live SurrealDB")]
async fn test_payment_internal_transfer_same_account() {
    let db = setup_payment_pipeline("same_acct").await;
    delete_doc(&db, "Payment", "PMT-SAME-ACCT").await.ok();

    // Insert a Cash and Bank account to reference
    insert_cash_account(&db, "PMT-CASH-ACCT").await;

    insert_doc(
        &db,
        "Payment",
        &json!({
            "name":              "PMT-SAME-ACCT",
            "payment_type":      "Internal Transfer",
            "payment_direction": "Internal",
            "from_account":      "PMT-CASH-ACCT",
            "to_account":        "PMT-CASH-ACCT",  // <-- same as from
            "amount":            100.0,
            "docstatus":         0
        }),
    )
    .await
    .expect("insert same-account transfer");

    let result = run_pipeline(&db, "tabPayment", "PMT-SAME-ACCT", "Payment", "save").await;
    assert!(result.is_err(), "same from/to account must fail; got: {result:?}");
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.to_lowercase().contains("same") || msg.to_lowercase().contains("account"),
        "error must mention same account; got: {msg}"
    );

    delete_doc(&db, "Payment", "PMT-SAME-ACCT").await.ok();
    delete_doc(&db, "Account", "PMT-CASH-ACCT").await.ok();
}

// ── 5. Casual Party on a Party Payment is rejected ────────────────────────────

/// `fn::validate::payment_casual_party_scope` — casual_party cannot be used
/// with Party Payment or Party Receipt; those require a full subledger Party.
#[tokio::test]
#[cfg_attr(not(feature = "integration"), ignore = "requires live SurrealDB")]
async fn test_payment_casual_party_not_on_party_payment() {
    let db = setup_payment_pipeline("casual_party").await;
    delete_doc(&db, "Payment", "PMT-CASUAL-WRONG").await.ok();

    insert_doc(
        &db,
        "Payment",
        &json!({
            "name":              "PMT-CASUAL-WRONG",
            "payment_type":      "Party Payment",
            "payment_direction": "Outgoing",
            "party_role":        "Supplier",
            "casual_party":      "CASUAL-XYZ",   // <-- not allowed on Party Payment
            "amount":            250.0,
            "docstatus":         0
        }),
    )
    .await
    .expect("insert payment with casual party on party payment");

    let result = run_pipeline(&db, "tabPayment", "PMT-CASUAL-WRONG", "Payment", "save").await;
    assert!(result.is_err(), "casual_party on Party Payment must fail; got: {result:?}");
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.to_lowercase().contains("casual") || msg.to_lowercase().contains("party"),
        "error must mention casual party restriction; got: {msg}"
    );

    delete_doc(&db, "Payment", "PMT-CASUAL-WRONG").await.ok();
}
