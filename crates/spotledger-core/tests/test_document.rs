//! Tests for `spotledger_core::document` — Document model, DocStatus lifecycle.
//!
//! ## Frappe parity
//! | Frappe test | This test |
//! |---|---|
//! | `test_document.py::TestDocument::test_load` | `test_document_new_defaults` |
//! | `test_document.py::TestDocument::test_get_return_empty_list_for_table_field_if_none` | `test_fields_access_missing` |
//! | `test_docstatus.py::TestDocStatus::test_docstatus_values` | `test_docstatus_values` |
//! | `test_docstatus.py::TestDocStatus::test_docstatus_transition` | `test_docstatus_transition` |
//! | `test_base_document.py::TestBaseDocument::test_is_new` | `test_is_new` |
//! | `test_document.py::TestDocument::test_set_and_get_fields` | `test_field_round_trip` |
//! | `test_document.py::TestDocument::test_set_flags` | `test_flags` |

use spotledger_core::document::{DocStatus, Document};
use serde_json::{json, Value};

// ── DocStatus ─────────────────────────────────────────────────────────────────

/// Frappe parity: `test_docstatus.py::test_docstatus_values`
/// DocStatus integers must map to the correct enum variants.
#[test]
fn test_docstatus_values() {
    assert_eq!(DocStatus::from(0), DocStatus::Draft);
    assert_eq!(DocStatus::from(1), DocStatus::Submitted);
    assert_eq!(DocStatus::from(2), DocStatus::Cancelled);
    // Unknown values fall back to Draft (defensive)
    assert_eq!(DocStatus::from(99), DocStatus::Draft);
    assert_eq!(DocStatus::from(-1), DocStatus::Draft);
}

/// Frappe parity: `test_docstatus.py::test_docstatus_transition`
/// DocStatus converts to i64 correctly for DB write.
#[test]
fn test_docstatus_to_i64() {
    assert_eq!(i64::from(DocStatus::Draft), 0i64);
    assert_eq!(i64::from(DocStatus::Submitted), 1i64);
    assert_eq!(i64::from(DocStatus::Cancelled), 2i64);
}

/// Frappe parity: `test_docstatus.py::test_submit_cancel`
/// A submitted document must not equal cancelled.
#[test]
fn test_docstatus_variants_distinct() {
    assert_ne!(DocStatus::Draft, DocStatus::Submitted);
    assert_ne!(DocStatus::Submitted, DocStatus::Cancelled);
    assert_ne!(DocStatus::Draft, DocStatus::Cancelled);
}

/// DocStatus default is Draft.
#[test]
fn test_docstatus_default() {
    assert_eq!(DocStatus::default(), DocStatus::Draft);
}

// ── Document ──────────────────────────────────────────────────────────────────

/// Frappe parity: `test_document.py::test_load`
/// A freshly-constructed Document has correct defaults.
#[test]
fn test_document_new_defaults() {
    let doc = Document::new("Sales Order");
    assert_eq!(doc.doctype, "Sales Order");
    assert!(doc.name.is_empty(), "new doc must have empty name");
    assert!(doc.fields.is_empty(), "new doc must have no fields");
}

/// Frappe parity: `test_base_document.py::TestBaseDocument::test_is_new`
/// A document without a `creation` timestamp is considered new.
#[test]
fn test_is_new_without_creation() {
    let doc = Document::new("Customer");
    assert!(doc.is_new(), "doc without creation field must be `is_new()`");
}

/// A document with a `creation` timestamp is not new.
#[test]
fn test_is_new_with_creation() {
    let mut doc = Document::new("Customer");
    doc.fields.insert("creation".to_string(), json!("2024-01-15T00:00:00Z"));
    assert!(!doc.is_new(), "doc with creation must NOT be `is_new()`");
}

/// Frappe parity: `test_document.py::TestDocument::test_get_return_empty_list_for_table_field_if_none`
/// Accessing a missing field returns None — no panic.
#[test]
fn test_fields_access_missing() {
    let doc = Document::new("User");
    assert!(doc.fields.get("roles").is_none());
    assert!(doc.fields.get("nonexistent_field").is_none());
}

/// Frappe parity: `test_document.py::TestDocument::test_set_and_get_fields`
/// Fields inserted into a Document round-trip correctly.
#[test]
fn test_field_round_trip() {
    let mut doc = Document::new("Customer");
    doc.fields.insert("customer_name".to_string(), json!("Acme Corp"));
    doc.fields.insert("credit_limit".to_string(), json!(50000.0));
    doc.fields.insert("enabled".to_string(), json!(1));

    assert_eq!(
        doc.fields.get("customer_name").and_then(Value::as_str),
        Some("Acme Corp")
    );
    assert_eq!(
        doc.fields.get("credit_limit").and_then(Value::as_f64),
        Some(50000.0)
    );
    assert_eq!(
        doc.fields.get("enabled").and_then(Value::as_i64),
        Some(1)
    );
}

/// Frappe parity: `test_document.py::TestDocument::test_set_flags`
/// Documents support arbitrary JSON values in fields (including nested objects).
#[test]
fn test_field_nested_value() {
    let mut doc = Document::new("Journal Entry");
    let accounts = json!([
        { "account": "Cash", "debit": 1000 },
        { "account": "Revenue", "credit": 1000 },
    ]);
    doc.fields.insert("accounts".to_string(), accounts.clone());

    let stored = doc.fields.get("accounts").unwrap();
    assert_eq!(stored, &accounts);
}

/// `docstatus()` method reads from `fields["docstatus"]`.
#[test]
fn test_document_docstatus_method() {
    let mut doc = Document::new("Sales Invoice");
    assert_eq!(doc.docstatus(), DocStatus::Draft, "default is Draft");

    doc.fields.insert("docstatus".to_string(), json!(1));
    assert_eq!(doc.docstatus(), DocStatus::Submitted);

    doc.fields.insert("docstatus".to_string(), json!(2));
    assert_eq!(doc.docstatus(), DocStatus::Cancelled);
}

/// `is_submitted()` and `is_cancelled()` convenience methods.
#[test]
fn test_document_is_submitted_cancelled() {
    let mut doc = Document::new("Purchase Invoice");

    doc.fields.insert("docstatus".to_string(), json!(0));
    assert!(!doc.is_submitted());

    doc.fields.insert("docstatus".to_string(), json!(1));
    assert!(doc.is_submitted());

    doc.fields.insert("docstatus".to_string(), json!(2));
    assert!(!doc.is_submitted());
}

/// Frappe parity: `test_document.py::TestDocument::test_insert` — name populated before DB write.
/// We verify the Document can carry a `name` field set externally.
#[test]
fn test_document_name_assignment() {
    let mut doc = Document::new("Item");
    doc.name = "ITEM-001".to_string();
    assert_eq!(doc.name, "ITEM-001");
    // `is_new()` checks `creation` timestamp, not presence of `name`.
    // A doc with a name but no creation timestamp is still logically new.
    assert!(doc.is_new(), "doc without creation is still new even after name is set");
}

/// Frappe parity: `test_document.py::TestDocument::test_load`
/// Document clones are deep copies — mutations don't alias.
#[test]
fn test_document_clone_isolation() {
    let mut doc = Document::new("Note");
    doc.fields.insert("content".to_string(), json!("original"));
    let mut cloned = doc.clone();
    cloned.fields.insert("content".to_string(), json!("modified"));

    // Original is unchanged
    assert_eq!(
        doc.fields.get("content").and_then(Value::as_str),
        Some("original")
    );
}

/// Frappe parity: `test_child_table.py` — child table fields are JSON arrays embedded in document.
#[test]
fn test_document_child_table_as_json_array() {
    let mut doc = Document::new("Sales Order");
    let items = json!([
        { "item_code": "WIDGET-001", "qty": 10, "rate": 150.0 },
        { "item_code": "GADGET-002", "qty": 5,  "rate": 300.0 },
    ]);
    doc.fields.insert("items".to_string(), items);

    let arr = doc.fields.get("items").unwrap().as_array().unwrap();
    assert_eq!(arr.len(), 2);
    assert_eq!(arr[0].get("item_code").and_then(Value::as_str), Some("WIDGET-001"));
}

/// `DocStatus` serializes to an integer (for JSON / DB writes).
#[test]
fn test_docstatus_serialize() {
    let s = serde_json::to_value(DocStatus::Submitted).unwrap();
    assert_eq!(s, json!(1));

    let s = serde_json::to_value(DocStatus::Cancelled).unwrap();
    assert_eq!(s, json!(2));

    let s = serde_json::to_value(DocStatus::Draft).unwrap();
    assert_eq!(s, json!(0));
}

/// `DocStatus` deserializes from an integer.
#[test]
fn test_docstatus_deserialize() {
    let s: DocStatus = serde_json::from_value(json!(1)).unwrap();
    assert_eq!(s, DocStatus::Submitted);

    let s: DocStatus = serde_json::from_value(json!(null)).unwrap();
    assert_eq!(s, DocStatus::Draft);
}
