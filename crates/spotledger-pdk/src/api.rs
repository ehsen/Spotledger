//! Guest-side document operation wrappers for SpotledgerCore plugins.
//!
//! Plugin authors use these safe wrappers instead of raw `extern "C"` calls.
//! All functions communicate with the host via extism's memory protocol:
//! data is serialized to JSON, allocated in extism memory, and the handle
//! (i64 offset) is passed to the host.
//!
//! ## Protocol
//! - **Inputs**: Strings and byte slices are allocated in extism memory.
//!   The host reads them via `CurrentPlugin::memory_str()`.
//! - **Outputs**: The host allocates the response in extism memory and returns
//!   the handle. Plugins read it via the returned pointer.
//!
//! ## Target
//! This module compiles for both `wasm32` (real extism calls) and the host
//! platform (stub implementations used in unit tests).

#[cfg(target_arch = "wasm32")]
mod raw {
    /// All SpotledgerCore host functions live under `extism:host/user`.
    /// Strings and byte slices are passed as i64 extism memory handles.
    #[link(wasm_import_module = "extism:host/user")]
    extern "C" {
        pub fn sl_log(level: i64, msg: i64);
        pub fn sl_exists(doctype: i64, name: i64) -> i64;
        pub fn sl_get_doc(doctype: i64, name: i64) -> i64;
        pub fn sl_save_doc(payload: i64) -> i64;
        pub fn sl_delete_doc(doctype: i64, name: i64);
        pub fn sl_get_value(doctype: i64, name: i64, field: i64) -> i64;
        pub fn sl_set_value(doctype: i64, name: i64, field: i64, val: i64);
        pub fn sl_throw(message: i64);
        pub fn sl_has_permission(doctype: i64, ptype: i64, name: i64) -> i64;
    }
}

// ── Memory helpers (WASM only) ─────────────────────────────────────────────────

/// Allocate a string in extism memory and return its handle.
/// This uses the extism env `alloc` / `store_u8` system.
#[cfg(target_arch = "wasm32")]
pub(crate) fn alloc_str(s: &str) -> i64 {
    // extism-pdk's Memory::from_bytes pattern:
    // The extism runtime provides extism_alloc + extism_store_u8 via "extism:env".
    use core::mem;
    let bytes = s.as_bytes();
    let handle = unsafe { extism_alloc(bytes.len() as i64) };
    for (i, &b) in bytes.iter().enumerate() {
        unsafe { extism_store_u8(handle + i as i64, b as i64) };
    }
    handle
}

#[cfg(target_arch = "wasm32")]
pub(crate) fn read_str_handle(handle: i64) -> String {
    // Read bytes from extism memory handle
    let len = unsafe { extism_length(handle) } as usize;
    let mut bytes = vec![0u8; len];
    for i in 0..len {
        bytes[i] = unsafe { extism_load_u8(handle + i as i64) } as u8;
    }
    String::from_utf8_lossy(&bytes).into_owned()
}

#[cfg(target_arch = "wasm32")]
pub(crate) fn read_bytes_handle(handle: i64) -> Vec<u8> {
    let len = unsafe { extism_length(handle) } as usize;
    let mut bytes = vec![0u8; len];
    for i in 0..len {
        bytes[i] = unsafe { extism_load_u8(handle + i as i64) } as u8;
    }
    bytes
}

#[cfg(target_arch = "wasm32")]
mod env_imports {
    #[link(wasm_import_module = "extism:env")]
    extern "C" {
        pub fn extism_alloc(n: i64) -> i64;
        pub fn extism_length(handle: i64) -> i64;
        pub fn extism_store_u8(offset: i64, value: i64);
        pub fn extism_load_u8(offset: i64) -> i64;
    }
}

#[cfg(target_arch = "wasm32")]
use env_imports::*;

// ── Public API ────────────────────────────────────────────────────────────────

/// Write to the host trace log. Level: "debug" | "info" | "warn" | "error".
pub fn sl_log(level: &str, msg: &str) {
    #[cfg(target_arch = "wasm32")]
    unsafe {
        raw::sl_log(alloc_str(level), alloc_str(msg));
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        match level {
            "error" => tracing::error!(source = "plugin", "{}", msg),
            "warn"  => tracing::warn!(source  = "plugin", "{}", msg),
            "info"  => tracing::info!(source  = "plugin", "{}", msg),
            _       => tracing::debug!(source = "plugin", "{}", msg),
        }
    }
}

/// Returns `true` if the document exists in the database.
pub fn sl_exists(doctype: &str, name: &str) -> bool {
    #[cfg(target_arch = "wasm32")]
    unsafe {
        raw::sl_exists(alloc_str(doctype), alloc_str(name)) != 0
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = (doctype, name);
        false // stub
    }
}

/// Fetch a document. Returns its JSON representation or an error.
pub fn sl_get_doc(doctype: &str, name: &str) -> Result<serde_json::Value, String> {
    #[cfg(target_arch = "wasm32")]
    unsafe {
        let handle = raw::sl_get_doc(alloc_str(doctype), alloc_str(name));
        if handle == 0 {
            return Err(format!("{doctype} {name} not found"));
        }
        let bytes = read_bytes_handle(handle);
        serde_json::from_slice(&bytes).map_err(|e| e.to_string())
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        Ok(serde_json::json!({ "doctype": doctype, "name": name }))
    }
}

/// Save a document (runs the full save pipeline on the host).
/// Returns the saved document or an error.
pub fn sl_save_doc(doc: &serde_json::Value) -> Result<serde_json::Value, String> {
    #[cfg(target_arch = "wasm32")]
    unsafe {
        let payload = serde_json::to_vec(doc).map_err(|e| e.to_string())?;
        let payload_handle = alloc_str(&String::from_utf8_lossy(&payload));
        let result_handle = raw::sl_save_doc(payload_handle);
        if result_handle == 0 {
            return Err("Save failed".to_string());
        }
        let bytes = read_bytes_handle(result_handle);
        serde_json::from_slice(&bytes).map_err(|e| e.to_string())
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        Ok(doc.clone())
    }
}

/// Delete a document permanently.
pub fn sl_delete_doc(doctype: &str, name: &str) {
    #[cfg(target_arch = "wasm32")]
    unsafe {
        raw::sl_delete_doc(alloc_str(doctype), alloc_str(name));
    }
    #[cfg(not(target_arch = "wasm32"))]
    let _ = (doctype, name);
}

/// Get a single field value. Returns `None` if the field is null or missing.
pub fn sl_get_value(doctype: &str, name: &str, field: &str) -> Option<serde_json::Value> {
    #[cfg(target_arch = "wasm32")]
    unsafe {
        let h = raw::sl_get_value(alloc_str(doctype), alloc_str(name), alloc_str(field));
        if h == 0 { return None; }
        let bytes = read_bytes_handle(h);
        serde_json::from_slice(&bytes).ok()
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = (doctype, name, field);
        None
    }
}

/// Set a single field directly in DB (no hooks — use `sl_save_doc` for hook dispatch).
pub fn sl_set_value(doctype: &str, name: &str, field: &str, val: &serde_json::Value) {
    #[cfg(target_arch = "wasm32")]
    unsafe {
        let val_json = serde_json::to_vec(val).unwrap_or_default();
        let val_str  = String::from_utf8_lossy(&val_json).into_owned();
        raw::sl_set_value(
            alloc_str(doctype), alloc_str(name),
            alloc_str(field),   alloc_str(&val_str),
        );
    }
    #[cfg(not(target_arch = "wasm32"))]
    let _ = (doctype, name, field, val);
}

/// Raise a validation error that aborts the current operation and shows a
/// message to the desk user. This function does not return.
pub fn sl_throw(message: &str) -> ! {
    #[cfg(target_arch = "wasm32")]
    unsafe {
        raw::sl_throw(alloc_str(message));
        // Unreachable after host traps the plugin
        core::arch::wasm32::unreachable()
    }
    #[cfg(not(target_arch = "wasm32"))]
    panic!("Plugin validation error: {message}")
}

/// Returns `true` if the current user has the given permission on this document.
pub fn sl_has_permission(doctype: &str, ptype: &str, name: &str) -> bool {
    #[cfg(target_arch = "wasm32")]
    unsafe {
        raw::sl_has_permission(alloc_str(doctype), alloc_str(ptype), alloc_str(name)) != 0
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = (doctype, ptype, name);
        true // stub grants all in tests
    }
}
