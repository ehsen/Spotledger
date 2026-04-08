//! Integration tests for the SpotledgerCore WASM plugin system.
//!
//! ## Test categories
//!
//! 1. **Metadata tests** — PluginManifest, VersioningResolver, dependency ordering
//! 2. **Adapter stubs**  — DbAdapter, GlAdapter operation stubs
//! 3. **Live WASM e2e** — load a real .wasm compiled from WAT at test time,
//!    call `sl_plugin_init()`, verify host functions are registered

use spotledger_plugins::{
    DbAdapter, GlAdapter,
    PluginDependency, PluginId, PluginManifest,
    VersioningResolver,
};

// ── 1. Metadata tests ─────────────────────────────────────────────────────────

#[test]
fn test_plugin_manifest_compatibility() {
    let manifest = PluginManifest {
        id: PluginId("selling".into()),
        abi_version: 1,
        name: "Selling".into(),
        version: "1.0".into(),
        dependencies: vec![],
        provides_doctypes: vec![],
        party_types: vec![],
    };
    assert!(manifest.is_compatible());
}

#[test]
fn test_future_manifest_incompatible() {
    let manifest = PluginManifest {
        id: PluginId("future_app".into()),
        abi_version: 999,
        name: "Future".into(),
        version: "99.0".into(),
        dependencies: vec![],
        provides_doctypes: vec![],
        party_types: vec![],
    };
    assert!(!manifest.is_compatible());
}

#[test]
fn test_versioning_resolver_register_and_query() {
    let mut resolver = VersioningResolver::new();
    assert!(resolver.validate().is_empty());

    resolver.register(PluginManifest {
        id: PluginId("selling".into()),
        abi_version: 1,
        name: "Selling".into(),
        version: "1.0".into(),
        dependencies: vec![],
        provides_doctypes: vec!["Customer".into(), "SalesOrder".into()],
        party_types: vec![],
    });

    assert!(resolver.get_manifest("selling").is_some());
    assert_eq!(
        resolver.get_manifest("selling").unwrap().provides_doctypes,
        vec!["Customer", "SalesOrder"]
    );
}

#[test]
fn test_load_order_no_cycle() {
    let mut resolver = VersioningResolver::new();

    resolver.register(PluginManifest {
        id: PluginId("selling".into()),
        abi_version: 1,
        name: "Selling".into(),
        version: "1.0".into(),
        dependencies: vec![],
        provides_doctypes: vec!["Customer".into()],
        party_types: vec![],
    });
    resolver.register(PluginManifest {
        id: PluginId("crm".into()),
        abi_version: 1,
        name: "CRM".into(),
        version: "1.0".into(),
        dependencies: vec![],
        provides_doctypes: vec!["Lead".into(), "Opportunity".into()],
        party_types: vec![],
    });

    let order = resolver.resolve_load_order();
    assert!(order.is_ok(), "No cycle expected: {:?}", order);
}

#[test]
fn test_load_order_cycle_detected() {
    let mut resolver = VersioningResolver::new();

    resolver.register(PluginManifest {
        id: PluginId("a".into()),
        abi_version: 1,
        name: "A".into(),
        version: "1.0".into(),
        dependencies: vec![PluginDependency { id: PluginId("b".into()), min_abi_version: 1 }],
        provides_doctypes: vec![],
        party_types: vec![],
    });
    resolver.register(PluginManifest {
        id: PluginId("b".into()),
        abi_version: 1,
        name: "B".into(),
        version: "1.0".into(),
        dependencies: vec![PluginDependency { id: PluginId("a".into()), min_abi_version: 1 }],
        provides_doctypes: vec![],
        party_types: vec![],
    });

    let order = resolver.resolve_load_order();
    assert!(order.is_err(), "Circular dependency must be detected");
    assert!(order.unwrap_err().contains("Cyclic"));
}

// ── 2. Adapter stub tests ─────────────────────────────────────────────────────

#[test]
fn test_db_adapter_get_doc() {
    let db = DbAdapter::new();
    let doc = db.get_doc("User", "Administrator");
    assert!(doc.is_ok());
    let v = doc.unwrap();
    assert_eq!(v["doctype"], "User");
    assert_eq!(v["name"], "Administrator");
}

#[test]
fn test_db_adapter_save_delete() {
    let db = DbAdapter::new();
    assert!(db.save_doc("User", "NewUser", &serde_json::json!({})).is_ok());
    assert!(db.delete_doc("User", "TempUser").is_ok());
}

#[test]
fn test_db_adapter_permissions() {
    let db = DbAdapter::new();
    let perm = db.has_permission("Administrator", "read", "User", "Administrator");
    assert!(perm.is_ok());
    assert!(perm.unwrap());
}

#[test]
fn test_gl_balanced_entries() {
    let gl = GlAdapter::new();
    let payload = serde_json::json!({
        "company": "Test Co",
        "posting_date": "2026-01-01",
        "entries": [
            { "account": "Debtors", "debit": 1000.0, "credit": 0.0 },
            { "account": "Sales",   "debit": 0.0,    "credit": 1000.0 },
        ]
    });
    assert!(gl.make_gl_entries(&payload).is_ok());
}

#[test]
fn test_gl_unbalanced_entries_rejected() {
    let gl = GlAdapter::new();
    let payload = serde_json::json!({
        "entries": [
            { "account": "Debtors", "debit": 1000.0, "credit": 0.0 },
            { "account": "Sales",   "debit": 0.0,    "credit": 500.0 },
        ]
    });
    assert!(gl.make_gl_entries(&payload).is_err());
}

#[test]
fn test_gl_adapter_extras() {
    let gl = GlAdapter::new();
    assert!(gl.reverse_gl_entries("SI", "SI-001").is_ok());
    assert!(gl.get_account_balance("Debtors", "Test Co", "2026-01-01").is_ok());
    assert!(gl.get_fiscal_year("Test Co", "2026-01-01").is_ok());
    let rate = gl.get_exchange_rate("USD", "INR", "2026-01-01");
    assert!(rate.is_ok());
    assert_eq!(rate.unwrap(), 1.0);
}

// ── 3. Live WASM end-to-end tests ─────────────────────────────────────────────
//
// The WAT modules below are compiled to WASM bytes at test time using the `wat`
// crate — no wasm32 toolchain is required.
//
// ▸ MINIMAL_PLUGIN_WAT: noop sl_plugin_init, no host function imports.
//   Proves the plugin lifecycle (load → init) works end-to-end.
//
// ▸ PLUGIN_WITH_HOST_FNS_WAT: imports sl_log from extism:host/user.
//   Proves that host functions are actually registered in Plugin::new().
//   If sl_log is missing from the host function list, Plugin::new() will fail
//   with an "unknown import" error.

/// Noop plugin — only exports sl_plugin_init, no host imports.
const MINIMAL_PLUGIN_WAT: &str = r#"
(module
  (func (export "sl_plugin_init"))
  (memory (export "memory") 1)
)
"#;

/// Plugin that imports sl_log — verifies the host function is registered.
const PLUGIN_WITH_HOST_FNS_WAT: &str = r#"
(module
  (import "extism:host/user" "sl_log" (func (param i64 i64)))
  (func (export "sl_plugin_init"))
  (memory (export "memory") 1)
)
"#;

#[tokio::test]
async fn test_live_wasm_minimal_plugin_loads_and_inits() {
    let wasm_bytes = wat::parse_str(MINIMAL_PLUGIN_WAT)
        .expect("WAT should compile");

    let tmp_dir  = std::env::temp_dir().join("sl_plugin_test_minimal");
    std::fs::create_dir_all(&tmp_dir).expect("create tmp dir");
    let wasm_path = tmp_dir.join("minimal.wasm");
    std::fs::write(&wasm_path, &wasm_bytes).expect("write wasm");

    let registry = spotledger_plugins::registry::PluginRegistry::new(&tmp_dir);

    registry
        .load_plugin("minimal", &wasm_path)
        .await
        .expect("Plugin should load");

    registry
        .plugin_init("minimal")
        .expect("sl_plugin_init() should run without error");

    let meta = registry.get_metadata("minimal");
    assert!(meta.is_some(), "Plugin metadata should exist after load");
    assert_eq!(meta.unwrap().id, PluginId("minimal".into()));

    std::fs::remove_dir_all(&tmp_dir).ok();
}

#[tokio::test]
async fn test_live_wasm_plugin_with_sl_log_import_loads() {
    // If sl_log is NOT registered, Plugin::new() will return an error like
    // "unknown import: `extism:host/user::sl_log`".
    // This test proves build_host_functions() correctly registers sl_log.
    let wasm_bytes = wat::parse_str(PLUGIN_WITH_HOST_FNS_WAT)
        .expect("WAT should compile");

    let tmp_dir  = std::env::temp_dir().join("sl_plugin_test_with_fns");
    std::fs::create_dir_all(&tmp_dir).expect("create tmp dir");
    let wasm_path = tmp_dir.join("with_fns.wasm");
    std::fs::write(&wasm_path, &wasm_bytes).expect("write wasm");

    let registry = spotledger_plugins::registry::PluginRegistry::new(&tmp_dir);

    registry
        .load_plugin("with_fns", &wasm_path)
        .await
        .expect("Plugin with sl_log import must load — host function is registered");

    assert!(registry.is_loaded("with_fns"));

    std::fs::remove_dir_all(&tmp_dir).ok();
}

#[tokio::test]
async fn test_live_wasm_is_loaded_flag() {
    let wasm_bytes = wat::parse_str(MINIMAL_PLUGIN_WAT).expect("WAT compiles");

    let tmp_dir   = std::env::temp_dir().join("sl_plugin_test_loaded_flag");
    std::fs::create_dir_all(&tmp_dir).expect("create tmp");
    let wasm_path = tmp_dir.join("flag_test.wasm");
    std::fs::write(&wasm_path, &wasm_bytes).expect("write wasm");

    let registry = spotledger_plugins::registry::PluginRegistry::new(&tmp_dir);

    assert!(!registry.is_loaded("flag_test"), "Before load: should not be loaded");
    registry.load_plugin("flag_test", &wasm_path).await.expect("load");
    assert!(registry.is_loaded("flag_test"), "After load: should be loaded");

    std::fs::remove_dir_all(&tmp_dir).ok();
}

