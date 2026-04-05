//! Plugin memory marshaling — read and write plugin state across the WASM boundary.
//!
//! Plugins run in sandboxed WASM memory. Host functions need to:
//! 1. Read strings/JSON from plugin memory (via pointers)
//! 2. Deserialize into Rust types
//! 3. Process in host
//! 4. Serialize response
//! 5. Write back to plugin memory
//! 6. Return offset to the host
//!
//! Phase 2.5: Stubs for memory operations.
//! Phase 3: Will integrate extism Memory API for actual marshaling

use serde_json::Value;
use std::str;

/// Result from memory operations.
pub type MemoryResult<T> = Result<T, MemoryError>;

/// Error types for memory operations.
#[derive(Debug)]
pub enum MemoryError {
    InvalidPointer { ptr: u64, len: u32 },
    InvalidUtf8,
    SerializationError(String),
    DeserializationError(String),
}

impl std::fmt::Display for MemoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MemoryError::InvalidPointer { ptr, len } => {
                write!(f, "Invalid pointer {}: length {}", ptr, len)
            }
            MemoryError::InvalidUtf8 => write!(f, "Invalid UTF-8 in plugin memory"),
            MemoryError::SerializationError(e) => write!(f, "Serialization error: {}", e),
            MemoryError::DeserializationError(e) => write!(f, "Deserialization error: {}", e),
        }
    }
}

impl std::error::Error for MemoryError {}

/// Read a null-terminated string from plugin memory.
/// Phase 3: Will read from actual plugin memory via extism Memory API
pub fn read_cstring(_ptr: u64, _len: u32) -> MemoryResult<String> {
    // TODO Phase 3: Implement actual memory read
    // For now: return stub
    Ok("stub_string".to_string())
}

/// Read a JSON value from plugin memory.
/// Phase 3: Will read from actual plugin memory via extism Memory API
pub fn read_json(_ptr: u64, _len: u32) -> MemoryResult<Value> {
    // TODO Phase 3: Implement actual memory read and JSON parsing
    Ok(serde_json::json!({}))
}

/// Write a string to plugin memory.
/// Returns the memory offset where the data was written.
/// Phase 3: Will write to actual plugin memory
pub fn write_string(_data: &str) -> MemoryResult<u64> {
    // TODO Phase 3: Implement proper memory allocation in plugin
    // For now, use a simple approach: return a stub offset
    Ok(0)
}

/// Write a JSON value to plugin memory.
/// Returns the memory offset where the data was written.
/// Phase 3: Will write to actual plugin memory
pub fn write_json(_value: &Value) -> MemoryResult<u64> {
    // TODO Phase 3: Implement actual JSON serialization and memory write
    Ok(0)
}

/// Serialize a Rust value to JSON and get ready to write to plugin memory.
pub fn serialize<T: serde::Serialize>(value: &T) -> MemoryResult<String> {
    serde_json::to_string(value)
        .map_err(|e| MemoryError::SerializationError(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_error_display() {
        let err = MemoryError::InvalidUtf8;
        assert_eq!(err.to_string(), "Invalid UTF-8 in plugin memory");
    }

    #[test]
    fn test_serialize_json() {
        let obj = serde_json::json!({ "name": "Test", "value": 42 });
        let result = serialize(&obj);
        assert!(result.is_ok());
        let s = result.unwrap();
        assert!(s.contains("name"));
    }
}
