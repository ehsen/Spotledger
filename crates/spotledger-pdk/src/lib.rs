//! SpotledgerCore Plugin Developer Kit — guest-side WASM helpers.
//!
//! Plugin authors import this crate to call host functions without writing
//! raw `extern "C"` declarations.
//!
//! # Modules
//!
//! - [`api`]        — Document operations: `sl_get_doc`, `sl_save_doc`, `sl_exists`, `sl_throw`, …
//! - [`accounting`] — GL engine: `make_gl_entries`, `reverse_gl_entries`, `get_fiscal_year`, …
//! - [`host`]       — Raw `extern "C"` declarations (use `api` instead)
//!
//! # Quick start
//!
//! ```ignore
//! use spotledger_pdk::api::{sl_log, sl_get_doc, sl_throw};
//! use spotledger_pdk::accounting::{make_gl_entries, GlEntry};
//!
//! pub extern "C" fn sl_plugin_init() {
//!     sl_log("info", "Selling plugin loaded");
//! }
//! ```

pub mod accounting;
pub mod api;
pub mod host;

// Re-export the most commonly used items at the crate root.
pub use api::{sl_log, sl_exists, sl_get_doc, sl_save_doc, sl_delete_doc,
              sl_get_value, sl_set_value, sl_throw, sl_has_permission};
pub use accounting::{make_gl_entries, reverse_gl_entries, get_account_balance,
                     get_fiscal_year, get_exchange_rate, GlEntry, FiscalYear};
