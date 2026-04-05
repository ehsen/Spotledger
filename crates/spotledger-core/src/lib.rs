pub mod config;
pub mod document;
pub mod doctype;
pub mod error;
pub mod meta;
pub mod registry;
pub mod response;

// Re-export the most commonly used types at the crate root.
pub use config::{GlobalConfig, SiteConfig};
pub use document::{DocRow, DocStatus, Document};
pub use doctype::DocType;
pub use error::CoreError;
pub use meta::{DocField, DocTypeMeta, FieldKind, FieldType, LayoutKind, Permission};
pub use registry::{DocTypeEntry, DocTypeRegistry, DynamicDocument, DynamicMeta};
