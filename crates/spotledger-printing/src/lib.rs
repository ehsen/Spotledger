//! Spotledger Printing — Letter Head and Print Heading DocTypes.
//!
//! Generated from Frappe JSON via `spotledger generate doctype`.
//! Registered into the binary inventory automatically at link time.

pub mod letter_head;
pub mod print_heading;
pub mod print_format;

/// Returns this crate's name. Used to force-link inventory submissions into the binary.
pub const fn name() -> &'static str { "spotledger-printing" }
