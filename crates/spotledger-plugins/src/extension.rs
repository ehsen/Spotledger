//! Plugin extension infrastructure — DocType registration, hooks, methods.
//!
//! Plugins call host functions to register their DocTypes and export metadata.
//! After `sl_plugin_init()`, plugins are discoverable via the REST API and
//! can receive lifecycle hooks.

use spotledger_core::meta::DocTypeMeta;
use std::sync::Arc;
use dashmap::DashMap;

/// A plugin-contributed DocType and its schema.
#[derive(Debug, Clone)]
pub struct PluginDocType {
    pub name: String,
    pub module: String,
    pub plugin_id: String,
    pub meta: DocTypeMeta,
    /// The plugin's declared ABI version (read from manifest)
    pub plugin_abi_version: u32,
}

/// A plugin-exported method (different from in-crate methods).
///
/// e.g. `selling.sales_order.get_so_qty_and_amount`
#[derive(Debug, Clone)]
pub struct PluginMethod {
    pub path: String,      // e.g. "selling.sales_order.get_so_qty_and_amount"
    pub plugin_id: String,
    pub method_name: String, // exported function name in WASM
}

/// A scheduled job definition from a plugin.
#[derive(Debug, Clone)]
pub struct PluginScheduledJob {
    pub name: String,
    pub doctype: String,
    pub frequency: String, // "Hourly", "Daily", "Weekly", etc.
    pub method: String,    // plugin method to call
    pub plugin_id: String,
}

/// A webhook definition from a plugin.
#[derive(Debug, Clone)]
pub struct PluginWebhook {
    pub name: String,
    pub doctype: String,
    pub doctype_event: String, // "after_insert", "on_update", etc.
    pub method: String,        // plugin method to call
    pub plugin_id: String,
}

/// A document hook definition (before_insert, on_submit, etc.)
#[derive(Debug, Clone)]
pub struct PluginHook {
    pub doctype: String,
    pub hook_type: String, // "validate", "before_insert", "on_submit", "on_cancel", etc.
    pub plugin_id: String,
    pub method_name: String, // exported function name in WASM
}

/// Registry of all extensions contributed by all loaded plugins.
pub struct ExtensionRegistry {
    pub doctypes: DashMap<String, PluginDocType>,
    pub methods: DashMap<String, Arc<PluginMethod>>,
    pub scheduled_jobs: DashMap<String, PluginScheduledJob>,
    pub webhooks: DashMap<String, PluginWebhook>,
    pub hooks_by_doctype: DashMap<String, Vec<PluginHook>>,
}

impl ExtensionRegistry {
    pub fn new() -> Self {
        Self {
            doctypes: DashMap::new(),
            methods: DashMap::new(),
            scheduled_jobs: DashMap::new(),
            webhooks: DashMap::new(),
            hooks_by_doctype: DashMap::new(),
        }
    }

    /// Register a DocType from a plugin.
    pub fn register_doctype(&self, doctype: PluginDocType) {
        self.doctypes.insert(doctype.name.clone(), doctype);
    }

    /// Register a method from a plugin.
    pub fn register_method(&self, method: PluginMethod) {
        self.methods.insert(method.path.clone(), Arc::new(method));
    }

    /// Register a hook from a plugin.
    pub fn register_hook(&self, doctype: String, hook: PluginHook) {
        self.hooks_by_doctype
            .entry(doctype)
            .or_insert_with(Vec::new)
            .push(hook);
    }

    /// Get all hooks for a doctype.
    pub fn get_hooks(&self, doctype: &str) -> Vec<PluginHook> {
        self.hooks_by_doctype
            .get(doctype)
            .map(|r| r.clone())
            .unwrap_or_default()
    }

    /// Check if a plugin has registered a DocType.
    pub fn has_doctype(&self, name: &str) -> bool {
        self.doctypes.contains_key(name)
    }
}

impl Default for ExtensionRegistry {
    fn default() -> Self {
        Self::new()
    }
}
