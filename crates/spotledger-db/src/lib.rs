pub mod adapter;
pub mod auth;
pub mod bootstrap;
pub mod connection;
pub mod controller;
pub mod document;
pub mod error;
pub mod graph_ops;
pub mod hooks;
pub mod meta_cache;
pub mod migrations;
pub mod naming;
pub mod permissions;
pub mod query;
pub mod save_proxy;
pub mod schema;

// Re-export the primary handle so callers only need to import one type.
pub use adapter::DbAdapter;
