//! SpotledgerCore — WASM plugin host.
//!
//! Responsible for loading, sandboxing, and calling Tier 4 domain plugins
//! compiled to WebAssembly (e.g. `selling.wasm`, `buying.wasm`).
//!
//! Each plugin receives a set of host functions (`sl_*`) exported from this
//! crate.  The GL engine host function is routed through `spotledger-accounting`.
//!
//! ## Plugin lifecycle
//! 1. Host calls `PluginRegistry::load_all()` → discovers .wasm files
//! 2. Host calls `PluginRegistry::plugin_init(id)` → plugin exports sl_plugin_init()
//! 3. Plugin calls host functions: sl_register_doctype, sl_make_gl_entries, etc.
//! 4. On document events, host calls plugin's exported handlers
//! 5. Plugin can call accounting engine via sl_make_gl_entries()

pub mod abi;
pub mod abi_accounting;
pub mod abi_utils;
pub mod context;
pub mod db;
pub mod extension;
pub mod gl;
pub mod host_fns;
pub mod memory;
pub mod registry;
pub mod versioning;

pub use context::{PluginExecutionContext, set_execution_context, get_execution_context, clear_execution_context};
pub use db::{DbAdapter, DbError, DbResult};
pub use extension::ExtensionRegistry;
pub use gl::{GlAdapter, GlError, GlResult};
pub use host_fns::build_host_functions;
pub use memory::{MemoryError, MemoryResult};
pub use registry::{PluginId, PluginInfo, PluginRegistry};
pub use versioning::{PluginManifest, PluginDependency, PartyTypeDecl, VersioningResolver};
