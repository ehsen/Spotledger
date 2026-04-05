use indexmap::IndexMap;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;

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
