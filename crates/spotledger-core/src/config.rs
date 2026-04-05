use serde::{Deserialize, Serialize};

/// Top-level global config loaded from `config/spotledger.toml`.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GlobalConfig {
    #[serde(default)]
    pub dev: ModeConfig,
    #[serde(default)]
    pub prod: ModeConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ModeConfig {
    #[serde(default = "default_true")]
    pub hot_reload: bool,
    #[serde(default = "default_log_level")]
    pub log_level: String,
    #[serde(default = "default_log_format")]
    pub log_format: String,
    #[serde(default = "default_workers")]
    pub workers: WorkerCount,
    #[serde(default)]
    pub show_queries: bool,
    #[serde(default = "default_bind")]
    pub bind: String,
}

impl Default for ModeConfig {
    fn default() -> Self {
        Self {
            hot_reload: true,
            log_level: default_log_level(),
            log_format: default_log_format(),
            workers: WorkerCount::Count(1),
            show_queries: false,
            bind: default_bind(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum WorkerCount {
    Auto(String),
    Count(usize),
}

impl WorkerCount {
    pub fn resolve(&self) -> usize {
        match self {
            WorkerCount::Auto(_) => num_cpus(),
            WorkerCount::Count(n) => *n,
        }
    }
}

fn num_cpus() -> usize {
    std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1)
}

fn default_true() -> bool { true }
fn default_log_level() -> String { "debug".into() }
fn default_log_format() -> String { "pretty".into() }
fn default_workers() -> WorkerCount { WorkerCount::Count(1) }
fn default_bind() -> String { "127.0.0.1:8000".into() }

/// Per-site config loaded from `sites/<hostname>/site_config.toml`.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SiteConfig {
    pub site: SiteInfo,
    pub database: DatabaseConfig,
    #[serde(default)]
    pub cache: CacheConfig,
    #[serde(default)]
    pub apps: AppsConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SiteInfo {
    pub name: String,
    pub namespace: String,
}

/// SurrealDB connection config — always WebSocket to an external instance.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DatabaseConfig {
    /// WebSocket URL e.g. `ws://localhost:8500`
    pub url: String,
    pub ns:   String,
    pub db:   String,
    pub user: String,
    pub pass: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CacheConfig {
    #[serde(default = "default_max_documents")]
    pub max_documents: u64,
    #[serde(default = "default_ttl_seconds")]
    pub ttl_seconds: u64,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            max_documents: default_max_documents(),
            ttl_seconds: default_ttl_seconds(),
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct AppsConfig {
    #[serde(default)]
    pub installed: Vec<String>,
}

fn default_max_documents() -> u64 { 10_000 }
fn default_ttl_seconds() -> u64 { 300 }

impl SiteConfig {
    pub fn from_file(path: &std::path::Path) -> Result<Self, crate::error::CoreError> {
        let raw = std::fs::read_to_string(path)
            .map_err(|e| crate::error::CoreError::Config(format!("read {path:?}: {e}")))?;
        toml::from_str(&raw)
            .map_err(|e| crate::error::CoreError::Config(format!("parse {path:?}: {e}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_site_config_toml() {
        let toml = r#"
[site]
name      = "fbr"
namespace = "fbr"

[database]
url  = "ws://localhost:8500"
ns   = "fbr"
db   = "fbr"
user = "root"
pass = "secret"

[cache]
max_documents = 5000
ttl_seconds   = 120

[apps]
installed = ["spotledger", "spotledger-accounting"]
"#;
        let cfg: SiteConfig = toml::from_str(toml).unwrap();
        assert_eq!(cfg.site.name, "fbr");
        assert_eq!(cfg.database.url, "ws://localhost:8500");
        assert_eq!(cfg.cache.max_documents, 5000);
    }

    #[test]
    fn site_config_defaults_apply() {
        let toml = r#"
[site]
name      = "test"
namespace = "test"

[database]
url  = "ws://127.0.0.1:8500"
ns   = "test"
db   = "test"
user = "root"
pass = "secret"
"#;
        let cfg: SiteConfig = toml::from_str(toml).unwrap();
        assert_eq!(cfg.cache.max_documents, 10_000);
        assert_eq!(cfg.cache.ttl_seconds, 300);
        assert!(cfg.apps.installed.is_empty());
    }
}
