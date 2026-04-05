//! Document CRUD operations backed by SurrealDB.
//!
//! ## Table convention
//!
//! Each Frappe DocType maps to a SurrealDB table: `tab<DocType>` with spaces
//! replaced by underscores.  Examples:
//! - `Customer`    -> `tabCustomer`
//! - `Sales Order` -> `tabSales_Order`
//!
//! ## Single chokepoint
//!
//! All SQL in this module flows through [`DbAdapter`].  No raw
//! `Surreal<Client>` handles escape this module.

use indexmap::IndexMap;
use serde_json::Value;

use spotledger_core::document::{DocRow, Document};

use crate::adapter::DbAdapter;
use crate::error::DbError;
use crate::query::{SetClause, WhereClause};

// -- helpers ------------------------------------------------------------------

/// Convert a Frappe DocType name to a SurrealDB table name.
///
/// `"Customer"` -> `"tabCustomer"`, `"Sales Order"` -> `"tabSales_Order"`
pub fn doctype_to_table(doctype: &str) -> String {
    format!("tab{}", doctype.replace(' ', "_"))
}

/// Deserialize a raw SurrealDB JSON row into a [`Document`].
fn value_to_document(v: Value, doctype: &str, name: &str) -> Result<Document, DbError> {
    let Value::Object(mut map) = v else {
        return Err(DbError::Other("expected JSON object from DB".into()));
    };
    map.remove("id");

    let mut doc = Document::new(doctype);
    doc.name = name.to_owned();
    for (k, val) in map {
        if k != "doctype" && k != "name" {
            doc.fields.insert(k, val);
        }
    }
    Ok(doc)
}

/// Convert a raw SurrealDB JSON Value to a DocRow.
fn value_to_docrow(v: Value) -> DocRow {
    let mut row: DocRow = IndexMap::new();
    if let Value::Object(map) = v {
        for (k, val) in map {
            if k != "id" {
                row.insert(k, val);
            }
        }
    }
    row
}

/// Return the set of field names that SurrealDB has DEFINE FIELD for on a table,
/// plus the standard Frappe system fields which are always valid.
pub(crate) async fn get_valid_columns(
    adapter: &DbAdapter,
    doctype: &str,
) -> std::collections::HashSet<String> {
    let mut cols: std::collections::HashSet<String> = [
        "name", "owner", "creation", "modified", "modified_by",
        "docstatus", "idx", "parent", "parenttype", "parentfield",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();

    let table = doctype_to_table(doctype);
    let sql   = format!("INFO FOR TABLE `{table}`;");

    let mut resp: surrealdb::IndexedResults = match adapter.raw_query(&sql).await {
        Ok(r)  => r,
        Err(_) => return cols,
    };
    let info: Vec<Value> = resp.take(0).unwrap_or_default();
    if let Some(Value::Object(map)) = info.into_iter().next() {
        if let Some(Value::Object(fields_map)) = map.get("fields") {
            for key in fields_map.keys() {
                cols.insert(key.clone());
            }
        }
    }
    cols
}

/// Filter fields to only those the schema accepts.
pub(crate) fn filter_to_valid_columns(
    fields: &Value,
    valid: &std::collections::HashSet<String>,
) -> Value {
    let Value::Object(map) = fields else { return fields.clone() };
    let filtered: serde_json::Map<String, Value> = map
        .iter()
        .filter(|(k, v)| valid.contains(*k) && !v.is_array())
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    Value::Object(filtered)
}

// -- READ ---------------------------------------------------------------------

/// Fetch a single document by doctype + name.
pub async fn get_doc(
    adapter: &DbAdapter,
    doctype: &str,
    name: &str,
) -> Result<Document, DbError> {
    let table = doctype_to_table(doctype);
    let sql   = format!("SELECT * FROM `{table}` WHERE name = $name LIMIT 1");
    let row   = adapter
        .run_one(&sql, vec![("name".into(), name.into())], doctype, name)
        .await?;
    value_to_document(row, doctype, name)
}

/// Fetch a list of documents with optional field selection and filters.
pub async fn get_list(
    adapter: &DbAdapter,
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

    let where_clause = WhereClause::from_filters(filters);
    let table        = doctype_to_table(doctype);
    let sql = format!(
        "SELECT {field_clause} FROM `{table}`{} LIMIT {limit} START {start}",
        where_clause.as_sql()
    );
    tracing::debug!(%sql, "get_list");

    let rows = adapter.run(&sql, where_clause.bindings().to_vec()).await?;
    Ok(rows.into_iter().map(value_to_docrow).collect())
}

/// Fetch a single field value from a document.
pub async fn get_value(
    adapter: &DbAdapter,
    doctype: &str,
    name: &str,
    fieldname: &str,
) -> Result<Option<Value>, DbError> {
    let table = doctype_to_table(doctype);
    let sql = format!(
        "SELECT type::field($field) AS val FROM `{table}` WHERE name = $name LIMIT 1"
    );
    let rows = adapter
        .run(
            &sql,
            vec![
                ("name".into(),  name.into()),
                ("field".into(), fieldname.into()),
            ],
        )
        .await?;

    Ok(rows.into_iter().next().and_then(|row| {
        if let Value::Object(map) = row { map.get("val").cloned() } else { None }
    }))
}

/// Count documents matching the given filters.
pub async fn get_count(
    adapter: &DbAdapter,
    doctype: &str,
    filters: Option<&Value>,
) -> Result<u64, DbError> {
    let where_clause = WhereClause::from_filters(filters);
    let table        = doctype_to_table(doctype);
    let sql = format!("SELECT count() FROM `{table}`{} GROUP ALL", where_clause.as_sql());

    let rows = adapter.run(&sql, where_clause.bindings().to_vec()).await?;
    Ok(rows
        .first()
        .and_then(|obj| obj.get("count"))
        .and_then(Value::as_u64)
        .unwrap_or(0))
}

// -- WRITE --------------------------------------------------------------------

/// Insert a new document.  `fields` must contain a non-empty `name`.
pub async fn insert_doc(
    adapter: &DbAdapter,
    doctype: &str,
    fields: &Value,
) -> Result<Document, DbError> {
    let name = fields
        .get("name")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| DbError::Other("insert_doc: 'name' field is required".into()))?
        .to_owned();

    let valid    = get_valid_columns(adapter, doctype).await;
    let filtered = filter_to_valid_columns(fields, &valid);
    let set      = SetClause::from_fields(&filtered);
    let table    = doctype_to_table(doctype);
    let sql = format!(
        "CREATE type::record($table, $name) SET \
         name = $name, creation = time::now(), modified = time::now(), {}",
        set.as_sql()
    );

    let mut bindings = vec![
        ("table".into(), Value::String(table)),
        ("name".into(),  Value::String(name.clone())),
    ];
    bindings.extend_from_slice(set.bindings());

    let row = adapter.run_one(&sql, bindings, doctype, &name).await?;
    let actual_name = row
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or(&name)
        .to_owned();
    value_to_document(row, doctype, &actual_name)
}

/// Upsert (save) a document -- merges supplied fields, leaves unmentioned fields intact.
pub async fn upsert_doc(
    adapter: &DbAdapter,
    doctype: &str,
    name: &str,
    fields: &Value,
) -> Result<Document, DbError> {
    let valid    = get_valid_columns(adapter, doctype).await;
    let filtered = filter_to_valid_columns(fields, &valid);
    let set      = SetClause::from_fields(&filtered);
    let table    = doctype_to_table(doctype);
    let sql = format!(
        "UPSERT type::record($table, $name) SET \
         name = $name, modified = time::now(), \
         creation = IF creation THEN creation ELSE time::now() END, {}",
        set.as_sql()
    );

    let mut bindings = vec![
        ("table".into(), Value::String(table)),
        ("name".into(),  Value::String(name.to_owned())),
    ];
    bindings.extend_from_slice(set.bindings());

    let row = adapter.run_one(&sql, bindings, doctype, name).await?;
    value_to_document(row, doctype, name)
}

/// Update a single field value on an existing document.
pub async fn set_field(
    adapter: &DbAdapter,
    doctype: &str,
    name: &str,
    fieldname: &str,
    val: Value,
) -> Result<(), DbError> {
    let table = doctype_to_table(doctype);
    let sql = format!(
        "UPDATE `{table}` SET `{fieldname}` = $val, modified = time::now() WHERE name = $name"
    );
    adapter
        .execute(&sql, vec![("name".into(), name.into()), ("val".into(), val)])
        .await
}

/// Delete a document.  Silent no-op if the document does not exist.
pub async fn delete_doc(
    adapter: &DbAdapter,
    doctype: &str,
    name: &str,
) -> Result<(), DbError> {
    let table = doctype_to_table(doctype);
    let sql   = format!("DELETE `{table}` WHERE name = $name");
    adapter.execute(&sql, vec![("name".into(), name.into())]).await
}

// -- docstatus transitions ----------------------------------------------------

/// Internal: atomically transition `docstatus` from `from` to `to`.
///
/// Shared implementation eliminating duplication between [`submit_doc`] and [`cancel_doc`].
async fn transition_docstatus(
    adapter: &DbAdapter,
    doctype: &str,
    name: &str,
    from: i64,
    to: i64,
) -> Result<Document, DbError> {
    let table = doctype_to_table(doctype);

    // Read current status.
    let chk_sql = format!("SELECT docstatus FROM `{table}` WHERE name = $name LIMIT 1");
    let chk_row = adapter
        .run_one(&chk_sql, vec![("name".into(), name.into())], doctype, name)
        .await?;

    let current = chk_row.get("docstatus").and_then(Value::as_i64).unwrap_or(0);
    if current != from {
        return Err(DbError::Other(format!(
            "{doctype}/{name}: expected docstatus {from}, got {current}"
        )));
    }

    // Atomic update.
    let upd_sql = format!(
        "UPDATE `{table}` SET docstatus = $to, modified = time::now() \
         WHERE name = $name RETURN AFTER"
    );
    let row = adapter
        .run_one(
            &upd_sql,
            vec![("name".into(), name.into()), ("to".into(), to.into())],
            doctype,
            name,
        )
        .await?;

    value_to_document(row, doctype, name)
}

/// Submit a document: `docstatus` 0 -> 1.  Errors if not in Draft state.
pub async fn submit_doc(
    adapter: &DbAdapter,
    doctype: &str,
    name: &str,
) -> Result<Document, DbError> {
    transition_docstatus(adapter, doctype, name, 0, 1).await
}

/// Cancel a document: `docstatus` 1 -> 2.  Errors if not Submitted.
pub async fn cancel_doc(
    adapter: &DbAdapter,
    doctype: &str,
    name: &str,
) -> Result<Document, DbError> {
    transition_docstatus(adapter, doctype, name, 1, 2).await
}

// -- tests --------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn doctype_to_table_variants() {
        assert_eq!(doctype_to_table("Customer"),    "tabCustomer");
        assert_eq!(doctype_to_table("Sales Order"), "tabSales_Order");
        assert_eq!(doctype_to_table("DocType"),     "tabDocType");
    }
}
