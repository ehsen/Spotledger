# SpotledgerCore Architecture Plan

**Date**: April 5, 2026
**Status**: Approved — ready for execution
**Scope**: Full architectural rewrite from current monolithic Axum crate to a Linux-kernel-style modular WASM plugin system, written entirely in Rust.

---

## Executive Summary

**SpotledgerCore** is a **financial framework** — not a generic app framework. It is purpose-built for financial and enterprise applications. The core binary ships with the complete accounting engine compiled in as a mandatory tier, not as a plugin. Vertical domain modules (Selling, Buying, Stock, HR, Manufacturing, CRM) are WASM plugins loaded at runtime.

Key design decisions:
- **No Python. No JavaScript runtime (V8). No Rhai.** The system is Rust end-to-end.
- **The accounting engine is CORE** — GL, Journal Entry, Chart of Accounts, Fiscal Year, Currency, Cost Center, Tax infrastructure. These compile directly into the binary. You cannot run SpotledgerCore without them.
- **Modules are `.wasm` binaries** — dropped into a `plugins/` folder. Selling, Buying, Stock, HR are plugins. Core never recompiles when a module changes.
- **Frappe REST API parity** — `/api/resource/`, `/api/method/`, `/api/v2/` are preserved so the Frappe desk frontend works unchanged.
- **Linux kernel model** — core is the kernel; modules are loadable kernel modules, sandboxed via WASM.

### What is "Core"?

> Core = everything a financial ledger *must* have to be useful. You cannot build a Sales Invoice plugin without a GL engine. You cannot build a Payment Entry plugin without accounts and currency. Therefore the accounting engine is part of the kernel, not an add-on.

---

## Current State Inventory

```
crates/
  spotledger-types/    # DocType trait, Document struct, DocTypeRegistry — BECOMES spotledger-core
  spotledger-db/       # DbAdapter (SurrealDB), CRUD, hooks, permissions, auth — STAYS spotledger-db
  spotledger/          # Axum HTTP server, method registry — SPLITS into spotledger-http + thin binary
  spotledger-proxy/    # Recording proxy — REMOVED
```

**What works today:**
- Full Axum HTTP server with multi-site routing
- SurrealDB adapter with CRUD, list queries, submit/cancel
- Permission system (role-based)
- Session/auth (login, logout, PBKDF2 + Argon2 passwords)
- Method registry (`/api/method/` dispatcher)
- Desk handlers: `getdoc`, `getdoctype`, `savedocs`, `search_link`, `get_boot_info`, reportview, notifications, listview
- `frappe.client.*` core methods (get, get_list, save, insert, set_value, delete, submit, cancel)
- Hook registry (Rust-native hooks only)
- DocType trait with full lifecycle hooks
- Document caching (Moka)

**What is missing / stubbed:**
- `frappe.client.rename_doc`, `attach_file`, `validate_link`
- Full `/api/v2/` endpoints
- WASM plugin host
- Plugin migration runner
- Typed `DocTypeMeta` / `DocField` / `FieldType`
- `spotledger-pdk` guest crate for plugin authors
- Background jobs / scheduler
- File storage
- Full-text search
- **Accounting engine** (GL entries, Chart of Accounts, Journal Entry)
- **Financial DocTypes** (Company, Fiscal Year, Account, Cost Center, Currency)

---

## Target Architecture

```
spotledger/                              ← workspace root
├── Cargo.toml                           ← workspace manifest
│
├── crates/
│   ├── spotledger-core/                 ← THE KERNEL (Tier 0 types, replaces spotledger-types)
│   │   src/
│   │     lib.rs           → public re-exports
│   │     context.rs       → AppContext singleton (OnceLock), init()
│   │     document.rs      → Document struct + base methods
│   │     doctype.rs       → DocType trait + lifecycle hooks
│   │     meta.rs          → DocTypeMeta, DocField, FieldType (typed schema)
│   │     registry.rs      → DocTypeRegistry (compiled + dynamic + plugin)
│   │     naming.rs        → Name generation, naming series
│   │     children.rs      → Child table load/save
│   │     hooks.rs         → Hook dispatcher (Rust + WASM)
│   │     error.rs         → CoreError, ValidationError
│   │     doctypes/        → Tier 0 kernel DocTypes
│   │       doctype.rs     → DocType, DocField, DocPerm, CustomField
│   │       user.rs        → User, Role, HasRole, UserPermission
│   │       system.rs      → SystemSettings, DefaultValue
│   │       naming.rs      → DocumentNamingRule, DocumentNamingSettings
│   │     utils/
│   │       mod.rs
│   │       formatting.rs  → cint, flt, fmt_money, formatdate, format_datetime
│   │       strings.rs     → scrub, unscrub, cstr, strip_html
│   │       dates.rs       → now, today, add_days, date_diff, get_first_day
│   │       numbers.rs     → rounded, floor, ceil, money_in_words
│   │       validation.rs  → validate_email, validate_phone, validate_url
│   │       passwords.rs   → hash_password, check_password (moved from spotledger-db)
│   │       data.rs        → unique, flatten, group_by_field
│   │       nestedset.rs   → get_ancestors_of, get_descendants_of, rebuild_tree
│   │       json.rs        → parse_json, as_json
│   │     migrations/
│   │       runner.rs      → MigrationRunner (plugin-aware)
│   │       tier0.rs       → embedded SurrealQL for Tier 0 tables
│   │       schema.rs      → DEFINE TABLE/FIELD helpers
│   │
│   ├── spotledger-db/                   ← DB ADAPTER (keep crate name, keep source)
│   │   src/
│   │     lib.rs
│   │     adapter.rs       → DbAdapter
│   │     document.rs      → get_doc, get_list, insert, upsert, delete
│   │     query.rs         → WhereClause, SetClause
│   │     auth.rs          → session management (password funcs move to spotledger-core)
│   │     permissions.rs   → role-based permission check
│   │     naming.rs        → naming series counter
│   │     error.rs         → DbError
│   │     child_table.rs   → child table read/write
│   │     hooks.rs         → hook dispatcher
│   │     connection.rs    → connect() helper
│   │
│   ├── spotledger-desk/                 ← TIER 1 FRAMEWORK DOCTYPES (compiled in)
│   │   src/
│   │     lib.rs
│   │     doctypes/
│   │       workspace.rs     → Workspace, WorkspaceSidebar
│   │       todo.rs          → ToDo, Note, Event, Comment
│   │       file.rs          → File (schema + hooks)
│   │       communication.rs → Communication, NotificationLog
│   │       workflow.rs      → Workflow, WorkflowAction, WorkflowState
│   │       version.rs       → Version, AuditTrail, DeletedDocument
│   │       naming.rs        → DocumentNamingRule, NamingSeries
│   │     migrations/
│   │       tier1.rs         → embedded SurrealQL for Tier 1 tables
│   │
│   ├── spotledger-accounting/           ← TIER 3: CORE FINANCIAL ENGINE (compiled in, mandatory)
│   │   src/
│   │     lib.rs
│   │     doctypes/
│   │       company.rs       → Company
│   │       account.rs       → Account (lft/rgt nested set tree)
│   │       cost_center.rs   → CostCenter (tree)
│   │       fiscal_year.rs   → FiscalYear, FiscalYearCompany
│   │       currency.rs      → Currency, CurrencyExchange
│   │       tax.rs           → TaxCategory, TaxTemplate, TaxTemplateDetail
│   │       gl_entry.rs      → GLEntry (schema only; engine writes directly)
│   │       journal_entry.rs → JournalEntry, JournalEntryAccount
│   │       payment_terms.rs → PaymentTerms, PaymentTermsTemplate
│   │     engine/
│   │       gl.rs            → make_gl_entries(), validate_gl_balance(), reverse_gl_entries()
│   │       fiscal.rs        → get_fiscal_year(), get_fiscal_year_start_end()
│   │       currency.rs      → convert_to_base_currency(), get_exchange_rate()
│   │       coa.rs           → get_account_balance(), get_account_tree()
│   │     migrations/
│   │       tier3.rs         → embedded SurrealQL for all Tier 3 tables
│   │
│   ├── spotledger-automation/           ← TIER 2 OPTIONAL (feature flag)
│   │   src/
│   │     lib.rs
│   │     assignment.rs    → AssignmentRule
│   │     auto_repeat.rs   → AutoRepeat
│   │     milestone.rs     → Milestone, MilestoneTracker
│   │
│   ├── spotledger-http/                 ← HTTP LAYER (extracted from spotledger)
│   │   src/
│   │     lib.rs
│   │     resource.rs      → /api/resource/:doctype/:name (REST CRUD)
│   │     resource_v2.rs   → /api/v2/document/* endpoints
│   │     method.rs        → /api/method/:path dispatcher
│   │     middleware.rs    → session extraction, CORS
│   │     server.rs        → Axum app builder
│   │
│   ├── spotledger-plugins/              ← WASM PLUGIN HOST
│   │   src/
│   │     lib.rs
│   │     host.rs          → PluginRegistry: load .wasm, dispatch hooks
│   │     abi.rs           → host_fn! document operation exports (stable ABI)
│   │     abi_accounting.rs → host_fn! GL engine exports (stable ABI)
│   │     abi_utils.rs     → host_fn! utils exposed to plugins
│   │     migrations.rs    → per-plugin migration runner
│   │     extension.rs     → method extension, boot + workspace contributions
│   │     versioning.rs    → ABI version check, plugin dependency graph
│   │
│   └── spotledger-pdk/                  ← PLUGIN DEVELOPER KIT (guest side)
│       src/
│         lib.rs           → re-exports
│         api.rs           → sl_get_doc(), sl_save(), sl_throw()...
│         accounting.rs    → make_gl_entries(), get_fiscal_year(), get_account_balance()
│         utils.rs         → scrub, formatdate, validate_email, get_ancestors_of...
│         meta.rs          → DocTypeMeta, DocField, FieldType, Permission, Migration
│         macros.rs        → #[plugin_fn] re-exports + helpers
│
├── plugins/                             ← RUNTIME: .wasm files dropped here
│   ├── selling.wasm
│   ├── buying.wasm
│   ├── stock.wasm
│   ├── hrms.wasm
│   └── crm.wasm
│
└── apps/                                ← PLUGIN SOURCE (each compiles to .wasm)
    ├── selling/           → SalesOrder, SalesInvoice, Quotation, Customer, PriceList
    ├── buying/            → PurchaseOrder, PurchaseInvoice, Supplier
    ├── stock/             → Item, ItemGroup, Warehouse, StockEntry, DeliveryNote
    │                         StockLedgerEntry, PaymentEntry
    └── hrms/              → Employee, Department, PayrollEntry, LeavePolicy
```

---

## Four-Tier Classification

```
Tier 0 — Absolute kernel (spotledger-core, compiled in)
  Required before ANY plugin can load.

  DocType, DocField, DocPerm, CustomField, PropertySetter
  User, Role, HasRole, UserType, UserPermission, UserGroup
  DefaultValue, SystemSettings, Session
  _spotledger_migrations (internal tracking table)

Tier 1 — Framework UI DocTypes (spotledger-desk, compiled in)
  Required for the desk to function.

  Workspace, WorkspaceSidebar, WorkspaceLink, WorkspaceChart
  ToDo, Note, Event, Comment, Communication, Tag, TagLink
  File, Folder
  NotificationLog, NotificationSettings, Notification
  Workflow, WorkflowAction, WorkflowState, WorkflowTransition
  Version, AuditTrail, DeletedDocument, ViewLog
  DocumentNamingRule, DocumentNamingSettings, NamingSeries
  Language

Tier 2 — Optional framework modules (feature-gated, compiled in)
  spotledger-automation: AssignmentRule, AutoRepeat, Milestone, Reminder

Tier 3 — Financial Core (spotledger-accounting, compiled in, MANDATORY)
  The accounting engine. Cannot be disabled.
  All transactional plugins depend on these host functions being present.

  Company, FiscalYear, FiscalYearCompany
  Account (tree), CostCenter (tree)
  Currency, CurrencyExchange
  TaxCategory, TaxTemplate, TaxTemplateDetail
  GLEntry                          ← the ledger — written by engine only
  JournalEntry, JournalEntryAccount
  PaymentTerms, PaymentTermsTemplate

Tier 4 — Domain plugins (WASM, .wasm files in plugins/)
  Everything above the financial engine.

  selling.wasm:  SalesOrder, SalesInvoice, Quotation, Customer, PriceList
  buying.wasm:   PurchaseOrder, PurchaseInvoice, Supplier
  stock.wasm:    Item, ItemGroup, Warehouse, StockEntry, StockLedgerEntry,
                 DeliveryNote, PaymentEntry
  hrms.wasm:     Employee, Department, Designation, PayrollEntry, LeavePolicy
  crm.wasm:      Lead, Opportunity, Contact, Territory
```

### Why Accounting Is in Core (Not a Plugin)

WASM plugins load AFTER all tier migrations run. A plugin like `selling.wasm` calls `make_gl_entries()` inside its `on_submit` hook — that host function lives in `spotledger-accounting`. If the accounting engine were itself a plugin, a circular dependency would exist: `selling.wasm` depends on `accounting.wasm` being loaded first, but plugin load order is resolved after Tier 3 is already running. By compiling the accounting engine into the binary, it is always available as a host function before any plugin loads.

```
Startup order (enforced):
  1.  spotledger-core   Tier 0 migrations   ← DocType, User, Role, System
  2.  spotledger-desk   Tier 1 migrations   ← Workspace, File, Workflow, Version
  3.  Tier 2 optional migrations            ← AssignmentRule, AutoRepeat (if feature enabled)
  4.  spotledger-accounting Tier 3 migrations ← Company, Account, GLEntry, JournalEntry
  5.  PluginRegistry::load_all()            ← WASM plugins load here
  6.  Plugin MigrationRunner::run_all()     ← plugin-specific migrations
  7.  DocTypeRegistry::init()               ← all metas merged
  8.  HTTP server starts
```

---

## The Stable Host ABI

The contract between the binary and all plugins. **Never breaks.**

```rust
// spotledger-plugins/src/abi.rs — document operations

host_fn!(sl_get_doc(doctype: String, name: String) -> Vec<u8>)
host_fn!(sl_get_list(query_json: Vec<u8>) -> Vec<u8>)
host_fn!(sl_save_doc(doc_json: Vec<u8>) -> Vec<u8>)
host_fn!(sl_insert_doc(doc_json: Vec<u8>) -> Vec<u8>)
host_fn!(sl_delete_doc(doctype: String, name: String) -> ())
host_fn!(sl_submit_doc(doctype: String, name: String) -> ())
host_fn!(sl_cancel_doc(doctype: String, name: String) -> ())
host_fn!(sl_get_value(doctype: String, name: String, field: String) -> Vec<u8>)
host_fn!(sl_set_value(doctype: String, name: String, field: String, val: Vec<u8>) -> ())
host_fn!(sl_db_set(doctype: String, name: String, field: String, val: Vec<u8>) -> ())
host_fn!(sl_get_all(query_json: Vec<u8>) -> Vec<u8>)
host_fn!(sl_count(doctype: String, filters_json: Vec<u8>) -> i64)
host_fn!(sl_exists(doctype: String, name: String) -> bool)
host_fn!(sl_generate_name(doctype: String, autoname: String, doc_json: Vec<u8>) -> String)
host_fn!(sl_throw(message: String) -> ())
host_fn!(sl_log(level: String, message: String) -> ())
host_fn!(sl_has_permission(doctype: String, ptype: String, name: String) -> bool)
host_fn!(sl_rename_doc(doctype: String, old: String, new: String) -> ())
host_fn!(sl_enqueue(method: String, kwargs_json: Vec<u8>, delay_secs: u64) -> ())
```

```rust
// spotledger-plugins/src/abi_accounting.rs — GL engine (Tier 3, always available)

host_fn!(sl_make_gl_entries(entries_json: Vec<u8>) -> ())
host_fn!(sl_reverse_gl_entries(voucher_type: String, voucher_no: String) -> ())
host_fn!(sl_get_account_balance(account: String, date: String, cost_center: String) -> f64)
host_fn!(sl_get_fiscal_year(date: String, company: String) -> Vec<u8>)
host_fn!(sl_get_exchange_rate(from_currency: String, to_currency: String, date: String) -> f64)
host_fn!(sl_convert_to_base_currency(amount: f64, currency: String, date: String) -> f64)
host_fn!(sl_validate_account(account: String, company: String) -> bool)
host_fn!(sl_get_default_account(account_type: String, company: String) -> String)
```

```rust
// spotledger-plugins/src/abi_utils.rs

host_fn!(sl_utils_cint(val_json: Vec<u8>) -> i64)
host_fn!(sl_utils_flt(val_json: Vec<u8>, precision: i32) -> f64)
host_fn!(sl_utils_scrub(s: String) -> String)
host_fn!(sl_utils_unscrub(s: String) -> String)
host_fn!(sl_utils_formatdate(date: String, fmt: String) -> String)
host_fn!(sl_utils_now() -> String)
host_fn!(sl_utils_today() -> String)
host_fn!(sl_utils_add_days(date: String, days: i64) -> String)
host_fn!(sl_utils_date_diff(from: String, to: String) -> i64)
host_fn!(sl_utils_validate_email(email: String) -> bool)
host_fn!(sl_utils_validate_phone(phone: String) -> bool)
host_fn!(sl_utils_money_in_words(amount: f64, currency: String) -> String)
host_fn!(sl_utils_get_ancestors_of(doctype: String, name: String) -> Vec<u8>)
host_fn!(sl_utils_get_descendants_of(doctype: String, name: String) -> Vec<u8>)
```

---

## Typed Schema — DocTypeMeta

Static Rust representation of a DocType's schema and form layout. Lives in `spotledger-core/src/meta.rs`.

```rust
pub enum FieldType {
    Data, SmallText, Text, LongText, Code, Password,
    Int, Float, Currency, Percent,
    Date, Datetime, Time, Duration,
    Check,
    Select(&'static [&'static str]),
    Link(&'static str),             // options = target doctype name
    DynamicLink,
    Table(&'static str),            // options = child doctype name
    TableMultiSelect(&'static str),
    Attach, AttachImage, Signature, Geolocation,
    Json, Autocomplete, BarCode, Color, Rating, Phone,
}

pub enum LayoutKind {
    SectionBreak { label: &'static str },
    ColumnBreak,
    Tab { label: &'static str },
    Fold,
    Heading { content: &'static str },
    Html { content: &'static str },
}

pub enum FieldKind {
    Data(FieldType),
    Layout(LayoutKind),
}

pub struct DocField {
    pub idx:                  u16,
    pub fieldname:            &'static str,
    pub label:                &'static str,
    pub kind:                 FieldKind,
    pub reqd:                 bool,
    pub unique:               bool,
    pub read_only:            bool,
    pub hidden:               bool,
    pub in_list_view:         bool,
    pub in_filter:            bool,
    pub in_standard_filter:   bool,
    pub bold:                 bool,
    pub description:          Option<&'static str>,
    pub default:              Option<&'static str>,
    pub depends_on:           Option<&'static str>,
    pub mandatory_depends_on: Option<&'static str>,
    pub read_only_depends_on: Option<&'static str>,
    pub precision:            Option<u8>,
    pub length:               Option<u32>,
}

impl DocField {
    pub const fn is_layout_only(&self) -> bool {
        matches!(self.kind, FieldKind::Layout(_))
    }
    pub const fn frappe_fieldtype(&self) -> &'static str { /* returns exact desk JS string */ }

    pub const LAYOUT_DEFAULTS: DocField = /* all flags false/None */;
    pub const DATA_DEFAULTS: DocField   = /* all flags false/None */;
}

pub struct Permission {
    pub role:   &'static str,
    pub read:   bool, pub write:  bool, pub create: bool,
    pub delete: bool, pub submit: bool, pub cancel: bool, pub amend: bool,
}

pub struct DocTypeMeta {
    pub name:           &'static str,
    pub module:         &'static str,
    pub is_submittable: bool,
    pub is_single:      bool,
    pub is_child_table: bool,
    pub is_tree:        bool,
    pub track_changes:  bool,
    pub title_field:    Option<&'static str>,
    pub search_fields:  &'static [&'static str],
    pub fields:         &'static [DocField],
    pub permissions:    &'static [Permission],
}

impl DocTypeMeta {
    /// Iterator over data fields only (for DB schema sync — skips layout fields).
    pub fn data_fields(&self) -> impl Iterator<Item = &DocField>;
    /// All fields in idx order (for getdoctype response — includes layout fields).
    pub fn all_fields_ordered(&self) -> &[DocField];
    /// Serialize to the exact JSON shape the Frappe desk expects.
    pub fn to_frappe_json(&self) -> serde_json::Value;
}
```

**Key invariant**: `DocTypeMeta.fields` is the single source of truth for both form layout and data schema. Layout fields are emitted in `to_frappe_json()` (desk renders breaks/tabs correctly) but skipped when syncing DB columns and reading/writing Document values.

---

## Financial Core — Key DocType Definitions

```rust
// spotledger-accounting/src/doctypes/account.rs

pub static META: DocTypeMeta = DocTypeMeta {
    name: "Account", module: "Accounts",
    is_submittable: false, is_single: false, is_child_table: false,
    is_tree: true,          // Uses lft/rgt nested set — important for balance queries
    track_changes: false,
    title_field: Some("account_name"),
    search_fields: &["account_name", "account_number"],
    fields: &[
        DocField { idx: 1, fieldname: "account_name", label: "Account Name",
                   kind: FieldKind::Data(FieldType::Data), reqd: true, in_list_view: true,
                   ..DocField::DATA_DEFAULTS },
        DocField { idx: 2, fieldname: "account_number", label: "Account Number",
                   kind: FieldKind::Data(FieldType::Data), ..DocField::DATA_DEFAULTS },
        DocField { idx: 3, fieldname: "parent_account", label: "Parent Account",
                   kind: FieldKind::Data(FieldType::Link("Account")), ..DocField::DATA_DEFAULTS },
        DocField { idx: 4, fieldname: "root_type", label: "Root Type",
                   kind: FieldKind::Data(FieldType::Select(&[
                       "Asset", "Liability", "Equity", "Income", "Expense"
                   ])), reqd: true, ..DocField::DATA_DEFAULTS },
        DocField { idx: 5, fieldname: "account_type", label: "Account Type",
                   kind: FieldKind::Data(FieldType::Select(&[
                       "", "Accumulated Depreciation", "Asset Received But Not Billed",
                       "Bank", "Cash", "Chargeable", "Cost of Goods Sold", "Depreciation",
                       "Equity", "Expense Account", "Expenses Included In Asset Valuation",
                       "Expenses Included In Valuation", "Fixed Asset", "Income Account",
                       "Payable", "Receivable", "Round Off", "Stock", "Stock Adjustment",
                       "Stock Received But Not Billed", "Tax", "Temporary", "Write Off",
                   ])), ..DocField::DATA_DEFAULTS },
        DocField { idx: 6, fieldname: "company", label: "Company",
                   kind: FieldKind::Data(FieldType::Link("Company")), reqd: true,
                   ..DocField::DATA_DEFAULTS },
        DocField { idx: 7, fieldname: "is_group", label: "Is Group",
                   kind: FieldKind::Data(FieldType::Check), ..DocField::DATA_DEFAULTS },
        DocField { idx: 8, fieldname: "account_currency", label: "Account Currency",
                   kind: FieldKind::Data(FieldType::Link("Currency")), ..DocField::DATA_DEFAULTS },
        // lft, rgt, old_parent are managed by nestedset engine — present in meta for schema sync
        DocField { idx: 9, fieldname: "lft", label: "Left",
                   kind: FieldKind::Data(FieldType::Int), hidden: true, ..DocField::DATA_DEFAULTS },
        DocField { idx: 10, fieldname: "rgt", label: "Right",
                   kind: FieldKind::Data(FieldType::Int), hidden: true, ..DocField::DATA_DEFAULTS },
    ],
    permissions: &[
        Permission { role: "Accounts Manager", read: true, write: true, create: true,
                     delete: true, submit: false, cancel: false, amend: false },
        Permission { role: "Accounts User", read: true, write: false, create: false,
                     delete: false, submit: false, cancel: false, amend: false },
    ],
};
```

```rust
// spotledger-accounting/src/engine/gl.rs

pub struct GlEntry {
    pub posting_date:  String,
    pub account:       String,
    pub debit:         f64,
    pub credit:        f64,
    pub voucher_type:  String,
    pub voucher_no:    String,
    pub company:       String,
    pub cost_center:   Option<String>,
    pub fiscal_year:   String,
    pub remarks:       Option<String>,
    pub is_opening:    bool,
}

/// Create GL entries for a voucher. Called directly by JournalEntry on_submit
/// and exposed as host function sl_make_gl_entries for plugins.
pub async fn make_gl_entries(
    db: &DbAdapter,
    entries: Vec<GlEntry>,
) -> Result<(), CoreError> {
    for entry in &entries {
        validate_entry(entry)?;
        // Debit must equal Credit across the full voucher — validated at the voucher level
    }
    // Validate debit == credit sum
    let debit_total:  f64 = entries.iter().map(|e| e.debit).sum();
    let credit_total: f64 = entries.iter().map(|e| e.credit).sum();
    if (debit_total - credit_total).abs() > 0.001 {
        return Err(CoreError::Validation(
            format!("Debit ({debit_total}) must equal Credit ({credit_total})").into()
        ));
    }
    for entry in entries {
        insert_gl_entry(db, &entry).await?;
    }
    Ok(())
}

/// Reverse all GL entries for a voucher (called on cancel).
pub async fn reverse_gl_entries(
    db: &DbAdapter,
    voucher_type: &str,
    voucher_no: &str,
) -> Result<(), CoreError> {
    let entries = get_gl_entries_for_voucher(db, voucher_type, voucher_no).await?;
    let reversed: Vec<GlEntry> = entries.into_iter().map(|e| GlEntry {
        debit:  e.credit,
        credit: e.debit,
        remarks: Some(format!("Reversal of {voucher_type} {voucher_no}")),
        ..e
    }).collect();
    make_gl_entries(db, reversed).await
}
```

---

## Plugin Lifecycle

### Writing a Plugin (using spotledger-pdk)

```rust
// apps/selling/src/lib.rs
// Cargo.toml: crate-type = ["cdylib"], target = wasm32-unknown-unknown

use spotledger_pdk::*;
use spotledger_pdk::accounting::{make_gl_entries, GlEntry};

#[plugin_fn]
pub fn register_doctypes() -> FnResult<Json<Vec<DocTypeMeta>>> {
    Ok(Json(vec![
        sales_order::META.clone(),
        sales_invoice::META.clone(),
    ]))
}

#[plugin_fn]
pub fn migrations() -> FnResult<Json<Vec<Migration>>> {
    Ok(Json(vec![Migration {
        version: 1,
        description: "Initial selling schema",
        up: include_str!("migrations/001_initial.surql"),
        down: None,
    }]))
}

#[plugin_fn]
pub fn Sales_Order__validate(Json(doc): Json<Document>) -> FnResult<Json<Document>> {
    if doc.get_f64("grand_total").unwrap_or(0.0) <= 0.0 {
        sl_throw("Grand total must be positive")?;
    }
    Ok(Json(doc))
}

#[plugin_fn]
pub fn Sales_Invoice__on_submit(Json(doc): Json<Document>) -> FnResult<Json<Document>> {
    // Calls into Tier 3 accounting engine via stable ABI host function
    let entries = build_gl_entries_for_invoice(&doc)?;
    make_gl_entries(entries)?;   // → sl_make_gl_entries host fn
    Ok(Json(doc))
}

#[plugin_fn]
pub fn Sales_Invoice__on_cancel(Json(doc): Json<Document>) -> FnResult<Json<Document>> {
    spotledger_pdk::accounting::reverse_gl_entries("Sales Invoice", doc.name())?;
    Ok(Json(doc))
}
```

### Plugin Deployment

```bash
cargo build -p selling --target wasm32-unknown-unknown --release
cp target/wasm32-unknown-unknown/release/selling.wasm ./plugins/
systemctl restart spotledger
# Core binary unchanged. No recompile needed.
```

---

## Crate Name Mapping

| Old | New | Notes |
|-----|-----|-------|
| `spotledger-types` | `spotledger-core` | Kernel — DocType, meta, registry, utils |
| `spotledger-db` | `spotledger-db` | Keep — already well-named |
| `spotledger/` (HTTP) | `spotledger-http` | HTTP layer extracted to own crate |
| `spotledger/` (binary) | `spotledger` | Thin: just `main.rs` + `cli.rs` |
| `spotledger-proxy` | **removed** | Dev tool no longer needed |
| *(new)* | `spotledger-desk` | Tier 1 desk DocTypes |
| *(new)* | `spotledger-accounting` | Tier 3 financial core |
| *(new)* | `spotledger-automation` | Tier 2 optional |
| *(new)* | `spotledger-plugins` | WASM plugin host |
| *(new)* | `spotledger-pdk` | Plugin developer kit |

**REST API paths**: `/api/method/frappe.*` paths remain **unchanged** — they are the external contract with the Frappe desk frontend. Only internal Rust identifiers change.

---

## Migration Phases

### Phase 0 — Repository Restructure *(~1 week)*

**Goal**: New crate layout in place. All existing code compiles in new locations. Zero functionality changes.

**Tasks:**
- [ ] Create `crates/spotledger-core/` — move content from `spotledger-types/`
- [ ] Update `spotledger-db/` to depend on `spotledger-core` instead of `spotledger-types`
- [ ] Create `crates/spotledger-http/` — extract `server.rs`, `routes.rs`, `middleware.rs`, `methods/` from `spotledger/`
- [ ] Create `crates/spotledger-plugins/` — stub only (empty `PluginRegistry`)
- [ ] Create `crates/spotledger-pdk/` — stub only
- [ ] Create `crates/spotledger-desk/` — stub only
- [ ] Create `crates/spotledger-accounting/` — stub only
- [ ] Create `crates/spotledger-automation/` — stub only with feature flag
- [ ] `crates/spotledger/` — thin `main.rs` + `cli.rs` only
- [ ] Create `apps/selling/`, `apps/buying/`, `apps/stock/`, `apps/hrms/` — stub Cargo.toml (`crate-type = ["cdylib"]`)
- [ ] Remove `crates/spotledger-proxy/` from workspace
- [ ] Update workspace `Cargo.toml` members list
- [ ] `cargo build` passes with zero warnings

**Exit criterion**: `cargo build` clean. All tests pass.

---

### Phase 1 — Typed Schema (DocTypeMeta) *(~1 week)*

**Goal**: Static `DocTypeMeta` in `spotledger-core`. `getdoctype` for compiled types returns correct data without a DB query.

**Tasks:**
- [ ] Add `meta.rs` to `spotledger-core/src/` with `FieldType`, `LayoutKind`, `FieldKind`, `DocField`, `DocTypeMeta`, `Permission`
- [ ] `DocField::LAYOUT_DEFAULTS` and `DocField::DATA_DEFAULTS` const defaults
- [ ] `DocField::is_layout_only()` and `DocField::frappe_fieldtype()`
- [ ] `DocTypeMeta::data_fields()`, `all_fields_ordered()`, `to_frappe_json()`
- [ ] Extend `DocType` trait with `fn meta() -> &'static DocTypeMeta where Self: Sized`
- [ ] Extend `DocTypeRegistry` with `get_compiled_meta(name)` lookup
- [ ] Update `handle_getdoctype`: try compiled meta first (zero DB hit), fall back to DB rows
- [ ] DB sync rule: `data_fields()` only for `DEFINE FIELD` — never create columns for layout fields

**Exit criterion**: `getdoctype?doctype=User` returns correct shape with no `get_list(db, "DocField")` call.

---

### Phase 1b — Framework DocTypes (Tier 0 + Tier 1 + Tier 3) *(~3 weeks)*

**Goal**: All framework DocTypes are typed Rust structs. Tier 0/1/3 migrations run at startup. No `seed_doctypes.rs` needed.

**Tier 0 (spotledger-core):**
- [ ] `doctypes/doctype.rs` — DocType, DocField, DocPerm, CustomField, PropertySetter
- [ ] `doctypes/user.rs` — User (`validate` email, `before_save` → full_name, `on_update` → role sync)
- [ ] `doctypes/system.rs` — SystemSettings (single), DefaultValue
- [ ] `doctypes/naming.rs` — DocumentNamingRule, DocumentNamingSettings
- [ ] `migrations/tier0.rs` — embedded SurrealQL for Tier 0 tables

**Tier 1 (spotledger-desk):**
- [ ] `doctypes/workspace.rs` — Workspace, WorkspaceSidebar
- [ ] `doctypes/todo.rs` — ToDo, Note, Event, Comment
- [ ] `doctypes/file.rs` — File (schema + hooks; storage in Phase 8)
- [ ] `doctypes/communication.rs` — Communication, NotificationLog, Notification
- [ ] `doctypes/workflow.rs` — Workflow, WorkflowAction, WorkflowState, WorkflowTransition
- [ ] `doctypes/version.rs` — Version (`before_insert` captures diff), AuditTrail, DeletedDocument
- [ ] `migrations/tier1.rs` — embedded SurrealQL for Tier 1 tables

**Tier 3 (spotledger-accounting):**
- [ ] `doctypes/company.rs` — Company (`after_insert` → create default CoA, CostCenter)
- [ ] `doctypes/account.rs` — Account (nested set tree, `validate` no circular parent)
- [ ] `doctypes/cost_center.rs` — CostCenter (nested set tree)
- [ ] `doctypes/fiscal_year.rs` — FiscalYear (`validate` no overlap), FiscalYearCompany
- [ ] `doctypes/currency.rs` — Currency, CurrencyExchange
- [ ] `doctypes/tax.rs` — TaxCategory, TaxTemplate, TaxTemplateDetail
- [ ] `doctypes/gl_entry.rs` — GLEntry (schema only; no lifecycle hooks)
- [ ] `doctypes/journal_entry.rs` — JournalEntry (`on_submit` → `make_gl_entries()`, `on_cancel` → `reverse_gl_entries()`), JournalEntryAccount
- [ ] `doctypes/payment_terms.rs` — PaymentTerms, PaymentTermsTemplate
- [ ] `engine/gl.rs` — `make_gl_entries()`, `reverse_gl_entries()`, `validate_gl_balance()`
- [ ] `engine/fiscal.rs` — `get_fiscal_year()`, `get_fiscal_year_start_end()`
- [ ] `engine/currency.rs` — `get_exchange_rate()`, `convert_to_base_currency()`
- [ ] `engine/coa.rs` — `get_account_balance()`, `get_account_tree()`, `validate_account()`
- [ ] `migrations/tier3.rs` — embedded SurrealQL for all Tier 3 tables

**Exit criterion**: `spotledger new-site` seeds all tables. Company + Chart of Accounts creatable. `get_account_balance()` returns 0.

---

### Phase 1c — Core Utils *(~1 week, parallel with Phase 1b)*

- [ ] `utils/formatting.rs` — `cint`, `flt`, `fmt_money`, `formatdate`, `format_datetime`
- [ ] `utils/strings.rs` — `scrub`, `unscrub`, `cstr`, `strip_html`
- [ ] `utils/dates.rs` — `now`, `today`, `add_days`, `date_diff`, `get_first_day`, `get_last_day`
- [ ] `utils/numbers.rs` — `rounded`, `money_in_words`
- [ ] `utils/validation.rs` — `validate_email_address`, `validate_phone_number`, `validate_url`
- [ ] `utils/passwords.rs` — move `hash_password`, `check_password` from `spotledger-db/src/auth.rs`
- [ ] `utils/nestedset.rs` — `get_ancestors_of`, `get_descendants_of`, `rebuild_tree`, `validate_loop`
- [ ] `utils/data.rs` — `unique`, `flatten`, `group_by_field`
- [ ] `utils/json.rs` — `parse_json`, `as_json`
- [ ] Register HTTP-callable utils in `spotledger-http/src/method.rs`
- [ ] All modules have `#[cfg(test)]` unit tests

**Exit criterion**: `User::validate` uses `spotledger_core::utils::validation::validate_email_address`. All unit tests pass.

---

### Phase 2 — WASM Plugin Host *(~2 weeks)*

**Goal**: Core can load a `.wasm` file, run `register_doctypes()`, `migrations()`, and dispatch lifecycle hooks. Accounting host functions are available to plugins.

**Tasks:**
- [ ] `extism = "1.0"` dependency in `spotledger-plugins/Cargo.toml`
- [ ] Implement `PluginRegistry` — `load_all()`, `dispatch_hook()`, `dispatch_method()`, `register_doctypes()`
- [ ] Implement all `host_fn!` exports: `abi.rs` (doc ops) + `abi_accounting.rs` (GL engine) + `abi_utils.rs`
- [ ] `extension.rs` — collect method registrations, boot contributions, workspaces, scheduled jobs, cross-hooks
- [ ] `versioning.rs` — ABI version check, topological plugin load order
- [ ] `MigrationRunner` for plugin-specific migrations
- [ ] Wire `PluginRegistry::dispatch_hook` into `HookRegistry` chain
- [ ] Update HTTP dispatcher to try plugins after core misses
- [ ] Minimal test plugin (`apps/test_plugin`) — declares DocType, hook, calls `sl_get_account_balance`
- [ ] Integration test: load plugin → migrations run → doctype appears → hook fires → accounting host fn works

**Exit criterion**: `selling.wasm` drops into `plugins/` → restart → `SalesOrder` appears → `Sales_Order__validate` fires → `sl_make_gl_entries` callable from plugin.

---

### Phase 3 — spotledger-pdk *(~1 week)*

**Goal**: Plugin authors depend only on `spotledger-pdk`. Zero direct `extism-pdk` references in app code.

**Tasks:**
- [ ] `api.rs` — document operation wrappers: `sl_get_doc()`, `sl_save()`, `sl_insert()`, `sl_delete()`, `sl_submit()`, `sl_cancel()`, `sl_get_value()`, `sl_set_value()`, `sl_get_all()`, `sl_exists()`, `sl_throw()`, `sl_log()`, `sl_rename_doc()`, `sl_has_permission()`
- [ ] `accounting.rs` — GL wrappers: `make_gl_entries()`, `reverse_gl_entries()`, `get_account_balance()`, `get_fiscal_year()`, `get_exchange_rate()`, `convert_to_base_currency()`, `validate_account()`, `get_default_account()`
- [ ] `utils.rs` — formatting, validation, date, nestedset wrappers
- [ ] `extension.rs` — `MethodRegistration`, `ScheduledJob`, `CrossHook`, `WorkspaceDefinition` structs
- [ ] Re-export `meta::*` and `extism_pdk::{plugin_fn, FnResult, Json}`
- [ ] `apps/selling` compiles using ONLY `spotledger-pdk`

**Exit criterion**: `apps/selling/Cargo.toml` has no direct `extism-pdk` dep. `make_gl_entries()` callable without `unsafe`.

---

### Phase 4 — Domain Plugins (Selling, Buying, Stock) *(~4 weeks)*

**Selling (apps/selling):**
- [ ] Customer, PriceList + PriceListItem
- [ ] SalesOrder + SalesOrderItem (`validate`, `before_save` totals, `on_submit` status)
- [ ] SalesInvoice + SalesInvoiceItem (`on_submit` → `make_gl_entries()`, `on_cancel` → `reverse_gl_entries()`)
- [ ] Quotation + QuotationItem
- [ ] DeliveryNote + DeliveryNoteItem

**Buying (apps/buying):**
- [ ] Supplier
- [ ] PurchaseOrder + PurchaseOrderItem
- [ ] PurchaseInvoice + PurchaseInvoiceItem (`on_submit` → `make_gl_entries()`)
- [ ] PurchaseReceipt + PurchaseReceiptItem

**Stock (apps/stock):**
- [ ] Item, ItemGroup (tree — uses `sl_utils_get_ancestors_of`), UOM, Warehouse
- [ ] StockEntry + StockEntryDetail (`on_submit` → `make_stock_ledger_entries()` + `make_gl_entries()`)
- [ ] StockLedgerEntry (schema; written by stock engine only)
- [ ] PaymentEntry + PaymentEntryReference (`on_submit` → `make_gl_entries()`)

**Accounts supplementary (apps/accounts):**
- [ ] Period Closing Voucher (`on_submit` → close fiscal period)
- [ ] Bank Reconciliation Statement
- [ ] Purchase Taxes and Charges Template, Sales Taxes and Charges Template

**Exit criterion**: Create a Sales Invoice → submit → GL entries appear in `tabGL_Entry`. Cancel → reversal entries appear. Create a Stock Entry → StockLedgerEntry rows created.

---

### Phase 5 — frappe.client API Completion *(~1 week)*

- [ ] `frappe.client.rename_doc` — cascading FK rewrite via SurrealQL
- [ ] `frappe.client.attach_file` — store in `tabFile`, return URL
- [ ] `frappe.client.validate_link` — check linked doc exists + permission
- [ ] `frappe.client.bulk_update` — batch field updates
- [ ] `frappe.client.has_permission` — delegate to permissions module
- [ ] `frappe.client.get_doc_permissions` — full permission object

---

### Phase 6 — API v2 Endpoints *(~1 week)*

- [ ] `GET    /api/v2/document/{doctype}` — list with advanced filter grammar
- [ ] `GET    /api/v2/document/{doctype}/{name}` — get single doc
- [ ] `POST   /api/v2/document/{doctype}` — insert
- [ ] `PATCH  /api/v2/document/{doctype}/{name}` — partial update
- [ ] `DELETE /api/v2/document/{doctype}/{name}` — delete
- [ ] `POST   /api/v2/document/{doctype}/{name}/run_doc_method`
- [ ] `GET    /api/v2/meta/{doctype}` — DocTypeMeta response
- [ ] `GET    /api/v2/document/{doctype}/count`

---

### Phase 7 — Background Jobs *(~2 weeks)*

SurrealDB as queue backend (no Redis dependency).

```
tabBackgroundJob:
  status: "queued" | "running" | "failed" | "completed"
  method: String, kwargs: JSON, plugin: String, scheduled_at: Datetime

Tokio worker:
  Polls every N seconds → claims job → dispatches → records result
```

- [ ] `tabBackgroundJob` Tier 0 migration
- [ ] `enqueue()` in `spotledger-core`
- [ ] `sl_enqueue()` host function in ABI
- [ ] Tokio background worker (spawned at startup)
- [ ] `tabScheduledJobType` — cron definitions
- [ ] Scheduler loop — tick every minute
- [ ] PDK: `spotledger_pdk::enqueue(method, kwargs, delay)`

---

### Phase 8 — File Storage *(~1 week)*

- [ ] `tabFile` DocType in `spotledger-desk`
- [ ] `POST /api/method/upload_file` — store binary, return URL
- [ ] `GET  /files/:path` — public files from disk
- [ ] `GET  /private/files/:path` — private files (permission-gated)
- [ ] `frappe.client.attach_file` wired to file storage
- [ ] `LocalStorage` backend trait (extensible to S3)
- [ ] `sl_upload_file()` host function in ABI

---

### Phase 9 — Full-Text Search *(~1 week)*

- [ ] SurrealDB FTS (`SEARCH ANALYZER`) for document search
- [ ] `frappe.desk.search.search_link` — complete implementation
- [ ] `frappe.utils.global_search.search` — global search
- [ ] Build search index on `after_save` hook

---

### Phase 10 — Notifications & Real-time *(~1 week)*

- [ ] Complete `NotificationLog` CRUD
- [ ] `frappe.desk.notifications.get_notification_info` — complete
- [ ] WebSocket endpoint (Axum `ws://`)
- [ ] Push notification on doc save/submit

---

## Framework DocType Table (Authoritative)

### spotledger-core (Tier 0)

| DocType | Critical Hooks |
|---------|---------------|
| `DocType` | `validate`, `on_update` (schema sync) |
| `DocField` | `validate`, `before_save` |
| `DocPerm` | minimal |
| `CustomField` | `validate`, `on_update`, `on_trash` |
| `PropertySetter` | `validate`, `on_update` |
| `User` | `validate`, `before_save`, `on_update` (role sync), `on_trash` |
| `Role` | `validate` |
| `HasRole` | minimal (child table) |
| `UserPermission` | `validate`, `on_update`, `on_trash` |
| `UserGroup` | `validate` |
| `UserGroupMember` | minimal (child table) |
| `UserType` | `validate`, `on_update` |
| `DefaultValue` | minimal |
| `SystemSettings` | `validate` (single) |
| `DocumentNamingRule` | `validate`, `apply` hook |
| `DocumentNamingSettings` | `validate` (single) |

### spotledger-desk (Tier 1)

| DocType | Critical Hooks |
|---------|---------------|
| `Workspace` | `validate` |
| `WorkspaceSidebar` | minimal |
| `ToDo` | `validate`, `on_update`, `on_trash` |
| `Note` | `validate` |
| `Event` | `validate` |
| `Comment` | `validate`, `on_update` |
| `Communication` | `validate`, `on_update` |
| `File` | `validate`, `before_insert`, `on_update`, `on_trash` |
| `NotificationLog` | minimal |
| `NotificationSettings` | minimal |
| `Notification` | `validate`, `on_update` |
| `Workflow` | `validate`, `on_update` |
| `WorkflowAction` | `on_update` |
| `WorkflowState` | minimal |
| `Version` | `before_insert` (capture field diff) |
| `AuditTrail` | `on_update`, `on_cancel` |
| `DeletedDocument` | `restore` (whitelist method) |
| `ViewLog` | minimal |
| `Tag` | minimal |
| `TagLink` | minimal |
| `Language` | minimal |

### spotledger-accounting (Tier 3)

| DocType | Critical Hooks |
|---------|---------------|
| `Company` | `validate`, `after_insert` (create default CoA, cost center) |
| `Account` | `validate` (no circular parent), `on_update` (rename cascades), nested set |
| `CostCenter` | `validate`, nested set |
| `FiscalYear` | `validate` (no overlap), `on_update` |
| `FiscalYearCompany` | minimal (child table) |
| `Currency` | `validate` |
| `CurrencyExchange` | `validate` |
| `TaxCategory` | `validate` |
| `TaxTemplate` | `validate` |
| `TaxTemplateDetail` | minimal (child table) |
| `GLEntry` | **no lifecycle hooks** — written by engine only |
| `JournalEntry` | `validate`, `before_save`, `on_submit` → `make_gl_entries()`, `on_cancel` → `reverse_gl_entries()` |
| `JournalEntryAccount` | `validate` (child table) |
| `PaymentTerms` | `validate` |
| `PaymentTermsTemplate` | `validate` |

### spotledger-automation (Tier 2, optional)

| DocType | Critical Hooks |
|---------|---------------|
| `AssignmentRule` | `validate`, `on_update`, `bulk_apply` |
| `AutoRepeat` | `validate`, `on_submit`, `on_cancel` |
| `Milestone` | `on_update`, `on_cancel` |
| `MilestoneTracker` | `validate`, `on_update` |
| `Reminder` | `validate` |

---

## Utils Sub-Module Map

```
spotledger-core/src/utils/
  mod.rs           ← re-exports
  formatting.rs    ← cint, flt, fmt_money, formatdate, format_datetime, format_duration
  strings.rs       ← scrub, unscrub, cstr, strip_html, camel_case_to_words, slug
  dates.rs         ← now, today, nowdate, add_days, add_months, date_diff,
                      get_first_day, get_last_day, getdate, get_datetime, get_time_zone
  numbers.rs       ← rounded, floor, ceil, money_in_words
  validation.rs    ← validate_email_address, validate_phone_number, validate_url,
                      validate_name, is_html, is_image
  passwords.rs     ← hash_password, check_password, generate_password_reset_key
  data.rs          ← unique, flatten, group_by_field, chunk
  nestedset.rs     ← get_ancestors_of, get_descendants_of, rebuild_tree, validate_loop
  file.rs          ← get_url, get_files_path, random_string, get_gravatar
  json.rs          ← parse_json, as_json
```

### HTTP-Callable Utils

| Method Path | Handler |
|-------------|---------|
| `frappe.utils.boot.get_boot_info` | exists |
| `frappe.utils.global_search.search` | Phase 9 |
| `frappe.utils.formatdate` | `utils::formatting::formatdate` |
| `frappe.utils.now` | `utils::dates::now` |
| `frappe.utils.validate_url` | `utils::validation::validate_url` |
| `frappe.utils.nestedset.get_ancestors_of` | `utils::nestedset::get_ancestors_of` |
| `frappe.utils.nestedset.get_descendants_of` | `utils::nestedset::get_descendants_of` |

---

## WASM Extension Rules

### Permitted Extension Points

1. **Method registry** — `register_methods()` → mounted under `/api/method/`. `frappe.*` namespace reserved for core.
2. **DocType registration** — `register_doctypes()` → `Vec<DocTypeMeta>`
3. **Lifecycle hooks** — `{DocType_snake}__{event}` exports
4. **Boot contributions** — `get_boot_contribution()` → merged into `get_boot_info`
5. **Workspace contributions** — `register_workspaces()` → sidebar entries
6. **Background jobs** — `sl_enqueue()` host fn + `register_scheduled_jobs()`
7. **Cross-plugin hooks** — `register_cross_hooks()` → interest in other doctypes' events

### Not Permitted

- Override core host functions (stable ABI is append-only)
- Claim `frappe.*` or another plugin's method namespace
- Write directly to `tabGL_Entry` — must use `sl_make_gl_entries()` host fn
- Load after Tier 3 accounting (sequential startup enforced; plugins always load last)
- Access filesystem or network directly (WASM sandbox; only host functions permitted)

---

## Security Model

- WASM sandbox provides memory isolation between plugins
- Each plugin runs in its own `extism::Plugin` instance with no shared memory
- Host functions are the only egress from the sandbox
- `sl_has_permission()` must be called by plugins before accessing sensitive data
- `frappe.*` HTTP namespace validated at dispatch — plugins cannot register `frappe.client.*` handlers
- OWASP: all user input validated server-side; no SQL interpolation (SurrealDB binding params used throughout)
