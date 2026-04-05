//! Host function bindings for accounting operations (GL engine).
//!
//! Phase 2: function stubs for the accounting ABI.
//! Phase 2.5: will integrate with extism and spotledger-accounting.
//!
//! Plugin host functions:
//!   - sl_make_gl_entries(payload) → list of created GL Entry IDs
//!   - sl_reverse_gl_entries(voucher_type, voucher_name) → reversal entries
//!   - sl_get_account_balance(account, company, date) → Decimal
//!   - sl_get_fiscal_year(company, date) → FiscalYear name
//!   - sl_get_exchange_rate(from_currency, to_currency, date) → rate

use tracing::debug;
use crate::context::get_execution_context;

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

    // Phase 2.5: deserialize GlPayload from plugin memory,
    // call spotledger_accounting::gl_engine::post_gl_entries,
    // validate balance, insert GL Entry rows, serialize results back to plugin memory.
    0
}

/// Reverse (cancel) GL entries for a voucher.
///
/// Used when a controlling document (e.g., Sales Invoice) is cancelled:
/// reverse all GL entries that were posted on submission.
///
/// Returns: memory offset to JSON array of reversal GL Entry names, or -1 on error
pub fn sl_reverse_gl_entries(
    voucher_type_ptr: u64,
    voucher_type_len: u32,
    voucher_name_ptr: u64,
    voucher_name_len: u32,
) -> i32 {
    debug!(
        "sl_reverse_gl_entries called: voucher_type_ptr={}, voucher_name_ptr={}",
        voucher_type_ptr, voucher_name_ptr
    );

    // Phase 2.5: look up original GL entries for voucher,
    // create reversal rows (same account, debit ↔ credit swapped),
    // post reversal entries, return list of names.
    0
}

/// Get the balance of an account at a point in time.
///
/// Returns account balance considering all GL entries posted before the date.
/// Balance = sum(debit) - sum(credit) for the account.
///
/// Returns: memory offset to serialized Decimal, or -1 on error
pub fn sl_get_account_balance(
    account_ptr: u64,
    account_len: u32,
    company_ptr: u64,
    company_len: u32,
    date_ptr: u64,
    date_len: u32,
) -> i32 {
    debug!(
        "sl_get_account_balance called: account_ptr={}, company_ptr={}, date_ptr={}",
        account_ptr, company_ptr, date_ptr
    );

    // Phase 2.5: deserialize account, company, date from plugin memory,
    // query GL_Entry table for account + company where posting_date <= date,
    // sum debit/credit, serialize result back to plugin memory.
    0
}

/// Get the fiscal year for a date.
///
/// Returns the fiscal year name that contains the date for the given company.
///
/// Returns: memory offset to serialized fiscal year name string, or -1 on error
pub fn sl_get_fiscal_year(
    company_ptr: u64,
    company_len: u32,
    date_ptr: u64,
    date_len: u32,
) -> i32 {
    debug!(
        "sl_get_fiscal_year called: company_ptr={}, date_ptr={}",
        company_ptr, date_ptr
    );

    // Phase 2.5: look up Fiscal Year document that contains date for company,
    // serialize FY name back to plugin memory.
    0
}

/// Get the exchange rate between two currencies on a date.
///
/// Returns the rate of conversion from from_currency to to_currency on the given date.
/// E.g., sl_get_exchange_rate("USD", "INR", "2026-01-01") might return 83.5
///
/// Returns: memory offset to serialized Decimal rate, or -1 on error
pub fn sl_get_exchange_rate(
    from_currency_ptr: u64,
    from_currency_len: u32,
    to_currency_ptr: u64,
    to_currency_len: u32,
    date_ptr: u64,
    date_len: u32,
) -> i32 {
    debug!(
        "sl_get_exchange_rate called: from={}, to={}, date={}",
        from_currency_ptr, to_currency_ptr, date_ptr
    );

    // Phase 2.5: look up Currency Exchange snapshot for from_currency + to_currency
    // on date, serialize rate back to plugin memory.
    0
}
