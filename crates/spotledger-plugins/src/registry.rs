//! Plugin registry stub — will hold loaded extism Plugin instances.
//!
//! Phase 2 will implement:
//!   - `PluginRegistry::load(path)` — load a .wasm file via extism
//!   - `PluginRegistry::call(plugin, fn_name, input)` — call an exported fn
//!   - Host function bindings: sl_make_gl_entries, sl_get_doc, sl_save_doc, etc.

use dashmap::DashMap;

/// Identifies a loaded WASM plugin.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PluginId(pub String);

/// Stub registry — will hold `extism::Plugin` instances in Phase 2.
pub struct PluginRegistry {
    // plugins: DashMap<PluginId, extism::Plugin>,
    loaded: DashMap<PluginId, ()>,
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self { loaded: DashMap::new() }
    }

    pub fn is_loaded(&self, id: &str) -> bool {
        self.loaded.contains_key(&PluginId(id.into()))
    }
}

impl Default for PluginRegistry {
    fn default() -> Self { Self::new() }
}
