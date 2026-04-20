//! Integration tests for Account Group — root nodes of the Chart of Accounts tree.
//!
//! ## Running
//! ```sh
//! $env:SURREAL_TEST_URL="ws://127.0.0.1:8000"
//! cargo test -p spotledger-db --features integration --test test_account_group -- --test-threads=1
//! ```
//!
//! ## Test plan
//! | # | Test | Verifies |
//! |---|------|---------|
//! | 1 | `test_account_group_insert`               | Basic insert + fetch round-trip |
//! | 2 | `test_account_group_number_range_required` | Missing number_from/number_to → error |
//! | 3 | `test_account_group_range_inverted`        | number_from >= number_to → error |
//! | 4 | `test_account_group_range_overlap`         | Overlapping range with existing group in same CoA → error |
//! | 5 | `test_account_group_derive_classification` | save/compute derives fs_type from account_category |

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

async fn setup_ag_pipeline(suffix: &str) -> DbAdapter {
    let db = common::make_pipeline_db(&format!("account_group_{suffix}")).await;
    common::seed_doctype(&db, "Account Group", false).await;

    db.execute(
        "DEFINE TABLE IF NOT EXISTS tabAccount_Group SCHEMAFULL; \
         DEFINE FIELD OVERWRITE name                    ON tabAccount_Group TYPE string; \
         DEFINE FIELD OVERWRITE group_name              ON tabAccount_Group TYPE option<string>; \
         DEFINE FIELD OVERWRITE chart_of_accounts       ON tabAccount_Group TYPE option<string>; \
         DEFINE FIELD OVERWRITE number_from             ON tabAccount_Group TYPE option<string>; \
         DEFINE FIELD OVERWRITE number_to               ON tabAccount_Group TYPE option<string>; \
         DEFINE FIELD OVERWRITE code                    ON tabAccount_Group TYPE option<string>; \
         DEFINE FIELD OVERWRITE account_category        ON tabAccount_Group TYPE option<string>; \
         DEFINE FIELD OVERWRITE account_type_internal   ON tabAccount_Group TYPE option<string>; \
         DEFINE FIELD OVERWRITE fs_type                 ON tabAccount_Group TYPE option<string>; \
         DEFINE FIELD OVERWRITE is_reconciliation_group ON tabAccount_Group TYPE bool DEFAULT false; \
         DEFINE FIELD OVERWRITE party_required          ON tabAccount_Group TYPE bool DEFAULT false; \
         DEFINE FIELD OVERWRITE docstatus               ON tabAccount_Group TYPE int DEFAULT 0; \
         DEFINE FIELD OVERWRITE owner                   ON tabAccount_Group TYPE option<string>; \
         DEFINE FIELD OVERWRITE creation                ON tabAccount_Group TYPE option<datetime>; \
         DEFINE FIELD OVERWRITE modified                ON tabAccount_Group TYPE option<datetime>; \
         DEFINE FIELD OVERWRITE modified_by             ON tabAccount_Group TYPE option<string>; \
         DEFINE FIELD OVERWRITE doctype                 ON tabAccount_Group TYPE option<string>; \
         DEFINE INDEX OVERWRITE idx_ag_name ON tabAccount_Group FIELDS name UNIQUE;",
        vec![],
    )
    .await
    .expect("define tabAccount_Group");

    db
}

// ── 1. Plain CRUD ─────────────────────────────────────────────────────────────

#[tokio::test]
#[cfg_attr(not(feature = "integration"), ignore = "requires live SurrealDB")]
async fn test_account_group_insert() {
    let url = std::env::var("SURREAL_TEST_URL")
        .unwrap_or_else(|_| "ws://127.0.0.1:8000".to_string());
    let cfg = DatabaseConfig {
        url, user: "root".into(), pass: "root".into(),
        ns: "test_ns".into(), db: "test_ag_crud".into(),
    };
    let db = DbAdapter::connect(&cfg).await.expect("connect");
    db.execute(
        "DEFINE TABLE IF NOT EXISTS tabAccount_Group SCHEMAFULL; \
         DEFINE FIELD OVERWRITE name               ON tabAccount_Group TYPE string; \
         DEFINE FIELD OVERWRITE group_name         ON tabAccount_Group TYPE option<string>; \
         DEFINE FIELD OVERWRITE chart_of_accounts  ON tabAccount_Group TYPE option<string>; \
         DEFINE FIELD OVERWRITE is_group           ON tabAccount_Group TYPE bool DEFAULT true; \
         DEFINE FIELD OVERWRITE docstatus          ON tabAccount_Group TYPE int DEFAULT 0; \
         DEFINE FIELD OVERWRITE owner              ON tabAccount_Group TYPE option<string>; \
         DEFINE FIELD OVERWRITE creation           ON tabAccount_Group TYPE option<datetime>; \
         DEFINE FIELD OVERWRITE modified           ON tabAccount_Group TYPE option<datetime>; \
         DEFINE FIELD OVERWRITE modified_by        ON tabAccount_Group TYPE option<string>; \
         DEFINE FIELD OVERWRITE doctype            ON tabAccount_Group TYPE option<string>; \
         DEFINE INDEX OVERWRITE idx_ag_crud ON tabAccount_Group FIELDS name UNIQUE;",
        vec![],
    )
    .await
    .expect("DDL");

    delete_doc(&db, "Account Group", "AG-CRUD-0001").await.ok();
    insert_doc(&db, "Account Group", &json!({
        "name": "AG-CRUD-0001",
        "group_name": "Asset",
        "chart_of_accounts": "Standard",
        "is_group": true
    }))
    .await
    .expect("insert Account Group");

    let fetched = get_doc(&db, "Account Group", "AG-CRUD-0001")
        .await
        .expect("get Account Group");
    assert_eq!(
        fetched.fields.get("group_name").and_then(|v| v.as_str()),
        Some("Asset")
    );
    delete_doc(&db, "Account Group", "AG-CRUD-0001").await.ok();
}

// ── 2. Missing number_from / number_to is rejected ────────────────────────────

/// `fn::validate::account_group_number_range` — both number_from and number_to
/// are required; omitting them returns an error.
#[tokio::test]
#[cfg_attr(not(feature = "integration"), ignore = "requires live SurrealDB")]
async fn test_account_group_number_range_required() {
    let db = setup_ag_pipeline("range_req").await;
    delete_doc(&db, "Account Group", "AG-NO-RANGE").await.ok();

    insert_doc(
        &db,
        "Account Group",
        &json!({
            "name":              "AG-NO-RANGE",
            "group_name":        "AG-NO-RANGE",
            "chart_of_accounts": "STANDARD",
            "account_category":  "Cash and Bank",
            "docstatus":         0
        }),
    )
    .await
    .expect("insert account group without range");

    let result = run_pipeline(&db, "tabAccount_Group", "AG-NO-RANGE", "Account Group", "save").await;
    assert!(result.is_err(), "missing number_from must fail; got: {result:?}");
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.to_lowercase().contains("number from") || msg.to_lowercase().contains("required"),
        "error must mention number_from; got: {msg}"
    );

    delete_doc(&db, "Account Group", "AG-NO-RANGE").await.ok();
}

// ── 3. Inverted range (number_from >= number_to) is rejected ─────────────────

/// `fn::validate::account_group_number_range` — number_from must be strictly
/// less than number_to.
#[tokio::test]
#[cfg_attr(not(feature = "integration"), ignore = "requires live SurrealDB")]
async fn test_account_group_range_inverted() {
    let db = setup_ag_pipeline("range_inv").await;
    delete_doc(&db, "Account Group", "AG-INVERTED").await.ok();

    insert_doc(
        &db,
        "Account Group",
        &json!({
            "name":              "AG-INVERTED",
            "group_name":        "AG-INVERTED",
            "chart_of_accounts": "STANDARD",
            "number_from":       "2000",
            "number_to":         "1000",   // <-- inverted
            "account_category":  "Cash and Bank",
            "docstatus":         0
        }),
    )
    .await
    .expect("insert inverted-range account group");

    let result = run_pipeline(&db, "tabAccount_Group", "AG-INVERTED", "Account Group", "save").await;
    assert!(result.is_err(), "inverted range must fail; got: {result:?}");
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.to_lowercase().contains("number from") || msg.to_lowercase().contains("less than"),
        "error must mention range order; got: {msg}"
    );

    delete_doc(&db, "Account Group", "AG-INVERTED").await.ok();
}

// ── 4. Overlapping range in the same CoA is rejected ─────────────────────────

/// `fn::validate::account_group_number_range` — two groups in the same CoA
/// cannot have overlapping number ranges.
#[tokio::test]
#[cfg_attr(not(feature = "integration"), ignore = "requires live SurrealDB")]
async fn test_account_group_range_overlap() {
    let db = setup_ag_pipeline("range_overlap").await;
    delete_doc(&db, "Account Group", "AG-OVL-FIRST").await.ok();
    delete_doc(&db, "Account Group", "AG-OVL-SECOND").await.ok();

    // Insert first group occupying 1000–1999
    insert_doc(
        &db,
        "Account Group",
        &json!({
            "name":              "AG-OVL-FIRST",
            "group_name":        "AG-OVL-FIRST",
            "chart_of_accounts": "STANDARD",
            "number_from":       "1000",
            "number_to":         "1999",
            "account_category":  "Cash and Bank",
            "docstatus":         0
        }),
    )
    .await
    .expect("insert first account group");

    // Save the first group to register it
    let first_save = run_pipeline(
        &db, "tabAccount_Group", "AG-OVL-FIRST", "Account Group", "save"
    ).await;
    assert!(first_save.is_ok(), "first group save must succeed; got: {first_save:?}");

    // Insert second group with overlapping range 1500–2500
    insert_doc(
        &db,
        "Account Group",
        &json!({
            "name":              "AG-OVL-SECOND",
            "group_name":        "AG-OVL-SECOND",
            "chart_of_accounts": "STANDARD",
            "number_from":       "1500",   // overlaps with 1000–1999
            "number_to":         "2500",
            "account_category":  "Trade Receivable",
            "docstatus":         0
        }),
    )
    .await
    .expect("insert overlapping account group");

    let result = run_pipeline(
        &db, "tabAccount_Group", "AG-OVL-SECOND", "Account Group", "save"
    ).await;
    assert!(result.is_err(), "overlapping range must fail; got: {result:?}");
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.to_lowercase().contains("overlap") || msg.to_lowercase().contains("range"),
        "error must mention overlap; got: {msg}"
    );

    delete_doc(&db, "Account Group", "AG-OVL-FIRST").await.ok();
    delete_doc(&db, "Account Group", "AG-OVL-SECOND").await.ok();
}

// ── 5. Compute stage derives classification from account_category ─────────────

/// `fn::compute::account_group_derive_classification` — saving an Account Group
/// with account_category="Revenue" must derive fs_type="Income",
/// account_type_internal="G", is_reconciliation_group=false.
#[tokio::test]
#[cfg_attr(not(feature = "integration"), ignore = "requires live SurrealDB")]
async fn test_account_group_derive_classification() {
    let db = setup_ag_pipeline("derive_class").await;
    delete_doc(&db, "Account Group", "AG-REVENUE-001").await.ok();

    insert_doc(
        &db,
        "Account Group",
        &json!({
            "name":              "AG-REVENUE-001",
            "group_name":        "AG-REVENUE-001",
            "chart_of_accounts": "STANDARD",
            "number_from":       "4000",
            "number_to":         "4999",
            "account_category":  "Revenue",  // → fs_type=Income, type=G
            "docstatus":         0
        }),
    )
    .await
    .expect("insert account group");

    let result = run_pipeline(
        &db, "tabAccount_Group", "AG-REVENUE-001", "Account Group", "save"
    ).await;
    assert!(
        result.is_ok(),
        "save with valid category must succeed; got: {result:?}"
    );

    let grp = get_doc(&db, "Account Group", "AG-REVENUE-001")
        .await
        .expect("fetch group after save");
    assert_eq!(
        grp.fields.get("fs_type").and_then(|v| v.as_str()),
        Some("Income"),
        "fs_type must be derived as 'Income' from category 'Revenue'"
    );
    assert_eq!(
        grp.fields.get("account_type_internal").and_then(|v| v.as_str()),
        Some("G"),
        "account_type_internal must be 'G' for Revenue category"
    );

    delete_doc(&db, "Account Group", "AG-REVENUE-001").await.ok();
}
