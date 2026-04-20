#![recursion_limit = "512"]

pub mod cli;
pub mod cleanup;
pub mod emit;
pub mod generate;
pub mod install_app;
pub mod migrate;
pub mod new_site;
pub mod seed_doctypes;
pub mod seed_finance_fixtures;
pub mod start;
pub mod use_site;
pub mod wire_app;
pub mod new_app;
pub mod pack_app;

// Re-export HTTP layer from spotledger-http
pub use spotledger_http::{serve, server};

// Non-Tier-0 DocTypes (Contacts, Geo, Desk, Printing, Automation) are now
// provided by the DB-native `spotledger-core` app (apps/spotledger-core/).
// They are seeded via `install-app` / `new-site` auto-install, not compiled in.
