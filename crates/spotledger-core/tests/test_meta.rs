//! Tests for `spotledger_core::meta` — DocTypeMeta, DocField, FieldType, Permission.
//!
//! ## Frappe parity
//! | Frappe test | This test |
//! |---|---|
//! | `test_document.py::test_load — fields/permissions are lists` | `test_doctype_meta_builder` |
//! | `test_doctype.py::TestDocType::test_field_types` | `test_fieldtype_kind` |
//! | `test_doctype.py::TestDocType::test_layout_fields` | `test_layout_fields_have_no_storage` |
//! | `test_permissions.py::test_full_perm` | `test_permission_full` |
//! | `test_permissions.py::test_read_only_perm` | `test_permission_read_only` |
//! | `test_non_nullable_docfield.py` | `test_docfield_not_nullable` |
//! | `test_doctype.py::test_set_only_once` | `test_docfield_set_only_once` |

use spotledger_core::meta::{DocField, DocTypeMeta, FieldType, Permission};

// ── FieldType ─────────────────────────────────────────────────────────────────

/// Frappe parity: `test_doctype.py::test_field_types`
/// Layout fields are correctly classified as layout-only.
#[test]
fn test_layout_fields_have_no_storage() {
    assert!(FieldType::SectionBreak.is_layout());
    assert!(FieldType::ColumnBreak.is_layout());
    assert!(FieldType::TabBreak.is_layout());
}

/// Frappe parity: `test_doctype.py::test_field_types`
/// Data fields are not layout.
#[test]
fn test_data_fields_are_not_layout() {
    let data_types = [
        FieldType::Data,
        FieldType::Int,
        FieldType::Float,
        FieldType::Currency,
        FieldType::Check,
        FieldType::Date,
        FieldType::Datetime,
        FieldType::Time,
        FieldType::Select,
        FieldType::Link,
        FieldType::Table,
        FieldType::Text,
        FieldType::LongText,
        FieldType::SmallText,
        FieldType::Html,
        FieldType::Json,
        FieldType::Password,
        FieldType::Rating,
        FieldType::Attach,
        FieldType::AttachImage,
    ];
    for ft in &data_types {
        assert!(!ft.is_layout(), "{ft:?} should not be layout");
    }
}

/// FieldType equality works correctly.
#[test]
fn test_fieldtype_equality() {
    assert_eq!(FieldType::Data, FieldType::Data);
    assert_ne!(FieldType::Data, FieldType::Text);
    assert_ne!(FieldType::Int, FieldType::Float);
}

/// FieldType can be serialized and deserialized (round-trip).
#[test]
fn test_fieldtype_serde_roundtrip() {
    let ft = FieldType::Currency;
    let s = serde_json::to_string(&ft).unwrap();
    let back: FieldType = serde_json::from_str(&s).unwrap();
    assert_eq!(ft, back);
}

// ── DocField ──────────────────────────────────────────────────────────────────

/// Frappe parity: `test_non_nullable_docfield.py`
/// DocField constructed with `not_nullable()` builder has the flag set.
#[test]
fn test_docfield_not_nullable() {
    let df = DocField::new("account_name", "Account Name", FieldType::Data)
        .required()
        .not_nullable();
    assert!(df.reqd);
    assert!(df.not_nullable);
}

/// Frappe parity: `test_doctype.py::test_set_only_once`
/// `set_only_once` flag is correctly set via builder.
#[test]
fn test_docfield_set_only_once() {
    let df = DocField::new("company", "Company", FieldType::Link)
        .set_only_once();
    assert!(df.set_only_once);
    assert!(!df.allow_on_submit);
}

/// `allow_on_submit` flag is correctly set for fields editable after submission.
#[test]
fn test_docfield_allow_on_submit() {
    let df = DocField::new("remarks", "Remarks", FieldType::Text)
        .allow_on_submit();
    assert!(df.allow_on_submit);
}

/// DocField builder defaults are sane.
#[test]
fn test_docfield_defaults() {
    let df = DocField::new("status", "Status", FieldType::Select);
    assert!(!df.reqd);
    assert!(!df.read_only);
    assert!(!df.hidden);
    assert!(!df.bold);
    assert!(!df.in_list_view);
    assert!(!df.set_only_once);
    assert!(!df.allow_on_submit);
    assert!(!df.ignore_xss_filter);
    assert!(!df.unique);
    assert!(!df.not_nullable);
    assert_eq!(df.permlevel, 0);
    assert!(df.options.is_none());
    assert!(df.select_options.is_none());
    assert!(df.default_value.is_none());
    assert!(df.length.is_none());
}

/// DocField `select_options` and `options` are separate concerns.
#[test]
fn test_docfield_select_vs_link_options() {
    let select = DocField::new("type", "Type", FieldType::Select)
        .select_options("Draft\nSubmitted\nCancelled");
    assert_eq!(select.select_options.as_deref(), Some("Draft\nSubmitted\nCancelled"));
    assert!(select.options.is_none());

    let link = DocField::new("customer", "Customer", FieldType::Link)
        .options("Customer");
    assert_eq!(link.options.as_deref(), Some("Customer"));
    assert!(link.select_options.is_none());
}

/// DocField length override is stored correctly.
#[test]
fn test_docfield_length_override() {
    let df = DocField::new("code", "Code", FieldType::Data).length(10);
    assert_eq!(df.length, Some(10));
}

/// DocField permlevel is stored correctly.
#[test]
fn test_docfield_permlevel() {
    let df = DocField::new("secret", "Secret", FieldType::Password).permlevel(1);
    assert_eq!(df.permlevel, 1);
}

// ── Permission ────────────────────────────────────────────────────────────────

/// Frappe parity: Manager gets all permissions.
#[test]
fn test_permission_full() {
    let p = Permission::full("System Manager");
    assert_eq!(p.role, "System Manager");
    assert!(p.read && p.write && p.create && p.delete);
    assert!(p.submit && p.cancel && p.amend);
    assert!(p.report && p.import && p.export);
    assert!(p.print && p.email && p.share);
}

/// Frappe parity: Read-only user gets read, report, export, print; nothing else.
#[test]
fn test_permission_read_only() {
    let p = Permission::read_only("Guest");
    assert_eq!(p.role, "Guest");
    assert!(p.read);
    assert!(!p.write);
    assert!(!p.create);
    assert!(!p.delete);
    assert!(!p.submit);
    assert!(!p.cancel);
    assert!(!p.share);
    assert!(p.report);
    assert!(p.export);
    assert!(p.print);
}

// ── DocTypeMeta ───────────────────────────────────────────────────────────────

/// Frappe parity: `test_document.py::test_load — DocType "User" has fields and permissions as lists`
/// DocTypeMeta builds correctly with fields and permissions.
#[test]
fn test_doctype_meta_builder() {
    let meta = DocTypeMeta::builder("Sales Order", "Selling")
        .submittable()
        .field(DocField::new("customer", "Customer", FieldType::Link).options("Customer").required())
        .field(DocField::new("naming_series", "Series", FieldType::Select))
        .field(DocField::new("status", "Status", FieldType::Select).select_options("Draft\nTo Deliver and Bill"))
        .permission(Permission::full("System Manager"))
        .permission(Permission::read_only("Sales User"))
        .build();

    assert_eq!(meta.name, "Sales Order");
    assert_eq!(meta.module, "Selling");
    assert!(meta.is_submittable);
    assert!(!meta.is_single);
    assert_eq!(meta.fields.len(), 3);
    assert_eq!(meta.permissions.len(), 2);
    assert_eq!(meta.fields[0].fieldname, "customer");
    assert!(meta.fields[0].reqd);
}

/// Single-type DocType builder.
#[test]
fn test_doctype_meta_single() {
    let meta = DocTypeMeta::builder("System Settings", "Core")
        .single()
        .build();
    assert!(meta.is_single);
    assert!(!meta.is_tree);
    assert!(!meta.is_child);
}

/// Tree DocType builder.
#[test]
fn test_doctype_meta_tree() {
    let meta = DocTypeMeta::builder("Account", "Accounts")
        .tree()
        .build();
    assert!(meta.is_tree);
}

/// Child DocType builder.
#[test]
fn test_doctype_meta_child() {
    let meta = DocTypeMeta::builder("Sales Order Item", "Selling")
        .child()
        .build();
    assert!(meta.is_child);
}

/// DocTypeMeta with autoname.
#[test]
fn test_doctype_meta_autoname() {
    let meta = DocTypeMeta::builder("Item", "Stock")
        .autoname("field:item_code")
        .build();
    assert_eq!(meta.autoname.as_deref(), Some("field:item_code"));
}

/// DocTypeMeta with naming_series default.
#[test]
fn test_doctype_meta_naming_series() {
    let meta = DocTypeMeta::builder("Sales Invoice", "Accounts")
        .submittable()
        .naming_series("SINV-.YYYY.-.####")
        .build();
    assert_eq!(meta.naming_series.as_deref(), Some("SINV-.YYYY.-.####"));
}

/// DocTypeMeta serialises and deserialises intact.
#[test]
fn test_doctype_meta_serde() {
    let meta = DocTypeMeta::builder("Note", "Core")
        .field(DocField::new("title", "Title", FieldType::Data).required())
        .build();
    let json = serde_json::to_string(&meta).unwrap();
    let back: DocTypeMeta = serde_json::from_str(&json).unwrap();
    assert_eq!(back.name, "Note");
    assert_eq!(back.fields.len(), 1);
    assert_eq!(back.fields[0].fieldname, "title");
}
