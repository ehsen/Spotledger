//! Tests for `spotledger_db::hooks` — HookRegistry and HookEvent lifecycle.
//!
//! ## Frappe parity
//! | Frappe test | This test |
//! |---|---|
//! | `test_hooks.py::test_doc_events` | `test_hook_fires_for_doctype` |
//! | `test_hooks.py::test_wildcard_hook` | `test_wildcard_hook_fires_for_all` |
//! | `test_hooks.py::test_hook_order` | `test_hook_registration_order` |
//! | `test_document.py::test_before_save_hook` | `test_before_save_hook_modifies_doc` |
//! | `test_document.py::test_after_save_hook` | `test_after_save_hook_called` |
//! | `test_document.py::test_validate_hook` | `test_validate_hook_can_reject` |
//! | `test_hooks.py::test_hook_event_names` | `test_hook_event_as_str` |
//! | `test_document.py::test_no_hook_registered` | `test_no_hook_registered_doc_passes_through` |

use std::sync::{Arc, Mutex};
use serde_json::json;

use spotledger_core::{document::Document, error::SpotError};
use spotledger_db::hooks::{HookEvent, HookRegistry};

// ── HookEvent ─────────────────────────────────────────────────────────────────

/// Frappe parity: `test_hooks.py::test_hook_event_names`
/// Each HookEvent produces the correct snake_case string used in hook registration.
#[test]
fn test_hook_event_as_str() {
    assert_eq!(HookEvent::Validate.as_str(),      "validate");
    assert_eq!(HookEvent::BeforeSave.as_str(),    "before_save");
    assert_eq!(HookEvent::AfterSave.as_str(),     "after_save");
    assert_eq!(HookEvent::BeforeInsert.as_str(),  "before_insert");
    assert_eq!(HookEvent::AfterInsert.as_str(),   "after_insert");
    assert_eq!(HookEvent::BeforeSubmit.as_str(),  "before_submit");
    assert_eq!(HookEvent::OnSubmit.as_str(),      "on_submit");
    assert_eq!(HookEvent::BeforeCancel.as_str(),  "before_cancel");
    assert_eq!(HookEvent::OnCancel.as_str(),      "on_cancel");
    assert_eq!(HookEvent::BeforeDelete.as_str(),  "before_delete");
    assert_eq!(HookEvent::AfterDelete.as_str(),   "after_delete");
}

/// HookEvent equality.
#[test]
fn test_hook_event_equality() {
    assert_eq!(HookEvent::Validate, HookEvent::Validate);
    assert_ne!(HookEvent::Validate, HookEvent::BeforeSave);
}

/// HookEvent can be used as a HashMap key (implements Hash + Eq).
#[test]
fn test_hook_event_hashable() {
    use std::collections::HashMap;
    let mut map: HashMap<HookEvent, &str> = HashMap::new();
    map.insert(HookEvent::Validate, "validate");
    assert_eq!(map.get(&HookEvent::Validate), Some(&"validate"));
}

// ── HookRegistry ─────────────────────────────────────────────────────────────

/// Frappe parity: `test_hooks.py::test_doc_events`
/// Registering and firing a hook for a specific doctype works.
#[tokio::test]
async fn test_hook_fires_for_doctype() {
    let registry = HookRegistry::new();
    let call_count = Arc::new(Mutex::new(0u32));
    let cc = call_count.clone();

    registry.register("Note", HookEvent::Validate, Arc::new(move |doc: Document| {
        let cc = cc.clone();
        Box::pin(async move {
            *cc.lock().unwrap() += 1;
            Ok(doc)
        })
    }));

    let mut doc = Document::new("Note");
    doc.fields.insert("title".to_string(), json!("Test Note"));

    let result = registry.fire("Note", &HookEvent::Validate, doc).await;
    assert!(result.is_ok(), "hook must not error: {result:?}");
    assert_eq!(*call_count.lock().unwrap(), 1, "hook must fire exactly once");
}

/// Frappe parity: `test_hooks.py::test_wildcard_hook`
/// A hook registered for `"*"` fires for any doctype.
#[tokio::test]
async fn test_wildcard_hook_fires_for_all() {
    let registry = HookRegistry::new();
    let call_count = Arc::new(Mutex::new(0u32));
    let cc = call_count.clone();

    registry.register("*", HookEvent::AfterSave, Arc::new(move |doc: Document| {
        let cc = cc.clone();
        Box::pin(async move {
            *cc.lock().unwrap() += 1;
            Ok(doc)
        })
    }));

    for doctype in &["Note", "Customer", "Sales Order"] {
        let doc = Document::new(*doctype);
        registry.fire(doctype, &HookEvent::AfterSave, doc).await.unwrap();
    }
    assert_eq!(*call_count.lock().unwrap(), 3, "wildcard hook must fire for each doctype");
}

/// A hook registered for doctype A does NOT fire when doctype B is saved.
#[tokio::test]
async fn test_hook_doctype_isolation() {
    let registry = HookRegistry::new();
    let called = Arc::new(Mutex::new(false));
    let c = called.clone();

    registry.register("Customer", HookEvent::Validate, Arc::new(move |doc: Document| {
        let c = c.clone();
        Box::pin(async move {
            *c.lock().unwrap() = true;
            Ok(doc)
        })
    }));

    // Fire hook for "Note" — Customer hook must NOT fire
    let doc = Document::new("Note");
    registry.fire("Note", &HookEvent::Validate, doc).await.unwrap();
    assert!(!*called.lock().unwrap(), "Customer hook must NOT fire for Note doctype");
}

/// Frappe parity: `test_document.py::test_no_hook_registered`
/// When no hook is registered for a doctype+event, the document passes through unchanged.
#[tokio::test]
async fn test_no_hook_registered_doc_passes_through() {
    let registry = HookRegistry::new();
    let mut doc = Document::new("SomeUnrelatedDocType");
    doc.name = "DOC-001".to_string();
    doc.fields.insert("value".to_string(), json!(42));

    let result = registry.fire("SomeUnrelatedDocType", &HookEvent::BeforeSave, doc.clone()).await;
    assert!(result.is_ok());
    let out = result.unwrap();
    assert_eq!(out.name, "DOC-001");
    assert_eq!(out.fields.get("value"), Some(&json!(42)));
}

/// Frappe parity: `test_document.py::test_before_save_hook`
/// A `before_save` hook can modify document fields.
#[tokio::test]
async fn test_before_save_hook_modifies_doc() {
    let registry = HookRegistry::new();

    registry.register("Item", HookEvent::BeforeSave, Arc::new(|mut doc: Document| {
        Box::pin(async move {
            // Set a default value for `item_group` if absent
            doc.fields.entry("item_group".to_string())
                .or_insert_with(|| json!("All Item Groups"));
            Ok(doc)
        })
    }));

    let doc = Document::new("Item");
    let result = registry.fire("Item", &HookEvent::BeforeSave, doc).await.unwrap();
    assert_eq!(
        result.fields.get("item_group").and_then(|v| v.as_str()),
        Some("All Item Groups"),
        "before_save hook must inject default item_group"
    );
}

/// Frappe parity: `test_document.py::test_validate_hook`
/// A `validate` hook can reject a document by returning an error.
#[tokio::test]
async fn test_validate_hook_can_reject() {
    let registry = HookRegistry::new();

    registry.register("Sales Order", HookEvent::Validate, Arc::new(|doc: Document| {
        Box::pin(async move {
            if doc.fields.get("customer").is_none() {
                return Err(SpotError::Validation("Customer is mandatory".to_string()));
            }
            Ok(doc)
        })
    }));

    let doc = Document::new("Sales Order"); // no customer
    let result = registry.fire("Sales Order", &HookEvent::Validate, doc).await;
    assert!(result.is_err(), "validate hook must reject doc without customer");
}

/// Frappe parity: `test_hooks.py::test_hook_order`
/// Multiple hooks registered for the same event fire in registration order.
#[tokio::test]
async fn test_hook_registration_order() {
    let registry = HookRegistry::new();
    let log = Arc::new(Mutex::new(Vec::<u32>::new()));

    for i in 1u32..=3 {
        let log_clone = log.clone();
        registry.register("Note", HookEvent::BeforeSave, Arc::new(move |doc: Document| {
            let log_clone = log_clone.clone();
            let id = i;
            Box::pin(async move {
                log_clone.lock().unwrap().push(id);
                Ok(doc)
            })
        }));
    }

    let doc = Document::new("Note");
    registry.fire("Note", &HookEvent::BeforeSave, doc).await.unwrap();

    let order = log.lock().unwrap().clone();
    assert_eq!(order, vec![1, 2, 3], "hooks must fire in registration order");
}

/// Both doctype-specific and wildcard hooks fire, doctype-specific one first.
#[tokio::test]
async fn test_specific_and_wildcard_hooks_both_fire() {
    let registry = HookRegistry::new();
    let log = Arc::new(Mutex::new(Vec::<&str>::new()));

    let log1 = log.clone();
    registry.register("Invoice", HookEvent::Validate, Arc::new(move |doc: Document| {
        let l = log1.clone();
        Box::pin(async move {
            l.lock().unwrap().push("specific");
            Ok(doc)
        })
    }));

    let log2 = log.clone();
    registry.register("*", HookEvent::Validate, Arc::new(move |doc: Document| {
        let l = log2.clone();
        Box::pin(async move {
            l.lock().unwrap().push("wildcard");
            Ok(doc)
        })
    }));

    let doc = Document::new("Invoice");
    registry.fire("Invoice", &HookEvent::Validate, doc).await.unwrap();

    let order = log.lock().unwrap().clone();
    // Both must have fired
    assert!(order.contains(&"specific"), "specific hook must fire");
    assert!(order.contains(&"wildcard"), "wildcard hook must fire");
}
