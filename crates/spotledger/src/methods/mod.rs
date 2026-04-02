//! Method registry for POST /api/method/{path}
//! Routes dotted-path method calls to registered Rust handlers.

mod auth;
mod client;
mod desk;
pub use auth::{get_logged_user_handler, login_handler, logout_handler};
pub use client::register_client_methods;
pub use desk::{getdoc_handler, getdoctype_handler, register_desk_methods};

use crate::state::SiteState;
use dashmap::DashMap;
use serde_json::Value;
use spotledger_types::error::SpotError;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

pub type BoxFuture = Pin<Box<dyn Future<Output = Result<Value, SpotError>> + Send>>;

pub type MethodHandler = Arc<
    dyn Fn(Arc<SiteState>, HashMap<String, Value>) -> BoxFuture + Send + Sync,
>;

pub struct MethodRegistry {
    handlers: DashMap<String, MethodHandler>,
}

impl MethodRegistry {
    pub fn new() -> Self {
        Self {
            handlers: DashMap::new(),
        }
    }

    pub fn register(&self, path: &str, handler: MethodHandler) {
        self.handlers.insert(path.to_owned(), handler);
    }

    pub fn get(&self, path: &str) -> Option<MethodHandler> {
        self.handlers.get(path).map(|r| r.clone())
    }
}

/// Build the default method registry with all built-in handlers.
pub fn build_registry() -> Arc<MethodRegistry> {
    let registry = Arc::new(MethodRegistry::new());
    register_client_methods(&registry);
    register_desk_methods(&registry);
    registry
}

/// Parse a form/JSON parameter map into `HashMap<String, Value>`.
/// Form values that look like JSON are decoded; plain strings are kept as-is.
pub fn parse_form_params(raw: HashMap<String, String>) -> HashMap<String, Value> {
    raw.into_iter()
        .map(|(k, v)| {
            let val = serde_json::from_str::<Value>(&v).unwrap_or(Value::String(v));
            (k, val)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_form_params_decodes_json_strings() {
        let raw: HashMap<String, String> = [
            ("doctype".into(), "Sales Order".into()),
            ("limit".into(), "20".into()),
            ("fields".into(), r#"["name","customer"]"#.into()),
        ]
        .into();

        let parsed = parse_form_params(raw);
        assert_eq!(parsed["doctype"], Value::String("Sales Order".into()));
        assert_eq!(parsed["limit"], Value::Number(20.into()));
        assert!(parsed["fields"].is_array());
    }

    #[tokio::test]
    async fn registry_lookup_returns_none_for_unknown_path() {
        let registry = build_registry();
        assert!(registry.get("frappe.nonexistent.method").is_none());
    }

    #[tokio::test]
    async fn registry_has_tier1_handlers() {
        let registry = build_registry();
        for path in &[
            "frappe.client.get_list",
            "frappe.client.get",
            "frappe.client.get_value",
            "frappe.client.get_count",
            "frappe.client.save",
            "frappe.client.insert",
            "frappe.client.set_value",
            "frappe.client.delete",
            "frappe.client.submit",
            "frappe.client.cancel",
            "frappe.desk.form.load.getdoctype",
            "frappe.desk.form.load.getdoc",
            "frappe.desk.reportview.get",
            "frappe.desk.reportview.get_count",
            "frappe.desk.notifications.get_notifications",
            "frappe.utils.boot.get_boot_info",
        ] {
            assert!(registry.get(path).is_some(), "missing: {path}");
        }
    }

    #[test]
    fn auth_methods_are_not_in_registry() {
        let registry = build_registry();
        // Auth methods are dedicated Axum routes, NOT dispatched through the registry.
        assert!(registry.get("login").is_none());
        assert!(registry.get("logout").is_none());
        assert!(registry.get("frappe.auth.get_logged_user").is_none());
    }
}
