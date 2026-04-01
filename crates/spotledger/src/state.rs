//! Global application state shared across all Axum handlers.

use dashmap::DashMap;
use moka::future::Cache;
use serde_json::Value;
use spotledger_db::connection::Db;
use spotledger_types::config::SiteConfig;
use std::sync::Arc;
use std::time::Duration;

use crate::methods::{build_registry, MethodRegistry};

/// Site-level state: one `SiteState` per live site.
#[derive(Clone)]
pub struct SiteState {
    pub config: SiteConfig,
    pub db: Db,
    /// Document cache keyed by `(doctype, name)` → Document as Value.
    pub doc_cache: Cache<(String, String), Value>,
    /// Registered `/api/method/` handlers (built-in Tier 1 + app-installed handlers).
    pub method_registry: Arc<MethodRegistry>,
}

impl SiteState {
    pub fn new(config: SiteConfig, db: Db) -> Self {
        let doc_cache = Cache::builder()
            .max_capacity(config.cache.max_documents)
            .time_to_live(Duration::from_secs(config.cache.ttl_seconds))
            .build();
        Self {
            config,
            db,
            doc_cache,
            method_registry: build_registry(),
        }
    }
}

/// Global app state: a registry of all loaded sites.
#[derive(Clone)]
pub struct AppState {
    /// hostname → SiteState
    pub sites: Arc<DashMap<String, Arc<SiteState>>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            sites: Arc::new(DashMap::new()),
        }
    }

    pub fn register(&self, hostname: impl Into<String>, site: SiteState) {
        self.sites.insert(hostname.into(), Arc::new(site));
    }

    pub fn get_site(&self, hostname: &str) -> Option<Arc<SiteState>> {
        self.sites.get(hostname).map(|r| r.value().clone())
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
