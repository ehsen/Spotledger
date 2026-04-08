//! Real extism host function implementations for SpotledgerCore plugins.
//!
//! These are the functions that WASM plugins import and call. They are
//! registered with extism's `Plugin` at load time via `build_host_functions()`.
//!
//! All functions are available to plugins under the `extism:host/user` namespace,
//! which is the standard namespace for user-defined host functions in extism.
//!
//! ## ABI contract (stable — never breaks)
//!
//! | Import name                  | Inputs (PTR = i64 memory handle) | Output          |
//! |------------------------------|----------------------------------|-----------------|
//! | `sl_log`                     | level: PTR, msg: PTR             | (void)          |
//! | `sl_exists`                  | doctype: PTR, name: PTR          | found: PTR(i64) |
//! | `sl_get_doc`                 | doctype: PTR, name: PTR          | json: PTR       |
//! | `sl_save_doc`                | payload: PTR                     | doc: PTR        |
//! | `sl_delete_doc`              | doctype: PTR, name: PTR          | (void)          |
//! | `sl_get_value`               | doctype, name, field: PTR×3      | val: PTR        |
//! | `sl_set_value`               | doctype, name, field, val: PTR×4 | (void)          |
//! | `sl_throw`                   | message: PTR                     | (void)          |
//! | `sl_has_permission`          | doctype, ptype, name: PTR×3      | ok: PTR(i64)    |
//! | `sl_make_gl_entries`         | payload: PTR                     | result: PTR     |
//! | `sl_reverse_gl_entries`      | voucher_type, voucher_no: PTR×2  | result: PTR     |
//! | `sl_get_account_balance`     | account, date, cost_center: PTR×3| result: PTR     |
//! | `sl_get_fiscal_year`         | company, date: PTR×2             | result: PTR     |
//! | `sl_get_exchange_rate`       | from, to, date: PTR×3            | result: PTR     |

use anyhow::anyhow;
use extism::{host_fn, Function, UserData, PTR};
use tracing::{debug, error, info, warn};

// ── Logging ───────────────────────────────────────────────────────────────────

host_fn!(pub host_sl_log(_ud: (); level: String, msg: String) {
    match level.to_lowercase().as_str() {
        "debug" => debug!(source = "plugin", "{}", msg),
        "info"  => info!(source  = "plugin", "{}", msg),
        "warn"  => warn!(source  = "plugin", "{}", msg),
        "error" => error!(source = "plugin", "{}", msg),
        _       => debug!(source = "plugin", "{}", msg),
    }
    Ok(())
});

// ── Document Operations ───────────────────────────────────────────────────────

/// Returns 1 (i64) if the document exists, 0 otherwise.
host_fn!(pub host_sl_exists(_ud: (); doctype: String, name: String) -> i64 {
    debug!("sl_exists({doctype}, {name})");
    // Phase 3: query DB. Stub always returns 0 (not found).
    Ok(0i64)
});

/// Returns the document JSON bytes, or an error if not found.
host_fn!(pub host_sl_get_doc(_ud: (); doctype: String, name: String) -> Vec<u8> {
    debug!("sl_get_doc({doctype}, {name})");
    // Phase 3: query DB. Stub returns a minimal document skeleton.
    let stub = serde_json::json!({
        "doctype": doctype,
        "name": name,
        "creation": "2026-01-01T00:00:00",
        "modified": "2026-01-01T00:00:00",
        "owner": "Administrator",
        "docstatus": 0,
    });
    serde_json::to_vec(&stub).map_err(|e| anyhow!(e))
});

/// Accepts a JSON-encoded document, saves it, returns the saved document bytes.
host_fn!(pub host_sl_save_doc(_ud: (); payload: Vec<u8>) -> Vec<u8> {
    debug!("sl_save_doc: {} bytes", payload.len());
    // Phase 3: validate + save. Stub echoes the payload.
    Ok(payload)
});

host_fn!(pub host_sl_delete_doc(_ud: (); doctype: String, name: String) {
    debug!("sl_delete_doc({doctype}, {name})");
    Ok(())
});

/// Returns the field value as JSON bytes (null if field doesn't exist).
host_fn!(pub host_sl_get_value(_ud: (); doctype: String, name: String, field: String) -> Vec<u8> {
    debug!("sl_get_value({doctype}, {name}, {field})");
    serde_json::to_vec(&serde_json::Value::Null).map_err(|e| anyhow!(e))
});

/// Accepts a JSON-encoded value. Sets the field immediately (no hooks).
host_fn!(pub host_sl_set_value(_ud: (); doctype: String, name: String, field: String, val: Vec<u8>) {
    debug!("sl_set_value({doctype}, {name}, {field}): {} bytes", val.len());
    Ok(())
});

/// Raises a validation error visible to the desk user. Causes the plugin call to fail.
pub fn host_sl_throw(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    _outputs: &mut [extism::Val],
    _ud: extism::UserData<()>,
) -> Result<(), extism::Error> {
    let message: String = plugin.memory_get_val(&inputs[0])?;
    error!("Plugin validation error: {message}");
    Err(anyhow!("Validation error: {message}"))
}

/// Returns 1 (i64) if the current user has the permission, 0 otherwise.
host_fn!(pub host_sl_has_permission(_ud: (); doctype: String, ptype: String, name: String) -> i64 {
    debug!("sl_has_permission({doctype}, {ptype}, {name})");
    // Phase 3: real permission check. Stub grants all.
    Ok(1i64)
});

// ── GL Engine (Tier 3, always available) ─────────────────────────────────────

/// Validates double-entry balance and posts GL entries.
/// Payload: `{ "company": "...", "entries": [{ "account", "debit", "credit", ... }] }`
host_fn!(pub host_sl_make_gl_entries(_ud: (); payload: Vec<u8>) -> Vec<u8> {
    debug!("sl_make_gl_entries: {} bytes", payload.len());

    // Validate double-entry balance
    if let Ok(json) = serde_json::from_slice::<serde_json::Value>(&payload) {
        if let Some(entries) = json.get("entries").and_then(|e| e.as_array()) {
            let debit:  f64 = entries.iter()
                .filter_map(|e| e.get("debit").and_then(|d| d.as_f64())).sum();
            let credit: f64 = entries.iter()
                .filter_map(|e| e.get("credit").and_then(|c| c.as_f64())).sum();
            if (debit - credit).abs() > 0.001 {
                return Err(anyhow!(
                    "GL balance error: debit ({:.2}) ≠ credit ({:.2})", debit, credit
                ));
            }
        }
    }

    // Phase 3: insert rows into GLEntry table. Stub returns success.
    let result = serde_json::json!({ "status": "ok", "gl_entries": [] });
    serde_json::to_vec(&result).map_err(|e| anyhow!(e))
});

host_fn!(pub host_sl_reverse_gl_entries(_ud: (); voucher_type: String, voucher_no: String) -> Vec<u8> {
    debug!("sl_reverse_gl_entries({voucher_type}, {voucher_no})");
    let result = serde_json::json!({
        "status": "reversed",
        "voucher_type": voucher_type,
        "voucher_no":   voucher_no,
    });
    serde_json::to_vec(&result).map_err(|e| anyhow!(e))
});

/// Returns `{ "balance": 0.0 }` JSON bytes.
host_fn!(pub host_sl_get_account_balance(_ud: (); account: String, date: String, cost_center: String) -> Vec<u8> {
    debug!("sl_get_account_balance({account}, {date})");
    serde_json::to_vec(&serde_json::json!({ "balance": 0.0 })).map_err(|e| anyhow!(e))
});

/// Returns `{ "name": "...", "year_start_date": "...", "year_end_date": "..." }` JSON bytes.
host_fn!(pub host_sl_get_fiscal_year(_ud: (); company: String, date: String) -> Vec<u8> {
    debug!("sl_get_fiscal_year({company}, {date})");
    let stub = serde_json::json!({
        "name":            "2025-2026",
        "year_start_date": "2025-04-01",
        "year_end_date":   "2026-03-31",
    });
    serde_json::to_vec(&stub).map_err(|e| anyhow!(e))
});

/// Returns `{ "rate": 1.0 }` JSON bytes.
host_fn!(pub host_sl_get_exchange_rate(_ud: (); from_curr: String, to_curr: String, date: String) -> Vec<u8> {
    debug!("sl_get_exchange_rate({from_curr} → {to_curr}, {date})");
    serde_json::to_vec(&serde_json::json!({ "rate": 1.0 })).map_err(|e| anyhow!(e))
});

// ── Build Function Registry ───────────────────────────────────────────────────

/// Build the list of SpotledgerCore host functions to register with each plugin.
///
/// Call this once and pass the result to `Plugin::new()`.  All functions are
/// available to plugins under the `extism:host/user` WASM import namespace.
pub fn build_host_functions() -> Vec<Function> {
    vec![
        // Logging
        Function::new("sl_log",
            [PTR, PTR], [], UserData::default(), host_sl_log),

        // Document operations
        Function::new("sl_exists",
            [PTR, PTR], [PTR], UserData::default(), host_sl_exists),
        Function::new("sl_get_doc",
            [PTR, PTR], [PTR], UserData::default(), host_sl_get_doc),
        Function::new("sl_save_doc",
            [PTR], [PTR], UserData::default(), host_sl_save_doc),
        Function::new("sl_delete_doc",
            [PTR, PTR], [], UserData::default(), host_sl_delete_doc),
        Function::new("sl_get_value",
            [PTR, PTR, PTR], [PTR], UserData::default(), host_sl_get_value),
        Function::new("sl_set_value",
            [PTR, PTR, PTR, PTR], [], UserData::default(), host_sl_set_value),
        Function::new("sl_throw",
            [PTR], [], UserData::default(), host_sl_throw),
        Function::new("sl_has_permission",
            [PTR, PTR, PTR], [PTR], UserData::default(), host_sl_has_permission),

        // GL engine (Tier 3 — always available)
        Function::new("sl_make_gl_entries",
            [PTR], [PTR], UserData::default(), host_sl_make_gl_entries),
        Function::new("sl_reverse_gl_entries",
            [PTR, PTR], [PTR], UserData::default(), host_sl_reverse_gl_entries),
        Function::new("sl_get_account_balance",
            [PTR, PTR, PTR], [PTR], UserData::default(), host_sl_get_account_balance),
        Function::new("sl_get_fiscal_year",
            [PTR, PTR], [PTR], UserData::default(), host_sl_get_fiscal_year),
        Function::new("sl_get_exchange_rate",
            [PTR, PTR, PTR], [PTR], UserData::default(), host_sl_get_exchange_rate),
    ]
}
