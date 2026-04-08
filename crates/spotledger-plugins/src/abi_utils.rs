//! Utils host functions — expose spotledger-core utilities to WASM plugins.
//!
//! Available to plugins under `extism:host/user` namespace.
//!
//! | Import name                    | Inputs              | Output       |
//! |-------------------------------|---------------------|--------------|
//! | `sl_utils_scrub`              | s: PTR              | result: PTR  |
//! | `sl_utils_unscrub`            | s: PTR              | result: PTR  |
//! | `sl_utils_now`                | (none)              | result: PTR  |
//! | `sl_utils_today`              | (none)              | result: PTR  |
//! | `sl_utils_add_days`           | date, days: PTR×2   | result: PTR  |
//! | `sl_utils_date_diff`          | from, to: PTR×2     | result: PTR  |
//! | `sl_utils_formatdate`         | date, fmt: PTR×2    | result: PTR  |
//! | `sl_utils_validate_email`     | email: PTR          | ok: PTR(i64) |
//! | `sl_utils_validate_phone`     | phone: PTR          | ok: PTR(i64) |
//! | `sl_utils_money_in_words`     | payload: PTR        | result: PTR  |
//! | `sl_utils_get_ancestors_of`   | doctype, name: PTR×2| result: PTR  |
//! | `sl_utils_get_descendants_of` | doctype, name: PTR×2| result: PTR  |

use anyhow::anyhow;
use extism::{host_fn, CurrentPlugin, Function, UserData, Val, PTR};
use spotledger_core::utils::strings::{scrub, unscrub};
use spotledger_core::utils::dates::{now, today, add_days, date_diff, getdate};
use spotledger_core::utils::formatting::formatdate;
use spotledger_core::utils::validation::{validate_email_address, validate_phone_number};
use spotledger_core::utils::numbers::money_in_words;
use tracing::debug;

// ── String utils ─────────────────────────────────────────────────────────────

host_fn!(pub host_sl_utils_scrub(_ud: (); s: String) -> Vec<u8> {
    Ok(scrub(&s).into_bytes())
});

host_fn!(pub host_sl_utils_unscrub(_ud: (); s: String) -> Vec<u8> {
    Ok(unscrub(&s).into_bytes())
});

// ── Date utils ────────────────────────────────────────────────────────────────

host_fn!(pub host_sl_utils_now(_ud: (); _dummy: String) -> Vec<u8> {
    Ok(now().into_bytes())
});

host_fn!(pub host_sl_utils_today(_ud: (); _dummy: String) -> Vec<u8> {
    Ok(today().into_bytes())
});

host_fn!(pub host_sl_utils_add_days(_ud: (); payload: Vec<u8>) -> Vec<u8> {
    let v: serde_json::Value = serde_json::from_slice(&payload).map_err(|e| anyhow!(e))?;
    let date_str = v["date"].as_str().unwrap_or("").to_string();
    let days = v["days"].as_i64().unwrap_or(0);
    let result = if let Some(date) = getdate(&date_str) {
        add_days(date, days).to_string()
    } else {
        date_str
    };
    Ok(result.into_bytes())
});

/// payload JSON: `{ "from": "YYYY-MM-DD", "to": "YYYY-MM-DD" }`
host_fn!(pub host_sl_utils_date_diff(_ud: (); payload: Vec<u8>) -> Vec<u8> {
    let v: serde_json::Value = serde_json::from_slice(&payload).map_err(|e| anyhow!(e))?;
    let from = v["from"].as_str().unwrap_or("").to_string();
    let to   = v["to"].as_str().unwrap_or("").to_string();
    let days = date_diff(&from, &to);
    serde_json::to_vec(&days).map_err(|e| anyhow!(e))
});

host_fn!(pub host_sl_utils_formatdate(_ud: (); payload: Vec<u8>) -> Vec<u8> {
    let v: serde_json::Value = serde_json::from_slice(&payload).map_err(|e| anyhow!(e))?;
    let date_str = v["date"].as_str().unwrap_or("").to_string();
    let fmt = v["fmt"].as_str().unwrap_or("%Y-%m-%d").to_string();
    let result = if let Some(date) = getdate(&date_str) {
        formatdate(date, &fmt)
    } else {
        date_str
    };
    Ok(result.into_bytes())
});

// ── Validation utils ──────────────────────────────────────────────────────────

pub fn host_sl_utils_validate_email(
    plugin: &mut CurrentPlugin,
    inputs: &[Val],
    outputs: &mut [Val],
    _ud: UserData<()>,
) -> Result<(), extism::Error> {
    let email: String = plugin.memory_get_val(&inputs[0])?;
    outputs[0] = Val::I64(if validate_email_address(&email) { 1 } else { 0 });
    Ok(())
}

pub fn host_sl_utils_validate_phone(
    plugin: &mut CurrentPlugin,
    inputs: &[Val],
    outputs: &mut [Val],
    _ud: UserData<()>,
) -> Result<(), extism::Error> {
    let phone: String = plugin.memory_get_val(&inputs[0])?;
    outputs[0] = Val::I64(if validate_phone_number(&phone) { 1 } else { 0 });
    Ok(())
}

// ── Number utils ──────────────────────────────────────────────────────────────

host_fn!(pub host_sl_utils_money_in_words(_ud: (); payload: Vec<u8>) -> Vec<u8> {
    let v: serde_json::Value = serde_json::from_slice(&payload).map_err(|e| anyhow!(e))?;
    let amount          = v["amount"].as_f64().unwrap_or(0.0);
    let main_currency   = v["currency"].as_str().unwrap_or("").to_string();
    let frac_currency   = v["fraction_currency"].as_str().unwrap_or("Cents").to_string();
    let result = money_in_words(amount, &main_currency, &frac_currency);
    Ok(result.into_bytes())
});

// ── Nested set utils (stubs — Phase 3: query DB) ──────────────────────────────

/// Returns a JSON array of ancestor names.
host_fn!(pub host_sl_utils_get_ancestors_of(_ud: (); doctype: String, name: String) -> Vec<u8> {
    debug!("sl_utils_get_ancestors_of({doctype}, {name}) — stub");
    serde_json::to_vec(&serde_json::json!([])).map_err(|e| anyhow!(e))
});

/// Returns a JSON array of descendant names.
host_fn!(pub host_sl_utils_get_descendants_of(_ud: (); doctype: String, name: String) -> Vec<u8> {
    debug!("sl_utils_get_descendants_of({doctype}, {name}) — stub");
    serde_json::to_vec(&serde_json::json!([])).map_err(|e| anyhow!(e))
});

// ── Build Function Registry ───────────────────────────────────────────────────

pub fn build_utils_host_functions() -> Vec<Function> {
    vec![
        Function::new("sl_utils_scrub",     [PTR], [PTR], UserData::default(), host_sl_utils_scrub),
        Function::new("sl_utils_unscrub",   [PTR], [PTR], UserData::default(), host_sl_utils_unscrub),
        // now/today take a dummy arg because host_fn! requires at least one arg with user_data form
        Function::new("sl_utils_now",       [PTR], [PTR], UserData::default(), host_sl_utils_now),
        Function::new("sl_utils_today",     [PTR], [PTR], UserData::default(), host_sl_utils_today),
        Function::new("sl_utils_add_days",  [PTR], [PTR], UserData::default(), host_sl_utils_add_days),
        Function::new("sl_utils_date_diff", [PTR], [PTR], UserData::default(), host_sl_utils_date_diff),
        Function::new("sl_utils_formatdate",[PTR], [PTR], UserData::default(), host_sl_utils_formatdate),
        Function::new("sl_utils_validate_email", [PTR], [PTR], UserData::default(), host_sl_utils_validate_email),
        Function::new("sl_utils_validate_phone",  [PTR], [PTR], UserData::default(), host_sl_utils_validate_phone),
        Function::new("sl_utils_money_in_words",  [PTR], [PTR], UserData::default(), host_sl_utils_money_in_words),
        Function::new("sl_utils_get_ancestors_of",   [PTR, PTR], [PTR], UserData::default(), host_sl_utils_get_ancestors_of),
        Function::new("sl_utils_get_descendants_of", [PTR, PTR], [PTR], UserData::default(), host_sl_utils_get_descendants_of),
    ]
}
