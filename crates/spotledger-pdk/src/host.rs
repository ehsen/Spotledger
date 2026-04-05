//! Guest-side bindings for SpotledgerCore host functions.
//!
//! Plugins use these helpers to call back to the host for document operations,
//! GL entries, and other framework services.
//!
//! All functions use a linear-memory ABI: serialize request to JSON,
//! pass pointers to plugin memory, host fills response, deserialize.

#[cfg(target_arch = "wasm32")]
extern "C" {
    // Document operations
    fn spotledger_sl_get_doc(doctype_ptr: u64, doctype_len: u32, name_ptr: u64, name_len: u32) -> i32;
    fn spotledger_sl_save_doc(payload_ptr: u64, payload_len: u32) -> i32;
    fn spotledger_sl_delete_doc(doctype_ptr: u64, doctype_len: u32, name_ptr: u64, name_len: u32) -> i32;
    fn spotledger_sl_get_value(doctype_ptr: u64, doctype_len: u32, name_ptr: u64, name_len: u32, field_ptr: u64, field_len: u32) -> i32;
    fn spotledger_sl_has_permission(user_ptr: u64, user_len: u32, action_ptr: u64, action_len: u32, doctype_ptr: u64, doctype_len: u32) -> i32;
    fn spotledger_sl_throw_error(msg_ptr: u64, msg_len: u32);
    fn spotledger_sl_log(level_ptr: u64, level_len: u32, msg_ptr: u64, msg_len: u32);

    // Accounting operations
    fn spotledger_sl_make_gl_entries(payload_ptr: u64, payload_len: u32) -> i32;
    fn spotledger_sl_reverse_gl_entries(voucher_type_ptr: u64, voucher_type_len: u32, voucher_name_ptr: u64, voucher_name_len: u32) -> i32;
    fn spotledger_sl_get_account_balance(account_ptr: u64, account_len: u32, company_ptr: u64, company_len: u32, date_ptr: u64, date_len: u32) -> i32;
    fn spotledger_sl_get_fiscal_year(company_ptr: u64, company_len: u32, date_ptr: u64, date_len: u32) -> i32;
    fn spotledger_sl_get_exchange_rate(from_currency_ptr: u64, from_currency_len: u32, to_currency_ptr: u64, to_currency_len: u32, date_ptr: u64, date_len: u32) -> i32;
}

/// Write a message to the host trace log.
pub fn log(level: &str, msg: &str) {
    #[cfg(target_arch = "wasm32")]
    unsafe {
        spotledger_sl_log(
            level.as_ptr() as u64, level.len() as u32,
            msg.as_ptr() as u64, msg.len() as u32,
        )
    }
    #[cfg(not(target_arch = "wasm32"))]
    eprintln!("[spotledger-pdk stub] {}:{}", level, msg);
}

/// Throw a validation error that stops document processing.
pub fn throw_error(msg: &str) -> ! {
    #[cfg(target_arch = "wasm32")]
    unsafe {
        spotledger_sl_throw_error(msg.as_ptr() as u64, msg.len() as u32)
    }
    #[cfg(not(target_arch = "wasm32"))]
    panic!("Plugin error: {}", msg);
}
