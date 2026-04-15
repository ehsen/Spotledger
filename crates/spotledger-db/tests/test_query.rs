//! Tests for `spotledger_db::query` — WhereClause and SetClause builders.
//!
//! ## Frappe parity
//! | Frappe test | This test |
//! |---|---|
//! | `test_db_query.py::TestDBQuery::test_simple_filter` | `test_where_simple_equality` |
//! | `test_db_query.py::TestDBQuery::test_operator_filter` | `test_where_with_operator` |
//! | `test_db_query.py::TestDBQuery::test_multiple_filters` | `test_where_multiple_conditions` |
//! | `test_db_query.py::TestDBQuery::test_empty_filter` | `test_where_none_filter` |
//! | `test_db_query.py::TestDBQuery::test_null_skip` | `test_set_skips_null` |
//! | `test_db_query.py::TestDBQuery::test_system_fields_skip` | `test_set_skips_system_fields` |
//! | `test_query_builder.py::test_where_array_filter` | `test_where_array_operator` |

use serde_json::{json, Value};
use spotledger_db::query::{SetClause, WhereClause};

// ── WhereClause ───────────────────────────────────────────────────────────────

/// Frappe parity: `test_db_query.py::test_simple_filter`
/// Simple equality filter generates correct SQL fragment and binding.
#[test]
fn test_where_simple_equality() {
    let filters = json!({ "status": "Active" });
    let wc = WhereClause::from_filters(Some(&filters));
    assert!(!wc.is_empty());
    let sql = wc.as_sql();
    assert!(sql.contains("WHERE"), "must include WHERE: {sql}");
    assert!(sql.contains("`status`"), "must quote field: {sql}");
    assert!(sql.contains("="), "must use equality: {sql}");
    // binding for "status" exists
    let bindings = wc.bindings();
    assert!(
        bindings.iter().any(|(k, v)| k.contains("status") && v == &json!("Active")),
        "binding for status=Active must exist; bindings={bindings:?}"
    );
}

/// Frappe parity: `test_db_query.py::test_operator_filter`
/// Array `[">", 100]` generates the correct operator.
#[test]
fn test_where_with_operator() {
    let filters = json!({ "total": [">", 100] });
    let wc = WhereClause::from_filters(Some(&filters));
    let sql = wc.as_sql();
    assert!(sql.contains("`total`"), "{sql}");
    assert!(sql.contains(">"), "{sql}");
    let bindings = wc.bindings();
    assert!(
        bindings.iter().any(|(_, v)| v == &json!(100)),
        "binding value 100 must exist; {bindings:?}"
    );
}

/// Frappe parity: `test_db_query.py::test_multiple_filters`
/// Multiple conditions are all included in the WHERE clause.
#[test]
fn test_where_multiple_conditions() {
    let filters = json!({
        "status": "Active",
        "company": "Acme"
    });
    let wc = WhereClause::from_filters(Some(&filters));
    let sql = wc.as_sql();
    assert!(sql.contains("`status`"), "{sql}");
    assert!(sql.contains("`company`"), "{sql}");
    assert!(sql.contains("AND"), "multiple conditions must use AND: {sql}");
    assert_eq!(wc.bindings().len(), 2);
}

/// Frappe parity: `test_db_query.py::test_empty_filter`
/// None filter produces an empty WHERE clause.
#[test]
fn test_where_none_filter() {
    let wc = WhereClause::from_filters(None);
    assert!(wc.is_empty());
    assert_eq!(wc.as_sql(), "");
    assert!(wc.bindings().is_empty());
}

/// Non-object JSON value for filters produces an empty WHERE clause.
#[test]
fn test_where_non_object_filter() {
    let wc = WhereClause::from_filters(Some(&json!(["not", "an", "object"])));
    assert!(wc.is_empty());
}

/// An empty object produces an empty WHERE clause.
#[test]
fn test_where_empty_object_filter() {
    let wc = WhereClause::from_filters(Some(&json!({})));
    assert!(wc.is_empty());
}

/// Frappe parity: `test_query_builder.py::test_where_array_filter`
/// `["like", "%acme%"]` generates LIKE operator.
#[test]
fn test_where_array_operator_like() {
    let filters = json!({ "customer_name": ["like", "%Acme%"] });
    let wc = WhereClause::from_filters(Some(&filters));
    let sql = wc.as_sql();
    assert!(sql.contains("like"), "must use like operator: {sql}");
    assert!(sql.contains("`customer_name`"), "{sql}");
}

/// `["!=", "Cancelled"]` generates != operator.
#[test]
fn test_where_array_not_equal() {
    let filters = json!({ "status": ["!=", "Cancelled"] });
    let wc = WhereClause::from_filters(Some(&filters));
    let sql = wc.as_sql();
    assert!(sql.contains("!="), "{sql}");
}

// ── SetClause ─────────────────────────────────────────────────────────────────

/// Frappe parity: `test_db_query.py::test_null_skip`
/// JSON null values in the document are skipped in the SET clause.
#[test]
fn test_set_skips_null() {
    let fields = json!({
        "customer_name": "Acme",
        "territory": null,
        "credit_limit": null
    });
    let sc = SetClause::from_fields(&fields);
    let sql = sc.as_sql();
    assert!(sql.contains("`customer_name`"), "non-null field must appear: {sql}");
    assert!(!sql.contains("`territory`"),    "null field must be skipped: {sql}");
    assert!(!sql.contains("`credit_limit`"), "null field must be skipped: {sql}");
}

/// Frappe parity: `test_db_query.py::test_system_fields_skip`
/// System fields (`name`, `doctype`, `modified`, `creation`, `id`) are skipped.
#[test]
fn test_set_skips_system_fields() {
    let fields = json!({
        "name": "CUST-001",
        "doctype": "Customer",
        "modified": "2024-01-01",
        "creation": "2024-01-01",
        "id": "customer:CUST-001",
        "customer_name": "Acme"
    });
    let sc = SetClause::from_fields(&fields);
    let sql = sc.as_sql();
    let skip = ["name", "doctype", "modified", "creation", "id"];
    for s in skip {
        assert!(!sql.contains(&format!("`{s}`")), "system field `{s}` must be skipped: {sql}");
    }
    assert!(sql.contains("`customer_name`"), "domain field must appear: {sql}");
}

/// A legitimate field with a non-null value is included in SET.
#[test]
fn test_set_includes_regular_fields() {
    let fields = json!({
        "status": "Active",
        "credit_limit": 50000,
        "enabled": true
    });
    let sc = SetClause::from_fields(&fields);
    let sql = sc.as_sql();
    assert!(sql.contains("`status`"), "{sql}");
    assert!(sql.contains("`credit_limit`"), "{sql}");
    assert!(sql.contains("`enabled`"), "{sql}");
    assert_eq!(sc.bindings().len(), 3);
}

/// Array values (child tables) are included — embedded JSON arrays.
#[test]
fn test_set_includes_array_fields() {
    let fields = json!({
        "items": [
            { "item_code": "WIDGET", "qty": 5 }
        ]
    });
    let sc = SetClause::from_fields(&fields);
    let sql = sc.as_sql();
    assert!(sql.contains("`items`"), "child table array must be in SET: {sql}");
}

/// Non-object input produces a harmless fallback.
#[test]
fn test_set_non_object_fallback() {
    let sc = SetClause::from_fields(&json!("not an object"));
    // Should not panic — produces a safe fallback SQL that DB ignores.
    let sql = sc.as_sql();
    assert!(!sql.is_empty(), "fallback SQL must not be empty");
}

/// Boolean false (not null) is included in SET.
#[test]
fn test_set_includes_false_boolean() {
    let fields = json!({ "enabled": false });
    let sc = SetClause::from_fields(&fields);
    let sql = sc.as_sql();
    assert!(sql.contains("`enabled`"), "false boolean must be in SET: {sql}");
}

/// Zero integer is included in SET.
#[test]
fn test_set_includes_zero_integer() {
    let fields = json!({ "docstatus": 0 });
    let sc = SetClause::from_fields(&fields);
    let sql = sc.as_sql();
    assert!(sql.contains("`docstatus`"), "zero integer must be in SET: {sql}");
}
