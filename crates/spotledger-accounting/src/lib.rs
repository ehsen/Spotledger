//! SpotledgerCore — Tier 3 Financial Engine.
//!
//! This crate is a **mandatory compiled-in** component of SpotledgerCore.
//! It provides the double-entry GL engine, Chart of Accounts, Fiscal Year
//! management, and Currency layer that all financial operations build upon.
//!
//! ## Why compiled-in (not WASM)?
//!
//! WASM plugins such as `selling.wasm` call the `sl_make_gl_entries()` host
//! function to post accounting entries.  The host function can only be provided
//! if the GL engine is already resident in the binary — putting accounting in a
//! plugin would create a circular dependency.
//!
//! ## Tier 3 DocTypes
//!
//! | DocType              | Purpose                                      |
//! |----------------------|----------------------------------------------|
//! | Company              | Legal entity; owns a Chart of Accounts       |
//! | Fiscal Year          | Accounting period (e.g. 2024-07-01 … 2025-06-30) |
//! | Fiscal Year Company  | M2M link between FY and Company              |
//! | Account              | Single node in the Chart of Accounts (tree)  |
//! | Cost Center          | Profit-centre allocation (tree)              |
//! | GL Entry             | Immutable double-entry ledger row            |
//! | Journal Entry        | Batch of GL Entries (submittable)            |
//! | Journal Entry Account| Child table row for Journal Entry            |
//! | Currency             | Currency master (USD, PKR, EUR…)             |
//! | Currency Exchange    | Daily FX rate snapshot                       |
//! | Tax Template         | Input / output tax head definitions          |

pub mod account;
pub mod company;
pub mod currency;
pub mod fiscal_year;
pub mod gl_engine;
pub mod journal_entry;
