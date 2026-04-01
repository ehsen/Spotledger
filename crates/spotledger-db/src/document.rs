//! Document CRUD operations backed by SurrealDB.
//!
//! SurrealDB table convention: each Frappe DocType → SurrealDB table of the same name.
//! Records: `type::thing(table, name)` e.g. `type::thing("Customer", "ACME Corp")`.
//!
//! SurrealDB SDK v3 requires all `.bind()` values to be `'static` — we always pass
//! owned `String` to satisfy this requirement.

use serde_json::Value;
use surrealdb::engine::remote::ws::Client;
use surrealdb::Surreal;

use spotledger_types::document::{DocRow, Document};

use crate::error::DbError;

/// Convert a Frappe DocType name to a SurrealDB table name.
///
/// Convention: `"tab" + doctype.replace(' ', '_')`
/// Examples:
///   "Customer"    → "tabCustomer"
///   "Sales Order" → "tabSales_Order"
///   "DocType"     → "tabDocType"
pub fn doctype_to_table(doctype: &str) -> String {
    format!("tab{}", doctype.replace(' ', "_"))
}

/// Fetch a single document by doctype + name.
pub async fn get_doc(
    db: &Surreal<Client>,
    doctype: &str,
    name: &str,
) -> Result<Document, DbError> {
    let table = doctype_to_table(doctype);
    let record_name = name.to_string();

    let query = "SELECT * FROM type::thing($table, $name)";
    let mut response = db
        .query(query)
        .bind(("table", table.clone()))
        .bind(("name", record_name.clone()))
        .await?;

    let rows: Vec<Value> = response.take(0)?;

    let row = rows.into_iter().next().ok_or_else(|| DbError::NotFound {
        doctype: table,
        name: record_name,
    })?;

    value_to_document(row, doctype, name)
}

/// Fetch a list of documents with optional field selection and filters.
pub async fn get_list(
    db: &Surreal<Client>,
    doctype: &str,
    fields: Option<&[&str]>,
    filters: Option<&Value>,
    limit: usize,
    start: usize,
) -> Result<Vec<DocRow>, DbError> {
    let field_clause = match fields {
        Some(f) if !f.is_empty() => {
            let mut cols: Vec<String> = f.iter().map(|s| s.to_string()).collect();
            if !cols.contains(&"name".to_string()) {
                cols.insert(0, "name".to_string());
            }
            cols.join(", ")
        }
        _ => "*".to_string(),
    };

    let (where_clause, bindings) = build_where(filters);

    let table = doctype_to_table(doctype);
    let query = format!(
        "SELECT {field_clause} FROM `{table}`{where_clause} LIMIT {limit} START {start}"
    );

    tracing::debug!(%query, "get_list");

    // Collect bindings as Vec<(String, Value)> before the async boundary.
    // We build the query with all bindings applied synchronously, then await.
    let mut q = db.query(query.as_str());
    for (k, v) in bindings {
        q = q.bind((k, v));
    }

    let mut response = q.await?;
    let rows: Vec<Value> = response.take(0)?;

    Ok(rows
        .into_iter()
        .map(|v| {
            let mut row: DocRow = Default::default();
            if let Value::Object(map) = v {
                for (k, val) in map {
                    if k == "id" {
                        continue; // skip SurrealDB internal record id
                    }
                    row.insert(k, val);
                }
            }
            row
        })
        .collect())
}

/// Fetch a single field value from a document.
pub async fn get_value(
    db: &Surreal<Client>,
    doctype: &str,
    name: &str,
    fieldname: &str,
) -> Result<Option<Value>, DbError> {
    let table = doctype_to_table(doctype);
    let record_name = name.to_string();
    let field = fieldname.to_string();

    let query = "SELECT type::field($field) AS val FROM type::thing($table, $name)";
    let mut response = db
        .query(query)
        .bind(("table", table))
        .bind(("name", record_name))
        .bind(("field", field))
        .await?;

    let rows: Vec<Value> = response.take(0)?;

    Ok(rows.into_iter().next().and_then(|row| {
        if let Value::Object(map) = row {
            map.get("val").cloned()
        } else {
            None
        }
    }))
}

// ── helpers ──────────────────────────────────────────────────────────────────

fn value_to_document(v: Value, doctype: &str, name: &str) -> Result<Document, DbError> {
    let mut doc = Document {
        doctype: doctype.to_string(),
        name: name.to_string(),
        fields: Default::default(),
    };

    if let Value::Object(map) = v {
        for (k, val) in map {
            match k.as_str() {
                "id" => {} // skip SurrealDB internal record id
                _ => {
                    doc.set(k, val);
                }
            }
        }
    }

    Ok(doc)
}

/// Build a simple WHERE clause and owned bindings from a JSON filters object.
/// Returns `(clause_string, Vec<(String, Value)>)`.
pub fn build_where(filters: Option<&Value>) -> (String, Vec<(String, Value)>) {
    let Some(Value::Object(map)) = filters else {
        return (String::new(), vec![]);
    };

    let mut conditions = Vec::new();
    let mut bindings: Vec<(String, Value)> = Vec::new();

    for (field, val) in map {
        let bind_key = format!("f_{field}");
        match val {
            Value::Array(arr) if arr.len() == 2 => {
                let op = arr[0].as_str().unwrap_or("=");
                let v = arr[1].clone();
                conditions.push(format!("{field} {op} ${bind_key}"));
                bindings.push((bind_key, v));
            }
            other => {
                conditions.push(format!("{field} = ${bind_key}"));
                bindings.push((bind_key, other.clone()));
            }
        }
    }

    if conditions.is_empty() {
        return (String::new(), vec![]);
    }

    (format!(" WHERE {}", conditions.join(" AND ")), bindings)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn build_where_empty() {
        let (clause, bindings) = build_where(None);
        assert!(clause.is_empty());
        assert!(bindings.is_empty());
    }

    #[test]
    fn build_where_simple_equality() {
        let filters = json!({"status": "Active"});
        let (clause, bindings) = build_where(Some(&filters));
        assert!(clause.contains("status = $f_status"), "clause: {clause}");
        assert_eq!(bindings.len(), 1);
        assert_eq!(bindings[0].1, json!("Active"));
    }

    #[test]
    fn build_where_operator_shape() {
        let filters = json!({"grand_total": [">", 1000]});
        let (clause, bindings) = build_where(Some(&filters));
        assert!(clause.contains("grand_total > $f_grand_total"), "clause: {clause}");
        assert_eq!(bindings[0].1, json!(1000));
    }

    #[test]
    fn build_where_multiple_fields() {
        let filters = json!({"status": "Active", "enabled": 1});
        let (clause, bindings) = build_where(Some(&filters));
        assert!(clause.contains("WHERE"), "clause: {clause}");
        assert_eq!(bindings.len(), 2);
    }
}
