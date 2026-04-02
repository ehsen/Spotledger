//! Hook dispatcher — fires document lifecycle hooks.
//!
//! Phase 2 implements the in-process Rust hook table.  Python hooks (via PyO3) and
//! TypeScript hooks (via V8) are Phase 2/3 additions — for now we support Rust-native
//! hooks registered at startup and a no-op fallback for unregistered hooks.
//!
//! Hook sequence for `save`:
//!   validate → before_save → [db write] → after_save
//!
//! Hook sequence for `submit`:
//!   validate → before_submit → [docstatus=1 write] → on_submit
//!
//! Hook sequence for `cancel`:
//!   before_cancel → [docstatus=2 write] → on_cancel
//!
//! Hook sequence for `delete`:
//!   before_delete → [db delete] → after_delete

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use dashmap::DashMap;

use spotledger_types::document::Document;
use spotledger_types::error::SpotError;

// ── Hook signature ────────────────────────────────────────────────────────────

pub type HookFn = Arc<
    dyn Fn(Document) -> Pin<Box<dyn Future<Output = Result<Document, SpotError>> + Send>>
        + Send
        + Sync,
>;

// ── Hook names ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum HookEvent {
    Validate,
    BeforeSave,
    AfterSave,
    BeforeInsert,
    AfterInsert,
    BeforeSubmit,
    OnSubmit,
    BeforeCancel,
    OnCancel,
    BeforeDelete,
    AfterDelete,
}

impl HookEvent {
    pub fn as_str(&self) -> &'static str {
        match self {
            HookEvent::Validate => "validate",
            HookEvent::BeforeSave => "before_save",
            HookEvent::AfterSave => "after_save",
            HookEvent::BeforeInsert => "before_insert",
            HookEvent::AfterInsert => "after_insert",
            HookEvent::BeforeSubmit => "before_submit",
            HookEvent::OnSubmit => "on_submit",
            HookEvent::BeforeCancel => "before_cancel",
            HookEvent::OnCancel => "on_cancel",
            HookEvent::BeforeDelete => "before_delete",
            HookEvent::AfterDelete => "after_delete",
        }
    }
}

// ── Registry ──────────────────────────────────────────────────────────────────

/// Registry keyed by (doctype, HookEvent) → list of handlers.
/// `"*"` as doctype means "all doctypes".
pub struct HookRegistry {
    /// Key: (doctype_or_star, event_str) → Vec<HookFn>
    hooks: DashMap<(String, String), Vec<HookFn>>,
}

impl HookRegistry {
    pub fn new() -> Self {
        Self {
            hooks: DashMap::new(),
        }
    }

    /// Register a hook for a specific doctype (or `"*"` for all doctypes).
    pub fn register(&self, doctype: impl Into<String>, event: HookEvent, handler: HookFn) {
        let key = (doctype.into(), event.as_str().to_string());
        self.hooks.entry(key).or_default().push(handler);
    }

    /// Fire all hooks for (doctype, event) then (*, event), passing the document
    /// through each handler in registration order.  Returns the (potentially modified) doc.
    pub async fn fire(
        &self,
        doctype: &str,
        event: &HookEvent,
        mut doc: Document,
    ) -> Result<Document, SpotError> {
        let event_str = event.as_str().to_string();

        // Doctype-specific hooks first
        let specific_key = (doctype.to_string(), event_str.clone());
        if let Some(handlers) = self.hooks.get(&specific_key) {
            for handler in handlers.value() {
                doc = handler(doc).await?;
            }
        }

        // Global wildcard hooks
        let global_key = ("*".to_string(), event_str);
        if let Some(handlers) = self.hooks.get(&global_key) {
            for handler in handlers.value() {
                doc = handler(doc).await?;
            }
        }

        Ok(doc)
    }
}

impl Default for HookRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ── Lifecycle helpers ─────────────────────────────────────────────────────────

/// Run the full SAVE lifecycle (new or existing document).
/// `is_new` distinguishes insert vs update hooks.
pub async fn run_save_hooks(
    hooks: &HookRegistry,
    doc: Document,
    is_new: bool,
) -> Result<Document, SpotError> {
    let doc = hooks.fire(&doc.doctype.clone(), &HookEvent::Validate, doc).await?;
    let doc = hooks.fire(&doc.doctype.clone(), &HookEvent::BeforeSave, doc).await?;
    let doc = if is_new {
        hooks.fire(&doc.doctype.clone(), &HookEvent::BeforeInsert, doc).await?
    } else {
        doc
    };
    Ok(doc)
}

/// Run post-save hooks (after DB write).
pub async fn run_after_save_hooks(
    hooks: &HookRegistry,
    doc: Document,
    is_new: bool,
) -> Result<Document, SpotError> {
    let doc = hooks.fire(&doc.doctype.clone(), &HookEvent::AfterSave, doc).await?;
    let doc = if is_new {
        hooks.fire(&doc.doctype.clone(), &HookEvent::AfterInsert, doc).await?
    } else {
        doc
    };
    Ok(doc)
}

/// Run submit lifecycle hooks (before DB write).
pub async fn run_before_submit_hooks(
    hooks: &HookRegistry,
    doc: Document,
) -> Result<Document, SpotError> {
    let doc = hooks.fire(&doc.doctype.clone(), &HookEvent::Validate, doc).await?;
    hooks.fire(&doc.doctype.clone(), &HookEvent::BeforeSubmit, doc).await
}

/// Run post-submit hooks (after DB write).
pub async fn run_on_submit_hooks(
    hooks: &HookRegistry,
    doc: Document,
) -> Result<Document, SpotError> {
    hooks.fire(&doc.doctype.clone(), &HookEvent::OnSubmit, doc).await
}

/// Run before-cancel hooks.
pub async fn run_before_cancel_hooks(
    hooks: &HookRegistry,
    doc: Document,
) -> Result<Document, SpotError> {
    hooks.fire(&doc.doctype.clone(), &HookEvent::BeforeCancel, doc).await
}

/// Run post-cancel hooks.
pub async fn run_on_cancel_hooks(
    hooks: &HookRegistry,
    doc: Document,
) -> Result<Document, SpotError> {
    hooks.fire(&doc.doctype.clone(), &HookEvent::OnCancel, doc).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use spotledger_types::document::Document;

    fn make_doc(doctype: &str, name: &str) -> Document {
        Document {
            doctype: doctype.into(),
            name: name.into(),
            fields: Default::default(),
        }
    }

    #[tokio::test]
    async fn fires_doctype_specific_hook() {
        let registry = HookRegistry::new();
        registry.register(
            "Customer",
            HookEvent::BeforeSave,
            Arc::new(|mut doc: Document| {
                Box::pin(async move {
                    doc.set("hook_fired", serde_json::json!(true));
                    Ok(doc)
                })
            }),
        );

        let doc = make_doc("Customer", "ACME");
        let result = registry.fire("Customer", &HookEvent::BeforeSave, doc).await.unwrap();
        assert_eq!(result.get_value("hook_fired"), Some(serde_json::json!(true)));
    }

    #[tokio::test]
    async fn global_hook_fires_for_all_doctypes() {
        let registry = HookRegistry::new();
        registry.register(
            "*",
            HookEvent::Validate,
            Arc::new(|mut doc: Document| {
                Box::pin(async move {
                    doc.set("validated", serde_json::json!(true));
                    Ok(doc)
                })
            }),
        );

        let doc = make_doc("SalesOrder", "SO-0001");
        let result = registry.fire("SalesOrder", &HookEvent::Validate, doc).await.unwrap();
        assert_eq!(result.get_value("validated"), Some(serde_json::json!(true)));
    }

    #[tokio::test]
    async fn no_hooks_returns_doc_unchanged() {
        let registry = HookRegistry::new();
        let doc = make_doc("Item", "ITEM-001");
        let result = registry.fire("Item", &HookEvent::BeforeSave, doc.clone()).await.unwrap();
        assert_eq!(result.name, doc.name);
    }

    #[tokio::test]
    async fn hooks_run_in_order() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc as StdArc;

        let counter = StdArc::new(AtomicUsize::new(0));
        let registry = HookRegistry::new();

        let c1 = counter.clone();
        registry.register("Item", HookEvent::Validate, Arc::new(move |mut doc: Document| {
            let c = c1.clone();
            Box::pin(async move {
                let n = c.fetch_add(1, Ordering::SeqCst);
                doc.set("order_0", serde_json::json!(n));
                Ok(doc)
            })
        }));

        let c2 = counter.clone();
        registry.register("Item", HookEvent::Validate, Arc::new(move |mut doc: Document| {
            let c = c2.clone();
            Box::pin(async move {
                let n = c.fetch_add(1, Ordering::SeqCst);
                doc.set("order_1", serde_json::json!(n));
                Ok(doc)
            })
        }));

        let doc = make_doc("Item", "ITEM-001");
        let result = registry.fire("Item", &HookEvent::Validate, doc).await.unwrap();
        assert_eq!(result.get_value("order_0"), Some(serde_json::json!(0)));
        assert_eq!(result.get_value("order_1"), Some(serde_json::json!(1)));
    }
}
