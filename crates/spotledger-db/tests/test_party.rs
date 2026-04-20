//! Integration tests for Party — customers, suppliers, employees in the ledger.
//!
//! ## Running
//! ```sh
//! $env:SURREAL_TEST_URL="ws://127.0.0.1:8000"
//! cargo test -p spotledger-db --features integration --test test_party -- --test-threads=1
//! ```
//!
//! ## Test plan
//! | # | Test | Verifies |
//! |---|------|---------|
//! | 1 | `test_party_insert`                 | Basic insert + fetch round-trip |
//! | 2 | `test_party_roles_required`         | Party with no roles → error "at least one role" |
//! | 3 | `test_party_invalid_role_rejected`  | Unknown role name → error |
//! | 4 | `test_party_tax_id_unique`          | Duplicate tax_id → error |
//! | 5 | `test_party_role_config_autocreated`| On save with a company, PRC record auto-created |

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

async fn setup_party_pipeline(suffix: &str) -> DbAdapter {
    let db = common::make_pipeline_db(&format!("party_{suffix}")).await;
    common::seed_doctype(&db, "Party", false).await;

    db.execute(
        "DEFINE TABLE IF NOT EXISTS tabParty SCHEMAFULL; \
         DEFINE FIELD OVERWRITE name             ON tabParty TYPE string; \
         DEFINE FIELD OVERWRITE party_name       ON tabParty TYPE option<string>; \
         DEFINE FIELD OVERWRITE roles            ON tabParty TYPE any DEFAULT []; \
         DEFINE FIELD OVERWRITE tax_id           ON tabParty TYPE option<string>; \
         DEFINE FIELD OVERWRITE default_currency ON tabParty TYPE option<string>; \
         DEFINE FIELD OVERWRITE company          ON tabParty TYPE option<string>; \
         DEFINE FIELD OVERWRITE disabled         ON tabParty TYPE bool DEFAULT false; \
         DEFINE FIELD OVERWRITE docstatus        ON tabParty TYPE int DEFAULT 0; \
         DEFINE FIELD OVERWRITE owner            ON tabParty TYPE option<string>; \
         DEFINE FIELD OVERWRITE creation         ON tabParty TYPE option<datetime>; \
         DEFINE FIELD OVERWRITE modified         ON tabParty TYPE option<datetime>; \
         DEFINE FIELD OVERWRITE modified_by      ON tabParty TYPE option<string>; \
         DEFINE FIELD OVERWRITE doctype          ON tabParty TYPE option<string>; \
         DEFINE INDEX OVERWRITE idx_party_name ON tabParty FIELDS name UNIQUE;",
        vec![],
    )
    .await
    .expect("define tabParty");

    // Party Role Config — needed by party_role_config_autocreate on_save
    db.execute(
        "DEFINE TABLE IF NOT EXISTS tabParty_Role_Config SCHEMALESS;",
        vec![],
    )
    .await
    .expect("define tabParty_Role_Config");

    // Company — needed by party_role_config_autocreate (SELECT FROM tabCompany)
    db.execute(
        "DEFINE TABLE IF NOT EXISTS tabCompany SCHEMALESS;",
        vec![],
    )
    .await
    .expect("define tabCompany");

    db
}

// ── 1. Plain CRUD ─────────────────────────────────────────────────────────────

#[tokio::test]
#[cfg_attr(not(feature = "integration"), ignore = "requires live SurrealDB")]
async fn test_party_insert() {
    let url = std::env::var("SURREAL_TEST_URL")
        .unwrap_or_else(|_| "ws://127.0.0.1:8000".to_string());
    let cfg = DatabaseConfig {
        url, user: "root".into(), pass: "root".into(),
        ns: "test_ns".into(), db: "test_party_crud".into(),
    };
    let db = DbAdapter::connect(&cfg).await.expect("connect");
    db.execute(
        "DEFINE TABLE IF NOT EXISTS tabParty SCHEMAFULL; \
         DEFINE FIELD OVERWRITE name             ON tabParty TYPE string; \
         DEFINE FIELD OVERWRITE party_name       ON tabParty TYPE option<string>; \
         DEFINE FIELD OVERWRITE roles            ON tabParty TYPE any DEFAULT []; \
         DEFINE FIELD OVERWRITE default_currency ON tabParty TYPE option<string>; \
         DEFINE FIELD OVERWRITE docstatus        ON tabParty TYPE int DEFAULT 0; \
         DEFINE FIELD OVERWRITE owner            ON tabParty TYPE option<string>; \
         DEFINE FIELD OVERWRITE creation         ON tabParty TYPE option<datetime>; \
         DEFINE FIELD OVERWRITE modified         ON tabParty TYPE option<datetime>; \
         DEFINE FIELD OVERWRITE modified_by      ON tabParty TYPE option<string>; \
         DEFINE FIELD OVERWRITE doctype          ON tabParty TYPE option<string>; \
         DEFINE INDEX OVERWRITE idx_party_crud ON tabParty FIELDS name UNIQUE;",
        vec![],
    )
    .await
    .expect("DDL");

    delete_doc(&db, "Party", "CUST-0001").await.ok();
    insert_doc(&db, "Party", &json!({
        "name": "CUST-0001",
        "party_name": "Acme Corp",
        "roles": [{"role": "Customer"}],
        "default_currency": "USD"
    }))
    .await
    .expect("insert Party");

    let fetched = get_doc(&db, "Party", "CUST-0001").await.expect("get Party");
    assert_eq!(
        fetched.fields.get("party_name").and_then(|v| v.as_str()),
        Some("Acme Corp")
    );
    delete_doc(&db, "Party", "CUST-0001").await.ok();
}

// ── 2. Party with no roles is rejected ────────────────────────────────────────

/// `fn::validate::party_roles` — roles array must have at least one entry.
#[tokio::test]
#[cfg_attr(not(feature = "integration"), ignore = "requires live SurrealDB")]
async fn test_party_roles_required() {
    let db = setup_party_pipeline("roles_req").await;
    delete_doc(&db, "Party", "PARTY-NO-ROLES").await.ok();

    insert_doc(
        &db,
        "Party",
        &json!({
            "name":       "PARTY-NO-ROLES",
            "party_name": "Empty Roles Co",
            "roles":      [],   // <-- empty (no role objects)
            "docstatus":  0
        }),
    )
    .await
    .expect("insert party with no roles");

    let result = run_pipeline(&db, "tabParty", "PARTY-NO-ROLES", "Party", "save").await;
    assert!(result.is_err(), "party with no roles must fail; got: {result:?}");
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.to_lowercase().contains("role") || msg.to_lowercase().contains("at least"),
        "error must mention roles; got: {msg}"
    );

    delete_doc(&db, "Party", "PARTY-NO-ROLES").await.ok();
}

// ── 3. Unknown role name is rejected ──────────────────────────────────────────

/// `fn::validate::party_roles` — each role must be one of the recognised set.
#[tokio::test]
#[cfg_attr(not(feature = "integration"), ignore = "requires live SurrealDB")]
async fn test_party_invalid_role_rejected() {
    let db = setup_party_pipeline("bad_role").await;
    delete_doc(&db, "Party", "PARTY-BAD-ROLE").await.ok();

    insert_doc(
        &db,
        "Party",
        &json!({
            "name":       "PARTY-BAD-ROLE",
            "party_name": "Acme Corp",
            "roles":      [{"role": "InvalidRole"}],   // <-- not in the allowed list
            "docstatus":  0
        }),
    )
    .await
    .expect("insert party with invalid role");

    let result = run_pipeline(&db, "tabParty", "PARTY-BAD-ROLE", "Party", "save").await;
    assert!(result.is_err(), "invalid role must fail; got: {result:?}");
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.to_lowercase().contains("invalid role") || msg.to_lowercase().contains("valid role"),
        "error must mention invalid role; got: {msg}"
    );

    delete_doc(&db, "Party", "PARTY-BAD-ROLE").await.ok();
}

// ── 4. Duplicate tax_id is rejected ───────────────────────────────────────────

/// `fn::validate::party_tax_id_unique` — two parties may not share the same tax_id.
#[tokio::test]
#[cfg_attr(not(feature = "integration"), ignore = "requires live SurrealDB")]
async fn test_party_tax_id_unique() {
    let db = setup_party_pipeline("tax_id").await;
    delete_doc(&db, "Party", "PARTY-TAX-FIRST").await.ok();
    delete_doc(&db, "Party", "PARTY-TAX-DUP").await.ok();

    // Insert first party with tax_id
    insert_doc(
        &db,
        "Party",
        &json!({
            "name":       "PARTY-TAX-FIRST",
            "party_name": "PARTY-TAX-FIRST",
            "roles":      [{"role": "Customer"}],
            "tax_id":     "TAX-99999",
            "docstatus":  0
        }),
    )
    .await
    .expect("insert first party");

    let first_save = run_pipeline(&db, "tabParty", "PARTY-TAX-FIRST", "Party", "save").await;
    assert!(first_save.is_ok(), "first party save must succeed; got: {first_save:?}");

    // Insert second party with the SAME tax_id
    insert_doc(
        &db,
        "Party",
        &json!({
            "name":       "PARTY-TAX-DUP",
            "party_name": "PARTY-TAX-DUP",
            "roles":      [{"role": "Supplier"}],
            "tax_id":     "TAX-99999",   // <-- duplicate
            "docstatus":  0
        }),
    )
    .await
    .expect("insert duplicate-tax party");

    let result = run_pipeline(&db, "tabParty", "PARTY-TAX-DUP", "Party", "save").await;
    assert!(result.is_err(), "duplicate tax_id must fail; got: {result:?}");
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.to_lowercase().contains("tax"),
        "error must mention tax ID; got: {msg}"
    );

    delete_doc(&db, "Party", "PARTY-TAX-FIRST").await.ok();
    delete_doc(&db, "Party", "PARTY-TAX-DUP").await.ok();
}

// ── 5. Party Role Config is auto-created on save ──────────────────────────────

/// `fn::on_save::party_role_config_autocreate` — when a Party is saved and a
/// Company record exists, a Party Role Config record must be created for each
/// (party, role, company) combination that doesn't yet exist.
#[tokio::test]
#[cfg_attr(not(feature = "integration"), ignore = "requires live SurrealDB")]
async fn test_party_role_config_autocreated() {
    let db = setup_party_pipeline("prc_autocreate").await;
    delete_doc(&db, "Party", "PARTY-PRC").await.ok();

    // Seed a Company so the loop in party_role_config_autocreate has something to iterate
    insert_doc(
        &db,
        "Company",
        &json!({
            "name": "TestCo-PRC",
            "company_name": "TestCo PRC",
            "abbr": "TPRC"
        }),
    )
    .await
    .ok();

    insert_doc(
        &db,
        "Party",
        &json!({
            "name":       "PARTY-PRC",
            "party_name": "PARTY-PRC",
            "roles":      [{"role": "Customer"}],
            "docstatus":  0
        }),
    )
    .await
    .expect("insert party for PRC test");

    let result = run_pipeline(&db, "tabParty", "PARTY-PRC", "Party", "save").await;
    assert!(
        result.is_ok(),
        "party save must succeed; got: {result:?}"
    );

    // A Party Role Config for (PARTY-PRC, Customer, TestCo-PRC) must now exist
    let prc = db
        .run(
            "SELECT id FROM tabParty_Role_Config WHERE party = 'PARTY-PRC' AND role = 'Customer' AND company = 'TestCo-PRC' LIMIT 1",
            vec![],
        )
        .await
        .expect("query PRC");
    assert!(
        !prc.is_empty(),
        "Party Role Config must be auto-created on Party save"
    );

    delete_doc(&db, "Party", "PARTY-PRC").await.ok();
}
