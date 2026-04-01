//! Frappe-compatible HTTP response envelopes.
//!
//! Frappe's wire shapes (must be reproduced exactly):
//!
//! List:          `{"data": [...], "total_count": 42}`
//! Single doc:    `{"data": { "name": "...", "doctype": "...", ... }}`
//! Business error:`HTTP 417  {"exc": "traceback", "exc_type": "ValidationError"}`
//! Success msg:   `{"message": "Saved", "data": null}`
//! Toast notify:  `{"_server_messages": "[\"message here\"]"}`
//!                 ^ note: JSON string inside JSON

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Successful single-document response: `{"data": { ... }}`
#[derive(Debug, Serialize, Deserialize)]
pub struct DocResponse {
    pub data: Value,
}

/// Successful list response: `{"data": [...], "total_count": N}`
#[derive(Debug, Serialize, Deserialize)]
pub struct ListResponse {
    pub data: Vec<Value>,
    pub total_count: usize,
}

/// Success with a message: `{"message": "...", "data": null}`
#[derive(Debug, Serialize, Deserialize)]
pub struct MessageResponse {
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

/// Method response: `{"message": <any JSON value>}`
/// Used by POST /api/method/ where the result can be an array, object, number, etc.
#[derive(Debug, Serialize, Deserialize)]
pub struct MethodResponse {
    pub message: Value,
}

/// Error response body matching Frappe v1 error shape.
/// HTTP status is set separately (typically 417 for ValidationError, 404, etc.)
#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    /// Short exception type name e.g. "ValidationError"
    pub exc_type: String,
    /// Full traceback string (empty in production)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exc: Option<String>,
    /// Human-readable message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    /// Frappe _server_messages is a JSON-encoded string inside JSON.
    /// e.g. `"[\"message here\"]"`
    #[serde(rename = "_server_messages", skip_serializing_if = "Option::is_none")]
    pub server_messages: Option<String>,
}

impl ErrorResponse {
    pub fn new(exc_type: impl Into<String>, message: impl Into<String>) -> Self {
        let msg = message.into();
        // Encode server_messages as Frappe does: JSON array string inside the JSON
        let server_messages = serde_json::to_string(&[&msg]).ok();
        Self {
            exc_type: exc_type.into(),
            exc: None,
            message: Some(msg),
            server_messages,
        }
    }

    pub fn with_traceback(mut self, tb: String) -> Self {
        self.exc = Some(tb);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn doc_response_shape() {
        let r = DocResponse { data: json!({"name": "ACME", "doctype": "Customer"}) };
        let s = serde_json::to_string(&r).unwrap();
        let v: Value = serde_json::from_str(&s).unwrap();
        assert_eq!(v["data"]["name"], json!("ACME"));
    }

    #[test]
    fn list_response_shape() {
        let r = ListResponse {
            data: vec![json!({"name": "SO-0001"}), json!({"name": "SO-0002"})],
            total_count: 2,
        };
        let s = serde_json::to_string(&r).unwrap();
        let v: Value = serde_json::from_str(&s).unwrap();
        assert_eq!(v["total_count"], json!(2));
        assert_eq!(v["data"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn error_response_server_messages_is_json_string_in_json() {
        // Frappe quirk: _server_messages is a JSON-encoded string inside the JSON body.
        let r = ErrorResponse::new("ValidationError", "Amount cannot be negative");
        let s = serde_json::to_string(&r).unwrap();
        let v: Value = serde_json::from_str(&s).unwrap();

        // _server_messages must be a string (not an array)
        assert!(v["_server_messages"].is_string(), "_server_messages must be a JSON string");

        // And that string must itself be a valid JSON array
        let inner: Value =
            serde_json::from_str(v["_server_messages"].as_str().unwrap()).unwrap();
        assert!(inner.is_array());
        assert_eq!(inner[0], json!("Amount cannot be negative"));
    }

    #[test]
    fn message_response_shape() {
        let r = MessageResponse { message: "Saved".into(), data: None };
        let s = serde_json::to_string(&r).unwrap();
        let v: Value = serde_json::from_str(&s).unwrap();
        assert_eq!(v["message"], json!("Saved"));
        // `data: None` with skip_serializing_if means the key is absent
        assert!(v.get("data").is_none());
    }
}
