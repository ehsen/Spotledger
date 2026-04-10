//! Tier 0 framework DocTypes — core metadata and user management.
//!
//! This module contains all Tier 0 DocType definitions, automatically registered
//! via the `MetaEntry` inventory.

pub mod doctype;
pub mod user;
pub mod system;
pub mod naming;
pub mod error_log;

// Re-exports for convenience
pub use doctype::*;
pub use user::*;
pub use system::*;
pub use naming::*;
