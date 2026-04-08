//! PluginRegistry — loads and manages WASM plugins.
//!
//! Responsibility:
//! - Discover .wasm files in plugins/ directory
//! - Load via extism with host function exports
//! - Call exported plugin_init() to register DocTypes
//! - Dispatch hooks and methods to loaded plugins
//! - Manage plugin versioning and load order

use dashmap::DashMap;
use extism::Plugin;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::{debug, error, info, warn};

use crate::host_fns::build_host_functions;
use crate::abi_utils::build_utils_host_functions;
use crate::versioning::{VersioningResolver, PluginManifest, HOST_ABI_VERSION};

/// Identifies a loaded WASM plugin (app name: "selling", "buying", etc.)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PluginId(pub String);

/// Metadata about a loaded plugin.
#[derive(Debug, Clone)]
pub struct PluginInfo {
    pub id: PluginId,
    pub path: PathBuf,
    pub abi_version: u32,
    pub manifest_hash: String,
}

/// Holds loaded extism Plugin instances and metadata.
pub struct PluginRegistry {
    // Map: PluginId → RefCell<extism::Plugin> (allows mutation through Arc)
    plugins: DashMap<PluginId, Arc<std::sync::Mutex<Plugin>>>,
    // Map: PluginId → PluginInfo
    metadata: DashMap<PluginId, PluginInfo>,
    // Plugins directory (e.g., ./plugins)
    plugins_dir: PathBuf,
    // Versioning resolver for compatibility checks and load ordering
    versioning: Arc<std::sync::Mutex<VersioningResolver>>,
    /// Maps DocType name → plugin_id for all plugin-provided DocTypes.
    ///
    /// Populated by `register_capabilities()` when a plugin loads.
    /// Used at validation time to determine whether a Link field's target
    /// DocType is available (`provided_by` dormancy check).
    provided_doctypes: DashMap<String, String>,
}

impl PluginRegistry {
    pub fn new(plugins_dir: impl Into<PathBuf>) -> Self {
        Self {
            plugins: DashMap::new(),
            metadata: DashMap::new(),
            plugins_dir: plugins_dir.into(),
            versioning: Arc::new(std::sync::Mutex::new(VersioningResolver::new())),
            provided_doctypes: DashMap::new(),
        }
    }

    /// Load all .wasm files from plugins/ directory.
    pub async fn load_all(&self) -> anyhow::Result<()> {
        if !self.plugins_dir.exists() {
            debug!("Plugins directory does not exist: {:?}", self.plugins_dir);
            return Ok(());
        }

        let mut read_dir = tokio::fs::read_dir(&self.plugins_dir).await?;
        while let Some(entry) = read_dir.next_entry().await? {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }

            let filename = match path.file_name().and_then(|n| n.to_str()) {
                Some(f) => f.to_owned(),
                None => continue,
            };

            if !filename.ends_with(".wasm") {
                continue;
            }

            // Extract plugin ID from filename (e.g., "selling.wasm" → "selling")
            let plugin_id = filename.trim_end_matches(".wasm").to_owned();
            match self.load_plugin(&plugin_id, &path).await {
                Ok(_) => info!(plugin = %plugin_id, "Plugin loaded"),
                Err(e) => error!(plugin = %plugin_id, error = %e, "Failed to load plugin"),
            }
        }

        Ok(())
    }

    /// Load a single .wasm file as a plugin.
    pub async fn load_plugin(&self, plugin_id: &str, path: &Path) -> anyhow::Result<()> {
        let wasm_bytes = tokio::fs::read(path).await?;

        // For hash, use a simple approach (MD5 would need a crate; use file size for now)
        let manifest_hash = format!("{:x}", wasm_bytes.len());

        // Register all SpotledgerCore host functions with this plugin instance.
        // Plugins import these under the `extism:host/user` WASM namespace.
        let mut host_fns = build_host_functions();
        host_fns.extend(build_utils_host_functions());
        let plugin = Plugin::new(&wasm_bytes, host_fns, true)?;

        // Create minimal manifest for Phase 2
        // Phase 2.5: read manifest from plugin's custom section or metadata
        let manifest = PluginManifest {
            id: PluginId(plugin_id.to_owned()),
            abi_version: HOST_ABI_VERSION,
            name: plugin_id.to_owned(),
            version: "1.0".to_owned(),
            dependencies: vec![],
            provides_doctypes: vec![],
            party_types: vec![],
        };

        // Validate ABI version
        if !manifest.is_compatible() {
            warn!(
                plugin = plugin_id,
                plugin_abi = manifest.abi_version,
                host_abi = HOST_ABI_VERSION,
                "Plugin ABI version mismatch"
            );
            // For Phase 2, warn but allow load. Phase 2.5 will enforce stricter versioning.
        }

        // Register in versioning resolver
        {
            let mut resolver = self
                .versioning
                .lock()
                .map_err(|_| anyhow::anyhow!("Versioning mutex poisoned"))?;
            resolver.register(manifest);
        }

        // Store plugin wrapped in Mutex for interior mutability
        self.plugins.insert(PluginId(plugin_id.to_owned()), Arc::new(std::sync::Mutex::new(plugin)));
        self.metadata.insert(
            PluginId(plugin_id.to_owned()),
            PluginInfo {
                id: PluginId(plugin_id.to_owned()),
                path: path.to_owned(),
                abi_version: HOST_ABI_VERSION,
                manifest_hash,
            },
        );

        Ok(())
    }

    /// Call the `sl_plugin_init()` export in a loaded plugin.
    /// This is where the plugin registers its DocTypes.
    pub fn plugin_init(&self, plugin_id: &str) -> anyhow::Result<()> {
        let plugin = self
            .plugins
            .get(&PluginId(plugin_id.to_owned()))
            .ok_or_else(|| anyhow::anyhow!("Plugin not loaded: {}", plugin_id))?;

        // Lock the mutex to call the plugin method
        let mut p = plugin.lock().map_err(|_| anyhow::anyhow!("Plugin mutex poisoned"))?;
        p.call::<(), ()>("sl_plugin_init", ())?;

        Ok(())
    }

    /// Get a loaded plugin by ID.
    pub fn get(&self, plugin_id: &str) -> Option<Arc<std::sync::Mutex<Plugin>>> {
        self.plugins.get(&PluginId(plugin_id.to_owned())).map(|p| p.clone())
    }

    /// Check if a plugin is loaded.
    pub fn is_loaded(&self, plugin_id: &str) -> bool {
        self.plugins.contains_key(&PluginId(plugin_id.to_owned()))
    }

    /// List all loaded plugins.
    pub fn list(&self) -> Vec<PluginInfo> {
        self.metadata
            .iter()
            .map(|r| r.value().clone())
            .collect()
    }

    /// Get plugin metadata.
    pub fn get_metadata(&self, plugin_id: &str) -> Option<PluginInfo> {
        self.metadata
            .get(&PluginId(plugin_id.to_owned()))
            .map(|r| r.clone())
    }

    // ── Capability registration ───────────────────────────────────────────

    /// Register a plugin's capability declarations into the in-memory index.
    ///
    /// Called by the plugin host after `load_plugin()` succeeds (or after
    /// the plugin's exported manifest function is parsed in Phase 3).
    ///
    /// This method only updates the **in-memory** `provided_doctypes` map.
    /// The `party_types` entries must be persisted to the DB separately by the
    /// caller (via `INSERT OR IGNORE INTO party_type …`).
    pub fn register_capabilities(&self, manifest: &PluginManifest) {
        let plugin_id = manifest.id.0.clone();
        for doctype in &manifest.provides_doctypes {
            info!(
                plugin = %plugin_id,
                doctype = %doctype,
                "Registered plugin-provided DocType"
            );
            self.provided_doctypes.insert(doctype.clone(), plugin_id.clone());
        }
        // party_types declare doctypes too — register them as well so Link
        // field dormancy checks work for the party doctype itself.
        for pt in &manifest.party_types {
            self.provided_doctypes
                .entry(pt.doctype_name.clone())
                .or_insert_with(|| plugin_id.clone());
        }
    }

    /// Returns `true` if a plugin that provides `doctype` is currently loaded.
    ///
    /// Used by Link-field validation: if a `DocField` has
    /// `provided_by = Some("selling")` and `provides_doctype("Customer")` is
    /// false, existence validation is skipped (field is dormant — plugin not
    /// installed).
    pub fn provides_doctype(&self, doctype: &str) -> bool {
        self.provided_doctypes.contains_key(doctype)
    }

    /// Returns the plugin_id that provides `doctype`, if any.
    pub fn provider_of(&self, doctype: &str) -> Option<String> {
        self.provided_doctypes.get(doctype).map(|r| r.clone())
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new("./plugins")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plugin_registry_new() {
        let reg = PluginRegistry::new("./test_plugins");
        assert_eq!(reg.list().len(), 0);
    }

    #[test]
    fn plugin_id_eq() {
        let id1 = PluginId("selling".into());
        let id2 = PluginId("selling".into());
        assert_eq!(id1, id2);
    }
}
