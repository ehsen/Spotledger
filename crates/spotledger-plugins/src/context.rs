//! Plugin execution context — provides access to host resources (DB, GL engine, etc.)
//!
//! Host functions execute in a sandboxed WASM environment without direct access to
//! host resources. This module provides a thread-local context that the host sets up
//! before calling plugin methods, and which host functions use to access the necessary resources.

use std::cell::RefCell;

/// Context available to executing host functions inside plugins.
/// Set up by the HTTP layer before calling into plugins, cleared after.
#[derive(Clone)]
pub struct PluginExecutionContext {
    /// Current user making the call
    pub current_user: String,
    /// Current site being accessed
    pub site_name: String,
}

thread_local! {
    static CURRENT_CONTEXT: RefCell<Option<PluginExecutionContext>> = RefCell::new(None);
}

/// Set the execution context for the current thread.
/// Called by the HTTP layer before invoking a plugin method.
pub fn set_execution_context(ctx: PluginExecutionContext) {
    CURRENT_CONTEXT.with(|c| {
        *c.borrow_mut() = Some(ctx);
    });
}

/// Get the current execution context.
/// Returns None if no context has been set up.
pub fn get_execution_context() -> Option<PluginExecutionContext> {
    CURRENT_CONTEXT.with(|c| c.borrow().clone())
}

/// Clear the execution context.
/// Called by the HTTP layer after plugin method completes.
pub fn clear_execution_context() {
    CURRENT_CONTEXT.with(|c| {
        *c.borrow_mut() = None;
    });
}
