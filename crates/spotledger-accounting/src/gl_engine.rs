//! GL Engine — the host-side implementation of `sl_make_gl_entries()`.
//!
//! WASM plugins call `sl_make_gl_entries(payload_ptr, payload_len)` to post
//! balanced double-entry accounting entries via the host.  The host deserialises
//! the payload, validates balance, and inserts `GL Entry` records.

use serde::{Deserialize, Serialize};
use rust_decimal::Decimal;

/// A single debit or credit line passed from a WASM plugin.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlLine {
    /// Account name (must exist in Chart of Accounts for the company).
    pub account: String,
    /// Company the entry belongs to.
    pub company: String,
    /// Cost centre (defaults to company's default cost centre if empty).
    pub cost_center: Option<String>,
    /// Debit amount in company currency (use 0.0 if credit).
    pub debit: Decimal,
    /// Credit amount in company currency (use 0.0 if debit).
    pub credit: Decimal,
    /// Source voucher type (e.g. "Sales Invoice").
    pub voucher_type: String,
    /// Source voucher name.
    pub voucher_no: String,
    /// Fiscal year (derived automatically if not provided).
    pub fiscal_year: Option<String>,
    /// Remarks.
    pub remarks: Option<String>,
}

/// Validated, balanced set of GL lines ready for insertion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlPayload {
    pub lines: Vec<GlLine>,
    pub posting_date: String,
    pub is_opening: bool,
}

impl GlPayload {
    /// Validate that debits == credits (books must balance).
    pub fn validate_balance(&self) -> Result<(), String> {
        let total_debit:  Decimal = self.lines.iter().map(|l| l.debit).sum();
        let total_credit: Decimal = self.lines.iter().map(|l| l.credit).sum();
        if total_debit != total_credit {
            return Err(format!(
                "GL imbalance: debit {total_debit} ≠ credit {total_credit}"
            ));
        }
        Ok(())
    }
}

// TODO: async fn post_gl_entries(db: &DbAdapter, payload: GlPayload) -> Result<Vec<String>, CoreError>
// Will insert GL Entry documents and return their names.
