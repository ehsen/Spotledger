//! Spotledger Contacts — Address, Contact, Gender DocTypes.
//!
//! Generated from Frappe JSON via `spotledger generate doctype`.
//! Registered into the binary inventory automatically at link time.

pub mod address;
pub mod contact;
pub mod gender;
pub mod salutation;

/// Returns this crate's name. Used to force-link inventory submissions into the binary.
pub const fn name() -> &'static str { "spotledger-contacts" }
