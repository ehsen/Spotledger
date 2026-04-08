//! Projects domain plugin — SpotledgerCore Tier 4 WASM app.
//!
//! DocTypes provided by this plugin:
//! - Project, Task, Timesheet, Timesheet Detail
//! - Project Type, Project Template, Project Template Task
//!
//! Plugin manifest declares:
//!   provides_doctypes: ["Project", "Task", "Timesheet"]
//!
//! This is read by PluginRegistry::register_capabilities() so that
//! GL Entry's `project` Link field (annotated `.provided_by("projects")`)
//! is validated when the plugin is loaded and dormant when not.

/// Plugin entry point called by the SpotledgerCore host at load time.
#[no_mangle]
pub extern "C" fn sl_plugin_init() {
    // TODO: register doctypes via sl_register_doctype() host call
}
