//! Integration tests for Account — individual ledger accounts in the Chart of Accounts.
//!
//! ## Running
//! ```sh
//! $env:SURREAL_TEST_URL="ws://127.0.0.1:8000"
//! cargo test -p spotledger-db --features integration --test test_account -- --test-threads=1
//! ```
//!
//! ## Test plan
//! | # | Test | Verifies |
//! |---|------|---------|
//! | 1 | `test_account_insert`                      | Basic insert + fetch round-trip |
//! | 2 | `test_account_group_mismatch_coa_rejected`  | Account group from different CoA → error |
//! | 3 | `test_account_number_out_of_range_rejected`  | account_number outside group range → error |
//! | 4 | `test_account_derive_from_group`             | save/compute copies fs_type from account group |
//! | 5 | `test_account_retained_earnings_required`    | Income account without retained_earnings_account → error |

#![cfg(feature = "integration")]

#[path = "common/mod.rs"]
mod common;

use serde_json::json;
use spotledger_core::config::DatabaseConfig;
use spotledger_db::{
    document::{delete_doc, get_doc, insert_doc},
    pipeline::run_pipeline,
    DbAdapter,
};

// ── Setup ─────────────────────────────────────────────────────────────────────

async fn setup_account_pipeline(suffix: &str) -> DbAdapter {
    let db = common::make_pipeline_db(&format!("account_{suffix}")).await;
    common::seed_doctype(&db, "Account", false).await;

    // Full Account table — add all fields needed by pipeline functions
    db.execute(
        "DEFINE TABLE IF NOT EXISTS tabAccount SCHEMAFULL; \
         DEFINE FIELD OVERWRITE name                      ON tabAccount TYPE string; \
         DEFINE FIELD OVERWRITE account_name              ON tabAccount TYPE option<string>; \
         DEFINE FIELD OVERWRITE account_number            ON tabAccount TYPE option<string>; \
         DEFINE FIELD OVERWRITE account_group             ON tabAccount TYPE option<string>; \
         DEFINE FIELD OVERWRITE chart_of_accounts         ON tabAccount TYPE option<string>; \
         DEFINE FIELD OVERWRITE account_type_internal     ON tabAccount TYPE option<string>; \
         DEFINE FIELD OVERWRITE account_category          ON tabAccount TYPE option<string>; \
         DEFINE FIELD OVERWRITE fs_type                   ON tabAccount TYPE option<string>; \
         DEFINE FIELD OVERWRITE is_reconciliation_account ON tabAccount TYPE bool DEFAULT false; \
         DEFINE FIELD OVERWRITE party_required            ON tabAccount TYPE bool DEFAULT false; \
         DEFINE FIELD OVERWRITE is_group                  ON tabAccount TYPE bool DEFAULT false; \
         DEFINE FIELD OVERWRITE disabled                  ON tabAccount TYPE bool DEFAULT false; \
         DEFINE FIELD OVERWRITE retained_earnings_account ON tabAccount TYPE option<string>; \
         DEFINE FIELD OVERWRITE docstatus                 ON tabAccount TYPE int DEFAULT 0; \
         DEFINE FIELD OVERWRITE owner                     ON tabAccount TYPE option<string>; \
         DEFINE FIELD OVERWRITE creation                  ON tabAccount TYPE option<datetime>; \
         DEFINE FIELD OVERWRITE modified                  ON tabAccount TYPE option<datetime>; \
         DEFINE FIELD OVERWRITE modified_by               ON tabAccount TYPE option<string>; \
         DEFINE FIELD OVERWRITE doctype                   ON tabAccount TYPE option<string>; \
         DEFINE INDEX OVERWRITE idx_acct_name ON tabAccount FIELDS name UNIQUE;",
        vec![],
    )
    .await
    .expect("define tabAccount");

    // Account Group table — needed by account_derive_from_group and account_group_matches_coa
    db.execute(
        "DEFINE TABLE IF NOT EXISTS tabAccount_Group SCHEMAFULL; \
         DEFINE FIELD OVERWRITE name                   ON tabAccount_Group TYPE string; \
         DEFINE FIELD OVERWRITE group_name             ON tabAccount_Group TYPE option<string>; \
         DEFINE FIELD OVERWRITE chart_of_accounts      ON tabAccount_Group TYPE option<string>; \
         DEFINE FIELD OVERWRITE number_from            ON tabAccount_Group TYPE option<string>; \
         DEFINE FIELD OVERWRITE number_to              ON tabAccount_Group TYPE option<string>; \
         DEFINE FIELD OVERWRITE account_category       ON tabAccount_Group TYPE option<string>; \
         DEFINE FIELD OVERWRITE account_type_internal  ON tabAccount_Group TYPE option<string>; \
         DEFINE FIELD OVERWRITE fs_type                ON tabAccount_Group TYPE option<string>; \
         DEFINE FIELD OVERWRITE is_reconciliation_group ON tabAccount_Group TYPE bool DEFAULT false; \
         DEFINE FIELD OVERWRITE party_required         ON tabAccount_Group TYPE bool DEFAULT false; \
         DEFINE FIELD OVERWRITE docstatus              ON tabAccount_Group TYPE int DEFAULT 0; \
         DEFINE FIELD OVERWRITE owner                  ON tabAccount_Group TYPE option<string>; \
         DEFINE FIELD OVERWRITE creation               ON tabAccount_Group TYPE option<datetime>; \
         DEFINE FIELD OVERWRITE modified               ON tabAccount_Group TYPE option<datetime>; \
         DEFINE FIELD OVERWRITE modified_by            ON tabAccount_Group TYPE option<string>; \
         DEFINE FIELD OVERWRITE doctype                ON tabAccount_Group TYPE option<string>; \
         DEFINE INDEX OVERWRITE idx_ag_name ON tabAccount_Group FIELDS name UNIQUE;",
        vec![],
    )
    .await
    .expect("define tabAccount_Group");

    // GL Entry stub — needed by account_type_immutable check
    common::define_gl_entry_table(&db).await;

    // child_of edge table — needed by account_relate_parent
    db.execute(
        "DEFINE TABLE IF NOT EXISTS child_of SCHEMALESS;",
        vec![],
    )
    .await
    .expect("define child_of");

    // Account Company stub — needed by account_extend_companies
    db.execute(
        "DEFINE TABLE IF NOT EXISTS tabAccount_Company SCHEMALESS;",
        vec![],
    )
    .await
    .expect("define tabAccount_Company");

    // tabCompany stub — needed by account_extend_companies (SELECT from tabCompany)
    db.execute(
        "DEFINE TABLE IF NOT EXISTS tabCompany SCHEMALESS;",
        vec![],
    )
    .await
    .expect("define tabCompany stub");

    db
}

// ── 1. Plain CRUD (no pipeline) ───────────────────────────────────────────────

#[tokio::test]
#[cfg_attr(not(feature = "integration"), ignore = "requires live SurrealDB")]
async fn test_account_insert() {
    let url = std::env::var("SURREAL_TEST_URL")
        .unwrap_or_else(|_| "ws://127.0.0.1:8000".to_string());
    let cfg = DatabaseConfig {
        url, user: "root".into(), pass: "root".into(),
        ns: "test_ns".into(), db: "test_account_crud".into(),
    };
    let db = DbAdapter::connect(&cfg).await.expect("connect");
    db.execute(
        "DEFINE TABLE IF NOT EXISTS tabAccount SCHEMAFULL; \
         DEFINE FIELD OVERWRITE name            ON tabAccount TYPE string; \
         DEFINE FIELD OVERWRITE account_name    ON tabAccount TYPE option<string>; \
         DEFINE FIELD OVERWRITE account_type    ON tabAccount TYPE option<string>; \
         DEFINE FIELD OVERWRITE is_group        ON tabAccount TYPE bool DEFAULT false; \
         DEFINE FIELD OVERWRITE disabled        ON tabAccount TYPE bool DEFAULT false; \
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
    .expect("DDL");

    delete_doc(&db, "Account", "1000 - Cash - COMP").await.ok();
    insert_doc(&db, "Account", &json!({
        "name": "1000 - Cash - COMP",
        "account_name": "Cash",
        "account_type": "Asset",
        "is_group": false
    }))
    .await
    .expect("insert Account");

    let fetched = get_doc(&db, "Account", "1000 - Cash - COMP")
        .await
        .expect("get Account");
    assert_eq!(
        fetched.fields.get("account_type").and_then(|v| v.as_str()),
        Some("Asset")
    );
    delete_doc(&db, "Account", "1000 - Cash - COMP").await.ok();
}

// ── 2. Account group from a different CoA is rejected ─────────────────────────

/// `fn::validate::account_group_matches_coa` — if account_group exists in a
/// different chart_of_accounts than the account itself, reject with an error.
#[tokio::test]
#[cfg_attr(not(feature = "integration"), ignore = "requires live SurrealDB")]
async fn test_account_group_mismatch_coa_rejected() {
    let db = setup_account_pipeline("coa_mismatch").await;
    delete_doc(&db, "Account", "ACCT-COA-MISMATCH").await.ok();

    // Insert an Account Group that belongs to CoA "IFRS"
    delete_doc(&db, "Account Group", "AG-IFRS-001").await.ok();
    insert_doc(
        &db,
        "Account Group",
        &json!({
            "name": "AG-IFRS-001",
            "group_name": "Cash and Bank",
            "chart_of_accounts": "IFRS",
            "number_from": "1000",
            "number_to":   "1999",
            "account_category": "Cash and Bank",
            "fs_type": "Asset"
        }),
    )
    .await
    .expect("insert account group");

    // Insert an Account that claims a different CoA "GAAP"
    insert_doc(
        &db,
        "Account",
        &json!({
            "name":              "ACCT-COA-MISMATCH",
            "account_name":      "Cash",
            "account_number":    "1001",
            "account_group":     "AG-IFRS-001",
            "chart_of_accounts": "GAAP",   // <-- different from group's IFRS
            "is_group":          false,
            "docstatus":         0
        }),
    )
    .await
    .expect("insert account");

    let result = run_pipeline(&db, "tabAccount", "ACCT-COA-MISMATCH", "Account", "save").await;
    assert!(result.is_err(), "CoA mismatch must fail; got: {result:?}");
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.to_lowercase().contains("chart") || msg.to_lowercase().contains("coa"),
        "error must mention chart of accounts; got: {msg}"
    );

    delete_doc(&db, "Account", "ACCT-COA-MISMATCH").await.ok();
    delete_doc(&db, "Account Group", "AG-IFRS-001").await.ok();
}

// ── 3. Account number outside group range is rejected ─────────────────────────

/// `fn::validate::account_number_in_group_range` — account_number must fall
/// within the [number_from, number_to] range of its assigned account group.
#[tokio::test]
#[cfg_attr(not(feature = "integration"), ignore = "requires live SurrealDB")]
async fn test_account_number_out_of_range_rejected() {
    let db = setup_account_pipeline("num_range").await;
    delete_doc(&db, "Account", "ACCT-NUM-RANGE").await.ok();

    // Group allows numbers 1000–1999
    delete_doc(&db, "Account Group", "AG-RANGE-001").await.ok();
    insert_doc(
        &db,
        "Account Group",
        &json!({
            "name": "AG-RANGE-001",
            "group_name": "Cash",
            "chart_of_accounts": "STANDARD",
            "number_from": "1000",
            "number_to":   "1999",
            "account_category": "Cash and Bank",
            "fs_type": "Asset"
        }),
    )
    .await
    .expect("insert account group");

    // Account number 2500 is outside the 1000–1999 range
    insert_doc(
        &db,
        "Account",
        &json!({
            "name":              "ACCT-NUM-RANGE",
            "account_name":      "Bad Number",
            "account_number":    "2500",
            "account_group":     "AG-RANGE-001",
            "chart_of_accounts": "STANDARD",
            "is_group":          false,
            "docstatus":         0
        }),
    )
    .await
    .expect("insert account");

    let result = run_pipeline(&db, "tabAccount", "ACCT-NUM-RANGE", "Account", "save").await;
    assert!(result.is_err(), "Out-of-range number must fail; got: {result:?}");
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.to_lowercase().contains("range") || msg.to_lowercase().contains("number"),
        "error must mention range; got: {msg}"
    );

    delete_doc(&db, "Account", "ACCT-NUM-RANGE").await.ok();
    delete_doc(&db, "Account Group", "AG-RANGE-001").await.ok();
}

// ── 4. Compute stage derives fs_type from account group ───────────────────────

/// `fn::compute::account_derive_from_group` — after save, the account should
/// have fs_type, account_type_internal, etc. copied from its Account Group.
#[tokio::test]
#[cfg_attr(not(feature = "integration"), ignore = "requires live SurrealDB")]
async fn test_account_derive_from_group() {
    let db = setup_account_pipeline("derive_group").await;
    delete_doc(&db, "Account", "ACCT-DERIVE").await.ok();

    // Insert an Account Group with fs_type="Income"
    delete_doc(&db, "Account Group", "AG-INCOME-001").await.ok();
    insert_doc(
        &db,
        "Account Group",
        &json!({
            "name": "AG-INCOME-001",
            "group_name": "Revenue",
            "chart_of_accounts": "STANDARD",
            "number_from": "4000",
            "number_to":   "4999",
            "account_category": "Revenue",
            "account_type_internal": "G",
            "fs_type": "Income",
            "is_reconciliation_group": false,
            "party_required": false
        }),
    )
    .await
    .expect("insert account group");

    // Insert a retained earnings account so the retained_earnings_required validator passes
    delete_doc(&db, "Account", "ACCT-RETAINED").await.ok();
    insert_doc(
        &db,
        "Account",
        &json!({
            "name":              "ACCT-RETAINED",
            "account_name":      "Retained Earnings",
            "account_number":    "3900",
            "chart_of_accounts": "STANDARD",
            "is_group":          false,
            "docstatus":         0
        }),
    )
    .await
    .expect("insert retained earnings account");

    insert_doc(
        &db,
        "Account",
        &json!({
            "name":                    "ACCT-DERIVE",
            "account_name":            "Sales Revenue",
            "account_number":          "4100",
            "account_group":           "AG-INCOME-001",
            "chart_of_accounts":       "STANDARD",
            "retained_earnings_account": "ACCT-RETAINED",
            "is_group":                false,
            "docstatus":               0
        }),
    )
    .await
    .expect("insert account");

    let result = run_pipeline(&db, "tabAccount", "ACCT-DERIVE", "Account", "save").await;
    assert!(
        result.is_ok(),
        "save with valid group must succeed; got: {result:?}"
    );

    // After save, fs_type and account_type_internal must be derived from the group
    let acct = get_doc(&db, "Account", "ACCT-DERIVE")
        .await
        .expect("fetch account after save");
    assert_eq!(
        acct.fields.get("fs_type").and_then(|v| v.as_str()),
        Some("Income"),
        "fs_type must be derived from account group"
    );
    assert_eq!(
        acct.fields.get("account_type_internal").and_then(|v| v.as_str()),
        Some("G"),
        "account_type_internal must be derived from account group"
    );

    delete_doc(&db, "Account", "ACCT-DERIVE").await.ok();
    delete_doc(&db, "Account", "ACCT-RETAINED").await.ok();
    delete_doc(&db, "Account Group", "AG-INCOME-001").await.ok();
}

// ── 5. Income/Expense account requires retained_earnings_account ──────────────

/// `fn::validate::account_retained_earnings_required` — a non-group account with
/// fs_type="Income" or "Expense" must have a `retained_earnings_account` set.
#[tokio::test]
#[cfg_attr(not(feature = "integration"), ignore = "requires live SurrealDB")]
async fn test_account_retained_earnings_required() {
    let db = setup_account_pipeline("retained_earnings").await;
    delete_doc(&db, "Account", "ACCT-RE-MISSING").await.ok();

    // Insert an account with fs_type=Income but no retained_earnings_account
    insert_doc(
        &db,
        "Account",
        &json!({
            "name":     "ACCT-RE-MISSING",
            "account_name": "Sales",
            "fs_type":  "Income",   // pipeline sets this directly; no group needed
            "is_group": false,
            "retained_earnings_account": null,  // missing — should fail
            "docstatus": 0
        }),
    )
    .await
    .expect("insert account");

    let result = run_pipeline(&db, "tabAccount", "ACCT-RE-MISSING", "Account", "save").await;
    assert!(
        result.is_err(),
        "Income account without retained_earnings must fail; got: {result:?}"
    );
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.to_lowercase().contains("retained"),
        "error must mention retained earnings; got: {msg}"
    );

    delete_doc(&db, "Account", "ACCT-RE-MISSING").await.ok();
}

