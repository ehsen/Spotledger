//! Global application state shared across all Axum handlers.

use dashmap::DashMap;
use moka::future::Cache;
use spotledger_db::adapter::DbAdapter;
use spotledger_db::hooks::HookRegistry;
use spotledger_db::meta_cache::MetaCache;
use spotledger_core::config::SiteConfig;
use std::sync::Arc;
use std::time::Duration;

use crate::methods::{build_registry, MethodRegistry};

/// How many distinct (doctype, txt) pairs to keep in the search cache.
const SEARCH_CACHE_CAPACITY: u64 = 2_000;
/// Search results expire after this many seconds.
const SEARCH_CACHE_TTL_SECS: u64 = 60;

/// Site-level state: one `SiteState` per live site.
#[derive(Clone)]
pub struct SiteState {
    pub config: SiteConfig,
    pub db: DbAdapter,
    /// Document cache keyed by `(doctype, name)` → Document as Value.
    pub doc_cache: Cache<(String, String), serde_json::Value>,
    /// Search-link cache keyed by `(doctype, txt)` → results JSON array.
    /// Short TTL (60 s) so stale data is never shown for long.
    pub search_cache: Cache<(String, String), serde_json::Value>,
    /// Registered `/api/method/` handlers (built-in Tier 1 + app-installed handlers).
    pub method_registry: Arc<MethodRegistry>,
    /// Document lifecycle hook registry.
    pub hook_registry: Arc<HookRegistry>,
    /// Runtime DocType meta cache — queries the graph for non-compiled DocTypes.
    pub meta_cache: MetaCache,
}

impl SiteState {
    pub fn new(config: SiteConfig, db: DbAdapter) -> Self {
        let doc_cache = Cache::builder()
            .max_capacity(config.cache.max_documents)
            .time_to_live(Duration::from_secs(config.cache.ttl_seconds))
            .build();
        let search_cache = Cache::builder()
            .max_capacity(SEARCH_CACHE_CAPACITY)
            .time_to_live(Duration::from_secs(SEARCH_CACHE_TTL_SECS))
            .build();
        Self {
            config,
            db,
            doc_cache,
            search_cache,
            method_registry: build_registry(),
            hook_registry: Arc::new(HookRegistry::new()),
            meta_cache: MetaCache::new(),
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
