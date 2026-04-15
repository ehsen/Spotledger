#![recursion_limit = "512"]

pub mod cli;
pub mod cleanup;
pub mod emit;
pub mod generate;
pub mod install_app;
pub mod migrate;
pub mod new_site;
pub mod seed_doctypes;
pub mod start;
pub mod use_site;
pub mod wire_app;

// Re-export HTTP layer from spotledger-http
pub use spotledger_http::{serve, server};

// Force-link DocType registration crates so that their inventory::submit!
// statics are included by the Windows linker.
#[allow(dead_code)]
const _LINKED_DOCTYPES: &[&str] = &[
    spotledger_geo::name(),
    spotledger_contacts::name(),
    spotledger_printing::name(),
    spotledger_automation::name(),
    spotledger_desk::name(),
];
