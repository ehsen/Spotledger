//! JSON parsing utilities — thin wrappers around `serde_json` with
//! Frappe-compatible error handling.
//!
//! Functions:
//! - `parse_json(s)` — parse a JSON string to `serde_json::Value`
//! - `as_json(v)` — serialise any `serde::Serialize` to a JSON string
//! - `as_json_pretty(v)` — pretty-print JSON
//! - `json_get_str(obj, key)` — extract a string field from a JSON object
//! - `json_get_f64(obj, key)` — extract a float field from a JSON object
//! - `json_get_bool(obj, key)` — extract a boolean field from a JSON object
//! - `json_merge(base, patch)` — recursively merge two JSON objects

use serde::Serialize;
use serde_json::{Map, Value};

use crate::error::CoreError;

// ── Parsing ────────────────────────────────────────────────────────────────────

/// Parse a JSON string to `serde_json::Value`.
///
/// Returns `CoreError::Serialization` on invalid JSON.
///
/// Mirrors Frappe's `frappe.parse_json(s)`.
///
/// # Examples
/// ```
/// use spotledger_core::utils::json::parse_json;
/// let v = parse_json(r#"{"key": "value"}"#).unwrap();
/// assert_eq!(v["key"], "value");
/// ```
pub fn parse_json(s: &str) -> Result<Value, CoreError> {
    serde_json::from_str(s).map_err(CoreError::Serialization)
}

/// Parse a JSON string, returning `Value::Null` on failure instead of an error.
///
/// Useful in contexts where you want a safe fallback.
pub fn parse_json_or_null(s: &str) -> Value {
    serde_json::from_str(s).unwrap_or(Value::Null)
}

// ── Serialisation ─────────────────────────────────────────────────────────────

/// Serialise any `serde::Serialize` value to a compact JSON string.
///
/// Mirrors Frappe's `frappe.as_json(obj)`.
///
/// # Examples
/// ```
/// use serde_json::json;
/// use spotledger_core::utils::json::as_json;
/// let s = as_json(&json!({"a": 1})).unwrap();
/// assert_eq!(s, r#"{"a":1}"#);
/// ```
pub fn as_json<T: Serialize>(value: &T) -> Result<String, CoreError> {
    serde_json::to_string(value).map_err(CoreError::Serialization)
}

/// Serialise any `serde::Serialize` value to a pretty-printed JSON string.
pub fn as_json_pretty<T: Serialize>(value: &T) -> Result<String, CoreError> {
    serde_json::to_string_pretty(value).map_err(CoreError::Serialization)
}

/// Serialise to `Vec<u8>` (compact). Used for WASM ABI message passing.
pub fn as_json_bytes<T: Serialize>(value: &T) -> Result<Vec<u8>, CoreError> {
    serde_json::to_vec(value).map_err(CoreError::Serialization)
}

// ── Field extraction helpers ──────────────────────────────────────────────────

/// Extract a `&str` value from a JSON object at `key`.
/// Returns `None` if the key is missing or not a string.
pub fn json_get_str<'a>(obj: &'a Value, key: &str) -> Option<&'a str> {
    obj.get(key).and_then(Value::as_str)
}

/// Extract an owned `String` from a JSON object at `key`.
pub fn json_get_string(obj: &Value, key: &str) -> Option<String> {
    obj.get(key).and_then(Value::as_str).map(str::to_owned)
}

/// Extract a `f64` from a JSON object at `key`.
/// Also accepts JSON strings that parse as numbers.
pub fn json_get_f64(obj: &Value, key: &str) -> Option<f64> {
    obj.get(key).and_then(|v| {
        v.as_f64()
            .or_else(|| v.as_str().and_then(|s| s.replace(',', "").parse().ok()))
    })
}

/// Extract an `i64` from a JSON object at `key`.
pub fn json_get_i64(obj: &Value, key: &str) -> Option<i64> {
    obj.get(key).and_then(|v| {
        v.as_i64()
            .or_else(|| v.as_f64().map(|f| f as i64))
            .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
    })
}

/// Extract a `bool` from a JSON object at `key`.
/// Also recognises string `"true"/"1"` and `"false"/"0"`.
pub fn json_get_bool(obj: &Value, key: &str) -> Option<bool> {
    obj.get(key).and_then(|v| {
        v.as_bool().or_else(|| {
            v.as_str().and_then(|s| match s.to_lowercase().as_str() {
                "true" | "1" | "yes" => Some(true),
                "false" | "0" | "no" => Some(false),
                _ => None,
            })
        })
    })
}

// ── Merging ────────────────────────────────────────────────────────────────────

/// Recursively merge `patch` into a clone of `base`.
///
/// For each key in `patch`:
/// - If both `base[key]` and `patch[key]` are objects, they are merged recursively.
/// - Otherwise `patch[key]` overwrites `base[key]`.
///
/// Similar to `Object.assign` but recursive.
pub fn json_merge(base: &Value, patch: &Value) -> Value {
    match (base, patch) {
        (Value::Object(base_map), Value::Object(patch_map)) => {
            let mut merged: Map<String, Value> = base_map.clone();
            for (k, v) in patch_map {
                let entry = merged.entry(k.clone()).or_insert(Value::Null);
                *entry = json_merge(entry, v);
            }
            Value::Object(merged)
        }
        (_, patch) => patch.clone(),
    }
}

// ── Null / empty checks ────────────────────────────────────────────────────────

/// Return `true` if `value` is `null`, an empty string, an empty array, or an
/// empty object.
///
/// Mirrors Frappe's falsy-value checks for document fields.
pub fn is_empty_value(value: &Value) -> bool {
    match value {
        Value::Null => true,
        Value::String(s) => s.is_empty(),
        Value::Array(a) => a.is_empty(),
        Value::Object(o) => o.is_empty(),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_parse_json() {
        let v = parse_json(r#"{"key": "value", "n": 42}"#).unwrap();
        assert_eq!(v["key"], "value");
        assert_eq!(v["n"], 42);
    }

    #[test]
    fn test_parse_json_error() {
        assert!(parse_json("{invalid}").is_err());
    }

    #[test]
    fn test_as_json() {
        let s = as_json(&json!({"a": 1})).unwrap();
        assert_eq!(s, r#"{"a":1}"#);
    }

    #[test]
    fn test_json_get_str() {
        let obj = json!({"name": "Alice", "age": 30});
        assert_eq!(json_get_str(&obj, "name"), Some("Alice"));
        assert_eq!(json_get_str(&obj, "age"), None); // not a string
        assert_eq!(json_get_str(&obj, "missing"), None);
    }

    #[test]
    fn test_json_get_f64() {
        let obj = json!({"price": 9.99, "qty": "5"});
        assert_eq!(json_get_f64(&obj, "price"), Some(9.99));
        assert_eq!(json_get_f64(&obj, "qty"), Some(5.0)); // coerced from string
    }

    #[test]
    fn test_json_get_bool() {
        let obj = json!({"flag": true, "str_flag": "true", "zero": "0"});
        assert_eq!(json_get_bool(&obj, "flag"), Some(true));
        assert_eq!(json_get_bool(&obj, "str_flag"), Some(true));
        assert_eq!(json_get_bool(&obj, "zero"), Some(false));
    }

    #[test]
    fn test_json_merge() {
        let base = json!({"a": 1, "b": {"x": 10, "y": 20}});
        let patch = json!({"b": {"y": 99, "z": 30}, "c": 3});
        let merged = json_merge(&base, &patch);
        assert_eq!(merged["a"], 1);
        assert_eq!(merged["b"]["x"], 10);
        assert_eq!(merged["b"]["y"], 99); // overwritten
        assert_eq!(merged["b"]["z"], 30); // new key
        assert_eq!(merged["c"], 3);       // new top-level
    }

    #[test]
    fn test_is_empty_value() {
        assert!(is_empty_value(&Value::Null));
        assert!(is_empty_value(&json!("")));
        assert!(is_empty_value(&json!([])));
        assert!(is_empty_value(&json!({})));
        assert!(!is_empty_value(&json!(0)));
        assert!(!is_empty_value(&json!("hello")));
    }
}
