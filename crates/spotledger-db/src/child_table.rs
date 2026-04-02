//! Child table handling — read/write nested child documents.
//!
//! In Frappe, child tables are separate DocType records stored in their own table
//! with `parent`, `parenttype`, `parentfield`, and `idx` fields.
//!
//! Examples:
//!   - `Sales Invoice Item` is a child of `Sales Invoice` via the `items` field.
//!   - `Address` is standalone (not a child table).
//!
//! Spotledger stores children in the parent record as a nested array for efficient
//! reads (no JOIN needed).  When reading a document that has child table fields,
//! we automatically fetch and embed the children.
//!
//! When writing, we scatter the child array back into individual child table records
//! (for backwards compatibility with the Frappe schema) AND keep the nested array.
//!
//! Phase 2 scope: read-path only — fetch children from `tabXxx` where
//! `parent = name AND parenttype = doctype`.

use serde_json::Value;
use surrealdb::engine::remote::ws::Client;
use surrealdb::Surreal;

use crate::document::doctype_to_table;
use spotledger_types::document::DocRow;
use crate::error::DbError;

/// Fetch all child rows for a parent document from a single child table.
///
/// # Arguments
/// * `child_doctype` — the child table DocType name (e.g. "Sales Invoice Item")
/// * `parent_name`   — the parent document name (e.g. "SINV-00001")
/// * `parenttype`    — the parent DocType name   (e.g. "Sales Invoice")
/// * `parentfield`   — the field name on the parent (e.g. "items")
pub async fn fetch_children(
    db: &Surreal<Client>,
    child_doctype: &str,
    parent_name: &str,
    parenttype: &str,
    parentfield: &str,
) -> Result<Vec<DocRow>, DbError> {
    let table = doctype_to_table(child_doctype);
    let mut resp = db
        .query(
            "SELECT * FROM type::table($table) \
             WHERE parent = $parent AND parenttype = $parenttype AND parentfield = $parentfield \
             ORDER BY idx ASC",
        )
        .bind(("table", table))
        .bind(("parent", parent_name.to_owned()))
        .bind(("parenttype", parenttype.to_owned()))
        .bind(("parentfield", parentfield.to_owned()))
        .await?;

    let rows: Vec<Value> = resp.take(0)?;
    Ok(rows
        .into_iter()
        .map(|v| {
            let mut row: DocRow = Default::default();
            if let Value::Object(map) = v {
                for (k, val) in map {
                    if k != "id" {
                        row.insert(k, val);
                    }
                }
            }
            row
        })
        .collect())
}

/// Save child rows for a parent document.
///
/// Strategy: delete existing children for this parent+parentfield, then re-insert.
/// `idx` is assigned automatically from the array position (1-based).
pub async fn save_children(
    db: &Surreal<Client>,
    child_doctype: &str,
    parent_name: &str,
    parenttype: &str,
    parentfield: &str,
    children: &[Value],
) -> Result<(), DbError> {
    let table = doctype_to_table(child_doctype);

    // Delete existing children
    db.query(
        "DELETE FROM type::table($table) \
         WHERE parent = $parent AND parenttype = $parenttype AND parentfield = $parentfield",
    )
    .bind(("table", table.clone()))
    .bind(("parent", parent_name.to_owned()))
    .bind(("parenttype", parenttype.to_owned()))
    .bind(("parentfield", parentfield.to_owned()))
    .await?;

    // Insert new children
    for (idx, child) in children.iter().enumerate() {
        let child_name = child
            .get("name")
            .and_then(Value::as_str)
            .map(str::to_owned)
            .unwrap_or_else(|| format!("{parent_name}-{parentfield}-{}", idx + 1));

        // Build field set from child value, skipping system fields
        let mut field_parts: Vec<String> = Vec::new();
        let mut bindings: Vec<(String, Value)> = Vec::new();

        const SKIP: &[&str] = &["name", "doctype", "id", "parent", "parenttype", "parentfield", "idx"];

        if let Value::Object(map) = child {
            for (k, v) in map {
                if !SKIP.contains(&k.as_str()) {
                    let bk = format!("cf_{}", k.replace('-', "_"));
                    field_parts.push(format!("`{k}` = ${bk}"));
                    bindings.push((bk, v.clone()));
                }
            }
        }

        let set_clause = if field_parts.is_empty() {
            "nothing = NONE".to_string()
        } else {
            field_parts.join(", ")
        };

        let query = format!(
            "CREATE type::thing($table, $name) SET \
             doctype = $child_doctype, \
             parent = $parent, parenttype = $parenttype, parentfield = $parentfield, \
             idx = $idx, creation = time::now(), modified = time::now(), \
             {set_clause}"
        );

        let mut q = db
            .query(&query)
            .bind(("table", table.clone()))
            .bind(("name", child_name))
            .bind(("child_doctype", child_doctype.to_owned()))
            .bind(("parent", parent_name.to_owned()))
            .bind(("parenttype", parenttype.to_owned()))
            .bind(("parentfield", parentfield.to_owned()))
            .bind(("idx", (idx + 1) as i64));
        for (k, v) in bindings {
            q = q.bind((k, v));
        }
        q.await?;
    }

    Ok(())
}

/// Given a parent document's fields, extract any array fields that are child tables.
///
/// Heuristic: an array field whose elements are objects with a "doctype" key is
/// treated as a child table array.
///
/// Returns `(parentfield, child_doctype, Vec<Value>)` tuples.
pub fn extract_child_tables(
    fields: &Value,
) -> Vec<(String, String, Vec<Value>)> {
    let Value::Object(map) = fields else {
        return vec![];
    };

    let mut result = Vec::new();
    for (field_name, val) in map {
        let Value::Array(arr) = val else { continue };
        if arr.is_empty() { continue; }
        // Check if first element looks like a child table row
        if let Some(first) = arr.first() {
            if first.is_object() {
                if let Some(child_doctype) = first.get("doctype").and_then(Value::as_str) {
                    result.push((field_name.clone(), child_doctype.to_owned(), arr.clone()));
                }
            }
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn extract_child_tables_finds_array_fields() {
        let fields = json!({
            "name": "SINV-001",
            "customer": "Acme",
            "items": [
                {"doctype": "Sales Invoice Item", "item_code": "ITEM-001", "qty": 1},
                {"doctype": "Sales Invoice Item", "item_code": "ITEM-002", "qty": 2}
            ],
            "taxes": []
        });

        let children = extract_child_tables(&fields);
        assert_eq!(children.len(), 1);
        assert_eq!(children[0].0, "items");
        assert_eq!(children[0].1, "Sales Invoice Item");
        assert_eq!(children[0].2.len(), 2);
    }

    #[test]
    fn extract_child_tables_ignores_non_doctype_arrays() {
        let fields = json!({
            "name": "X",
            "tags": ["a", "b", "c"],
            "items": [
                {"doctype": "Item Row", "qty": 1}
            ]
        });

        let children = extract_child_tables(&fields);
        // "tags" is array of strings, not objects with doctype
        assert_eq!(children.len(), 1);
        assert_eq!(children[0].0, "items");
    }

    #[test]
    fn extract_child_tables_empty_arrays_skipped() {
        let fields = json!({"name": "X", "items": []});
        let children = extract_child_tables(&fields);
        assert!(children.is_empty());
    }
}
