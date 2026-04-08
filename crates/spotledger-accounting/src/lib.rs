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
//! | Currency Exchange    | Daily FX rate snapshot                       |
//! | Tax Template         | Input / output tax head definitions          |


pub mod journal_entry;
pub mod party_type;
pub mod party_validation;

// ── Generated from Frappe JSON (topological order) ────────────────────────────
// 1. Finance Book — no deps, pure Accounts module
pub mod finance_book;
// 2. Company — Accounts/Setup; plugin links annotated with .provided_by()
pub mod company;
// 3. Account — depends on Company, Currency, Account (self)
pub mod account;
// 4. Cost Center — depends on Company, CostCenter (self)
pub mod cost_center;
// 5. Fiscal Year Company (child table) — depends on Company
pub mod fiscal_year_company;
// 6. Fiscal Year — depends on Fiscal Year Company child table
pub mod fiscal_year;
// 7. GL Entry — depends on Account, Cost Center, Currency, Fiscal Year, Company, Finance Book
//    Project link annotated .provided_by("projects")
pub mod gl_entry;
