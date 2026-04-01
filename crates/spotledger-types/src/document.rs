use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// A Frappe document as returned by get_doc / saved to SurrealDB.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Document {
    pub doctype: String,
    pub name: String,
    #[serde(flatten)]
    pub fields: HashMap<String, Value>,
}

impl Document {
    /// Returns a cloned Value for the field (doctype/name aliases handled).
    pub fn get_value(&self, key: &str) -> Option<Value> {
        match key {
            "doctype" => Some(Value::String(self.doctype.clone())),
            "name" => Some(Value::String(self.name.clone())),
            _ => self.fields.get(key).cloned(),
        }
    }

    pub fn set(&mut self, key: impl Into<String>, value: Value) {
        let key = key.into();
        match key.as_str() {
            "doctype" => {
                if let Value::String(v) = value {
                    self.doctype = v;
                }
            }
            "name" => {
                if let Value::String(v) = value {
                    self.name = v;
                }
            }
            _ => {
                self.fields.insert(key, value);
            }
        }
    }

    /// Flat JSON object matching Frappe's `as_dict()` output.
    pub fn as_dict(&self) -> Value {
        let mut map = serde_json::Map::new();
        map.insert("doctype".into(), Value::String(self.doctype.clone()));
        map.insert("name".into(), Value::String(self.name.clone()));
        for (k, v) in &self.fields {
            map.insert(k.clone(), v.clone());
        }
        Value::Object(map)
    }
}

/// A lightweight list row returned by get_list.
pub type DocRow = HashMap<String, Value>;

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn document_roundtrip() {
        let mut doc = Document {
            doctype: "Customer".into(),
            name: "ACME Corp".into(),
            fields: HashMap::new(),
        };
        doc.set("customer_name", json!("ACME Corp"));
        doc.set("credit_limit", json!(50000.0));

        assert_eq!(doc.get_value("doctype"), Some(json!("Customer")));
        assert_eq!(doc.get_value("name"), Some(json!("ACME Corp")));
        assert_eq!(doc.get_value("credit_limit"), Some(json!(50000.0)));
    }

    #[test]
    fn document_as_dict_flattens() {
        let doc = Document {
            doctype: "Item".into(),
            name: "ITEM-001".into(),
            fields: [("item_name".into(), json!("Test Item"))].into(),
        };
        let dict = doc.as_dict();
        assert_eq!(dict["doctype"], json!("Item"));
        assert_eq!(dict["name"], json!("ITEM-001"));
        assert_eq!(dict["item_name"], json!("Test Item"));
    }
}
