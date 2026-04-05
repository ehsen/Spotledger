use indexmap::IndexMap;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;

use crate::meta::{DocTypeMeta, FieldType};

// ── DocStatus ─────────────────────────────────────────────────────────────────

/// Frappe document lifecycle status.
///
/// Stored as an integer in the DB (`0 / 1 / 2`).  Using a typed enum rather
/// than a bare integer makes state transitions explicit and exhaustive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DocStatus {
    #[default]
    Draft     = 0,
    Submitted = 1,
    Cancelled = 2,
}

impl From<i64> for DocStatus {
    fn from(v: i64) -> Self {
        match v {
            1 => Self::Submitted,
            2 => Self::Cancelled,
            _ => Self::Draft,
        }
    }
}

impl From<DocStatus> for i64 {
    fn from(s: DocStatus) -> Self { s as i64 }
}

impl Serialize for DocStatus {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_i64(*self as i64)
    }
}

impl<'de> Deserialize<'de> for DocStatus {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let v: Option<i64> = Option::deserialize(d)?;
        Ok(DocStatus::from(v.unwrap_or(0)))
    }
}

// ── Document ──────────────────────────────────────────────────────────────────

/// The base document — every DocType carries one of these.
///
/// `fields` uses [`IndexMap`] to preserve insertion order — Frappe's REST API
/// returns fields in schema-definition order, and tests rely on that.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Document {
    pub doctype: String,
    pub name:    String,
    #[serde(flatten)]
    pub fields: IndexMap<String, Value>,
}

impl Document {
    pub fn new(doctype: impl Into<String>) -> Self {
        Self { doctype: doctype.into(), ..Default::default() }
    }

    pub fn is_new(&self) -> bool {
        self.fields.get("creation").map(|v| v.is_null()).unwrap_or(true)
    }

    pub fn docstatus(&self) -> DocStatus {
        self.fields
            .get("docstatus")
            .and_then(Value::as_i64)
            .map(DocStatus::from)
            .unwrap_or_default()
    }

    pub fn is_submitted(&self) -> bool { self.docstatus() == DocStatus::Submitted }
    pub fn is_cancelled(&self) -> bool { self.docstatus() == DocStatus::Cancelled }

    pub fn owner(&self) -> &str {
        self.fields.get("owner").and_then(Value::as_str).unwrap_or_default()
    }

    pub fn modified_by(&self) -> &str {
        self.fields.get("modified_by").and_then(Value::as_str).unwrap_or_default()
    }

    pub fn get(&self, key: &str) -> Option<&Value> {
        match key {
            "doctype" | "name" => None,
            _ => self.fields.get(key),
        }
    }

    pub fn set(&mut self, key: impl Into<String>, value: impl Into<Value>) {
        let key   = key.into();
        let value = value.into();
        match key.as_str() {
            "doctype" => { if let Value::String(v) = value { self.doctype = v; } }
            "name"    => { if let Value::String(v) = value { self.name = v; } }
            _         => { self.fields.insert(key, value); }
        }
    }

    pub fn get_value(&self, key: &str) -> Option<Value> {
        match key {
            "doctype" => Some(Value::String(self.doctype.clone())),
            "name"    => Some(Value::String(self.name.clone())),
            _         => self.fields.get(key).cloned(),
        }
    }

    pub fn get_str(&self, key: &str) -> Option<&str> {
        match key {
            "doctype" => Some(&self.doctype),
            "name"    => Some(&self.name),
            _         => self.fields.get(key).and_then(Value::as_str),
        }
    }

    pub fn get_f64(&self, key: &str) -> f64 {
        self.fields.get(key).and_then(Value::as_f64).unwrap_or_default()
    }

    pub fn get_children(&self, fieldname: &str) -> Option<&Value> {
        self.fields.get(fieldname).filter(|v| v.is_array())
    }

    /// Append a child row to a child-table field, auto-assigning a 1-based `idx`.
    ///
    /// `row` must be a JSON object.  If the field does not yet exist it is
    /// initialised to an empty array first.
    pub fn append_child(&mut self, fieldname: &str, mut row: Value) {
        let arr = self
            .fields
            .entry(fieldname.to_owned())
            .or_insert_with(|| Value::Array(vec![]));
        if let Value::Array(v) = arr {
            let idx = (v.len() + 1) as i64;
            if let Value::Object(ref mut m) = row {
                m.insert("idx".to_owned(), idx.into());
            }
            v.push(row);
        }
    }

    /// Extend a child-table field with multiple rows, each assigned a sequential `idx`.
    pub fn extend_children(
        &mut self,
        fieldname: &str,
        rows: impl IntoIterator<Item = Value>,
    ) {
        for row in rows {
            self.append_child(fieldname, row);
        }
    }

    /// Remove the child row at 1-based position `idx` and resequence the remaining rows.
    ///
    /// No-op when `idx` is out of range.
    pub fn remove_child_at(&mut self, fieldname: &str, idx: usize) {
        if let Some(Value::Array(v)) = self.fields.get_mut(fieldname) {
            if idx > 0 && idx <= v.len() {
                v.remove(idx - 1);
                for (i, row) in v.iter_mut().enumerate() {
                    if let Value::Object(ref mut m) = row {
                        m.insert("idx".to_owned(), ((i + 1) as i64).into());
                    }
                }
            }
        }
    }

    /// Set `key` to `value` only when the field is currently absent or null.
    pub fn update_if_missing(&mut self, key: &str, value: Value) {
        let current = match key {
            "doctype" => Some(Value::String(self.doctype.clone())),
            "name"    => Some(Value::String(self.name.clone())),
            _         => self.fields.get(key).cloned(),
        };
        if matches!(current, None | Some(Value::Null)) {
            self.set(key, value);
        }
    }

    /// Remove a field entirely.  No-op for `doctype` and `name`.
    pub fn delete_key(&mut self, key: &str) {
        match key {
            "doctype" | "name" => {}
            _ => { self.fields.shift_remove(key); }
        }
    }

    /// Flat JSON object matching Frappe's `as_dict()` output.
    pub fn as_dict(&self) -> Value {
        let mut map = serde_json::Map::new();
        map.insert("doctype".into(), Value::String(self.doctype.clone()));
        map.insert("name".into(),    Value::String(self.name.clone()));
        for (k, v) in &self.fields {
            map.insert(k.clone(), v.clone());
        }
        Value::Object(map)
    }

    pub fn to_value(&self) -> serde_json::Result<Value> {
        serde_json::to_value(self)
    }

    /// Build a new blank document with field defaults applied from `meta`.
    ///
    /// Equivalent to Frappe's `frappe.new_doc(doctype)`:
    /// - Sets `docstatus = 0`, `idx = 0`
    /// - Applies `DocField.default_value` for every field
    /// - Marks document as new (`__islocal = 1`)
    pub fn new_with_defaults(meta: &DocTypeMeta) -> Self {
        let mut doc = Self::new(meta.name.clone());
        doc.fields.insert("docstatus".into(), Value::Number(0.into()));
        doc.fields.insert("idx".into(),       Value::Number(0.into()));
        doc.fields.insert("__islocal".into(), Value::Number(1.into()));

        for df in &meta.fields {
            if df.fieldtype.is_layout() { continue; }
            if let Some(ref dv) = df.default_value {
                let value = match df.fieldtype {
                    FieldType::Check | FieldType::Int => {
                        dv.parse::<i64>().map(Value::from).unwrap_or_else(|_| Value::Number(0.into()))
                    }
                    FieldType::Float | FieldType::Currency | FieldType::Percent | FieldType::Rating => {
                        dv.parse::<f64>().map(|f| Value::Number(
                            serde_json::Number::from_f64(f).unwrap_or(serde_json::Number::from(0))
                        )).unwrap_or_else(|_| Value::Number(0.into()))
                    }
                    _ => Value::String(dv.clone()),
                };
                doc.fields.insert(df.fieldname.clone(), value);
            }
        }

        doc
    }

    /// Stamp every child row in every Table field with:
    /// `parent`, `parenttype`, `parentfield`, `docstatus`, and resequence `idx`.
    ///
    /// Must be called before `db_insert` / `db_upsert` so child linking
    /// fields are always consistent, even when the caller omitted them.
    pub fn set_parent_in_children(&mut self) {
        let parent     = self.name.clone();
        let parenttype = self.doctype.clone();

        // Collect fieldnames that hold child arrays to avoid borrow conflict
        let child_fields: Vec<String> = self
            .fields
            .iter()
            .filter_map(|(k, v)| if v.is_array() { Some(k.clone()) } else { None })
            .collect();

        for parentfield in child_fields {
            if let Some(Value::Array(rows)) = self.fields.get_mut(&parentfield) {
                for (i, row) in rows.iter_mut().enumerate() {
                    if let Value::Object(ref mut m) = row {
                        m.insert("parent".into(),      Value::String(parent.clone()));
                        m.insert("parenttype".into(),  Value::String(parenttype.clone()));
                        m.insert("parentfield".into(), Value::String(parentfield.clone()));
                        m.insert("idx".into(),         Value::Number((i as i64 + 1).into()));
                        // Inherit parent docstatus for child rows
                        m.entry("docstatus").or_insert(Value::Number(0.into()));
                    }
                }
            }
        }
    }

    /// Set owner/creation/modified/modified_by and `docstatus = 0` on a new document,
    /// or update modified/modified_by on an existing one.
    ///
    /// `now_iso` must be an ISO-8601 datetime string (e.g. `"2026-04-05T12:00:00Z"`).
    pub fn set_user_and_timestamp(&mut self, user: &str, now_iso: &str, is_new: bool) {
        self.fields.insert("modified".into(),    Value::String(now_iso.to_owned()));
        self.fields.insert("modified_by".into(), Value::String(user.to_owned()));
        if is_new {
            self.fields.insert("owner".into(),    Value::String(user.to_owned()));
            self.fields.insert("creation".into(), Value::String(now_iso.to_owned()));
        }
    }

    /// Coerce numeric fields according to their SurrealDB storage type:
    /// - `Check` → `0` or `1` (i64)
    /// - `Int`   → i64
    /// - `Float / Currency / Percent / Rating` → f64
    ///
    /// Called before validation and before DB write so stored values are clean.
    pub fn fix_numeric_types(&mut self, meta: &DocTypeMeta) {
        for df in &meta.fields {
            let Some(raw) = self.fields.get(&df.fieldname) else { continue };
            let coerced: Option<Value> = match df.fieldtype {
                FieldType::Check => {
                    let v = match raw {
                        Value::Bool(b)   => if *b { 1i64 } else { 0 },
                        Value::Number(n) => if n.as_f64().unwrap_or(0.0) != 0.0 { 1 } else { 0 },
                        Value::String(s) => if s == "1" || s.eq_ignore_ascii_case("true") { 1 } else { 0 },
                        _ => 0,
                    };
                    Some(Value::Number(v.into()))
                }
                FieldType::Int => {
                    let v = match raw {
                        Value::Number(n) => n.as_i64(),
                        Value::String(s) => s.parse::<i64>().ok(),
                        _ => None,
                    };
                    v.map(|n| Value::Number(n.into()))
                }
                FieldType::Float | FieldType::Currency | FieldType::Percent | FieldType::Rating => {
                    let v = match raw {
                        Value::Number(n) => n.as_f64(),
                        Value::String(s) => s.parse::<f64>().ok(),
                        _ => None,
                    };
                    v.and_then(|f| serde_json::Number::from_f64(f).map(Value::Number))
                }
                _ => None,
            };
            if let Some(v) = coerced {
                self.fields.insert(df.fieldname.clone(), v);
            }
        }
    }
}

// ── DocRow ────────────────────────────────────────────────────────────────────

/// A lightweight list row returned by `get_list`.
pub type DocRow = IndexMap<String, Value>;

// ── impl_document! ────────────────────────────────────────────────────────────

/// Convenience macro: generates shortcut methods and `AsRef`/`AsMut` impls.
#[macro_export]
macro_rules! impl_document {
    ($t:ty) => {
        impl $t {
            pub fn name(&self) -> &str { &self.doc.name }
            pub fn is_new(&self) -> bool { self.doc.is_new() }
            pub fn is_submitted(&self) -> bool { self.doc.is_submitted() }
            pub fn is_cancelled(&self) -> bool { self.doc.is_cancelled() }
            pub fn get(&self, field: &str) -> Option<&serde_json::Value> { self.doc.get(field) }
            pub fn set(&mut self, field: &str, value: impl Into<serde_json::Value>) {
                self.doc.set(field, value)
            }
        }
        impl AsRef<$crate::document::Document> for $t {
            fn as_ref(&self) -> &$crate::document::Document { &self.doc }
        }
        impl AsMut<$crate::document::Document> for $t {
            fn as_mut(&mut self) -> &mut $crate::document::Document { &mut self.doc }
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn document_roundtrip() {
        let mut doc = Document::new("Customer");
        doc.name = "ACME Corp".into();
        doc.set("customer_name", json!("ACME Corp"));
        doc.set("credit_limit", json!(50000.0));
        assert_eq!(doc.get_value("doctype"), Some(json!("Customer")));
        assert_eq!(doc.get_value("name"),    Some(json!("ACME Corp")));
        assert_eq!(doc.get_value("credit_limit"), Some(json!(50000.0)));
    }

    #[test]
    fn docstatus_typed_access() {
        let mut doc = Document::new("Sales Order");
        assert_eq!(doc.docstatus(), DocStatus::Draft);
        assert!(doc.is_new());
        doc.set("docstatus", json!(1));
        assert_eq!(doc.docstatus(), DocStatus::Submitted);
        doc.set("docstatus", json!(2));
        assert!(doc.is_cancelled());
    }

    #[test]
    fn field_order_preserved() {
        let mut doc = Document::new("Test");
        doc.set("zzz", json!(3));
        doc.set("aaa", json!(1));
        doc.set("mmm", json!(2));
        let keys: Vec<&str> = doc.fields.keys().map(String::as_str).collect();
        assert_eq!(keys, &["zzz", "aaa", "mmm"]);
    }
}
