//! Host function bindings for document operations.
//!
//! Phase 2: function stubs for the plugin ABI.
//! Phase 2.5: will integrate with extism for actual WASM plugin execution.
//!
//! Plugin host functions:
//!   - sl_get_doc(doctype, name) → Document JSON
//!   - sl_save_doc(payload) → success/error
//!   - sl_delete_doc(doctype, name) → success/error
//!   - sl_get_value(doctype, name, field) → field value
//!   - sl_set_value(doctype, name, field, value) → success/error
//!   - sl_has_permission(action, doctype, name) → bool
//!   - sl_throw_error(message) → exits plugin
//!   - sl_log(level, message) → logs to tracing

use tracing::{debug, info, warn};
use crate::context::get_execution_context;

/// Host function: get a document from the database.
///
/// Plugin memory layout:
///   - doctype: pointer to null-terminated string (in plugin memory)
///   - name: pointer to null-terminated string (in plugin memory)
///
/// Returns: 0 on success, -1 on error
pub fn sl_get_doc(doctype_ptr: u64, doctype_len: u32, name_ptr: u64, name_len: u32) -> i32 {
    debug!(
        "sl_get_doc called: doctype_ptr={}, name_ptr={}",
        doctype_ptr, name_ptr
    );

    if let Some(ctx) = get_execution_context() {
        debug!(
            "Executing in context: user={}, site={}",
            ctx.current_user, ctx.site_name
        );
    }

    // Phase 2.5: deserialize doctype and name from plugin memory,
    // fetch from DB via spotledger-db adapter, serialize response back to plugin memory.
    // For now, return success stub.
    0
}

/// Host function: save a document.
///
/// Plugin memory layout:
///   - payload JSON: { "doctype": "DocType", "name": "docname", "fields": {...} }
///
/// Returns: 0 on success, -1 on error
pub fn sl_save_doc(payload_ptr: u64, payload_len: u32) -> i32 {
    debug!("sl_save_doc called: len={}", payload_len);

    // Phase 2.5: deserialize payload JSON from plugin memory,
    // validate against DocType schema, update DB, return success.
    0
}

/// Host function: delete a document.
///
/// Returns: 0 on success, -1 on error
pub fn sl_delete_doc(doctype_ptr: u64, doctype_len: u32, name_ptr: u64, name_len: u32) -> i32 {
    debug!(
        "sl_delete_doc called: doctype_ptr={}, name_ptr={}",
        doctype_ptr, name_ptr
    );
    0
}

/// Host function: get a single field value.
///
/// Returns: memory offset to serialized value, or -1 on error
pub fn sl_get_value(
    doctype_ptr: u64,
    doctype_len: u32,
    name_ptr: u64,
    name_len: u32,
    field_ptr: u64,
    field_len: u32,
) -> i32 {
    debug!(
        "sl_get_value called: doctype_ptr={}, name_ptr={}, field_ptr={}",
        doctype_ptr, name_ptr, field_ptr
    );
    0
}

/// Host function: set a single field value.
///
/// Returns: 0 on success, -1 on error
pub fn sl_set_value(
    doctype_ptr: u64,
    doctype_len: u32,
    name_ptr: u64,
    name_len: u32,
    field_ptr: u64,
    field_len: u32,
    value_ptr: u64,
    value_len: u32,
) -> i32 {
    debug!(
        "sl_set_value called: doctype_ptr={}, name_ptr={}, field_ptr={}",
        doctype_ptr, name_ptr, field_ptr
    );
    0
}

/// Host function: check if the current user has permission for an action.
///
/// Returns: 1 for YES, 0 for NO, -1 on error
pub fn sl_has_permission(
    action_ptr: u64,
    action_len: u32,
    doctype_ptr: u64,
    doctype_len: u32,
    name_ptr: u64,
    name_len: u32,
) -> i32 {
    debug!(
        "sl_has_permission called: action_ptr={}, doctype_ptr={}, name_ptr={}",
        action_ptr, doctype_ptr, name_ptr
    );

    // Phase 2.5: extract action, doctype, name from plugin memory,
    // check against current_user's permissions from context.
    // For now, grant all permissions.
    1
}

/// Host function: throw a validation error (exits plugin execution).
///
/// This should be called when the plugin detects invalid data.
/// The host will catch this and return a ValidationError to the client.
pub fn sl_throw_error(msg_ptr: u64, msg_len: u32) {
    warn!("sl_throw_error called: message_ptr={}", msg_ptr);
    // Phase 2.5: convert message to String from plugin memory, raise extism error
    // For now, just log the call.
}

/// Host function: log a message to tracing infrastructure.
///
/// levels: "debug" | "info" | "warn" | "error"
pub fn sl_log(level_ptr: u64, level_len: u32, msg_ptr: u64, msg_len: u32) {
    // Phase 2.5: deserialize level and message from plugin memory, call tracing macros
    // For now, just log that a plugin called us.
    debug!(
        "sl_log called: level_ptr={}, msg_ptr={}",
        level_ptr, msg_ptr
    );
}
