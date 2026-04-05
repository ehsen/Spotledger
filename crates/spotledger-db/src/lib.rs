pub mod adapter;
pub mod auth;
pub mod child_table;
pub mod connection;
pub mod document;
pub mod error;
pub mod hooks;
pub mod naming;
pub mod permissions;
pub mod query;

// Re-export the primary handle so callers only need to import one type.
pub use adapter::DbAdapter;
