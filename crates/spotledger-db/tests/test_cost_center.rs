//! Integration tests for Cost Center — expense tracking dimension in the ledger.
//!
//! ## Running
//! ```sh
//! $env:SURREAL_TEST_URL="ws://127.0.0.1:8000"
//! cargo test -p spotledger-db --features integration --test test_cost_center -- --test-threads=1
//! ```
//!
//! ## Test plan
//! | # | Test | Verifies |
//! |---|------|---------|
//! | 1 | `test_cost_center_insert`            | Basic insert + fetch round-trip |
//! | 2 | `test_cost_center_code_required`     | Missing cost_center_code → error |
//! | 3 | `test_cost_center_code_unique`       | Duplicate code within same company → error |
//! | 4 | `test_cost_center_parent_edge`       | child_of edge is created on save when parent set |

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

const COMPANY: &str = "TestCo-CC";

// ── Setup ─────────────────────────────────────────────────────────────────────

async fn setup_cc_pipeline(suffix: &str) -> DbAdapter {
    let db = common::make_pipeline_db(&format!("cost_center_{suffix}")).await;
    common::seed_doctype(&db, "Cost Center", false).await;

    db.execute(
        "DEFINE TABLE IF NOT EXISTS tabCost_Center SCHEMAFULL; \
         DEFINE FIELD OVERWRITE name               ON tabCost_Center TYPE string; \
         DEFINE FIELD OVERWRITE cost_center_name   ON tabCost_Center TYPE option<string>; \
         DEFINE FIELD OVERWRITE cost_center_code   ON tabCost_Center TYPE option<string>; \
         DEFINE FIELD OVERWRITE company            ON tabCost_Center TYPE option<string>; \
         DEFINE FIELD OVERWRITE parent_cost_center ON tabCost_Center TYPE option<string>; \
         DEFINE FIELD OVERWRITE is_group           ON tabCost_Center TYPE bool DEFAULT false; \
         DEFINE FIELD OVERWRITE docstatus          ON tabCost_Center TYPE int DEFAULT 0; \
         DEFINE FIELD OVERWRITE owner              ON tabCost_Center TYPE option<string>; \
         DEFINE FIELD OVERWRITE creation           ON tabCost_Center TYPE option<datetime>; \
         DEFINE FIELD OVERWRITE modified           ON tabCost_Center TYPE option<datetime>; \
         DEFINE FIELD OVERWRITE modified_by        ON tabCost_Center TYPE option<string>; \
         DEFINE FIELD OVERWRITE doctype            ON tabCost_Center TYPE option<string>; \
         DEFINE INDEX OVERWRITE idx_cc_name ON tabCost_Center FIELDS name UNIQUE;",
        vec![],
    )
    .await
    .expect("define tabCost_Center");

    // child_of edge table — needed by cost_center_relate_parent
    db.execute(
        "REMOVE TABLE IF EXISTS child_of; DEFINE TABLE child_of TYPE RELATION SCHEMALESS;",
        vec![],
    )
    .await
    .expect("define child_of");

    db
}

// ── 1. Plain CRUD ─────────────────────────────────────────────────────────────

#[tokio::test]
#[cfg_attr(not(feature = "integration"), ignore = "requires live SurrealDB")]
async fn test_cost_center_insert() {
    let url = std::env::var("SURREAL_TEST_URL")
        .unwrap_or_else(|_| "ws://127.0.0.1:8000".to_string());
    let cfg = DatabaseConfig {
        url, user: "root".into(), pass: "root".into(),
        ns: "test_ns".into(), db: "test_cc_crud".into(),
    };
    let db = DbAdapter::connect(&cfg).await.expect("connect");
    db.execute(
        "DEFINE TABLE IF NOT EXISTS tabCost_Center SCHEMAFULL; \
         DEFINE FIELD OVERWRITE name             ON tabCost_Center TYPE string; \
         DEFINE FIELD OVERWRITE cost_center_name ON tabCost_Center TYPE option<string>; \
         DEFINE FIELD OVERWRITE cost_center_code ON tabCost_Center TYPE option<string>; \
         DEFINE FIELD OVERWRITE company          ON tabCost_Center TYPE option<string>; \
         DEFINE FIELD OVERWRITE docstatus        ON tabCost_Center TYPE int DEFAULT 0; \
         DEFINE FIELD OVERWRITE owner            ON tabCost_Center TYPE option<string>; \
         DEFINE FIELD OVERWRITE creation         ON tabCost_Center TYPE option<datetime>; \
         DEFINE FIELD OVERWRITE modified         ON tabCost_Center TYPE option<datetime>; \
         DEFINE FIELD OVERWRITE modified_by      ON tabCost_Center TYPE option<string>; \
         DEFINE FIELD OVERWRITE doctype          ON tabCost_Center TYPE option<string>; \
         DEFINE INDEX OVERWRITE idx_cc_crud ON tabCost_Center FIELDS name UNIQUE;",
        vec![],
    )
    .await
    .expect("DDL");

    delete_doc(&db, "Cost Center", "CC-MAIN").await.ok();
    insert_doc(&db, "Cost Center", &json!({
        "name": "CC-MAIN",
        "cost_center_name": "Main Operations",
        "cost_center_code": "OPS",
        "company": COMPANY
    }))
    .await
    .expect("insert Cost Center");

    let fetched = get_doc(&db, "Cost Center", "CC-MAIN").await.expect("get Cost Center");
    assert_eq!(
        fetched.fields.get("cost_center_name").and_then(|v| v.as_str()),
        Some("Main Operations")
    );
    delete_doc(&db, "Cost Center", "CC-MAIN").await.ok();
}

// ── 2. Missing cost_center_code is rejected ────────────────────────────────────

/// `fn::validate::cost_center_code_unique` — cost_center_code is required.
#[tokio::test]
#[cfg_attr(not(feature = "integration"), ignore = "requires live SurrealDB")]
async fn test_cost_center_code_required() {
    let db = setup_cc_pipeline("code_req").await;
    delete_doc(&db, "Cost Center", "CC-NO-CODE").await.ok();

    insert_doc(
        &db,
        "Cost Center",
        &json!({
            "name":             "CC-NO-CODE",
            "cost_center_name": "CC-NO-CODE",
            "company":          COMPANY,
            // cost_center_code intentionally omitted
            "docstatus": 0
        }),
    )
    .await
    .expect("insert cc without code");

    let result = run_pipeline(&db, "tabCost_Center", "CC-NO-CODE", "Cost Center", "save").await;
    assert!(result.is_err(), "missing code must fail; got: {result:?}");
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.to_lowercase().contains("code") || msg.to_lowercase().contains("required"),
        "error must mention code; got: {msg}"
    );

    delete_doc(&db, "Cost Center", "CC-NO-CODE").await.ok();
}

// ── 3. Duplicate code within same company is rejected ─────────────────────────

/// `fn::validate::cost_center_code_unique` — two Cost Centers in the same company
/// cannot share a cost_center_code.
#[tokio::test]
#[cfg_attr(not(feature = "integration"), ignore = "requires live SurrealDB")]
async fn test_cost_center_code_unique() {
    let db = setup_cc_pipeline("code_unique").await;
    delete_doc(&db, "Cost Center", "CC-CODE-FIRST").await.ok();
    delete_doc(&db, "Cost Center", "CC-CODE-DUP").await.ok();

    // Insert and save first cost center
    insert_doc(
        &db,
        "Cost Center",
        &json!({
            "name":             "CC-CODE-FIRST",
            "cost_center_name": "CC-CODE-FIRST",
            "cost_center_code": "OPS001",
            "company":          COMPANY,
            "docstatus":        0
        }),
    )
    .await
    .expect("insert first cc");
    let first = run_pipeline(&db, "tabCost_Center", "CC-CODE-FIRST", "Cost Center", "save").await;
    assert!(first.is_ok(), "first cc save must succeed; got: {first:?}");

    // Insert second cost center with the same code in the same company
    insert_doc(
        &db,
        "Cost Center",
        &json!({
            "name":             "CC-CODE-DUP",
            "cost_center_name": "CC-CODE-DUP",
            "cost_center_code": "OPS001",   // <-- duplicate
            "company":          COMPANY,
            "docstatus":        0
        }),
    )
    .await
    .expect("insert duplicate-code cc");

    let result = run_pipeline(&db, "tabCost_Center", "CC-CODE-DUP", "Cost Center", "save").await;
    assert!(result.is_err(), "duplicate code must fail; got: {result:?}");
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.to_lowercase().contains("already used") || msg.to_lowercase().contains("code"),
        "error must mention duplicate code; got: {msg}"
    );

    delete_doc(&db, "Cost Center", "CC-CODE-FIRST").await.ok();
    delete_doc(&db, "Cost Center", "CC-CODE-DUP").await.ok();
}

// ── 4. child_of edge is created when parent_cost_center is set ────────────────

/// `fn::on_save::cost_center_relate_parent` — saving a Cost Center with a parent
/// must create a `child_of` edge from the child to the parent.
#[tokio::test]
#[cfg_attr(not(feature = "integration"), ignore = "requires live SurrealDB")]
async fn test_cost_center_parent_edge() {
    let db = setup_cc_pipeline("parent_edge").await;
    delete_doc(&db, "Cost Center", "CC-PARENT-ROOT").await.ok();
    delete_doc(&db, "Cost Center", "CC-CHILD").await.ok();

    // Insert parent cost center
    insert_doc(
        &db,
        "Cost Center",
        &json!({
            "name":             "CC-PARENT-ROOT",
            "cost_center_name": "CC-PARENT-ROOT",
            "cost_center_code": "ROOT",
            "company":          COMPANY,
            "is_group":         true,
            "docstatus":        0
        }),
    )
    .await
    .expect("insert parent cc");
    let parent_save = run_pipeline(
        &db, "tabCost_Center", "CC-PARENT-ROOT", "Cost Center", "save"
    ).await;
    assert!(parent_save.is_ok(), "parent cc save must succeed; got: {parent_save:?}");

    // Insert child cost center pointing at the parent
    insert_doc(
        &db,
        "Cost Center",
        &json!({
            "name":               "CC-CHILD",
            "cost_center_name":   "CC-CHILD",
            "cost_center_code":   "CHILD001",
            "company":            COMPANY,
            "parent_cost_center": "CC-PARENT-ROOT",
            "is_group":           false,
            "docstatus":          0
        }),
    )
    .await
    .expect("insert child cc");
    let child_save = run_pipeline(
        &db, "tabCost_Center", "CC-CHILD", "Cost Center", "save"
    ).await;
    assert!(child_save.is_ok(), "child cc save must succeed; got: {child_save:?}");

    // Verify the child_of edge exists: tabCost_Center:CC-CHILD → child_of → tabCost_Center:CC-PARENT-ROOT
    let edges = db
        .run(
            "SELECT id FROM child_of WHERE in = tabCost_Center:⟨CC-CHILD⟩ LIMIT 1",
            vec![],
        )
        .await
        .expect("query child_of edges");
    assert!(
        !edges.is_empty(),
        "child_of edge must be created for child Cost Center with a parent"
    );

    delete_doc(&db, "Cost Center", "CC-PARENT-ROOT").await.ok();
    delete_doc(&db, "Cost Center", "CC-CHILD").await.ok();
}
