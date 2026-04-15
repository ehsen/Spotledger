//! Tests for `spotledger_core::validation` — all pure validation functions.
//!
//! ## Frappe parity
//! | Frappe test | This test |
//! |---|---|
//! | `test_document.py::test_mandatory_fields` | `test_missing_mandatory_string_field` |
//! | `test_document.py::test_mandatory_check_field` | `test_missing_check_field_not_flagged` |
//! | `test_document.py::test_set_only_once` | `test_validate_constants_blocks_change` |
//! | `test_document.py::test_set_only_once_new_doc` | `test_validate_constants_skipped_for_new_doc` |
//! | `test_docstatus.py::test_update_after_submit` | `test_validate_update_after_submit_blocks` |
//! | `test_docstatus.py::test_allow_on_submit` | `test_validate_update_after_submit_allows_marked` |
//! | `test_document.py::test_select_validation` | `test_validate_selects_valid` / `_invalid` |
//! | `test_document.py::test_max_length_validation` | `test_validate_length_exceeds` / `_within` |
//! | `test_document.py::test_xss_script` | `test_sanitize_strips_script` |
//! | `test_document.py::test_xss_event_handler` | `test_sanitize_strips_event_handler` |
//! | `test_document.py::test_xss_plain_field` | `test_sanitize_escapes_plain_html` |
//! | `test_document.py::test_xss_ignore_filter` | `test_sanitize_respects_ignore_xss_filter` |

use serde_json::json;

use spotledger_core::{
    document::Document,
    meta::{DocField, DocTypeMeta, FieldType},
    validation::{
        get_missing_mandatory_fields,
        sanitize_content,
        validate_constants,
        validate_length,
        validate_selects,
        validate_update_after_submit,
    },
};

// ── helpers ───────────────────────────────────────────────────────────────────

fn simple_meta(name: &str) -> DocTypeMeta {
    DocTypeMeta::builder(name, "Core").build()
}

fn meta_with_fields(name: &str, fields: Vec<DocField>) -> DocTypeMeta {
    let mut b = DocTypeMeta::builder(name, "Core");
    for f in fields {
        b = b.field(f);
    }
    b.build()
}

fn doc_with(pairs: &[(&str, serde_json::Value)]) -> Document {
    let mut doc = Document::new("TestDoc");
    for (k, v) in pairs {
        doc.fields.insert(k.to_string(), v.clone());
    }
    doc
}

// ── validate_constants ────────────────────────────────────────────────────────

/// Frappe parity: `test_document.py::test_set_only_once`
/// Changing a `set_only_once` field that already has a value raises a validation error.
#[test]
fn test_validate_constants_blocks_change() {
    let meta = meta_with_fields("JE", vec![
        DocField::new("company", "Company", FieldType::Data).set_only_once(),
    ]);

    let mut saved = Document::new("JE");
    saved.fields.insert("company".to_string(), json!("Acme"));

    let mut doc = Document::new("JE");
    doc.fields.insert("company".to_string(), json!("BetaCorp"));

    let result = validate_constants(&doc, Some(&saved), &meta);
    assert!(result.is_err(), "changing set_only_once field must fail");
    let msg = result.unwrap_err().to_string();
    assert!(msg.contains("Company") || msg.contains("cannot be changed"), "{msg}");
}

/// Frappe parity: `test_document.py::test_set_only_once_new_doc`
/// For new documents, `set_only_once` is not checked (there is no saved copy).
#[test]
fn test_validate_constants_skipped_for_new_doc() {
    let meta = meta_with_fields("JE", vec![
        DocField::new("company", "Company", FieldType::Data).set_only_once(),
    ]);
    let doc = doc_with(&[("company", json!("Acme"))]);

    assert!(validate_constants(&doc, None, &meta).is_ok());
}

/// If the saved value is empty (never set), the field is allowed to change.
#[test]
fn test_validate_constants_allows_first_set() {
    let meta = meta_with_fields("JE", vec![
        DocField::new("company", "Company", FieldType::Data).set_only_once(),
    ]);

    let mut saved = Document::new("JE");
    saved.fields.insert("company".to_string(), json!(""));

    let doc = doc_with(&[("company", json!("Acme"))]);
    assert!(validate_constants(&doc, Some(&saved), &meta).is_ok());
}

/// `set_only_once` does not fire for layout fields.
#[test]
fn test_validate_constants_ignores_layout_fields() {
    let meta = meta_with_fields("JE", vec![
        DocField::new("section_break_1", "Details", FieldType::SectionBreak),
    ]);

    let mut saved = Document::new("JE");
    saved.fields.insert("section_break_1".to_string(), json!("old"));

    let doc = doc_with(&[("section_break_1", json!("new"))]);
    assert!(validate_constants(&doc, Some(&saved), &meta).is_ok());
}

// ── validate_update_after_submit ───────────────────────────────────────────────

/// Frappe parity: `test_docstatus.py::test_update_after_submit`
/// Editing a regular field on a submitted doc must fail.
#[test]
fn test_validate_update_after_submit_blocks() {
    let meta = meta_with_fields("SI", vec![
        DocField::new("remarks", "Remarks", FieldType::Text),
    ]);

    let mut saved = Document::new("SI");
    saved.fields.insert("docstatus".to_string(), json!(1)); // Submitted
    saved.fields.insert("remarks".to_string(), json!("original"));

    let doc = doc_with(&[
        ("docstatus", json!(1)),
        ("remarks", json!("changed")),
    ]);

    let result = validate_update_after_submit(&doc, Some(&saved), &meta);
    assert!(result.is_err(), "must block change on submitted doc");
}

/// Frappe parity: `test_docstatus.py::test_allow_on_submit`
/// An `allow_on_submit` field can be changed on a submitted doc.
#[test]
fn test_validate_update_after_submit_allows_marked() {
    let meta = meta_with_fields("SI", vec![
        DocField::new("remarks", "Remarks", FieldType::Text).allow_on_submit(),
    ]);

    let mut saved = Document::new("SI");
    saved.fields.insert("docstatus".to_string(), json!(1));
    saved.fields.insert("remarks".to_string(), json!("original"));

    let doc = doc_with(&[
        ("docstatus", json!(1)),
        ("remarks", json!("updated remarks")),
    ]);

    assert!(validate_update_after_submit(&doc, Some(&saved), &meta).is_ok());
}

/// Changes on a Draft document are always allowed.
#[test]
fn test_validate_update_after_submit_draft_ok() {
    let meta = meta_with_fields("SI", vec![
        DocField::new("remarks", "Remarks", FieldType::Text),
    ]);

    let mut saved = Document::new("SI");
    saved.fields.insert("docstatus".to_string(), json!(0)); // Draft
    saved.fields.insert("remarks".to_string(), json!("old"));

    let doc = doc_with(&[("remarks", json!("new"))]);
    assert!(validate_update_after_submit(&doc, Some(&saved), &meta).is_ok());
}

/// For new documents (no saved copy), update-after-submit is skipped.
#[test]
fn test_validate_update_after_submit_new_doc_ok() {
    let meta = meta_with_fields("SI", vec![
        DocField::new("customer", "Customer", FieldType::Data),
    ]);
    let doc = doc_with(&[("customer", json!("Acme"))]);
    assert!(validate_update_after_submit(&doc, None, &meta).is_ok());
}

// ── get_missing_mandatory_fields ─────────────────────────────────────────────

/// Frappe parity: `test_document.py::test_mandatory_fields`
/// Missing required string field is reported.
#[test]
fn test_missing_mandatory_string_field() {
    let meta = meta_with_fields("Task", vec![
        DocField::new("subject", "Subject", FieldType::Data).required(),
    ]);
    let doc = Document::new("Task"); // no subject
    let missing = get_missing_mandatory_fields(&doc, &meta);
    assert_eq!(missing.len(), 1);
    assert_eq!(missing[0].0, "subject");
}

/// Empty-string value for a mandatory field is also flagged as missing.
#[test]
fn test_missing_mandatory_empty_string() {
    let meta = meta_with_fields("Task", vec![
        DocField::new("subject", "Subject", FieldType::Data).required(),
    ]);
    let doc = doc_with(&[("subject", json!("   "))]);
    let missing = get_missing_mandatory_fields(&doc, &meta);
    assert_eq!(missing.len(), 1, "whitespace-only must count as missing");
}

/// Frappe parity: `test_document.py::test_mandatory_check_field`
/// A mandatory Check field with value 0 is NOT flagged as missing.
#[test]
fn test_missing_check_field_not_flagged() {
    let meta = meta_with_fields("Task", vec![
        DocField::new("is_active", "Is Active", FieldType::Check).required(),
    ]);
    let doc = doc_with(&[("is_active", json!(0))]);
    let missing = get_missing_mandatory_fields(&doc, &meta);
    assert!(missing.is_empty(), "Check=0 is a valid explicit value, not missing");
}

/// Non-required fields do not appear in missing even when absent.
#[test]
fn test_non_required_field_not_flagged() {
    let meta = meta_with_fields("Note", vec![
        DocField::new("title", "Title", FieldType::Data),          // not reqd
        DocField::new("content", "Content", FieldType::Text),     // not reqd
    ]);
    let doc = Document::new("Note"); // no fields
    let missing = get_missing_mandatory_fields(&doc, &meta);
    assert!(missing.is_empty());
}

/// Layout fields (SectionBreak etc.) are never reported as missing.
#[test]
fn test_layout_fields_never_missing() {
    let meta = meta_with_fields("Form", vec![
        DocField::new("sec1", "Details", FieldType::SectionBreak),
    ]);
    let doc = Document::new("Form");
    assert!(get_missing_mandatory_fields(&doc, &meta).is_empty());
}

/// Multiple missing fields are all reported.
#[test]
fn test_multiple_missing_mandatory_fields() {
    let meta = meta_with_fields("SO", vec![
        DocField::new("customer", "Customer", FieldType::Link).required(),
        DocField::new("company", "Company", FieldType::Link).required(),
        DocField::new("notes", "Notes", FieldType::Text),          // optional
    ]);
    let doc = Document::new("SO");
    let missing = get_missing_mandatory_fields(&doc, &meta);
    assert_eq!(missing.len(), 2);
    let names: Vec<&str> = missing.iter().map(|(f, _)| f.as_str()).collect();
    assert!(names.contains(&"customer"));
    assert!(names.contains(&"company"));
}

// ── validate_selects ──────────────────────────────────────────────────────────

/// Frappe parity: `test_document.py::test_select_validation`
/// A valid option passes.
#[test]
fn test_validate_selects_valid() {
    let meta = meta_with_fields("ToDo", vec![
        DocField::new("status", "Status", FieldType::Select)
            .select_options("Open\nClosed\nCancelled"),
    ]);
    let doc = doc_with(&[("status", json!("Open"))]);
    assert!(validate_selects(&doc, &meta).is_ok());
}

/// An invalid option fails.
#[test]
fn test_validate_selects_invalid() {
    let meta = meta_with_fields("ToDo", vec![
        DocField::new("status", "Status", FieldType::Select)
            .select_options("Open\nClosed"),
    ]);
    let doc = doc_with(&[("status", json!("Pending"))]);
    let result = validate_selects(&doc, &meta);
    assert!(result.is_err(), "invalid select value must fail");
    let msg = result.unwrap_err().to_string();
    assert!(msg.contains("Pending") || msg.contains("not valid"), "{msg}");
}

/// Empty value on a Select field passes (use `reqd` to enforce non-empty).
#[test]
fn test_validate_selects_empty_string_passes() {
    let meta = meta_with_fields("ToDo", vec![
        DocField::new("status", "Status", FieldType::Select)
            .select_options("Open\nClosed"),
    ]);
    let doc = doc_with(&[("status", json!(""))]);
    assert!(validate_selects(&doc, &meta).is_ok());
}

/// Null value on a Select field passes.
#[test]
fn test_validate_selects_null_passes() {
    let meta = meta_with_fields("ToDo", vec![
        DocField::new("priority", "Priority", FieldType::Select)
            .select_options("Low\nMedium\nHigh"),
    ]);
    let doc = doc_with(&[("priority", json!(null))]);
    assert!(validate_selects(&doc, &meta).is_ok());
}

/// Select field without `select_options` doesn't crash — any value passes.
#[test]
fn test_validate_selects_no_options_defined() {
    let meta = meta_with_fields("Doc", vec![
        DocField::new("type", "Type", FieldType::Select), // no select_options
    ]);
    let doc = doc_with(&[("type", json!("anything"))]);
    assert!(validate_selects(&doc, &meta).is_ok());
}

// ── validate_length ───────────────────────────────────────────────────────────

/// Frappe parity: `test_document.py::test_max_length_validation` — Data field default max 255.
#[test]
fn test_validate_length_data_field_default_255() {
    let meta = meta_with_fields("Item", vec![
        DocField::new("item_code", "Item Code", FieldType::Data),
    ]);
    let long_value = "A".repeat(256);
    let doc = doc_with(&[("item_code", json!(long_value))]);
    assert!(validate_length(&doc, &meta).is_err(), "256 chars must exceed Data default of 255");
}

/// Value within the default 255-char limit passes.
#[test]
fn test_validate_length_data_field_within_limit() {
    let meta = meta_with_fields("Item", vec![
        DocField::new("item_code", "Item Code", FieldType::Data),
    ]);
    let ok_value = "A".repeat(255);
    let doc = doc_with(&[("item_code", json!(ok_value))]);
    assert!(validate_length(&doc, &meta).is_ok());
}

/// SmallText default is 140 chars.
#[test]
fn test_validate_length_smalltext_default_140() {
    let meta = meta_with_fields("Doc", vec![
        DocField::new("note", "Note", FieldType::SmallText),
    ]);
    let too_long = "x".repeat(141);
    let doc = doc_with(&[("note", json!(too_long))]);
    assert!(validate_length(&doc, &meta).is_err(), "141 chars must exceed SmallText limit of 140");
}

/// Explicit `length(n)` overrides the default.
#[test]
fn test_validate_length_explicit_override() {
    let meta = meta_with_fields("Code", vec![
        DocField::new("pin", "PIN", FieldType::Data).length(4),
    ]);
    let doc = doc_with(&[("pin", json!("12345"))]);
    assert!(validate_length(&doc, &meta).is_err(), "5 chars must exceed explicit limit of 4");

    let doc2 = doc_with(&[("pin", json!("1234"))]);
    assert!(validate_length(&doc2, &meta).is_ok(), "4 chars must be within explicit limit of 4");
}

/// LongText has no default length limit.
#[test]
fn test_validate_length_longtext_uncapped() {
    let meta = meta_with_fields("Doc", vec![
        DocField::new("body", "Body", FieldType::LongText),
    ]);
    let very_long = "x".repeat(100_000);
    let doc = doc_with(&[("body", json!(very_long))]);
    assert!(validate_length(&doc, &meta).is_ok(), "LongText should not have a default cap");
}

// ── sanitize_content ─────────────────────────────────────────────────────────

/// Frappe parity: `test_document.py::test_xss_script`
/// `<script>` tags are stripped from Html fields.
#[test]
fn test_sanitize_strips_script() {
    let meta = meta_with_fields("Page", vec![
        DocField::new("content", "Content", FieldType::Html),
    ]);
    let mut doc = doc_with(&[("content", json!("<script>alert('xss')</script><p>Hello</p>"))]);
    sanitize_content(&mut doc, &meta);
    let v = doc.fields.get("content").and_then(|v| v.as_str()).unwrap();
    assert!(!v.contains("<script>"), "script tag must be stripped: {v}");
    assert!(v.contains("<p>Hello</p>") || v.contains("Hello"), "content must survive: {v}");
}

/// Frappe parity: `test_document.py::test_xss_event_handler`
/// `onerror=` event handlers are stripped from Html fields.
#[test]
fn test_sanitize_strips_event_handler() {
    let meta = meta_with_fields("Page", vec![
        DocField::new("content", "Content", FieldType::Html),
    ]);
    let mut doc = doc_with(&[("content", json!("<img src=x onerror=alert(1) />"))]);
    sanitize_content(&mut doc, &meta);
    let v = doc.fields.get("content").and_then(|v| v.as_str()).unwrap();
    assert!(!v.contains("onerror"), "onerror must be stripped: {v}");
}

/// Frappe parity: `test_document.py::test_xss_plain_field`
/// HTML in a plain `Data` field is escaped, not stripped.
#[test]
fn test_sanitize_escapes_plain_html() {
    let meta = meta_with_fields("Item", vec![
        DocField::new("item_name", "Item Name", FieldType::Data),
    ]);
    let mut doc = doc_with(&[("item_name", json!("<script>xss</script>"))]);
    sanitize_content(&mut doc, &meta);
    let v = doc.fields.get("item_name").and_then(|v| v.as_str()).unwrap();
    assert!(!v.contains("<script>"), "raw script must be escaped: {v}");
}

/// Frappe parity: `test_document.py::test_xss_ignore_filter`
/// Fields with `ignore_xss_filter = true` are left untouched.
#[test]
fn test_sanitize_respects_ignore_xss_filter() {
    let meta = meta_with_fields("Email Template", vec![
        DocField::new("body_html", "Body", FieldType::Html).ignore_xss_filter(),
    ]);
    let original = "<script>alert('intentional')</script>";
    let mut doc = doc_with(&[("body_html", json!(original))]);
    sanitize_content(&mut doc, &meta);
    let v = doc.fields.get("body_html").and_then(|v| v.as_str()).unwrap();
    assert_eq!(v, original, "ignore_xss_filter must preserve content");
}

/// Layout fields are untouched by sanitizer.
#[test]
fn test_sanitize_skips_layout_fields() {
    let meta = meta_with_fields("Form", vec![
        DocField::new("section_1", "Section", FieldType::SectionBreak),
    ]);
    let original = "<script>x</script>";
    let mut doc = doc_with(&[("section_1", json!(original))]);
    sanitize_content(&mut doc, &meta);
    // Layout fields are never sanitised — they're also never stored
    let v = doc.fields.get("section_1").and_then(|v| v.as_str()).unwrap();
    assert_eq!(v, original);
}
