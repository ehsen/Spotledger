//! Frappe-compatible HTTP response envelopes.
//!
//! Wire shapes must be reproduced exactly:
//!   List:          `{"data": [...], "total_count": 42}`
//!   Single doc:    `{"data": { "name": "...", "doctype": "...", ... }}`
//!   Business error:`HTTP 417  {"exc": "traceback", "exc_type": "ValidationError"}`
//!   Success msg:   `{"message": "Saved", "data": null}`

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Serialize, Deserialize)]
pub struct DocResponse {
    pub data: Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ListResponse {
    pub data: Vec<Value>,
    pub total_count: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MessageResponse {
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

/// Method response: `{"message": <any JSON value>}`
#[derive(Debug, Serialize, Deserialize)]
pub struct MethodResponse {
    pub message: Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub exc_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exc: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(rename = "_server_messages", skip_serializing_if = "Option::is_none")]
    pub server_messages: Option<String>,
}

impl ErrorResponse {
    pub fn new(exc_type: impl Into<String>, message: impl Into<String>) -> Self {
        let msg = message.into();
        let inner_obj = serde_json::json!({"message": msg, "indicator": "red", "title": "Error"});
        let inner_str = serde_json::to_string(&inner_obj).unwrap_or_default();
        let server_messages = serde_json::to_string(&[inner_str]).ok();
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
