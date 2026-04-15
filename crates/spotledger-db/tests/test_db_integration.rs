//! Database-integration tests — require a live SurrealDB instance.
//!
//! ## Running these tests
//!
//! ```sh
//! # Start SurrealDB on port 8500 for tests (separate from dev on 8000)
//! surreal start --bind 127.0.0.1:8500 --username root --password root memory
//!
//! # Run with the integration feature
//! cargo test -p spotledger-db --features integration -- --test-threads=1
//! ```
//!
//! Without the `integration` feature all tests in this file are skipped
//! (marked `#[ignore]`).  CI can enable them by exporting `SURREAL_TEST_URL`.
//!
//! ## Frappe parity
//! | Frappe test | This test |
//! |---|---|
//! | `test_db.py::test_get_value` | `test_db_get_value` |
//! | `test_db.py::test_get_list` | `test_db_get_list` |
//! | `test_db.py::test_insert` | `test_db_insert_doc` |
//! | `test_db.py::test_upsert` | `test_db_upsert_doc` |
//! | `test_db.py::test_delete` | `test_db_delete_doc` |
//! | `test_auth.py::test_lookup_user` | `test_auth_lookup_user` |
//! | `test_auth.py::test_create_session` | `test_auth_create_and_get_session` |
//! | `test_naming.py::test_getseries` | `test_naming_increment` |
//! | `test_naming.py::test_naming_concurrent` | `test_naming_concurrent_increment` |
//! | `test_permissions.py::test_has_permission` | `test_permissions_has_permission` |
//! | `test_document.py::test_rename_doc` | `test_document_rename` |
//! | `test_graph_ops.py::test_upsert_app_node` | `test_graph_upsert_app_node` |
//! | `test_graph_ops.py::test_upsert_module_node` | `test_graph_upsert_module_node` |
//! | `test_migrations.py::test_run_pending` | `test_migrations_run_pending` |

#![allow(dead_code)]

/// Connect to the test SurrealDB instance.
async fn test_adapter() -> spotledger_db::DbAdapter {
    use spotledger_core::config::DatabaseConfig;
    use spotledger_db::DbAdapter;
    let url = std::env::var("SURREAL_TEST_URL")
        .unwrap_or_else(|_| "ws://127.0.0.1:8500".to_string());
    let cfg = DatabaseConfig {
        url,
        user: "root".into(),
        pass: "root".into(),
        ns: "test_ns".into(),
        db: "test_db".into(),
    };
    DbAdapter::connect(&cfg).await.expect("connect to test SurrealDB")
}

#[tokio::test]
#[cfg_attr(not(feature = "integration"), ignore = "requires live SurrealDB")]
async fn test_db_insert_doc() {
    use spotledger_db::document::{insert_doc, get_doc, delete_doc};
    use serde_json::json;
    let adapter = test_adapter().await;
    adapter.execute(
        "DEFINE TABLE IF NOT EXISTS tabTestInsert SCHEMAFULL; \
         DEFINE FIELD IF NOT EXISTS name  ON tabTestInsert TYPE string; \
         DEFINE FIELD IF NOT EXISTS title ON tabTestInsert TYPE option<string>; \
         DEFINE INDEX IF NOT EXISTS idx_testinsert_name ON tabTestInsert FIELDS name UNIQUE;",
        vec![],
    ).await.expect("define table");
    let name = format!("TI-{}", chrono::Utc::now().timestamp_millis());
    let fields = json!({"name": &name, "title": "Hello from integration test"});
    insert_doc(&adapter, "TestInsert", &fields).await.expect("insert_doc");
    let fetched = get_doc(&adapter, "TestInsert", &name).await.expect("get_doc");
    assert_eq!(fetched.fields.get("title").and_then(|v| v.as_str()), Some("Hello from integration test"));
    delete_doc(&adapter, "TestInsert", &name).await.ok();
}

#[tokio::test]
#[cfg_attr(not(feature = "integration"), ignore = "requires live SurrealDB")]
async fn test_db_upsert_doc() {
    use spotledger_db::document::{insert_doc, upsert_doc, get_doc, delete_doc};
    use serde_json::json;
    let adapter = test_adapter().await;
    adapter.execute(
        "DEFINE TABLE IF NOT EXISTS tabTestUpsert SCHEMAFULL; \
         DEFINE FIELD IF NOT EXISTS name  ON tabTestUpsert TYPE string; \
         DEFINE FIELD IF NOT EXISTS value ON tabTestUpsert TYPE option<string>; \
         DEFINE INDEX IF NOT EXISTS idx_testupsert_name ON tabTestUpsert FIELDS name UNIQUE;",
        vec![],
    ).await.expect("define table");
    let name = format!("TU-{}", chrono::Utc::now().timestamp_millis());
    insert_doc(&adapter, "TestUpsert", &json!({"name": &name, "value": "initial"})).await.expect("insert");
    upsert_doc(&adapter, "TestUpsert", &name, &json!({"value": "updated"})).await.expect("upsert");
    let fetched = get_doc(&adapter, "TestUpsert", &name).await.expect("get");
    assert_eq!(fetched.fields.get("value").and_then(|v| v.as_str()), Some("updated"));
    delete_doc(&adapter, "TestUpsert", &name).await.ok();
}

#[tokio::test]
#[cfg_attr(not(feature = "integration"), ignore = "requires live SurrealDB")]
async fn test_db_delete_doc() {
    use spotledger_db::{document::{insert_doc, delete_doc, get_doc}, error::DbError};
    use serde_json::json;
    let adapter = test_adapter().await;
    adapter.execute(
        "DEFINE TABLE IF NOT EXISTS tabTestDelete SCHEMAFULL; \
         DEFINE FIELD IF NOT EXISTS name ON tabTestDelete TYPE string; \
         DEFINE INDEX IF NOT EXISTS idx_testdelete_name ON tabTestDelete FIELDS name UNIQUE;",
        vec![],
    ).await.expect("define table");
    let name = format!("TD-{}", chrono::Utc::now().timestamp_millis());
    insert_doc(&adapter, "TestDelete", &json!({"name": &name})).await.expect("insert");
    delete_doc(&adapter, "TestDelete", &name).await.expect("delete");
    let result = get_doc(&adapter, "TestDelete", &name).await;
    assert!(matches!(result, Err(DbError::NotFound { .. })), "got: {result:?}");
}

#[tokio::test]
#[cfg_attr(not(feature = "integration"), ignore = "requires live SurrealDB")]
async fn test_db_get_value() {
    use spotledger_db::document::{insert_doc, get_value, delete_doc};
    use serde_json::json;
    let adapter = test_adapter().await;
    adapter.execute(
        "DEFINE TABLE IF NOT EXISTS tabTestGetValue SCHEMAFULL; \
         DEFINE FIELD IF NOT EXISTS name  ON tabTestGetValue TYPE string; \
         DEFINE FIELD IF NOT EXISTS score ON tabTestGetValue TYPE option<string>; \
         DEFINE INDEX IF NOT EXISTS idx_tgv_name ON tabTestGetValue FIELDS name UNIQUE;",
        vec![],
    ).await.expect("define table");
    let name = format!("TGV-{}", chrono::Utc::now().timestamp_millis());
    insert_doc(&adapter, "TestGetValue", &json!({"name": &name, "score": "A+"})).await.expect("insert");
    let val = get_value(&adapter, "TestGetValue", &name, "score").await.expect("get_value");
    assert_eq!(val.as_ref().and_then(|v| v.as_str()), Some("A+"));
    delete_doc(&adapter, "TestGetValue", &name).await.ok();
}

#[tokio::test]
#[cfg_attr(not(feature = "integration"), ignore = "requires live SurrealDB")]
async fn test_db_get_list() {
    use spotledger_db::document::{insert_doc, get_list, delete_doc};
    use serde_json::json;
    let adapter = test_adapter().await;
    adapter.execute(
        "DEFINE TABLE IF NOT EXISTS tabTestList SCHEMAFULL; \
         DEFINE FIELD IF NOT EXISTS name   ON tabTestList TYPE string; \
         DEFINE FIELD IF NOT EXISTS status ON tabTestList TYPE option<string>; \
         DEFINE INDEX IF NOT EXISTS idx_tl_name ON tabTestList FIELDS name UNIQUE;",
        vec![],
    ).await.expect("define table");
    let ts = chrono::Utc::now().timestamp_millis();
    for i in 0..3_u32 {
        let n = format!("TL-{ts}-{i}");
        insert_doc(&adapter, "TestList", &json!({"name": &n, "status": "Active"})).await.expect("insert");
    }
    let filter = json!({ "status": "Active" });
    let list = get_list(&adapter, "TestList", None, Some(&filter), 10, 0).await.expect("get_list");
    assert!(list.len() >= 3, "got {}", list.len());
    for i in 0..3_u32 {
        delete_doc(&adapter, "TestList", &format!("TL-{ts}-{i}")).await.ok();
    }
}

#[tokio::test]
#[cfg_attr(not(feature = "integration"), ignore = "requires live SurrealDB")]
async fn test_document_rename() {
    use spotledger_db::document::{insert_doc, rename_doc, get_doc, delete_doc};
    use serde_json::json;
    let adapter = test_adapter().await;
    adapter.execute(
        "DEFINE TABLE IF NOT EXISTS tabTestRename SCHEMAFULL; \
         DEFINE FIELD IF NOT EXISTS name ON tabTestRename TYPE string; \
         DEFINE INDEX IF NOT EXISTS idx_tr_name ON tabTestRename FIELDS name UNIQUE;",
        vec![],
    ).await.expect("define table");
    let ts = chrono::Utc::now().timestamp_millis();
    let old_name = format!("TR-OLD-{ts}");
    let new_name = format!("TR-NEW-{ts}");
    insert_doc(&adapter, "TestRename", &json!({"name": &old_name})).await.expect("insert");
    rename_doc(&adapter, "TestRename", &old_name, &new_name).await.expect("rename");
    assert!(get_doc(&adapter, "TestRename", &old_name).await.is_err(), "old name must be gone");
    let new_doc = get_doc(&adapter, "TestRename", &new_name).await.expect("new name must exist");
    assert_eq!(new_doc.name, new_name);
    delete_doc(&adapter, "TestRename", &new_name).await.ok();
}

#[tokio::test]
#[cfg_attr(not(feature = "integration"), ignore = "requires live SurrealDB")]
async fn test_naming_increment() {
    use spotledger_db::naming::next_name;
    let adapter = test_adapter().await;
    adapter.execute("DELETE tabSeries WHERE name CONTAINS 'TEST-INTG-'", vec![]).await.ok();
    let n1 = next_name(&adapter, "TEST-INTG-.####").await.expect("n1");
    let n2 = next_name(&adapter, "TEST-INTG-.####").await.expect("n2");
    let n3 = next_name(&adapter, "TEST-INTG-.####").await.expect("n3");
    assert_ne!(n1, n2);
    assert_ne!(n2, n3);
    let last_num = |s: &str| -> u64 {
        s.chars().rev().take_while(|c| c.is_ascii_digit())
            .collect::<String>().chars().rev().collect::<String>()
            .parse().unwrap_or(0)
    };
    assert!(last_num(&n2) > last_num(&n1));
    assert!(last_num(&n3) > last_num(&n2));
}

#[tokio::test]
#[cfg_attr(not(feature = "integration"), ignore = "requires live SurrealDB")]
async fn test_naming_concurrent_increment() {
    use spotledger_db::naming::next_name;
    use std::collections::HashSet;
    let adapter = test_adapter().await;
    adapter.execute("DELETE tabSeries WHERE name CONTAINS 'CONCURRENT-'", vec![]).await.ok();
    let tasks: Vec<_> = (0..20).map(|_| {
        let a = adapter.clone();
        tokio::spawn(async move { next_name(&a, "CONCURRENT-########").await })
    }).collect();
    let mut names = HashSet::new();
    for task in tasks {
        let n = task.await.expect("task panicked").expect("next_name failed");
        assert!(names.insert(n.clone()), "duplicate: {n}");
    }
    assert_eq!(names.len(), 20);
}

#[tokio::test]
#[cfg_attr(not(feature = "integration"), ignore = "requires live SurrealDB")]
async fn test_auth_lookup_user() {
    use spotledger_db::{auth::lookup_user, document::{insert_doc, delete_doc}};
    use serde_json::json;
    let adapter = test_adapter().await;
    adapter.execute(
        "DEFINE TABLE IF NOT EXISTS tabUser SCHEMAFULL; \
         DEFINE FIELD IF NOT EXISTS name       ON tabUser TYPE string; \
         DEFINE FIELD IF NOT EXISTS email      ON tabUser TYPE option<string>; \
         DEFINE FIELD IF NOT EXISTS enabled    ON tabUser TYPE bool DEFAULT true; \
         DEFINE FIELD IF NOT EXISTS user_type  ON tabUser TYPE option<string>; \
         DEFINE FIELD IF NOT EXISTS first_name ON tabUser TYPE option<string>; \
         DEFINE FIELD IF NOT EXISTS last_name  ON tabUser TYPE option<string>; \
         DEFINE FIELD IF NOT EXISTS full_name  ON tabUser TYPE option<string>; \
         DEFINE FIELD IF NOT EXISTS language   ON tabUser TYPE option<string>; \
         DEFINE INDEX IF NOT EXISTS idx_user_name ON tabUser FIELDS name UNIQUE;",
        vec![],
    ).await.ok();
    let email = format!("test_{}@integration.example.com", chrono::Utc::now().timestamp_millis());
    insert_doc(&adapter, "User", &json!({"name": &email, "email": &email, "enabled": true, "first_name": "Test"}))
        .await.expect("insert user");
    let user = lookup_user(&adapter, &email).await.expect("lookup_user");
    assert_eq!(user.name, email);
    assert!(user.enabled);
    delete_doc(&adapter, "User", &email).await.ok();
}

#[tokio::test]
#[cfg_attr(not(feature = "integration"), ignore = "requires live SurrealDB")]
async fn test_auth_create_and_get_session() {
    use spotledger_db::auth::{create_session, get_session};
    let adapter = test_adapter().await;
    let sid = create_session(&adapter, "Administrator", None).await.expect("create_session");
    let info = get_session(&adapter, &sid).await.expect("get_session").expect("session exists");
    assert_eq!(info.user, "Administrator");
    assert_eq!(info.status, "Active");
}

#[tokio::test]
#[cfg_attr(not(feature = "integration"), ignore = "requires live SurrealDB")]
async fn test_permissions_has_permission() {
    use spotledger_db::permissions::{has_permission, PermissionType};
    let adapter = test_adapter().await;
    let ok = has_permission(&adapter, "Administrator", "Account", PermissionType::Write)
        .await.expect("has_permission");
    assert!(ok, "Administrator must have Write permission");
}

#[tokio::test]
#[cfg_attr(not(feature = "integration"), ignore = "requires live SurrealDB")]
async fn test_permissions_user_roles() {
    use spotledger_db::permissions::get_user_roles;
    let adapter = test_adapter().await;
    let roles = get_user_roles(&adapter, "Administrator").await.expect("get_user_roles");
    assert!(!roles.is_empty(), "Administrator must have at least one role");
}

#[tokio::test]
#[cfg_attr(not(feature = "integration"), ignore = "requires live SurrealDB")]
async fn test_graph_upsert_app_node() {
    use spotledger_db::graph_ops::upsert_app_node;
    let adapter = test_adapter().await;
    adapter.execute(
        "DEFINE TABLE IF NOT EXISTS app SCHEMALESS; \
         DEFINE TABLE IF NOT EXISTS provides_module SCHEMALESS;",
        vec![],
    ).await.ok();
    upsert_app_node(&adapter, "test_app", Some("Test App"), Some("1.0.0"), None).await.expect("first upsert");
    upsert_app_node(&adapter, "test_app", Some("Test App"), Some("1.1.0"), None).await.expect("idempotent upsert");
}

#[tokio::test]
#[cfg_attr(not(feature = "integration"), ignore = "requires live SurrealDB")]
async fn test_graph_upsert_module_node() {
    use spotledger_db::graph_ops::{upsert_app_node, upsert_module_node};
    let adapter = test_adapter().await;
    adapter.execute(
        "DEFINE TABLE IF NOT EXISTS app SCHEMALESS; \
         DEFINE TABLE IF NOT EXISTS module SCHEMALESS; \
         DEFINE TABLE IF NOT EXISTS provides_module SCHEMALESS;",
        vec![],
    ).await.ok();
    upsert_app_node(&adapter, "test_graph_app", None, Some("1.0.0"), None).await.ok();
    upsert_module_node(&adapter, "Accounts", Some("Accounts"), "test_graph_app").await.expect("upsert_module_node");
    upsert_module_node(&adapter, "Accounts", Some("Accounts"), "test_graph_app").await.expect("idempotent");
}

#[tokio::test]
#[cfg_attr(not(feature = "integration"), ignore = "requires live SurrealDB")]
async fn test_migrations_run_pending() {
    use spotledger_db::migrations::run_pending_migrations;
    let adapter = test_adapter().await;
    adapter.execute(
        "DEFINE TABLE IF NOT EXISTS tabMigration SCHEMAFULL; \
         DEFINE FIELD IF NOT EXISTS name       ON tabMigration TYPE string; \
         DEFINE FIELD IF NOT EXISTS applied_at ON tabMigration TYPE option<datetime>; \
         DEFINE INDEX IF NOT EXISTS idx_migration_name ON tabMigration FIELDS name UNIQUE;",
        vec![],
    ).await.ok();
    let applied = run_pending_migrations(&adapter, 1_u64).await.expect("first run");
    let applied2 = run_pending_migrations(&adapter, 2_u64).await.expect("second run");
    assert!(applied2 <= applied, "second run should not apply more than first");
}

// ── Skipped / N/A tests ───────────────────────────────────────────────────────
//
// 1.  `test_nestedset.py` — lft/rgt removed; Spotledger uses graph edges.
// 2.  `test_redis.py` — No Redis; uses moka in-process cache.
// 3.  `test_scheduler.py` — No RQ; uses Tokio tasks + SurrealDB LIVE SELECT.
// 4.  `test_safe_exec.py` — No Python Server Scripts; uses WASM functions.
// 5.  `test_patches.py` — Python patches replaced by typed MigrationEntry.
// 6.  `test_translate.py` — i18n not yet implemented.
// 7.  `test_email.py` — Email module not yet implemented.
// 8.  `test_pdf.py` — PDF rendering not yet implemented.
// 9.  `test_website.py` — Web framework not yet implemented.
// 10. `test_recorder.py` — MariaDB query recorder not applicable to SurrealDB.
// 11. `test_query_builder.py` — Replaced by WhereClause/SetClause in test_query.rs.
// 12. `test_perf.py` — Use cargo bench (criterion) for perf tests.
// 13. `test_oauth20.py` — OAuth2 not yet implemented.
// 14. `test_twofactor.py` — 2FA not yet implemented.
// 15. `test_global_search.py` — SQLite FTS not applicable; SurrealDB has native FTS.
