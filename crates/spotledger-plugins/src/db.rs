//! Database integration for plugin host functions.
//!
//! Phase 2.5: Provides wired-up access to document operations.
//! Phase 3: Will be fully integrated with spotledger-db.

use serde_json::{json, Value};
use tracing::debug;

/// Document operation results.
pub type DbResult<T> = Result<T, DbError>;

#[derive(Debug)]
pub enum DbError {
    NotFound(String),
    ValidationError(String),
    PermissionDenied,
    DatabaseError(String),
}

impl std::fmt::Display for DbError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DbError::NotFound(msg) => write!(f, "Not found: {}", msg),
            DbError::ValidationError(msg) => write!(f, "Validation error: {}", msg),
            DbError::PermissionDenied => write!(f, "Permission denied"),
            DbError::DatabaseError(msg) => write!(f, "Database error: {}", msg),
        }
    }
}

impl std::error::Error for DbError {}

/// Stub database adapter for plugin operations.
/// Phase 3: Will be replaced with actual spotledger-db adapter.
pub struct DbAdapter {
    // Phase 2.5: placeholder
    _phantom: std::marker::PhantomData<()>,
}

impl DbAdapter {
    pub fn new() -> Self {
        Self {
            _phantom: std::marker::PhantomData,
        }
    }

    /// Get a document by name.
    pub fn get_doc(&self, doctype: &str, name: &str) -> DbResult<Value> {
        debug!("get_doc({}, {})", doctype, name);
        
        // Phase 2.5: Return stub document
        Ok(json!({
            "doctype": doctype,
            "name": name,
            "creation": "2026-01-01T00:00:00",
            "modified": "2026-01-01T00:00:00",
            "owner": "Administrator"
        }))
    }

    /// Save a document.
    pub fn save_doc(&self, doctype: &str, _name: &str, doc: &Value) -> DbResult<String> {
        debug!("save_doc({}, {:?})", doctype, doc);
        
        // Phase 2.5: Return stub success
        Ok(format!("{}_{}", doctype, "1"))
    }

    /// Delete a document.
    pub fn delete_doc(&self, doctype: &str, name: &str) -> DbResult<()> {
        debug!("delete_doc({}, {})", doctype, name);
        
        // Phase 2.5: Return stub success
        Ok(())
    }

    /// Get a single field value.
    pub fn get_value(&self, doctype: &str, name: &str, field: &str) -> DbResult<Value> {
        debug!("get_value({}, {}, {})", doctype, name, field);
        
        // Phase 2.5: Return stub value
        Ok(Value::Null)
    }

    /// Set a single field value.
    pub fn set_value(&self, doctype: &str, name: &str, field: &str, value: &Value) -> DbResult<()> {
        debug!("set_value({}, {}, {}, {:?})", doctype, name, field, value);
        
        // Phase 2.5: Return stub success
        Ok(())
    }

    /// Check permission for an action on a document.
    pub fn has_permission(&self, _user: &str, _action: &str, _doctype: &str, _name: &str) -> DbResult<bool> {
        // Phase 2.5: Grant all permissions for now
        Ok(true)
    }
}

impl Default for DbAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_db_error_display() {
        let err = DbError::NotFound("TestDoc".to_string());
        assert_eq!(err.to_string(), "Not found: TestDoc");
    }

    #[test]
    fn test_db_adapter_get_doc() {
        let adapter = DbAdapter::new();
        let result = adapter.get_doc("User", "Administrator");
        assert!(result.is_ok());
        let doc = result.unwrap();
        assert_eq!(doc["doctype"], "User");
        assert_eq!(doc["name"], "Administrator");
    }
}
