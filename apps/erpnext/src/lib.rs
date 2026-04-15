//! ERPNext domain app — SpotledgerCore WASM plugin.
//!
//! This crate is a thin WASM carrier. All schema is seeded from the
//! erpnext/ subdirectory JSON files via `spotledger install-app erpnext`.
//! Domain logic runs as SurrealDB DEFINE EVENT / DEFINE FIELD VALUE expressions.

#[no_mangle]
pub extern "C" fn sl_plugin_init() {}
