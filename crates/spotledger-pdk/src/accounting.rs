//! Guest-side GL engine wrappers for SpotledgerCore plugins.
//!
//! Plugins call these to post and reverse GL entries, query account balances,
//! fiscal years, and exchange rates — all routed to the Tier 3 accounting engine
//! compiled into the host binary.
//!
//! These functions must NOT be called outside of a document lifecycle hook
//! (e.g. `on_submit`, `on_cancel`). They will trap the plugin if the host
//! determines the call is out of context.

use serde::{Deserialize, Serialize};

#[cfg(target_arch = "wasm32")]
use crate::api::{alloc_str, read_bytes_handle};

#[cfg(target_arch = "wasm32")]
mod raw {
    #[link(wasm_import_module = "extism:host/user")]
    extern "C" {
        pub fn sl_make_gl_entries(payload: i64) -> i64;
        pub fn sl_reverse_gl_entries(voucher_type: i64, voucher_no: i64) -> i64;
        pub fn sl_get_account_balance(account: i64, date: i64, cost_center: i64) -> i64;
        pub fn sl_get_fiscal_year(company: i64, date: i64) -> i64;
        pub fn sl_get_exchange_rate(from_curr: i64, to_curr: i64, date: i64) -> i64;
    }
}

// ── Types ─────────────────────────────────────────────────────────────────────

/// A single GL entry line. Debit and credit must be ≥ 0.
/// The full voucher must satisfy `sum(debit) == sum(credit)`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlEntry {
    pub account:      String,
    pub debit:        f64,
    pub credit:       f64,
    pub voucher_type: String,
    pub voucher_no:   String,
    pub company:      String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost_center:  Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project:      Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remarks:      Option<String>,
    pub posting_date: String,
    pub fiscal_year:  String,
    #[serde(default)]
    pub is_opening:   bool,
}

/// Response from a fiscal year query.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FiscalYear {
    pub name:            String,
    pub year_start_date: String,
    pub year_end_date:   String,
}

// ── GL API ───────────────────────────────────────────────────────────────────

/// Post GL entries via the Tier 3 accounting engine.
///
/// # Rules
/// - `sum(debit)` must equal `sum(credit)` across the full `entries` list.
/// - `account` must exist in the host's `Account` table.
/// - `fiscal_year` must be open.
///
/// Returns an error string if validation or insertion fails.
pub fn make_gl_entries(entries: Vec<GlEntry>) -> Result<(), String> {
    let payload = serde_json::json!({ "entries": entries });

    #[cfg(target_arch = "wasm32")]
    unsafe {
        let bytes    = serde_json::to_vec(&payload).map_err(|e| e.to_string())?;
        let in_str   = String::from_utf8_lossy(&bytes).into_owned();
        let result_h = raw::sl_make_gl_entries(alloc_str(&in_str));
        if result_h == 0 {
            return Err("make_gl_entries failed".to_string());
        }
        let out_bytes = read_bytes_handle(result_h);
        let v: serde_json::Value = serde_json::from_slice(&out_bytes).map_err(|e| e.to_string())?;
        if v["status"].as_str() == Some("ok") { Ok(()) } else {
            Err(v["error"].as_str().unwrap_or("unknown GL error").to_string())
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        // Stub: validate balance only
        let debit:  f64 = entries.iter().map(|e| e.debit).sum();
        let credit: f64 = entries.iter().map(|e| e.credit).sum();
        if (debit - credit).abs() > 0.001 {
            return Err(format!("GL balance error: debit ({debit:.2}) ≠ credit ({credit:.2})"));
        }
        Ok(())
    }
}

/// Cancel all GL entries for a voucher by posting reversals.
pub fn reverse_gl_entries(voucher_type: &str, voucher_no: &str) -> Result<(), String> {
    #[cfg(target_arch = "wasm32")]
    unsafe {
        let h = raw::sl_reverse_gl_entries(alloc_str(voucher_type), alloc_str(voucher_no));
        if h == 0 { return Err("reverse_gl_entries failed".to_string()); }
        Ok(())
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = (voucher_type, voucher_no);
        Ok(())
    }
}

/// Get the current balance of an account.
/// Returns 0.0 if the account has no entries.
pub fn get_account_balance(account: &str, date: &str, cost_center: &str) -> Result<f64, String> {
    #[cfg(target_arch = "wasm32")]
    unsafe {
        let h = raw::sl_get_account_balance(
            alloc_str(account), alloc_str(date), alloc_str(cost_center)
        );
        if h == 0 { return Err("get_account_balance failed".to_string()); }
        let bytes = read_bytes_handle(h);
        let v: serde_json::Value = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
        Ok(v["balance"].as_f64().unwrap_or(0.0))
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = (account, date, cost_center);
        Ok(0.0)
    }
}

/// Find the fiscal year that contains `date` for the given company.
pub fn get_fiscal_year(company: &str, date: &str) -> Result<FiscalYear, String> {
    #[cfg(target_arch = "wasm32")]
    unsafe {
        let h = raw::sl_get_fiscal_year(alloc_str(company), alloc_str(date));
        if h == 0 { return Err("get_fiscal_year failed".to_string()); }
        let bytes = read_bytes_handle(h);
        serde_json::from_slice(&bytes).map_err(|e| e.to_string())
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = (company, date);
        Ok(FiscalYear {
            name:            "2025-2026".to_string(),
            year_start_date: "2025-04-01".to_string(),
            year_end_date:   "2026-03-31".to_string(),
        })
    }
}

/// Get the exchange rate between two currencies on a given date.
/// Returns 1.0 (i.e. same currency or no rate found) for stub.
pub fn get_exchange_rate(from_currency: &str, to_currency: &str, date: &str) -> Result<f64, String> {
    #[cfg(target_arch = "wasm32")]
    unsafe {
        let h = raw::sl_get_exchange_rate(
            alloc_str(from_currency), alloc_str(to_currency), alloc_str(date)
        );
        if h == 0 { return Err("get_exchange_rate failed".to_string()); }
        let bytes = read_bytes_handle(h);
        let v: serde_json::Value = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
        Ok(v["rate"].as_f64().unwrap_or(1.0))
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = (from_currency, to_currency, date);
        Ok(1.0)
    }
}
