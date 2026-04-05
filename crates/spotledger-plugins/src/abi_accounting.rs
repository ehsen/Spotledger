//! Host function bindings for accounting operations (GL engine).
//!
//! Phase 2.5: function stubs with GL adapter integration.
//! Phase 3: will fully integrate with spotledger-accounting.
//!
//! Plugin host functions:
//!   - sl_make_gl_entries(payload) → list of created GL Entry IDs
//!   - sl_reverse_gl_entries(voucher_type, voucher_name) → reversal entries
//!   - sl_get_account_balance(account, company, date) → Decimal
//!   - sl_get_fiscal_year(company, date) → FiscalYear name
//!   - sl_get_exchange_rate(from_currency, to_currency, date) → rate

use tracing::debug;
use crate::context::get_execution_context;
use crate::gl::GlAdapter;

/// Post GL entries via the accounting engine.
///
/// Plugin serialises a GlPayload JSON:
///   {
///     "company": "...",
///     "posting_date": "2026-01-01",
///     "entries": [
///       { "account": "...", "debit": 100, "credit": 0, "ref_doctype": "...", "ref_docname": "..." },
///       ...
///     ]
///   }
///
/// Host validates that debits = credits, posts entries, returns list of created GL Entry names.
/// Returns: memory offset to JSON array of GL Entry names, or -1 on error
pub fn sl_make_gl_entries(payload_ptr: u64, payload_len: u32) -> i32 {
    debug!(
        "sl_make_gl_entries called: payload_ptr={}, payload_len={}",
        payload_ptr, payload_len
    );

    if let Some(ctx) = get_execution_context() {
        debug!(
            "GL entries context: user={}, site={}",
            ctx.current_user, ctx.site_name
        );
    }

    let gl = GlAdapter::new();
    
    // TODO: Deserialize GlPayload from plugin memory
    match gl.make_gl_entries(&serde_json::json!({})) {
        Ok(_entries) => {
            // TODO: Serialize results back to plugin memory
            0
        }
        Err(_e) => -1,
    }
}

/// Reverse (cancel) GL entries for a voucher.
///
/// Used when a controlling document (e.g., Sales Invoice) is cancelled:
/// reverse all GL entries that were posted on submission.
///
/// Returns: memory offset to JSON array of reversal GL Entry names, or -1 on error
pub fn sl_reverse_gl_entries(
    voucher_type_ptr: u64,
    _voucher_type_len: u32,
    voucher_name_ptr: u64,
    _voucher_name_len: u32,
) -> i32 {
    debug!(
        "sl_reverse_gl_entries called: voucher_type_ptr={}, voucher_name_ptr={}",
        voucher_type_ptr, voucher_name_ptr
    );

    let gl = GlAdapter::new();
    
    // TODO: Deserialize voucher_type and voucher_name from plugin memory
    match gl.reverse_gl_entries("SI", "SI-001") {
        Ok(_entries) => {
            // TODO: Serialize results back to plugin memory
            0
        }
        Err(_) => -1,
    }
}

/// Get the balance of an account at a point in time.
///
/// Returns account balance considering all GL entries posted before the date.
/// Balance = sum(debit) - sum(credit) for the account.
///
/// Returns: memory offset to serialized Decimal, or -1 on error
pub fn sl_get_account_balance(
    account_ptr: u64,
    _account_len: u32,
    company_ptr: u64,
    _company_len: u32,
    date_ptr: u64,
    _date_len: u32,
) -> i32 {
    debug!(
        "sl_get_account_balance called: account_ptr={}, company_ptr={}, date_ptr={}",
        account_ptr, company_ptr, date_ptr
    );

    let gl = GlAdapter::new();
    
    // TODO: Deserialize account, company, date from plugin memory
    match gl.get_account_balance("Debtors", "Company", "2026-01-01") {
        Ok(_balance) => {
            // TODO: Serialize balance back to plugin memory
            0
        }
        Err(_) => -1,
    }
}

/// Get the fiscal year for a date.
///
/// Returns the fiscal year name that contains the date for the given company.
///
/// Returns: memory offset to serialized fiscal year name string, or -1 on error
pub fn sl_get_fiscal_year(
    company_ptr: u64,
    _company_len: u32,
    date_ptr: u64,
    _date_len: u32,
) -> i32 {
    debug!(
        "sl_get_fiscal_year called: company_ptr={}, date_ptr={}",
        company_ptr, date_ptr
    );

    let gl = GlAdapter::new();
    
    // TODO: Deserialize company and date from plugin memory
    match gl.get_fiscal_year("Company", "2026-01-01") {
        Ok(_fy) => {
            // TODO: Serialize FY name back to plugin memory
            0
        }
        Err(_) => -1,
    }
}

/// Get the exchange rate between two currencies on a date.
///
/// Returns the rate of conversion from from_currency to to_currency on the given date.
/// E.g., sl_get_exchange_rate("USD", "INR", "2026-01-01") might return 83.5
///
/// Returns: memory offset to serialized Decimal rate, or -1 on error
pub fn sl_get_exchange_rate(
    from_currency_ptr: u64,
    _from_currency_len: u32,
    to_currency_ptr: u64,
    _to_currency_len: u32,
    date_ptr: u64,
    _date_len: u32,
) -> i32 {
    debug!(
        "sl_get_exchange_rate called: from={}, to={}, date={}",
        from_currency_ptr, to_currency_ptr, date_ptr
    );

    let gl = GlAdapter::new();
    
    // TODO: Deserialize from_currency, to_currency, date from plugin memory
    match gl.get_exchange_rate("USD", "INR", "2026-01-01") {
        Ok(_rate) => {
            // TODO: Serialize rate back to plugin memory
            0
        }
        Err(_) => -1,
    }
}
