//! PartyType DocType — kernel-level party registry.
//!
//! ## Why this exists
//!
//! `JournalEntryAccount` has `party_type` (who owes/is owed) and `party` (the
//! specific document name).  The possible party types — Customer, Supplier,
//! Employee — are defined in WASM plugins, not in the kernel.  Hardcoding those
//! names in the kernel would create an upward dependency (kernel → plugin).
//!
//! Instead, `PartyType` is a **kernel-level registry table** that plugins write
//! into at load time.  The accounting engine only ever talks to `PartyType`; it
//! never imports `Customer` or `Supplier` as Rust types.
//!
//! ## Who inserts rows
//!
//! The plugin host calls `PluginRegistry::register_capabilities()` after a
//! plugin loads.  That method reads `PluginManifest::party_types` and does an
//! `INSERT OR IGNORE` into this table for each declared party type.
//!
//! Core itself inserts nothing — if no plugin declares a party type, the table
//! remains empty and all party validation on JE rows fails explicitly.
//!
//! ## Validation flow (three tiers)
//!
//! See `party_validation.rs`.

use spotledger_core::meta::{DocField, DocTypeMeta, FieldType, Permission};

// ── AccountPartyType ──────────────────────────────────────────────────────────

/// Which side of the AR/AP ledger this party type sits on.
///
/// This is a **closed** enum because Receivable and Payable are double-entry
/// accounting primitives — you cannot invent a third one.  Plugins declare
/// which bucket their party type belongs to via [`PartyTypeDecl`].
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum AccountPartyType {
    Receivable,
    Payable,
}

impl AccountPartyType {
    pub fn as_str(&self) -> &'static str {
        match self {
            AccountPartyType::Receivable => "Receivable",
            AccountPartyType::Payable    => "Payable",
        }
    }
}

impl std::fmt::Display for AccountPartyType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

// ── DocTypeMeta ───────────────────────────────────────────────────────────────

/// Static DocTypeMeta for `PartyType`.
///
/// This is a **kernel table** — its schema is created by Tier 3 migrations
/// before any plugin loads.  Plugins never define this table; they only insert
/// rows into it.
pub fn meta() -> DocTypeMeta {
    DocTypeMeta::builder("PartyType", "Accounts")
        // `name` is the PK (e.g. "Customer", "Supplier", "Employee")
        .field(
            DocField::new("party_type", "Party Type", FieldType::Data)
                .required()
                .unique()
                .in_list()
                .in_standard_filter(),
        )
        // The actual DocType table to query for existence checks.
        // Usually the same as `name` but may differ.
        .field(
            DocField::new("doctype_name", "DocType Name", FieldType::Data)
                .required()
                .description("The DocType whose records serve as parties of this type"),
        )
        // Receivable or Payable — drives GL account selection.
        .field(
            DocField::new("account_type", "Account Type", FieldType::Select)
                .select_options("Receivable\nPayable")
                .required()
                .in_list()
                .in_standard_filter(),
        )
        // Which plugin registered this party type.
        .field(
            DocField::new("plugin_id", "Plugin ID", FieldType::Data)
                .required()
                .description("Plugin that registered this party type (e.g. \"selling\")")
                .in_list(),
        )
        .permission(Permission::full("System Manager"))
        .permission(Permission::read_only("Accounts Manager"))
        .permission(Permission::read_only("Accounts User"))
        .build()
}

// ── SurrealQL DDL ─────────────────────────────────────────────────────────────

/// Inline SurrealQL for the `party_type` table.
///
/// Embedded into the Tier 3 migration so it runs before any plugin loads.
pub const DDL: &str = r#"
-- PartyType: kernel-level registry of who can be a party on accounting entries.
-- Plugins INSERT rows at load time; the kernel never writes to this table itself.

DEFINE TABLE party_type SCHEMAFULL
    PERMISSIONS
        FOR select FULL
        FOR create, update, delete WHERE $auth.roles CONTAINS "System Manager";

DEFINE FIELD party_type  ON TABLE party_type TYPE string  ASSERT $value != NONE;
DEFINE FIELD doctype_name ON TABLE party_type TYPE string ASSERT $value != NONE;
DEFINE FIELD account_type ON TABLE party_type TYPE string
    ASSERT $value IN ["Receivable", "Payable"];
DEFINE FIELD plugin_id   ON TABLE party_type TYPE string  ASSERT $value != NONE;

DEFINE INDEX party_type_unique ON TABLE party_type COLUMNS party_type UNIQUE;
"#;
