//! GL engine integration for plugin host functions.
//!
//! Phase 2.5: Provides wired-up access to accounting operations.
//! Phase 3: Will be fully integrated with spotledger-accounting.

use serde_json::{Value};
use tracing::debug;

/// GL operation results.
pub type GlResult<T> = Result<T, GlError>;

#[derive(Debug)]
pub enum GlError {
    InvalidBalance(String),
    DocumentNotFound(String),
    AccountingError(String),
}

impl std::fmt::Display for GlError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GlError::InvalidBalance(msg) => write!(f, "Invalid balance: {}", msg),
            GlError::DocumentNotFound(msg) => write!(f, "Document not found: {}", msg),
            GlError::AccountingError(msg) => write!(f, "Accounting error: {}", msg),
        }
    }
}

impl std::error::Error for GlError {}

/// Stub GL engine adapter for plugin operations.
/// Phase 3: Will be replaced with actual spotledger-accounting adapter.
pub struct GlAdapter {
    _phantom: std::marker::PhantomData<()>,
}

impl GlAdapter {
    pub fn new() -> Self {
        Self {
            _phantom: std::marker::PhantomData,
        }
    }

    /// Make GL entries.
    pub fn make_gl_entries(&self, payload: &Value) -> GlResult<Vec<String>> {
        debug!("make_gl_entries({:?})", payload);

        // Phase 2.5: Validate balance
        if let Some(entries) = payload.get("entries").and_then(|e| e.as_array()) {
            let mut debit_total = 0.0;
            let mut credit_total = 0.0;

            for entry in entries {
                if let Some(debit) = entry.get("debit").and_then(|d| d.as_f64()) {
                    debit_total += debit;
                }
                if let Some(credit) = entry.get("credit").and_then(|c| c.as_f64()) {
                    credit_total += credit;
                }
            }

            if (debit_total - credit_total).abs() > 0.01 {
                return Err(GlError::InvalidBalance(
                    format!("Debit {} != Credit {}", debit_total, credit_total),
                ));
            }
        }

        // Phase 2.5: Return stub GL entry names
        Ok(vec!["GL_Entry_001".to_string(), "GL_Entry_002".to_string()])
    }

    /// Reverse GL entries for a voucher.
    pub fn reverse_gl_entries(&self, voucher_type: &str, voucher_name: &str) -> GlResult<Vec<String>> {
        debug!("reverse_gl_entries({}, {})", voucher_type, voucher_name);

        // Phase 2.5: Return stub reversal entries
        Ok(vec![format!("{}_REV_001", voucher_name)])
    }

    /// Get account balance on a date.
    pub fn get_account_balance(&self, account: &str, company: &str, date: &str) -> GlResult<f64> {
        debug!("get_account_balance({}, {}, {})", account, company, date);

        // Phase 2.5: Return stub balance
        Ok(0.0)
    }

    /// Get fiscal year for a date.
    pub fn get_fiscal_year(&self, company: &str, date: &str) -> GlResult<String> {
        debug!("get_fiscal_year({}, {})", company, date);

        // Phase 2.5: Return stub FY name
        Ok(format!("FY_{}", date.split('-').next().unwrap_or("2026")))
    }

    /// Get exchange rate.
    pub fn get_exchange_rate(&self, from_curr: &str, to_curr: &str, date: &str) -> GlResult<f64> {
        debug!("get_exchange_rate({}, {}, {})", from_curr, to_curr, date);

        // Phase 2.5: Return stub rate (1.0 if same currency)
        if from_curr == to_curr {
            Ok(1.0)
        } else {
            Ok(1.0) // Stub
        }
    }
}

impl Default for GlAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_gl_error_display() {
        let err = GlError::InvalidBalance("test".to_string());
        assert_eq!(err.to_string(), "Invalid balance: test");
    }

    #[test]
    fn test_gl_adapter_balanced_entries() {
        let adapter = GlAdapter::new();
        let payload = json!({
            "company": "Test Co",
            "entries": [
                { "account": "Debtors", "debit": 100.0, "credit": 0.0 },
                { "account": "Sales", "debit": 0.0, "credit": 100.0 }
            ]
        });

        let result = adapter.make_gl_entries(&payload);
        assert!(result.is_ok());
    }

    #[test]
    fn test_gl_adapter_unbalanced_entries() {
        let adapter = GlAdapter::new();
        let payload = json!({
            "company": "Test Co",
            "entries": [
                { "account": "Debtors", "debit": 100.0, "credit": 0.0 },
                { "account": "Sales", "debit": 0.0, "credit": 50.0 }
            ]
        });

        let result = adapter.make_gl_entries(&payload);
        assert!(result.is_err());
    }
}
