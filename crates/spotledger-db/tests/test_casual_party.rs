//! Integration tests for Casual Party — lightweight counterparties used on
//! non-subledger payment flows.
//!
//! ## Running
//! ```sh
//! $env:SURREAL_TEST_URL="ws://127.0.0.1:8000"
//! cargo test -p spotledger-db --features integration --test test_casual_party -- --test-threads=1
//! ```
//!
//! ## Test plan
//! | # | Test | Verifies |
//! |---|------|---------|
//! | 1 | `test_casual_party_insert` | Basic insert + fetch round-trip |
//! | 2 | `test_casual_party_duplicate_name_warns_but_save_succeeds` | duplicate party_name returns warning but does not block save |

#![cfg(feature = "integration")]

#[path = "common/mod.rs"]
mod common;

use serde_json::{json, Value};
use spotledger_core::config::DatabaseConfig;
use spotledger_db::{
    document::{delete_doc, get_doc, insert_doc},
    pipeline::{run_pipeline, PipelineResult},
    DbAdapter,
};

// ── Setup ─────────────────────────────────────────────────────────────────────

async fn setup_casual_party_pipeline(suffix: &str) -> DbAdapter {
    let db = common::make_pipeline_db(&format!("casual_party_{suffix}")).await;
    common::seed_doctype(&db, "Casual Party", false).await;

    db.execute(
        "DEFINE TABLE IF NOT EXISTS tabCasual_Party SCHEMAFULL; \
         DEFINE FIELD OVERWRITE name           ON tabCasual_Party TYPE string; \
         DEFINE FIELD OVERWRITE party_name     ON tabCasual_Party TYPE option<string>; \
         DEFINE FIELD OVERWRITE mobile_no      ON tabCasual_Party TYPE option<string>; \
         DEFINE FIELD OVERWRITE email_id       ON tabCasual_Party TYPE option<string>; \
         DEFINE FIELD OVERWRITE is_promoted    ON tabCasual_Party TYPE bool DEFAULT false; \
         DEFINE FIELD OVERWRITE promoted_to    ON tabCasual_Party TYPE option<string>; \
         DEFINE FIELD OVERWRITE default_account ON tabCasual_Party TYPE option<string>; \
         DEFINE FIELD OVERWRITE docstatus      ON tabCasual_Party TYPE int DEFAULT 0; \
         DEFINE FIELD OVERWRITE owner          ON tabCasual_Party TYPE option<string>; \
         DEFINE FIELD OVERWRITE creation       ON tabCasual_Party TYPE option<datetime>; \
         DEFINE FIELD OVERWRITE modified       ON tabCasual_Party TYPE option<datetime>; \
         DEFINE FIELD OVERWRITE modified_by    ON tabCasual_Party TYPE option<string>; \
         DEFINE FIELD OVERWRITE doctype        ON tabCasual_Party TYPE option<string>; \
         DEFINE INDEX OVERWRITE idx_cp_name ON tabCasual_Party FIELDS name UNIQUE;",
        vec![],
    )
    .await
    .expect("define tabCasual_Party");

    db
}

// ── 1. Plain CRUD ─────────────────────────────────────────────────────────────

#[tokio::test]
#[cfg_attr(not(feature = "integration"), ignore = "requires live SurrealDB")]
async fn test_casual_party_insert() {
    let url = std::env::var("SURREAL_TEST_URL")
        .unwrap_or_else(|_| "ws://127.0.0.1:8000".to_string());
    let cfg = DatabaseConfig {
        url,
        user: "root".into(),
        pass: "root".into(),
        ns: "test_ns".into(),
        db: "test_casual_party_crud".into(),
    };
    let db = DbAdapter::connect(&cfg).await.expect("connect");

    db.execute(
        "DEFINE TABLE IF NOT EXISTS tabCasual_Party SCHEMAFULL; \
         DEFINE FIELD OVERWRITE name        ON tabCasual_Party TYPE string; \
         DEFINE FIELD OVERWRITE party_name  ON tabCasual_Party TYPE option<string>; \
         DEFINE FIELD OVERWRITE docstatus   ON tabCasual_Party TYPE int DEFAULT 0; \
         DEFINE FIELD OVERWRITE owner       ON tabCasual_Party TYPE option<string>; \
         DEFINE FIELD OVERWRITE creation    ON tabCasual_Party TYPE option<datetime>; \
         DEFINE FIELD OVERWRITE modified    ON tabCasual_Party TYPE option<datetime>; \
         DEFINE FIELD OVERWRITE modified_by ON tabCasual_Party TYPE option<string>; \
         DEFINE FIELD OVERWRITE doctype     ON tabCasual_Party TYPE option<string>; \
         DEFINE INDEX OVERWRITE idx_cp_crud ON tabCasual_Party FIELDS name UNIQUE;",
        vec![],
    )
    .await
    .expect("DDL");

    delete_doc(&db, "Casual Party", "CP-CRUD").await.ok();
    insert_doc(
        &db,
        "Casual Party",
        &json!({
            "name": "CP-CRUD",
            "party_name": "Walk In Guest"
        }),
    )
    .await
    .expect("insert Casual Party");

    let fetched = get_doc(&db, "Casual Party", "CP-CRUD")
        .await
        .expect("get Casual Party");
    assert_eq!(
        fetched.fields.get("party_name").and_then(|v| v.as_str()),
        Some("Walk In Guest")
    );

    delete_doc(&db, "Casual Party", "CP-CRUD").await.ok();
}

// ── 2. Duplicate party_name warns but does not block save ────────────────────

#[tokio::test]
#[cfg_attr(not(feature = "integration"), ignore = "requires live SurrealDB")]
async fn test_casual_party_duplicate_name_warns_but_save_succeeds() {
    let db = setup_casual_party_pipeline("duplicate_warning").await;
    delete_doc(&db, "Casual Party", "CP-FIRST").await.ok();
    delete_doc(&db, "Casual Party", "CP-SECOND").await.ok();

    insert_doc(
        &db,
        "Casual Party",
        &json!({
            "name": "CP-FIRST",
            "party_name": "Walk In Guest",
            "is_promoted": false,
            "docstatus": 0
        }),
    )
    .await
    .expect("insert first casual party");

    let first_save = run_pipeline(&db, "tabCasual_Party", "CP-FIRST", "Casual Party", "save").await;
    assert!(
        matches!(first_save, Ok(PipelineResult::Ok)),
        "first casual party save must succeed; got: {first_save:?}"
    );

    insert_doc(
        &db,
        "Casual Party",
        &json!({
            "name": "CP-SECOND",
            "party_name": "walk in guest",
            "is_promoted": false,
            "docstatus": 0
        }),
    )
    .await
    .expect("insert duplicate-name casual party");

    let rows = db
        .run(
            "RETURN fn::validate::casual_party_not_duplicate(type::record('tabCasual_Party', $name), {});",
            vec![("name".into(), json!("CP-SECOND"))],
        )
        .await
        .expect("call casual_party_not_duplicate");
    let result = rows.into_iter().next().unwrap_or(Value::Null);
    let warning = result
        .get("warning")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_lowercase();
    assert!(
        warning.contains("already exists"),
        "duplicate-name validation should return a warning; got: {result:?}"
    );

    let second_save = run_pipeline(&db, "tabCasual_Party", "CP-SECOND", "Casual Party", "save").await;
    assert!(
        matches!(second_save, Ok(PipelineResult::Ok)),
        "duplicate-name casual party should warn but not fail; got: {second_save:?}"
    );

    let duplicate = get_doc(&db, "Casual Party", "CP-SECOND")
        .await
        .expect("fetch duplicate-name casual party");
    assert_eq!(
        duplicate.fields.get("party_name").and_then(|v| v.as_str()),
        Some("walk in guest")
    );

    delete_doc(&db, "Casual Party", "CP-FIRST").await.ok();
    delete_doc(&db, "Casual Party", "CP-SECOND").await.ok();
}