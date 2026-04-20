//! Integration tests for Currency reference data.
//!
//! Tests 1–3 are plain CRUD/schema tests that run without the finance pipeline.
//! Tests 4–5 are pipeline-integrated tests that load the full spotledger-finance
//! domain functions and exercise `fn::pipeline::run`.
//!
//! ## Running
//! ```sh
//! $env:SURREAL_TEST_URL="ws://127.0.0.1:8000"
//! cargo test -p spotledger-db --features integration --test test_currency -- --test-threads=1
//! ```
//!
//! ## Test plan
//! | # | Test | Verifies |
//! |---|------|---------|
//! | 1 | `test_currency_table_accepts_insert` | tabCurrency SCHEMAFULL fields match fixture schema |
//! | 2 | `test_currency_name_is_code`         | name = ISO 4217 code (e.g. "USD") |
//! | 3 | `test_currency_duplicate_rejected`   | UNIQUE index on name rejects duplicate codes |
//! | 4 | `test_currency_code_format_rejected` | Lowercase or wrong-length code → pipeline error |
//! | 5 | `test_currency_single_base_enforced` | Two currencies both marked is_base → pipeline error |

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

async fn test_adapter(suffix: &str) -> DbAdapter {
    let url = std::env::var("SURREAL_TEST_URL")
        .unwrap_or_else(|_| "ws://127.0.0.1:8000".to_string());
    let cfg = DatabaseConfig {
        url,
        user: "root".into(),
        pass: "root".into(),
        ns: "test_ns".into(),
        db: format!("test_currency_{suffix}"),
    };
    DbAdapter::connect(&cfg).await.expect("connect to test SurrealDB")
}

async fn define_currency_table(db: &DbAdapter) {
    db.execute(
        "DEFINE TABLE IF NOT EXISTS tabCurrency SCHEMAFULL; \
         DEFINE FIELD OVERWRITE name          ON tabCurrency TYPE string; \
         DEFINE FIELD OVERWRITE currency_name ON tabCurrency TYPE option<string>; \
         DEFINE FIELD OVERWRITE symbol        ON tabCurrency TYPE option<string>; \
         DEFINE FIELD OVERWRITE decimal_places ON tabCurrency TYPE option<int>; \
         DEFINE FIELD OVERWRITE is_base       ON tabCurrency TYPE bool DEFAULT false; \
         DEFINE FIELD OVERWRITE code          ON tabCurrency TYPE option<string>; \
         DEFINE FIELD OVERWRITE docstatus     ON tabCurrency TYPE int DEFAULT 0; \
         DEFINE FIELD OVERWRITE status        ON tabCurrency TYPE option<string>; \
         DEFINE FIELD OVERWRITE owner         ON tabCurrency TYPE option<string>; \
         DEFINE FIELD OVERWRITE creation      ON tabCurrency TYPE option<datetime>; \
         DEFINE FIELD OVERWRITE modified      ON tabCurrency TYPE option<datetime>; \
         DEFINE FIELD OVERWRITE modified_by   ON tabCurrency TYPE option<string>; \
         DEFINE FIELD OVERWRITE doctype       ON tabCurrency TYPE option<string>; \
         DEFINE INDEX OVERWRITE idx_currency_name ON tabCurrency FIELDS name UNIQUE;",
        vec![],
    )
    .await
    .expect("define tabCurrency");
}

// ── 1. Basic insert round-trip (no pipeline) ─────────────────────────────────

#[tokio::test]
#[cfg_attr(not(feature = "integration"), ignore = "requires live SurrealDB")]
async fn test_currency_table_accepts_insert() {
    let db = test_adapter("insert").await;
    define_currency_table(&db).await;

    let doc = json!({
        "name": "USD",
        "currency_name": "USD",
        "code": "USD",
        "symbol": "$",
        "decimal_places": 2
    });
    delete_doc(&db, "Currency", "USD").await.ok();
    insert_doc(&db, "Currency", &doc).await.expect("insert USD");

    let fetched = get_doc(&db, "Currency", "USD").await.expect("get USD");
    assert_eq!(
        fetched.fields.get("currency_name").and_then(|v| v.as_str()),
        Some("USD")
    );
    delete_doc(&db, "Currency", "USD").await.ok();
}

// ── 2. Naming by field ───────────────────────────────────────────────────────

/// `fn::naming::resolve` must derive the document name from `currency_name`
/// when `autoname = "field:currency_name"`.
#[tokio::test]
#[cfg_attr(not(feature = "integration"), ignore = "requires live SurrealDB")]
async fn test_currency_name_is_iso_code() {
    use spotledger_db::schema::apply_naming_functions;

    let url = std::env::var("SURREAL_TEST_URL")
        .unwrap_or_else(|_| "ws://127.0.0.1:8000".to_string());
    let cfg = spotledger_core::config::DatabaseConfig {
        url,
        user: "root".into(),
        pass: "root".into(),
        ns: "test_ns".into(),
        db: "test_currency_naming".into(),
    };
    let db = DbAdapter::connect(&cfg).await.expect("connect");

    db.execute(
        "DEFINE TABLE IF NOT EXISTS tabDocType SCHEMALESS; \
         DEFINE TABLE IF NOT EXISTS tabDocumentNamingRule SCHEMALESS; \
         DEFINE TABLE IF NOT EXISTS tabSeries SCHEMAFULL; \
         DEFINE FIELD IF NOT EXISTS name    ON tabSeries TYPE string; \
         DEFINE FIELD IF NOT EXISTS current ON tabSeries TYPE int; \
         DEFINE INDEX IF NOT EXISTS idx_series_name ON tabSeries FIELDS name UNIQUE;",
        vec![],
    )
    .await
    .expect("setup naming tables");
    apply_naming_functions(&db).await.expect("apply naming fns");

    db.execute(
        "UPSERT type::record('tabDocType', 'Currency') CONTENT \
         { name: 'Currency', autoname: 'field:currency_name' }",
        vec![],
    )
    .await
    .expect("seed Currency doctype");

    let rows = db
        .run(
            "RETURN fn::naming::resolve($dt, $doc)",
            vec![
                ("dt".into(),  json!("Currency")),
                ("doc".into(), json!({ "currency_name": "EUR" })),
            ],
        )
        .await
        .expect("fn::naming::resolve");

    let resolved = rows
        .into_iter()
        .next()
        .and_then(|v| v.as_str().map(str::to_owned))
        .expect("non-null result");

    assert_eq!(resolved, "EUR", "naming by fieldname must return the ISO 4217 code");
}

// ── 3. Duplicate code rejected by UNIQUE index ───────────────────────────────

#[tokio::test]
#[cfg_attr(not(feature = "integration"), ignore = "requires live SurrealDB")]
async fn test_currency_duplicate_rejected() {
    let db = test_adapter("dup").await;
    define_currency_table(&db).await;

    let doc = json!({"name": "GBP", "currency_name": "British Pound Sterling", "code": "GBP", "symbol": "£"});
    delete_doc(&db, "Currency", "GBP").await.ok();
    insert_doc(&db, "Currency", &doc).await.expect("first insert OK");

    let result = insert_doc(&db, "Currency", &doc).await;
    assert!(result.is_err(), "duplicate currency code must be rejected by UNIQUE index");
}

// ── Pipeline helpers ──────────────────────────────────────────────────────────

async fn setup_currency_pipeline(suffix: &str) -> DbAdapter {
    let db = common::make_pipeline_db(&format!("currency_{suffix}")).await;
    common::seed_doctype(&db, "Currency", false).await;
    define_currency_table(&db).await;
    db
}

// ── 4. Code format validation: must be exactly 3 uppercase characters ─────────

/// ERPNext parity: ISO 4217 code must be exactly 3 uppercase letters.
/// `fn::validate::currency_code_format` enforces this rule.
#[tokio::test]
#[cfg_attr(not(feature = "integration"), ignore = "requires live SurrealDB")]
async fn test_currency_code_format_rejected() {
    let db = setup_currency_pipeline("code_format").await;

    // Test 1: lowercase code
    delete_doc(&db, "Currency", "eur").await.ok();
    insert_doc(
        &db,
        "Currency",
        &json!({
            "name": "eur",
            "currency_name": "Euro",
            "code": "eur",      // lowercase — invalid
            "is_base": false,
        }),
    )
    .await
    .expect("insert lowercase currency");

    let result = run_pipeline(&db, "tabCurrency", "eur", "Currency", "save").await;
    assert!(
        result.is_err(),
        "lowercase currency code must fail pipeline"
    );
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.to_lowercase().contains("uppercase") || msg.to_lowercase().contains("3"),
        "error must mention uppercase or length requirement; got: {msg}"
    );
    delete_doc(&db, "Currency", "eur").await.ok();

    // Test 2: too-long code
    delete_doc(&db, "Currency", "EURO").await.ok();
    insert_doc(
        &db,
        "Currency",
        &json!({
            "name": "EURO",
            "currency_name": "Euro",
            "code": "EURO",     // 4 chars — invalid
            "is_base": false,
        }),
    )
    .await
    .expect("insert 4-char currency code");

    let result2 = run_pipeline(&db, "tabCurrency", "EURO", "Currency", "save").await;
    assert!(
        result2.is_err(),
        "4-char currency code must fail pipeline"
    );
    let msg2 = result2.unwrap_err().to_string();
    assert!(
        msg2.to_lowercase().contains("3") || msg2.to_lowercase().contains("iso"),
        "error must mention length or ISO; got: {msg2}"
    );
    delete_doc(&db, "Currency", "EURO").await.ok();

    // Test 3: valid uppercase code must pass
    delete_doc(&db, "Currency", "EUR").await.ok();
    insert_doc(
        &db,
        "Currency",
        &json!({
            "name": "EUR",
            "currency_name": "Euro",
            "code": "EUR",      // valid ISO 4217
            "is_base": false,
        }),
    )
    .await
    .expect("insert valid EUR currency");

    let result3 = run_pipeline(&db, "tabCurrency", "EUR", "Currency", "save").await;
    assert!(
        matches!(result3, Ok(PipelineResult::Ok)),
        "valid 3-char uppercase code must pass pipeline; got: {result3:?}"
    );
    delete_doc(&db, "Currency", "EUR").await.ok();
}

// ── 5. Only one base currency allowed ────────────────────────────────────────

/// `fn::validate::currency_single_base` must reject a second currency marked
/// `is_base = true` when one base currency already exists in the DB.
#[tokio::test]
#[cfg_attr(not(feature = "integration"), ignore = "requires live SurrealDB")]
async fn test_currency_single_base_enforced() {
    let db = setup_currency_pipeline("single_base").await;

    // Insert the first base currency (USD) and validate it
    delete_doc(&db, "Currency", "USD").await.ok();
    insert_doc(
        &db,
        "Currency",
        &json!({
            "name": "USD",
            "currency_name": "US Dollar",
            "code": "USD",
            "is_base": true,    // first base currency
        }),
    )
    .await
    .expect("insert USD base currency");

    let r1 = run_pipeline(&db, "tabCurrency", "USD", "Currency", "save").await;
    assert!(
        matches!(r1, Ok(PipelineResult::Ok)),
        "first base currency must pass pipeline; got: {r1:?}"
    );

    // Try to insert a second base currency (EUR)
    delete_doc(&db, "Currency", "EUR").await.ok();
    insert_doc(
        &db,
        "Currency",
        &json!({
            "name": "EUR",
            "currency_name": "Euro",
            "code": "EUR",
            "is_base": true,    // second base currency — not allowed
        }),
    )
    .await
    .expect("insert EUR as second base currency");

    let r2 = run_pipeline(&db, "tabCurrency", "EUR", "Currency", "save").await;
    assert!(
        r2.is_err(),
        "second base currency must fail pipeline"
    );
    let msg = r2.unwrap_err().to_string();
    assert!(
        msg.to_lowercase().contains("base") || msg.to_lowercase().contains("one"),
        "error must mention base currency constraint; got: {msg}"
    );

    delete_doc(&db, "Currency", "USD").await.ok();
    delete_doc(&db, "Currency", "EUR").await.ok();
}

