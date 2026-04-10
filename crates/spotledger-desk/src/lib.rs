//! SpotledgerCore — Tier 1 Desk DocTypes.
//!
//! Provides Workspace, File, Workflow, Print Format, Email Template, and
//! related infrastructure DocTypes that power the SpotledgerCore desk UI.
//!
//! All DocTypes in this crate are compiled into the binary.

pub mod doctype;
pub mod email_account;
pub mod email_template;
pub mod event;
pub mod utm;

/// Returns this crate's name. Used to force-link inventory submissions into the binary.
pub const fn name() -> &'static str { "spotledger-desk" }
