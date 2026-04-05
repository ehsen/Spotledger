//! Selling domain plugin — SpotledgerCore Tier 4 WASM app.
//!
//! DocTypes provided by this plugin:
//! - Customer, Customer Group, Territory
//! - Sales Order, Sales Order Item
//! - Sales Invoice, Sales Invoice Item
//! - Delivery Note, Delivery Note Item
//! - Quotation, Quotation Item
//! - Price List, Item Price
//! - Sales Taxes and Charges Template
//! - Shipping Rule

/// Plugin entry point called by the SpotledgerCore host at load time.
#[no_mangle]
pub extern "C" fn sl_plugin_init() {
    // TODO: register doctypes via sl_register_doctype() host call
}
