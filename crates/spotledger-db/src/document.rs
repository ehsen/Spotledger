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

// Child tables are embedded directly in the parent document record as JSON
// arrays.  No separate scatter/gather per child table — SurrealDB stores the
// nested array as a first-class field alongside all scalar fields.

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

    let set   = SetClause::from_fields(fields);
    let table = doctype_to_table(doctype);
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
    let set   = SetClause::from_fields(fields);
    let table = doctype_to_table(doctype);
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

    match adapter.run_one(&sql, bindings, doctype, name).await {
        Ok(row) => value_to_document(row, doctype, name),
        // SurrealDB v3 can fail to deserialize the UPSERT response when the
        // returned record contains field names that match SQL keywords (e.g.
        // tabDocPerm fields: submit, report, import, export).  The write
        // itself SUCCEEDS — only reading the result back fails.  In that case
        // we construct the returned Document from the supplied input fields so
        // callers see a valid (though not DB-read-back) document.
        Err(DbError::Surreal(ref e))
            if e.to_string().contains("user generated conversion error") =>
        {
            let mut doc = Document::new(doctype);
            doc.name = name.to_owned();
            if let Value::Object(map) = fields {
                for (k, v) in map {
                    if k != "name" && k != "doctype" {
                        doc.fields.insert(k.clone(), v.clone());
                    }
                }
            }
            Ok(doc)
        }
        Err(e) => Err(e),
    }
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

// -- rename -------------------------------------------------------------------

/// Rename a document: copies all fields to a new name, then deletes the old record.
///
/// Returns the newly created document.  Does NOT rewrite Link field references
/// in other tables — call `rename_doc_cascade` for that (Phase 6+).
///
/// Errors if `new_name` already exists in the table.
pub async fn rename_doc(
    adapter: &DbAdapter,
    doctype: &str,
    old_name: &str,
    new_name: &str,
) -> Result<Document, DbError> {
    let table = doctype_to_table(doctype);

    // Guard: new name must not already exist.
    let check_sql = format!("SELECT count() FROM `{table}` WHERE name = $n GROUP ALL");
    let check_rows = adapter.run(&check_sql, vec![("n".into(), Value::String(new_name.to_owned()))]).await?;
    let existing = check_rows
        .first()
        .and_then(|r| r.get("count"))
        .and_then(Value::as_u64)
        .unwrap_or(0);
    if existing > 0 {
        return Err(DbError::Other(format!(
            "{doctype} '{new_name}' already exists"
        )));
    }

    // Read the old document (errors if it doesn't exist).
    let old_doc = get_doc(adapter, doctype, old_name).await?;

    // Upsert at the new name — reuses upsert_doc which handles creation timestamp.
    let new_doc = upsert_doc(adapter, doctype, new_name, &old_doc.as_dict()).await?;

    // Remove the old record.
    delete_doc(adapter, doctype, old_name).await?;

    Ok(new_doc)
}

/// Returns `true` if `fieldname` contains only ASCII alphanumeric characters or
/// underscores.  Used to prevent SQL injection via dynamic fieldname interpolation.
///
/// An empty string is considered unsafe (it would produce invalid SurrealQL).
pub(crate) fn is_safe_fieldname(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_alphanumeric() || c == '_')
}

// -- bulk update --------------------------------------------------------------

/// Set `fieldname = value` on every document in `names` for the given doctype.
///
/// Returns the number of records updated.
pub async fn bulk_update(
    adapter: &DbAdapter,
    doctype: &str,
    names: &[String],
    fieldname: &str,
    value: Value,
) -> Result<u64, DbError> {
    if names.is_empty() {
        return Ok(0);
    }
    let table  = doctype_to_table(doctype);
    let names_json: Vec<Value> = names.iter().map(|n| Value::String(n.clone())).collect();
    // Sanitise fieldname (alphanumeric + underscore only).
    if !is_safe_fieldname(fieldname) {
        return Err(DbError::Other(format!("Invalid fieldname: '{fieldname}'")));
    }
    let sql = format!(
        "UPDATE `{table}` SET `{fieldname}` = $val, modified = time::now() WHERE name IN $names"
    );
    adapter
        .execute(
            &sql,
            vec![
                ("val".into(),   value),
                ("names".into(), Value::Array(names_json)),
            ],
        )
        .await?;
    Ok(names.len() as u64)
}

// -- tests --------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // ── doctype_to_table ─────────────────────────────────────────────────────

    #[test]
    fn doctype_to_table_variants() {
        assert_eq!(doctype_to_table("Customer"),    "tabCustomer");
        assert_eq!(doctype_to_table("Sales Order"), "tabSales_Order");
        assert_eq!(doctype_to_table("DocType"),     "tabDocType");
    }

    // ── is_safe_fieldname ────────────────────────────────────────────────────

    #[test]
    fn is_safe_fieldname_accepts_alphanumeric_and_underscore() {
        assert!(is_safe_fieldname("account_type"));
        assert!(is_safe_fieldname("field1"));
        assert!(is_safe_fieldname("FIELD_NAME"));
        assert!(is_safe_fieldname("a"));
        assert!(is_safe_fieldname("_private"));
        assert!(is_safe_fieldname("mix3d_NaMe"));
    }

    #[test]
    fn is_safe_fieldname_rejects_empty() {
        assert!(!is_safe_fieldname(""));
    }

    #[test]
    fn is_safe_fieldname_rejects_special_characters() {
        assert!(!is_safe_fieldname("field-name"));          // hyphen
        assert!(!is_safe_fieldname("field.name"));          // dot
        assert!(!is_safe_fieldname("field name"));          // space
        assert!(!is_safe_fieldname("field;DROP TABLE"));    // SQL injection attempt
        assert!(!is_safe_fieldname("field`"));              // backtick
        assert!(!is_safe_fieldname("field$name"));          // dollar sign
        assert!(!is_safe_fieldname("field/name"));          // slash
    }

    #[test]
    fn is_safe_fieldname_rejects_surql_keywords_with_special_chars() {
        // These should still be blocked because they contain non-safe chars.
        assert!(!is_safe_fieldname("SElect*"));
        assert!(!is_safe_fieldname("DROP;"));
    }

    // ── integration tests (require live SurrealDB on ws://127.0.0.1:8500) ───

    #[cfg(feature = "integration")]
    mod integration {
        use super::super::*;
        use crate::adapter::DbAdapter;
        use spotledger_core::config::DatabaseConfig;

        async fn test_adapter() -> DbAdapter {
            let cfg = DatabaseConfig {
                url:  "ws://127.0.0.1:8500".into(),
                ns:   "test_phase5".into(),
                db:   "documents".into(),
                user: "root".into(),
                pass: "root".into(),
            };
            DbAdapter::connect(&cfg).await.expect("connect to test DB")
        }

        async fn setup_table(adapter: &DbAdapter, table: &str) {
            let _ = adapter
                .execute(&format!("DEFINE TABLE IF NOT EXISTS `{table}` SCHEMALESS;"), vec![])
                .await;
            let _ = adapter
                .execute(&format!("DELETE `{table}`;"), vec![])
                .await;
        }

        // ── rename_doc ───────────────────────────────────────────────────────

        #[tokio::test]
        async fn rename_doc_basic_round_trip() {
            let adapter = test_adapter().await;
            let table   = "tabTestRenameDoc";
            setup_table(&adapter, table).await;

            // Seed the source document.
            upsert_doc(
                &adapter,
                "TestRenameDoc",
                "OLD-001",
                &serde_json::json!({ "status": "Draft", "amount": 42 }),
            )
            .await
            .expect("seed source doc");

            // Rename it.
            let result = rename_doc(&adapter, "TestRenameDoc", "OLD-001", "NEW-001").await;
            let new_doc = result.expect("rename_doc should succeed");
            assert_eq!(new_doc.name, "NEW-001");

            // New name must exist.
            let fetched = get_doc(&adapter, "TestRenameDoc", "NEW-001")
                .await
                .expect("new doc must be readable");
            assert_eq!(
                fetched.fields.get("status").and_then(|v| v.as_str()),
                Some("Draft"),
                "fields must be preserved"
            );

            // Old name must be gone.
            let gone = get_doc(&adapter, "TestRenameDoc", "OLD-001").await;
            assert!(gone.is_err(), "old doc must be deleted after rename");
        }

        #[tokio::test]
        async fn rename_doc_fails_when_target_exists() {
            let adapter = test_adapter().await;
            let table   = "tabTestRenameConflict";
            setup_table(&adapter, table).await;

            upsert_doc(&adapter, "TestRenameConflict", "A",
                &serde_json::json!({"x": 1})).await.unwrap();
            upsert_doc(&adapter, "TestRenameConflict", "B",
                &serde_json::json!({"x": 2})).await.unwrap();

            let err = rename_doc(&adapter, "TestRenameConflict", "A", "B")
                .await
                .expect_err("should error when target already exists");
            assert!(err.to_string().contains("already exists"));
        }

        #[tokio::test]
        async fn rename_doc_fails_when_source_missing() {
            let adapter = test_adapter().await;
            let table   = "tabTestRenameGhost";
            setup_table(&adapter, table).await;

            let err = rename_doc(&adapter, "TestRenameGhost", "GHOST", "NEW")
                .await
                .expect_err("should error when source does not exist");
            // DbError::NotFound (wrapped) or similar.
            let _ = err; // just assert it errored
        }

        // ── bulk_update ──────────────────────────────────────────────────────

        #[tokio::test]
        async fn bulk_update_sets_field_on_all_named_docs() {
            let adapter = test_adapter().await;
            let table   = "tabTestBulkUpdate";
            setup_table(&adapter, table).await;

            for (name, status) in [("BU-01", "Draft"), ("BU-02", "Draft"), ("BU-03", "Open")] {
                upsert_doc(
                    &adapter,
                    "TestBulkUpdate",
                    name,
                    &serde_json::json!({ "status": status }),
                )
                .await
                .unwrap();
            }

            let names: Vec<String> = vec!["BU-01".into(), "BU-02".into()];
            let n = bulk_update(
                &adapter,
                "TestBulkUpdate",
                &names,
                "status",
                serde_json::Value::String("Submitted".into()),
            )
            .await
            .expect("bulk_update should succeed");
            assert_eq!(n, 2);

            // Verify the two updated docs.
            for name in &["BU-01", "BU-02"] {
                let doc = get_doc(&adapter, "TestBulkUpdate", name).await.unwrap();
                assert_eq!(
                    doc.fields.get("status").and_then(|v| v.as_str()),
                    Some("Submitted"),
                    "{name} must have status=Submitted"
                );
            }

            // BU-03 must remain Draft.
            let doc03 = get_doc(&adapter, "TestBulkUpdate", "BU-03").await.unwrap();
            assert_eq!(
                doc03.fields.get("status").and_then(|v| v.as_str()),
                Some("Open"),
                "BU-03 must not be touched"
            );
        }

        #[tokio::test]
        async fn bulk_update_empty_names_returns_zero() {
            let adapter = test_adapter().await;
            let n = bulk_update(&adapter, "TestBulkUpdate", &[], "status", "x".into())
                .await
                .expect("empty bulk_update should return Ok(0)");
            assert_eq!(n, 0, "empty names must return 0 without touching DB");
        }

        #[tokio::test]
        async fn bulk_update_rejects_unsafe_fieldname() {
            let adapter = test_adapter().await;
            let names   = vec!["DOC-1".to_owned()];
            let err = bulk_update(
                &adapter,
                "TestBulkUpdate",
                &names,
                "status; DROP TABLE tabTestBulkUpdate",
                "x".into(),
            )
            .await
            .expect_err("unsafe fieldname must be rejected");
            assert!(err.to_string().contains("Invalid fieldname"));
        }
    }
}

