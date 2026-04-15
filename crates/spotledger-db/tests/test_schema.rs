//! Tests for `spotledger_db::schema` — SurrealDB DDL type mapping and table naming.
//!
//! ## Frappe parity
//! | Frappe test | This test |
//! |---|---|
//! | `test_db_update.py::test_column_types` | `test_surql_type_*` |
//! | `test_doctype.py::test_fieldtype_storage` | `test_surql_type_layout_returns_none` |
//! | `test_db.py::test_table_naming` | `test_doctype_to_table_*` |
//! | `test_db_update.py::test_child_table_fields` | `test_surql_type_table_field` |

use spotledger_core::meta::FieldType;
use spotledger_db::{document::doctype_to_table, schema::surql_type};

// ── doctype_to_table ──────────────────────────────────────────────────────────

/// Frappe parity: `test_db.py::test_table_naming`
/// Simple single-word DocType names get a `tab` prefix.
#[test]
fn test_doctype_to_table_simple() {
    assert_eq!(doctype_to_table("Customer"), "tabCustomer");
    assert_eq!(doctype_to_table("User"), "tabUser");
    assert_eq!(doctype_to_table("Item"), "tabItem");
    assert_eq!(doctype_to_table("Account"), "tabAccount");
}

/// Multi-word DocType names have spaces replaced with underscores.
#[test]
fn test_doctype_to_table_multi_word() {
    assert_eq!(doctype_to_table("Sales Order"), "tabSales_Order");
    assert_eq!(doctype_to_table("GL Entry"), "tabGL_Entry");
    assert_eq!(doctype_to_table("Purchase Invoice"), "tabPurchase_Invoice");
    assert_eq!(doctype_to_table("Journal Entry"), "tabJournal_Entry");
}

/// Three-word names are handled correctly.
#[test]
fn test_doctype_to_table_three_words() {
    assert_eq!(doctype_to_table("Sales Order Item"), "tabSales_Order_Item");
    assert_eq!(doctype_to_table("Purchase Invoice Item"), "tabPurchase_Invoice_Item");
}

/// Single-character name is handled without panic.
#[test]
fn test_doctype_to_table_single_char() {
    assert_eq!(doctype_to_table("X"), "tabX");
}

// ── surql_type ────────────────────────────────────────────────────────────────

/// Frappe parity: `test_db_update.py::test_column_types` — integer types.
#[test]
fn test_surql_type_int() {
    let t = surql_type(&FieldType::Int).unwrap();
    assert!(
        t.contains("int"),
        "Int must map to an integer SurrealDB type: {t}"
    );
}

#[test]
fn test_surql_type_check() {
    let t = surql_type(&FieldType::Check).unwrap();
    assert!(t.contains("int"), "Check must map to an integer type: {t}");
}

/// Float-family maps to `float`.
#[test]
fn test_surql_type_float() {
    assert_eq!(surql_type(&FieldType::Float).unwrap(), "float");
    assert_eq!(surql_type(&FieldType::Currency).unwrap(), "float");
    assert_eq!(surql_type(&FieldType::Percent).unwrap(), "float");
    assert_eq!(surql_type(&FieldType::Rating).unwrap(), "float");
}

/// Datetime maps to `option<datetime>`.
#[test]
fn test_surql_type_datetime() {
    assert_eq!(surql_type(&FieldType::Datetime).unwrap(), "option<datetime>");
}

/// Date and Time map to `option<string>` (Frappe format strings).
#[test]
fn test_surql_type_date_time() {
    assert_eq!(surql_type(&FieldType::Date).unwrap(), "option<string>");
    assert_eq!(surql_type(&FieldType::Time).unwrap(), "option<string>");
}

/// Table (child table) maps to `array<any>`.
#[test]
fn test_surql_type_table_field() {
    assert_eq!(surql_type(&FieldType::Table).unwrap(), "array<any>");
    assert_eq!(surql_type(&FieldType::TableMultiSelect).unwrap(), "array<any>");
}

/// Json maps to `any`.
#[test]
fn test_surql_type_json() {
    assert_eq!(surql_type(&FieldType::Json).unwrap(), "any");
}

/// Frappe parity: `test_doctype.py::test_fieldtype_storage` — layout fields have no DB column.
#[test]
fn test_surql_type_layout_returns_none() {
    assert!(surql_type(&FieldType::SectionBreak).is_none(), "SectionBreak has no column");
    assert!(surql_type(&FieldType::ColumnBreak).is_none(), "ColumnBreak has no column");
    assert!(surql_type(&FieldType::TabBreak).is_none(), "TabBreak has no column");
}

/// Remaining string-like types default to `option<string>`.
#[test]
fn test_surql_type_string_family() {
    let string_types = [
        FieldType::Data,
        FieldType::Text,
        FieldType::LongText,
        FieldType::SmallText,
        FieldType::Select,
        FieldType::Link,
        FieldType::DynamicLink,
        FieldType::Attach,
        FieldType::AttachImage,
        FieldType::Html,
        FieldType::Code,
        FieldType::Signature,
        FieldType::Password,
        FieldType::Color,
    ];
    for ft in &string_types {
        match surql_type(ft) {
            Some(t) => assert!(
                t.contains("string"),
                "{ft:?} must map to a string SurrealDB type; got: {t}"
            ),
            None => panic!("{ft:?} must have a SurrealDB type mapping"),
        }
    }
}

/// All `FieldType` variants (except layout) produce a non-None type.
#[test]
fn test_surql_type_coverage() {
    let layout = [FieldType::SectionBreak, FieldType::ColumnBreak, FieldType::TabBreak];
    let all_types = [
        FieldType::Data, FieldType::Text, FieldType::LongText, FieldType::SmallText,
        FieldType::Int, FieldType::Float, FieldType::Currency, FieldType::Percent,
        FieldType::Check, FieldType::Date, FieldType::Datetime, FieldType::Time,
        FieldType::Select, FieldType::Link, FieldType::DynamicLink,
        FieldType::Table, FieldType::TableMultiSelect,
        FieldType::Attach, FieldType::AttachImage,
        FieldType::Html, FieldType::Code, FieldType::Signature,
        FieldType::Password, FieldType::Color, FieldType::Rating,
        FieldType::Json,
        FieldType::SectionBreak, FieldType::ColumnBreak, FieldType::TabBreak,
    ];
    for ft in &all_types {
        if layout.contains(ft) {
            assert!(surql_type(ft).is_none(), "layout {ft:?} must return None");
        } else {
            assert!(surql_type(ft).is_some(), "data {ft:?} must return Some(...)");
        }
    }
}
