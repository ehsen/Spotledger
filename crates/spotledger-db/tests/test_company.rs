//! Integration tests for Company — the top-level legal entity in SpotLedger Finance.
//!
//! ## Running
//! ```sh
//! $env:SURREAL_TEST_URL="ws://127.0.0.1:8000"
//! cargo test -p spotledger-db --features integration --test test_company -- --test-threads=1
//! ```
//!
//! ## Test plan
//! | # | Test | Verifies |
//! |---|------|---------|
//! | 1 | `test_company_insert`             | Basic insert + fetch round-trip |
//! | 2 | `test_company_abbr_required`      | Missing abbreviation → error |
//! | 3 | `test_company_abbr_too_long`      | Abbreviation > 10 chars → error |
//! | 4 | `test_company_abbr_unique`        | Duplicate abbreviation → error |

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

async fn setup_company_pipeline(suffix: &str) -> DbAdapter {
    let db = common::make_pipeline_db(&format!("company_{suffix}")).await;
    common::seed_doctype(&db, "Company", false).await;

    db.execute(
        "DEFINE TABLE IF NOT EXISTS tabCompany SCHEMAFULL; \
         DEFINE FIELD OVERWRITE name          ON tabCompany TYPE string; \
         DEFINE FIELD OVERWRITE company_name  ON tabCompany TYPE option<string>; \
         DEFINE FIELD OVERWRITE abbr          ON tabCompany TYPE option<string>; \
         DEFINE FIELD OVERWRITE default_currency ON tabCompany TYPE option<string>; \
         DEFINE FIELD OVERWRITE country       ON tabCompany TYPE option<string>; \
         DEFINE FIELD OVERWRITE docstatus     ON tabCompany TYPE int DEFAULT 0; \
         DEFINE FIELD OVERWRITE owner         ON tabCompany TYPE option<string>; \
         DEFINE FIELD OVERWRITE creation      ON tabCompany TYPE option<datetime>; \
         DEFINE FIELD OVERWRITE modified      ON tabCompany TYPE option<datetime>; \
         DEFINE FIELD OVERWRITE modified_by   ON tabCompany TYPE option<string>; \
         DEFINE FIELD OVERWRITE doctype       ON tabCompany TYPE option<string>; \
         DEFINE INDEX OVERWRITE idx_co_name ON tabCompany FIELDS name UNIQUE;",
        vec![],
    )
    .await
    .expect("define tabCompany");

    db
}

// ── 1. Plain CRUD ─────────────────────────────────────────────────────────────

#[tokio::test]
#[cfg_attr(not(feature = "integration"), ignore = "requires live SurrealDB")]
async fn test_company_insert() {
    let url = std::env::var("SURREAL_TEST_URL")
        .unwrap_or_else(|_| "ws://127.0.0.1:8000".to_string());
    let cfg = DatabaseConfig {
        url, user: "root".into(), pass: "root".into(),
        ns: "test_ns".into(), db: "test_company_crud".into(),
    };
    let db = DbAdapter::connect(&cfg).await.expect("connect");
    db.execute(
        "DEFINE TABLE IF NOT EXISTS tabCompany SCHEMAFULL; \
         DEFINE FIELD OVERWRITE name         ON tabCompany TYPE string; \
         DEFINE FIELD OVERWRITE company_name ON tabCompany TYPE option<string>; \
         DEFINE FIELD OVERWRITE abbr         ON tabCompany TYPE option<string>; \
         DEFINE FIELD OVERWRITE docstatus    ON tabCompany TYPE int DEFAULT 0; \
         DEFINE FIELD OVERWRITE owner        ON tabCompany TYPE option<string>; \
         DEFINE FIELD OVERWRITE creation     ON tabCompany TYPE option<datetime>; \
         DEFINE FIELD OVERWRITE modified     ON tabCompany TYPE option<datetime>; \
         DEFINE FIELD OVERWRITE modified_by  ON tabCompany TYPE option<string>; \
         DEFINE FIELD OVERWRITE doctype      ON tabCompany TYPE option<string>; \
         DEFINE INDEX OVERWRITE idx_co_crud ON tabCompany FIELDS name UNIQUE;",
        vec![],
    )
    .await
    .expect("DDL");

    delete_doc(&db, "Company", "Acme Ltd").await.ok();
    insert_doc(&db, "Company", &json!({
        "name": "Acme Ltd",
        "company_name": "Acme Limited",
        "abbr": "ACME"
    }))
    .await
    .expect("insert Company");

    let fetched = get_doc(&db, "Company", "Acme Ltd").await.expect("get Company");
    assert_eq!(
        fetched.fields.get("abbr").and_then(|v| v.as_str()),
        Some("ACME")
    );
    delete_doc(&db, "Company", "Acme Ltd").await.ok();
}

// ── 2. Missing abbreviation is rejected ───────────────────────────────────────

/// `fn::validate::company_abbr_format` — abbr is required.
#[tokio::test]
#[cfg_attr(not(feature = "integration"), ignore = "requires live SurrealDB")]
async fn test_company_abbr_required() {
    let db = setup_company_pipeline("abbr_req").await;
    delete_doc(&db, "Company", "CO-NO-ABBR").await.ok();

    insert_doc(
        &db,
        "Company",
        &json!({
            "name":         "CO-NO-ABBR",
            "company_name": "No Abbreviation Co",
            // abbr intentionally omitted
            "docstatus":    0
        }),
    )
    .await
    .expect("insert company without abbr");

    let result = run_pipeline(&db, "tabCompany", "CO-NO-ABBR", "Company", "save").await;
    assert!(result.is_err(), "missing abbr must fail; got: {result:?}");
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.to_lowercase().contains("abbreviation") || msg.to_lowercase().contains("required"),
        "error must mention abbreviation; got: {msg}"
    );

    delete_doc(&db, "Company", "CO-NO-ABBR").await.ok();
}

// ── 3. Abbreviation longer than 10 characters is rejected ─────────────────────

/// `fn::validate::company_abbr_format` — abbr must be ≤ 10 characters.
#[tokio::test]
#[cfg_attr(not(feature = "integration"), ignore = "requires live SurrealDB")]
async fn test_company_abbr_too_long() {
    let db = setup_company_pipeline("abbr_long").await;
    delete_doc(&db, "Company", "CO-LONG-ABBR").await.ok();

    insert_doc(
        &db,
        "Company",
        &json!({
            "name":         "CO-LONG-ABBR",
            "company_name": "Long Abbreviation Co",
            "abbr":         "TOOLONGABBR",  // 11 chars — over the 10-char limit
            "docstatus":    0
        }),
    )
    .await
    .expect("insert company with long abbr");

    let result = run_pipeline(&db, "tabCompany", "CO-LONG-ABBR", "Company", "save").await;
    assert!(result.is_err(), "long abbr must fail; got: {result:?}");
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.to_lowercase().contains("10") || msg.to_lowercase().contains("abbreviation"),
        "error must mention abbreviation length; got: {msg}"
    );

    delete_doc(&db, "Company", "CO-LONG-ABBR").await.ok();
}

// ── 4. Duplicate abbreviation is rejected ─────────────────────────────────────

/// `fn::validate::company_abbr_unique` — no two companies may share the same abbr.
#[tokio::test]
#[cfg_attr(not(feature = "integration"), ignore = "requires live SurrealDB")]
async fn test_company_abbr_unique() {
    let db = setup_company_pipeline("abbr_unique").await;
    delete_doc(&db, "Company", "CO-ABBR-FIRST").await.ok();
    delete_doc(&db, "Company", "CO-ABBR-DUP").await.ok();

    // Insert and save first company
    insert_doc(
        &db,
        "Company",
        &json!({
            "name":         "CO-ABBR-FIRST",
            "company_name": "CO-ABBR-FIRST",
            "abbr":         "UNIQ",
            "docstatus":    0
        }),
    )
    .await
    .expect("insert first company");
    let first = run_pipeline(&db, "tabCompany", "CO-ABBR-FIRST", "Company", "save").await;
    assert!(first.is_ok(), "first company save must succeed; got: {first:?}");

    // Insert second company with the same abbreviation
    insert_doc(
        &db,
        "Company",
        &json!({
            "name":         "CO-ABBR-DUP",
            "company_name": "CO-ABBR-DUP",
            "abbr":         "UNIQ",   // <-- duplicate
            "docstatus":    0
        }),
    )
    .await
    .expect("insert duplicate-abbr company");

    let result = run_pipeline(&db, "tabCompany", "CO-ABBR-DUP", "Company", "save").await;
    assert!(result.is_err(), "duplicate abbr must fail; got: {result:?}");
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.to_lowercase().contains("abbreviation") || msg.to_lowercase().contains("already used"),
        "error must mention duplicate abbreviation; got: {msg}"
    );

    delete_doc(&db, "Company", "CO-ABBR-FIRST").await.ok();
    delete_doc(&db, "Company", "CO-ABBR-DUP").await.ok();
}
