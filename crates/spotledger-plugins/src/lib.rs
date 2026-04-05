//! SpotledgerCore — WASM plugin host.
//!
//! Responsible for loading, sandboxing, and calling Tier 4 domain plugins
//! compiled to WebAssembly (e.g. `selling.wasm`, `buying.wasm`).
//!
//! Each plugin receives a set of host functions (`sl_*`) exported from this
//! crate.  The GL engine host function is routed through `spotledger-accounting`.
//!
//! ## Plugin lifecycle
//! 1. Host calls `sl_plugin_init()` → plugin registers its DocTypes
//! 2. Plugin calls `sl_register_doctype(meta_ptr, meta_len)` for each DocType
//! 3. Plugin returns DocType metadata (DocTypeMeta serialised as MsgPack)
//! 4. Host stores meta and exposes DocTypes via REST API
//! 5. On document events, host calls plugin's exported handlers
//! 6. Plugin calls `sl_make_gl_entries()` for accounting entries

pub mod registry;
