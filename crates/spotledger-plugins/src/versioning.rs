//! Plugin ABI versioning and dependency resolution.
//!
//! Ensures plugins are compatible with the host. Each plugin declares:
//! - `abi_version`: the SpotledgerCore ABI version it targets
//! - `dependencies`: set of plugins (by PluginId) it requires
//!
//! On plugin load, validate ABI version and build dependency graph.
//! On init, topologically sort to initialize plugins in dependency order.

use crate::registry::PluginId;
use std::collections::{HashMap, HashSet};
use tracing::warn;

/// ABI version of the host (SpotledgerCore).
/// Plugins must declare an abi_version that matches this.
pub const HOST_ABI_VERSION: u32 = 1;

/// Represents a plugin dependency at load time.
#[derive(Debug, Clone)]
pub struct PluginDependency {
    pub id: PluginId,
    /// Minimum ABI version required by this plugin
    pub min_abi_version: u32,
}

/// Plugin manifest read from plugin metadata.
/// (In Phase 2.5, this may come from a .toml or comment block in the .wasm)
#[derive(Debug, Clone)]
pub struct PluginManifest {
    pub id: PluginId,
    pub abi_version: u32,
    pub name: String,
    pub version: String,
    pub dependencies: Vec<PluginDependency>,

    // ── Capability declarations ───────────────────────────────────────────
    /// All DocType names this plugin provides.
    ///
    /// Used by Link-field validation routing: if a `DocField` has
    /// `provided_by = Some("selling")` and `"selling"` is not in the loaded
    /// registry, that field's existence check is skipped (field is dormant).
    pub provides_doctypes: Vec<String>,

    /// Party types this plugin registers into the `PartyType` kernel table.
    ///
    /// The plugin host calls `PluginRegistry::register_capabilities()` after
    /// loading, which does `INSERT OR IGNORE` for each entry here.
    pub party_types: Vec<PartyTypeDecl>,
}

/// Declaration of a party type that a plugin contributes to the kernel registry.
///
/// Example — the `selling` plugin declares:
/// ```ignore
/// PartyTypeDecl {
///     name:         "Customer".into(),
///     doctype_name: "Customer".into(),
///     account_type: "Receivable".into(),
/// }
/// ```
#[derive(Debug, Clone)]
pub struct PartyTypeDecl {
    /// The party type name (PK in `PartyType` table, e.g. `"Customer"`).
    pub name: String,
    /// The DocType whose records are looked up for existence validation.
    /// Usually identical to `name` but may differ.
    pub doctype_name: String,
    /// `"Receivable"` or `"Payable"` — determines which side of AR/AP this sits on.
    pub account_type: String,
}

impl PluginManifest {
    /// Check if this manifest is compatible with the host.
    pub fn is_compatible(&self) -> bool {
        self.abi_version <= HOST_ABI_VERSION
    }
}

/// Manages ABI version compatibility and plugin load ordering.
pub struct VersioningResolver {
    /// Map from PluginId to its manifest
    manifests: HashMap<PluginId, PluginManifest>,
    /// Adjacency list for the dependency graph (plugin → dependencies)
    dependency_graph: HashMap<PluginId, Vec<PluginId>>,
}

impl VersioningResolver {
    pub fn new() -> Self {
        Self {
            manifests: HashMap::new(),
            dependency_graph: HashMap::new(),
        }
    }

    /// Register a plugin manifest.
    pub fn register(&mut self, manifest: PluginManifest) {
        let deps: Vec<PluginId> = manifest.dependencies.iter().map(|d| d.id.clone()).collect();
        self.dependency_graph.insert(manifest.id.clone(), deps);
        self.manifests.insert(manifest.id.clone(), manifest);
    }

    /// Validate all registered plugins for ABI compatibility.
    /// Returns a list of incompatible plugin IDs.
    pub fn validate(&self) -> Vec<PluginId> {
        self.manifests
            .iter()
            .filter(|(_, m)| !m.is_compatible())
            .map(|(id, _)| id.clone())
            .collect()
    }

    /// Topologically sort plugins by their dependencies.
    /// Returns a list of PluginIds in dependency order (dependencies first).
    /// Returns Err if a dependency is missing or cyclic.
    pub fn resolve_load_order(&self) -> Result<Vec<PluginId>, String> {
        let mut visited = HashSet::new();
        let mut temp_visited = HashSet::new();
        let mut order = Vec::new();

        for plugin_id in self.manifests.keys() {
            if !visited.contains(plugin_id) {
                self.dfs_visit(
                    plugin_id,
                    &mut visited,
                    &mut temp_visited,
                    &mut order,
                )?;
            }
        }

        Ok(order)
    }

    fn dfs_visit(
        &self,
        plugin_id: &PluginId,
        visited: &mut HashSet<PluginId>,
        temp_visited: &mut HashSet<PluginId>,
        order: &mut Vec<PluginId>,
    ) -> Result<(), String> {
        if visited.contains(plugin_id) {
            return Ok(());
        }

        if temp_visited.contains(plugin_id) {
            return Err(format!("Cyclic dependency detected: {}", plugin_id.0));
        }

        temp_visited.insert(plugin_id.clone());

        if let Some(deps) = self.dependency_graph.get(plugin_id) {
            for dep in deps {
                if !self.manifests.contains_key(dep) {
                    warn!("Plugin {} requires missing plugin: {}", plugin_id.0, dep.0);
                    // Don't fail, just warn — allow partial load
                } else {
                    self.dfs_visit(dep, visited, temp_visited, order)?;
                }
            }
        }

        temp_visited.remove(plugin_id);
        visited.insert(plugin_id.clone());
        order.push(plugin_id.clone());

        Ok(())
    }

    /// Get the manifest for a plugin.
    pub fn get_manifest(&self, plugin_id: &str) -> Option<&PluginManifest> {
        self.manifests.get(&PluginId(plugin_id.to_string()))
    }
}

impl Default for VersioningResolver {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_abi_compatibility() {
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

        let future_manifest = PluginManifest {
            id: PluginId("future_app".into()),
            abi_version: 999,
            name: "Future".into(),
            version: "1.0".into(),
            dependencies: vec![],
            provides_doctypes: vec![],
            party_types: vec![],
        };
        assert!(!future_manifest.is_compatible());
    }

    #[test]
    fn test_dependency_order() {
        let mut resolver = VersioningResolver::new();
        
        // selling depends on nothing
        resolver.register(PluginManifest {
            id: PluginId("selling".into()),
            abi_version: 1,
            name: "Selling".into(),
            version: "1.0".into(),
            dependencies: vec![],
            provides_doctypes: vec![],
            party_types: vec![],
        });

        // buying depends on selling
        resolver.register(PluginManifest {
            id: PluginId("buying".into()),
            abi_version: 1,
            name: "Buying".into(),
            version: "1.0".into(),
            dependencies: vec![PluginDependency {
                id: PluginId("selling".into()),
                min_abi_version: 1,
            }],
            provides_doctypes: vec![],
            party_types: vec![],
        });

        let order = resolver.resolve_load_order().unwrap();
        assert_eq!(order.len(), 2);
        assert_eq!(order[0].0, "selling");
        assert_eq!(order[1].0, "buying");
    }

    #[test]
    fn test_cycle_detection() {
        let mut resolver = VersioningResolver::new();
        
        resolver.register(PluginManifest {
            id: PluginId("a".into()),
            abi_version: 1,
            name: "A".into(),
            version: "1.0".into(),
            dependencies: vec![PluginDependency {
                id: PluginId("b".into()),
                min_abi_version: 1,
            }],
            provides_doctypes: vec![],
            party_types: vec![],
        });

        resolver.register(PluginManifest {
            id: PluginId("b".into()),
            abi_version: 1,
            name: "B".into(),
            version: "1.0".into(),
            dependencies: vec![PluginDependency {
                id: PluginId("a".into()),
                min_abi_version: 1,
            }],
            provides_doctypes: vec![],
            party_types: vec![],
        });

        let result = resolver.resolve_load_order();
        assert!(result.is_err());
    }
}
