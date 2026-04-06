pub mod config;
pub mod document;
pub mod doctype;
pub mod error;
pub mod meta;
pub mod registry;
pub mod response;
pub mod validation;
pub mod doctypes;
pub mod migrations;
pub mod utils;

// Re-export the most commonly used types at the crate root.
pub use config::{GlobalConfig, SiteConfig};
pub use document::{DocRow, DocStatus, Document};
pub use doctype::DocType;
pub use error::CoreError;
pub use meta::{DocField, DocTypeMeta, FieldKind, FieldType, LayoutKind, Permission};
pub use registry::{DocTypeEntry, DocTypeRegistry, DynamicDocument, DynamicMeta, MetaEntry};

// Re-export high-use utils at the crate root for ergonomics.
pub use utils::{
    cint, cstr, flt, fmt_money, formatdate, format_datetime, format_duration,
    scrub, unscrub, strip_html,
    now, today, add_days, date_diff, get_first_day, get_last_day,
    rounded, money_in_words,
    validate_email_address, validate_phone_number, validate_url,
    hash_password, check_password,
    unique, flatten, group_by_field,
    parse_json, as_json,
};
