pub mod adapter;
pub mod apply_surql;
pub mod auth;
pub mod bootstrap;
pub mod connection;
pub mod controller;
pub mod document;
pub mod doctype_save;
pub mod doctype_validate;
pub mod error;
pub mod graph_ops;
pub mod hooks;
pub mod meta_cache;
pub mod migrations;
pub mod naming;
pub mod permissions;
pub mod pipeline;
pub mod query;
pub mod save_proxy;
pub mod schema;

// Re-export the primary handle so callers only need to import one type.
pub use adapter::DbAdapter;
pub use doctype_save::{save_doctype, DoctypeSaveInput, DocFieldInput, DocPermInput, DoctypeSaveError,
                       parse_docfield_from_value, parse_docperm_from_value};
