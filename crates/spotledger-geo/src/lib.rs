//! Spotledger Geo — Currency, Country, and Language DocTypes.
//!
//! Generated from Frappe JSON via `spotledger generate doctype`.
//! Registered into the binary inventory automatically at link time.

pub mod country;
pub mod currency;
pub mod language;

/// Returns this crate's name. Used to force-link inventory submissions into the binary.
pub const fn name() -> &'static str { "spotledger-geo" }
