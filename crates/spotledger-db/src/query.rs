//! Typed SQL fragment builders.
//!
//! Instead of returning `(String, Vec<(String, Value)>)` tuples from helper
//! functions, we use named structs so call sites are self-documenting and the
//! compiler can enforce correct pairing of SQL + bindings.

use serde_json::Value;

// ── Shared skip list ──────────────────────────────────────────────────────────

/// System fields managed by the framework — skipped in SET clauses.
const SKIP_FIELDS: &[&str] = &["name", "doctype", "modified", "creation", "id"];

// ── WhereClause ───────────────────────────────────────────────────────────────

/// A compiled WHERE clause with its parameter bindings.
///
/// ```text
/// // Simple equality:  {"status": "Active"} → WHERE `status` = $f_status
/// // With operator:    {"total": [">", 100]} → WHERE `total` > $f_total
/// ```
pub struct WhereClause {
    sql:      String,
    bindings: Vec<(String, Value)>,
}

impl WhereClause {
    /// Build from an optional filters `Value`.
    ///
    /// `None` or a non-object value produces an empty clause.
    pub fn from_filters(filters: Option<&Value>) -> Self {
        let Some(Value::Object(map)) = filters else {
            return Self { sql: String::new(), bindings: vec![] };
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
            return Self { sql: String::new(), bindings: vec![] };
        }

        Self {
            sql:      format!(" WHERE {}", conditions.join(" AND ")),
            bindings,
        }
    }

    /// The WHERE fragment (empty string when there are no filters).
    pub fn as_sql(&self)   -> &str              { &self.sql }
    /// The parameter bindings to attach to the query.
    pub fn bindings(&self) -> &[(String, Value)] { &self.bindings }
    /// `true` when no filters were provided.
    pub fn is_empty(&self) -> bool              { self.sql.is_empty() }
}

// ── SetClause ─────────────────────────────────────────────────────────────────

/// A compiled SET clause with its parameter bindings.
///
/// System fields (`name`, `doctype`, `modified`, `creation`, `id`) are
/// excluded — the DB layer inserts them explicitly.  Array fields (child
/// tables) are included so they are embedded directly in the parent record.
pub struct SetClause {
    sql:      String,
    bindings: Vec<(String, Value)>,
}

impl SetClause {
    /// Build from the full document field map.
    pub fn from_fields(fields: &Value) -> Self {
        let Value::Object(map) = fields else {
            return Self { sql: "nothing = NONE".into(), bindings: vec![] };
        };

        let mut parts    = Vec::new();
        let mut bindings = Vec::new();

        for (k, v) in map {
            if SKIP_FIELDS.contains(&k.as_str()) {
                continue;
            }
            // Skip JSON null — SurrealDB v3 SCHEMAFULL rejects null for non-nullable
            // fields (e.g. TYPE int). Omitting the field from SET lets DEFAULT apply
            // for new records and preserves the existing value for updates.
            if v.is_null() {
                continue;
            }
            let key = format!("f_{k}");
            parts.push(format!("`{k}` = ${key}"));
            bindings.push((key, v.clone()));
        }

        if parts.is_empty() {
            return Self { sql: "nothing = NONE".into(), bindings: vec![] };
        }

        Self { sql: parts.join(", "), bindings }
    }

    /// The `field = $param, ...` fragment (never empty — falls back to `nothing = NONE`).
    pub fn as_sql(&self)   -> &str              { &self.sql }
    /// The parameter bindings to attach to the query.
    pub fn bindings(&self) -> &[(String, Value)] { &self.bindings }
}

// ── tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn where_empty_on_none() {
        let w = WhereClause::from_filters(None);
        assert!(w.is_empty());
        assert!(w.bindings().is_empty());
    }

    #[test]
    fn where_simple_equality() {
        let filters = json!({"status": "Active"});
        let w = WhereClause::from_filters(Some(&filters));
        assert!(w.as_sql().contains("`status` = $f_status"), "sql: {}", w.as_sql());
        assert_eq!(w.bindings().len(), 1);
        assert_eq!(w.bindings()[0].1, json!("Active"));
    }

    #[test]
    fn where_operator_shape() {
        let filters = json!({"grand_total": [">", 1000]});
        let w = WhereClause::from_filters(Some(&filters));
        assert!(w.as_sql().contains("`grand_total` > $f_grand_total"), "sql: {}", w.as_sql());
        assert_eq!(w.bindings()[0].1, json!(1000));
    }

    #[test]
    fn where_multiple_fields() {
        let filters = json!({"status": "Active", "enabled": 1});
        let w = WhereClause::from_filters(Some(&filters));
        assert!(w.as_sql().contains("WHERE"), "sql: {}", w.as_sql());
        assert_eq!(w.bindings().len(), 2);
    }

    #[test]
    fn set_skips_system_fields() {
        let fields = json!({
            "name": "SO-0001",
            "doctype": "Sales Order",
            "customer": "Acme",
            "total": 100,
            "items": [{"item_code": "X", "qty": 1}]
        });
        let s = SetClause::from_fields(&fields);
        assert!(!s.as_sql().contains("`name`"),    "should skip name");
        assert!(!s.as_sql().contains("`doctype`"), "should skip doctype");
        // arrays (child tables) are now embedded — must appear in SET
        assert!(s.as_sql().contains("`items`"),    "child table array must be in SET");
        let keys: Vec<&str> = s.bindings().iter().map(|(k, _)| k.as_str()).collect();
        assert!(keys.contains(&"f_customer"));
        assert!(keys.contains(&"f_total"));
        assert!(keys.contains(&"f_items"));
    }

    #[test]
    fn set_empty_returns_sentinel() {
        let fields = json!({"name": "X", "doctype": "Test"});
        let s = SetClause::from_fields(&fields);
        assert_eq!(s.as_sql(), "nothing = NONE");
        assert!(s.bindings().is_empty());
    }
}
