//! Document CRUD operations backed by SurrealDB.
//!
//! Table convention: each Frappe DocType → SurrealDB table `tab<DocType>`.
//! Records: `type::record(table, name)` e.g. `type::record("tabCustomer", "ACME Corp")`.
//!
//! SurrealDB SDK v3 requires all `.bind()` values to be owned Strings.

use serde_json::Value;
use surrealdb::engine::remote::ws::Client;
use surrealdb::Surreal;

use spotledger_types::document::{DocRow, Document};

use crate::error::DbError;

/// Convert a Frappe DocType name to a SurrealDB table name.
///
/// "Customer"    → "tabCustomer"
/// "Sales Order" → "tabSales_Order"
pub fn doctype_to_table(doctype: &str) -> String {
    format!("tab{}", doctype.replace(' ', "_"))
}

// ── READ ─────────────────────────────────────────────────────────────────────

/// Fetch a single document by doctype + name.
pub async fn get_doc(
    db: &Surreal<Client>,
    doctype: &str,
    name: &str,
) -> Result<Document, DbError> {
    let table = doctype_to_table(doctype);
    let record_name = name.to_string();

    let query = format!("SELECT * FROM `{table}` WHERE name = $name LIMIT 1");
    let mut response = db
        .query(query.as_str())
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
                    if k != "id" {
                        row.insert(k, val);
                    }
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
    let query = format!("SELECT type::field($field) AS val FROM `{table}` WHERE name = $name LIMIT 1");
    let mut response = db
        .query(query.as_str())
        .bind(("name", name.to_string()))
        .bind(("field", fieldname.to_string()))
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

// ── WRITE ────────────────────────────────────────────────────────────────────

/// Insert a new document (CREATE).  `fields` must include a non-empty `name`.
pub async fn insert_doc(
    db: &Surreal<Client>,
    doctype: &str,
    fields: &Value,
) -> Result<Document, DbError> {
    let table = doctype_to_table(doctype);
    let name = fields
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_owned();

    let (set_clause, bindings) = build_set_from_fields(fields);
    let query = format!(
        "CREATE type::record($table, $name) SET \
         doctype = $doctype, creation = time::now(), modified = time::now(), {set_clause}"
    );

    let mut q = db
        .query(query.as_str())
        .bind(("table", table.clone()))
        .bind(("name", name.clone()))
        .bind(("doctype", doctype.to_owned()));
    for (k, v) in bindings {
        q = q.bind((k, v));
    }

    let mut response = q.await?;
    let rows: Vec<Value> = response.take(0)?;
    let row = rows.into_iter().next().ok_or_else(|| DbError::NotFound {
        doctype: table,
        name: name.clone(),
    })?;

    let actual_name = row
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or(&name)
        .to_owned();
    value_to_document(row, doctype, &actual_name)
}

/// Save (upsert) a document by name.  Supplied fields are merged;
/// unmentioned fields are left unchanged.
pub async fn upsert_doc(
    db: &Surreal<Client>,
    doctype: &str,
    name: &str,
    fields: &Value,
) -> Result<Document, DbError> {
    let table = doctype_to_table(doctype);
    let (set_clause, bindings) = build_set_from_fields(fields);
    // Use type::record so that new documents get the Frappe name as their SurrealDB ID
    // (e.g. tabUser:Administrator).  For existing ULID-keyed records this will insert a
    // duplicate keyed record; callers should prefer insert_doc for truly new documents.
    let query = format!(
        "UPSERT type::record($table, $name) SET \
         doctype = $doctype, name = $name, modified = time::now(), {set_clause}"
    );

    let mut q = db
        .query(query.as_str())
        .bind(("table", table.clone()))
        .bind(("name", name.to_owned()))
        .bind(("doctype", doctype.to_owned()));
    for (k, v) in bindings {
        q = q.bind((k, v));
    }

    let mut response = q.await?;
    let rows: Vec<Value> = response.take(0)?;
    let row = rows.into_iter().next().ok_or_else(|| DbError::NotFound {
        doctype: table,
        name: name.into(),
    })?;
    value_to_document(row, doctype, name)
}

/// Update a single field value on an existing document.
pub async fn set_field(
    db: &Surreal<Client>,
    doctype: &str,
    name: &str,
    fieldname: &str,
    val: Value,
) -> Result<(), DbError> {
    let table = doctype_to_table(doctype);
    let query = format!(
        "UPDATE `{table}` SET `{fieldname}` = $val, modified = time::now() WHERE name = $name"
    );
    db.query(query.as_str())
        .bind(("name", name.to_owned()))
        .bind(("val", val))
        .await?;
    Ok(())
}

/// Delete a document.  Silent no-op if the document does not exist.
pub async fn delete_doc(
    db: &Surreal<Client>,
    doctype: &str,
    name: &str,
) -> Result<(), DbError> {
    let table = doctype_to_table(doctype);
    let query = format!("DELETE `{table}` WHERE name = $name");
    db.query(query.as_str())
        .bind(("name", name.to_owned()))
        .await?;
    Ok(())
}

/// Submit a document: set `docstatus = 1`.
/// Returns the updated document.  Errors if document is not in Draft (docstatus 0).
pub async fn submit_doc(
    db: &Surreal<Client>,
    doctype: &str,
    name: &str,
) -> Result<Document, DbError> {
    let table = doctype_to_table(doctype);
    // Verify current docstatus is 0 (Draft)
    let chk_q = format!("SELECT docstatus FROM `{table}` WHERE name = $name LIMIT 1");
    let mut chk = db
        .query(chk_q.as_str())
        .bind(("name", name.to_owned()))
        .await?;
    let rows: Vec<Value> = chk.take(0)?;
    let row = rows.into_iter().next().ok_or_else(|| DbError::NotFound {
        doctype: table.clone(),
        name: name.into(),
    })?;
    let docstatus = row.get("docstatus").and_then(Value::as_i64).unwrap_or(0);
    if docstatus != 0 {
        return Err(DbError::Other(format!(
            "{doctype} {name} is not in Draft state (docstatus={docstatus})"
        )));
    }

    let upd_q = format!("UPDATE `{table}` SET docstatus = 1, modified = time::now() WHERE name = $name RETURN AFTER");
    let mut resp = db
        .query(upd_q.as_str())
        .bind(("name", name.to_owned()))
        .await?;
    let rows: Vec<Value> = resp.take(0)?;
    let row = rows.into_iter().next().ok_or_else(|| DbError::NotFound {
        doctype: table,
        name: name.into(),
    })?;
    value_to_document(row, doctype, name)
}

/// Cancel a submitted document: set `docstatus = 2`.
/// Returns the updated document.  Errors if document is not Submitted (docstatus 1).
pub async fn cancel_doc(
    db: &Surreal<Client>,
    doctype: &str,
    name: &str,
) -> Result<Document, DbError> {
    let table = doctype_to_table(doctype);
    // Verify current docstatus is 1 (Submitted)
    let chk_q = format!("SELECT docstatus FROM `{table}` WHERE name = $name LIMIT 1");
    let mut chk = db
        .query(chk_q.as_str())
        .bind(("name", name.to_owned()))
        .await?;
    let rows: Vec<Value> = chk.take(0)?;
    let row = rows.into_iter().next().ok_or_else(|| DbError::NotFound {
        doctype: table.clone(),
        name: name.into(),
    })?;
    let docstatus = row.get("docstatus").and_then(Value::as_i64).unwrap_or(0);
    if docstatus != 1 {
        return Err(DbError::Other(format!(
            "{doctype} {name} is not Submitted (docstatus={docstatus})"
        )));
    }

    let upd_q = format!("UPDATE `{table}` SET docstatus = 2, modified = time::now() WHERE name = $name RETURN AFTER");
    let mut resp = db
        .query(upd_q.as_str())
        .bind(("name", name.to_owned()))
        .await?;
    let rows: Vec<Value> = resp.take(0)?;
    let row = rows.into_iter().next().ok_or_else(|| DbError::NotFound {
        doctype: table,
        name: name.into(),
    })?;
    value_to_document(row, doctype, name)
}

/// Count documents matching the given filters.
pub async fn get_count(
    db: &Surreal<Client>,
    doctype: &str,
    filters: Option<&Value>,
) -> Result<u64, DbError> {
    let table = doctype_to_table(doctype);
    let (where_clause, bindings) = build_where(filters);
    let surql = format!("SELECT count() FROM `{table}`{where_clause} GROUP ALL");
    let mut q = db.query(&surql);
    for (k, v) in bindings {
        q = q.bind((k, v));
    }
    let mut resp = q.await?;
    let rows: Vec<Value> = resp.take(0)?;
    let n = rows
        .first()
        .and_then(|obj| obj.get("count"))
        .and_then(Value::as_u64)
        .unwrap_or(0);
    Ok(n)
}

// ── HELPERS ───────────────────────────────────────────────────────────────────

/// Build a WHERE clause + bindings from a JSON filters object.
/// Returns `(clause_string, Vec<(key, value)>)`.
///
/// Simple equality:  `{"status": "Active"}` → `status = $f_status`
/// With operator:    `{"total": [">", 100]}` → `total > $f_total`
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
                conditions.push(format!("`{field}` {op} ${bind_key}"));
                bindings.push((bind_key, arr[1].clone()));
            }
            other => {
                conditions.push(format!("`{field}` = ${bind_key}"));
                bindings.push((bind_key, other.clone()));
            }
        }
    }

    if conditions.is_empty() {
        return (String::new(), vec![]);
    }
    (format!(" WHERE {}", conditions.join(" AND ")), bindings)
}

/// Build a SET clause + owned bindings from a JSON object for INSERT/UPSERT.
/// Skips system fields managed elsewhere (name, doctype, modified, creation, id).
pub fn build_set_from_fields(fields: &Value) -> (String, Vec<(String, Value)>) {
    const SKIP: &[&str] = &["name", "doctype", "modified", "creation", "id"];
    let Value::Object(map) = fields else {
        return ("nothing = NONE".to_owned(), vec![]);
    };

    let mut parts = Vec::new();
    let mut bindings = Vec::new();
    for (k, v) in map {
        if !SKIP.contains(&k.as_str()) {
            let key = format!("f_{k}");
            parts.push(format!("`{k}` = ${key}"));
            bindings.push((key, v.clone()));
        }
    }
    if parts.is_empty() {
        return ("nothing = NONE".to_owned(), vec![]);
    }
    (parts.join(", "), bindings)
}

fn value_to_document(v: Value, doctype: &str, name: &str) -> Result<Document, DbError> {
    let mut doc = Document {
        doctype: doctype.to_string(),
        name: name.to_string(),
        fields: Default::default(),
    };
    if let Value::Object(map) = v {
        for (k, val) in map {
            if k != "id" {
                doc.set(k, val);
            }
        }
    }
    Ok(doc)
}

// ── TESTS ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn doctype_to_table_simple() {
        assert_eq!(doctype_to_table("Customer"), "tabCustomer");
        assert_eq!(doctype_to_table("Sales Order"), "tabSales_Order");
        assert_eq!(doctype_to_table("DocType"), "tabDocType");
    }

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
        assert!(clause.contains("`status` = $f_status"), "clause: {clause}");
        assert_eq!(bindings.len(), 1);
        assert_eq!(bindings[0].1, json!("Active"));
    }

    #[test]
    fn build_where_operator_shape() {
        let filters = json!({"grand_total": [">", 1000]});
        let (clause, bindings) = build_where(Some(&filters));
        assert!(clause.contains("`grand_total` > $f_grand_total"), "clause: {clause}");
        assert_eq!(bindings[0].1, json!(1000));
    }

    #[test]
    fn build_where_multiple_fields() {
        let filters = json!({"status": "Active", "enabled": 1});
        let (clause, bindings) = build_where(Some(&filters));
        assert!(clause.contains("WHERE"), "clause: {clause}");
        assert_eq!(bindings.len(), 2);
    }

    #[test]
    fn build_set_from_fields_skips_system_fields() {
        let fields = json!({"name": "SO-0001", "doctype": "Sales Order", "customer": "Acme", "total": 100});
        let (clause, bindings) = build_set_from_fields(&fields);
        assert!(!clause.contains("`name`"), "should skip name");
        assert!(!clause.contains("`doctype`"), "should skip doctype");
        // bindings should only contain non-system fields
        let keys: Vec<&str> = bindings.iter().map(|(k, _)| k.as_str()).collect();
        assert!(keys.contains(&"f_customer"));
        assert!(keys.contains(&"f_total"));
        assert!(!keys.contains(&"f_name"));
    }

    #[test]
    fn build_set_from_fields_empty_returns_sentinel() {
        let fields = json!({"name": "X", "doctype": "Test"});
        let (clause, bindings) = build_set_from_fields(&fields);
        assert_eq!(clause, "nothing = NONE");
        assert!(bindings.is_empty());
    }
}
