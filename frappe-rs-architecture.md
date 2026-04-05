# frappe-rs — Clean Architecture Guide

> A Frappe Framework reimplementation in Rust.  
> Single binary. Zero Python. Zero Node. Zero Redis.  
> This document is the authoritative guide on how the core should be structured — idiomatic, testable, with clean separation of concerns.

---

## Guiding Principles

1. **Ugly is fine — but ugly must be quarantined.** Framework internals can be complex. App-developer-facing code must be clean.
2. **One responsibility per file.** The DB adapter does not validate. The registry does not query. The hooks do not serialize.
3. **Types over stringly-typed code.** `DocStatus::Submitted` not `"1"`. `FieldType::Link` not `"Link"`.
4. **No defensive code the compiler already handles.** No null checks on `String`. No type coercions on typed fields.
5. **Testable by default.** Every layer is injectable. No hidden globals in hot paths.

---

## Repository Layout

```
frappe-rs/
├── crates/
│   ├── frappe-core/          # The framework — app devs depend on this
│   │   ├── src/
│   │   │   ├── lib.rs        # Public API: get_doc, save, get_list, submit
│   │   │   ├── context.rs    # FrappeContext, OnceLock singleton, init()
│   │   │   ├── document.rs   # Document struct + base methods
│   │   │   ├── doctype.rs    # DocType trait + Hooks trait
│   │   │   ├── registry.rs   # DocTypeRegistry — compiled + dynamic
│   │   │   ├── naming.rs     # Name generation, naming series
│   │   │   ├── children.rs   # Child table load/save
│   │   │   └── error.rs      # FrappeError, ValidationError
│   │   └── Cargo.toml
│   │
│   ├── frappe-db/            # DB adapter — isolated SurrealDB concern
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── adapter.rs    # DbAdapter — single point of DB contact
│   │   │   ├── crud.rs       # get_doc, get_list, insert, upsert, delete
│   │   │   ├── query.rs      # WhereClause, SetClause types
│   │   │   ├── docstatus.rs  # submit_doc, cancel_doc, transition_docstatus
│   │   │   └── helpers.rs    # doctype_to_table, value_to_document
│   │   └── Cargo.toml
│   │
│   ├── frappe-macros/        # Proc macros — #[doctype], impl_document!
│   │   ├── src/
│   │   │   └── lib.rs
│   │   └── Cargo.toml
│   │
│   └── frappe-http/          # Axum HTTP layer — REST API
│       ├── src/
│       │   ├── lib.rs
│       │   ├── resource.rs   # /api/resource/:doctype/:name
│       │   └── method.rs     # /api/method/:dotted.path
│       └── Cargo.toml
│
└── apps/
    └── your-app/             # App developer code lives here
        ├── src/
        │   ├── main.rs
        │   └── doctypes/
        │       ├── sales_order.rs
        │       └── customer.rs
        └── Cargo.toml
```

---

## Layer 1 — `frappe-db`: The DB Adapter

This crate owns **all SurrealDB interaction**. Nothing outside this crate calls `.bind()` or `.query()`.

### `adapter.rs` — The Single Chokepoint

```rust
use serde_json::Value;
use surrealdb::{engine::remote::ws::Client, Surreal};
use crate::error::DbError;
use crate::helpers::is_table_not_found;

/// Every SurrealDB call goes through here.
/// No other file in the codebase calls db.query() directly.
pub struct DbAdapter {
    db: Surreal<Client>,
}

impl DbAdapter {
    pub async fn connect(url: &str) -> Result<Self, DbError> {
        let db = Surreal::new::<surrealdb::engine::remote::ws::Ws>(url).await?;
        db.signin(surrealdb::opt::auth::Root {
            username: "root",
            password: "root",
        })
        .await?;
        db.use_ns("frappe").use_db("frappe").await?;
        Ok(Self { db })
    }

    /// Execute SQL, return all rows. Returns empty vec if table doesn't exist.
    pub(crate) async fn run(
        &self,
        sql: &str,
        bindings: Vec<(String, Value)>,
    ) -> Result<Vec<Value>, DbError> {
        let mut q = self.db.query(sql);
        for (k, v) in bindings {
            q = q.bind((k, v));
        }
        match q.await {
            Err(e) if is_table_not_found(&e) => Ok(vec![]),
            Err(e) => Err(DbError::Surreal(e)),
            Ok(mut resp) => match resp.take::<Vec<Value>>(0) {
                Err(e) if is_table_not_found(&e) => Ok(vec![]),
                Err(e) => Err(DbError::Surreal(e)),
                Ok(rows) => Ok(rows),
            },
        }
    }

    /// Execute SQL, expect exactly one row. Errors with NotFound if empty.
    pub(crate) async fn run_one(
        &self,
        sql: &str,
        bindings: Vec<(String, Value)>,
        doctype: &str,
        name: &str,
    ) -> Result<Value, DbError> {
        self.run(sql, bindings)
            .await?
            .into_iter()
            .next()
            .ok_or_else(|| DbError::NotFound {
                doctype: doctype.into(),
                name: name.into(),
            })
    }

    /// Execute with no return value (DELETE, UPDATE without RETURN AFTER).
    pub(crate) async fn execute(
        &self,
        sql: &str,
        bindings: Vec<(String, Value)>,
    ) -> Result<(), DbError> {
        let mut q = self.db.query(sql);
        for (k, v) in bindings {
            q = q.bind((k, v));
        }
        q.await.map(|_| ()).map_err(DbError::Surreal)
    }
}
```

### `query.rs` — SQL Fragments as Types

Never return `(String, Vec<(String, Value)>)` tuples. Give them names.

```rust
use serde_json::Value;

/// A compiled WHERE clause with its parameter bindings.
pub struct WhereClause {
    sql:      String,
    bindings: Vec<(String, Value)>,
}

impl WhereClause {
    pub fn from_filters(filters: Option<&Value>) -> Self {
        let Some(Value::Object(map)) = filters else {
            return Self { sql: String::new(), bindings: vec![] };
        };

        let (conditions, bindings): (Vec<_>, Vec<_>) = map
            .iter()
            .map(|(field, val)| {
                let key = format!("f_{field}");
                let (op, bind_val) = match val {
                    Value::Array(arr) if arr.len() == 2 => {
                        (arr[0].as_str().unwrap_or("=").to_string(), arr[1].clone())
                    }
                    other => ("=".to_string(), other.clone()),
                };
                (format!("`{field}` {op} ${key}"), (key, bind_val))
            })
            .unzip();

        let sql = if conditions.is_empty() {
            String::new()
        } else {
            format!(" WHERE {}", conditions.join(" AND "))
        };

        Self { sql, bindings }
    }

    pub fn as_sql(&self)      -> &str               { &self.sql }
    pub fn bindings(&self)    -> &[(String, Value)]  { &self.bindings }
    pub fn is_empty(&self)    -> bool                { self.sql.is_empty() }
}

const SKIP_FIELDS: &[&str] = &["name", "doctype", "modified", "creation", "id"];

/// A compiled SET clause with its parameter bindings.
pub struct SetClause {
    sql:      String,
    bindings: Vec<(String, Value)>,
}

impl SetClause {
    pub fn from_fields(fields: &Value) -> Self {
        let Value::Object(map) = fields else {
            return Self { sql: "nothing = NONE".into(), bindings: vec![] };
        };

        let (parts, bindings): (Vec<_>, Vec<_>) = map
            .iter()
            .filter(|(k, v)| !SKIP_FIELDS.contains(&k.as_str()) && !v.is_array())
            .map(|(k, v)| {
                let key = format!("f_{k}");
                (format!("`{k}` = ${key}"), (key, v.clone()))
            })
            .unzip();

        if parts.is_empty() {
            return Self { sql: "nothing = NONE".into(), bindings: vec![] };
        }

        Self { sql: parts.join(", "), bindings }
    }

    pub fn as_sql(&self)   -> &str               { &self.sql }
    pub fn bindings(&self) -> &[(String, Value)]  { &self.bindings }
}
```

### `crud.rs` — Thin CRUD Functions

Each function is a thin wrapper: build SQL → call adapter → map result. No business logic here.

```rust
use serde_json::Value;
use crate::{adapter::DbAdapter, error::DbError, helpers::*, query::{SetClause, WhereClause}};
use frappe_core::document::{DocRow, Document};

pub async fn get_doc(
    adapter: &DbAdapter,
    doctype: &str,
    name: &str,
) -> Result<Document, DbError> {
    let table = doctype_to_table(doctype);
    let sql   = format!("SELECT * FROM `{table}` WHERE name = $name LIMIT 1");
    let row   = adapter.run_one(&sql, bind!(name), doctype, name).await?;
    value_to_document(row, doctype, name)
}

pub async fn get_list(
    adapter: &DbAdapter,
    doctype: &str,
    fields: Option<&[&str]>,
    filters: Option<&Value>,
    limit: usize,
    start: usize,
) -> Result<Vec<DocRow>, DbError> {
    let field_clause  = fields_to_select(fields);
    let where_clause  = WhereClause::from_filters(filters);
    let table         = doctype_to_table(doctype);
    let sql = format!(
        "SELECT {field_clause} FROM `{table}`{} LIMIT {limit} START {start}",
        where_clause.as_sql()
    );

    Ok(adapter
        .run(&sql, where_clause.bindings().to_vec())
        .await?
        .into_iter()
        .map(row_to_docrow)
        .collect())
}

pub async fn insert_doc(
    adapter: &DbAdapter,
    doctype: &str,
    fields: &Value,
) -> Result<Document, DbError> {
    let name  = extract_name(fields)?;
    let set   = SetClause::from_fields(fields);
    let table = doctype_to_table(doctype);
    let sql   = format!(
        "CREATE type::record($table, $name) SET \
         name = $name, creation = time::now(), modified = time::now(), {}",
        set.as_sql()
    );

    let mut bindings = vec![
        ("table".into(), table.into()),
        ("name".into(),  name.clone().into()),
    ];
    bindings.extend_from_slice(set.bindings());

    let row = adapter.run_one(&sql, bindings, doctype, &name).await?;
    value_to_document(row, doctype, &name)
}

pub async fn delete_doc(
    adapter: &DbAdapter,
    doctype: &str,
    name: &str,
) -> Result<(), DbError> {
    let table = doctype_to_table(doctype);
    let sql   = format!("DELETE `{table}` WHERE name = $name");
    adapter.execute(&sql, bind!(name)).await
}
```

### `docstatus.rs` — Submit / Cancel Without Duplication

```rust
/// The single implementation. submit_doc and cancel_doc are thin wrappers.
async fn transition_docstatus(
    adapter: &DbAdapter,
    doctype: &str,
    name: &str,
    from: i64,
    to: i64,
) -> Result<Document, DbError> {
    let table = doctype_to_table(doctype);

    let row = adapter.run_one(
        &format!("SELECT docstatus FROM `{table}` WHERE name = $name LIMIT 1"),
        bind!(name),
        doctype, name,
    ).await?;

    let current = row.get("docstatus").and_then(Value::as_i64).unwrap_or(0);
    if current != from {
        return Err(DbError::InvalidDocStatus {
            doctype: doctype.into(),
            name: name.into(),
            expected: from,
            got: current,
        });
    }

    let row = adapter.run_one(
        &format!(
            "UPDATE `{table}` SET docstatus = $to, modified = time::now() \
             WHERE name = $name RETURN AFTER"
        ),
        vec![("name".into(), name.into()), ("to".into(), to.into())],
        doctype, name,
    ).await?;

    value_to_document(row, doctype, name)
}

pub async fn submit_doc(a: &DbAdapter, doctype: &str, name: &str) -> Result<Document, DbError> {
    transition_docstatus(a, doctype, name, 0, 1).await
}

pub async fn cancel_doc(a: &DbAdapter, doctype: &str, name: &str) -> Result<Document, DbError> {
    transition_docstatus(a, doctype, name, 1, 2).await
}
```

---

## Layer 2 — `frappe-core`: The Framework

### `document.rs` — The Base Data Carrier

```rust
use chrono::{DateTime, Utc};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub type DocRow = IndexMap<String, Value>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(from = "i64", into = "i64")]
pub enum DocStatus {
    #[default]
    Draft     = 0,
    Submitted = 1,
    Cancelled = 2,
}

impl From<i64> for DocStatus {
    fn from(v: i64) -> Self {
        match v { 1 => Self::Submitted, 2 => Self::Cancelled, _ => Self::Draft }
    }
}

impl From<DocStatus> for i64 {
    fn from(s: DocStatus) -> Self { s as i64 }
}

/// The base document — analogous to Frappe's Document class.
/// Every DocType embeds this as `pub doc: Document`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Document {
    pub doctype:     String,
    pub name:        String,
    pub owner:       String,
    pub docstatus:   DocStatus,
    pub creation:    Option<DateTime<Utc>>,
    pub modified:    Option<DateTime<Utc>>,
    pub modified_by: String,

    // Dynamic / overflow fields — for generic access and child tables
    #[serde(flatten)]
    pub fields: IndexMap<String, Value>,
}

impl Document {
    pub fn new(doctype: impl Into<String>) -> Self {
        Self { doctype: doctype.into(), ..Default::default() }
    }

    pub fn is_new(&self) -> bool       { self.creation.is_none() }
    pub fn is_submitted(&self) -> bool { self.docstatus == DocStatus::Submitted }
    pub fn is_cancelled(&self) -> bool { self.docstatus == DocStatus::Cancelled }

    // Dynamic field access — for when you don't have a typed struct
    pub fn get(&self, field: &str) -> Option<&Value> {
        self.fields.get(field)
    }

    pub fn set(&mut self, field: impl Into<String>, value: impl Into<Value>) {
        self.fields.insert(field.into(), value.into());
    }

    pub fn get_str(&self, field: &str) -> &str {
        self.fields.get(field).and_then(Value::as_str).unwrap_or_default()
    }

    pub fn get_f64(&self, field: &str) -> f64 {
        self.fields.get(field).and_then(Value::as_f64).unwrap_or_default()
    }

    pub fn get_children(&self, fieldname: &str) -> Option<&Value> {
        self.fields.get(fieldname)
    }

    pub fn to_value(&self) -> serde_json::Result<Value> {
        serde_json::to_value(self)
    }
}
```

### `doctype.rs` — The DocType Trait

```rust
use async_trait::async_trait;
use crate::document::Document;
use crate::error::FrappeError;

pub type Result<T> = std::result::Result<T, FrappeError>;

/// Every compiled DocType implements this trait.
/// Analogous to Frappe's Document class hooks.
#[async_trait]
pub trait DocType: Send + Sync + 'static {
    /// The string name of this DocType e.g. "Sales Order"
    fn doctype_name() -> &'static str where Self: Sized;

    /// Access the embedded base Document
    fn doc(&self)         -> &Document;
    fn doc_mut(&mut self) -> &mut Document;

    // ── Lifecycle hooks — all optional ───────────────────────────────────────

    async fn validate(&self)           -> Result<()> { Ok(()) }
    async fn before_insert(&mut self)  -> Result<()> { Ok(()) }
    async fn after_insert(&self)       -> Result<()> { Ok(()) }
    async fn before_save(&mut self)    -> Result<()> { Ok(()) }
    async fn after_save(&self)         -> Result<()> { Ok(()) }
    async fn before_submit(&mut self)  -> Result<()> { Ok(()) }
    async fn on_submit(&mut self)      -> Result<()> { Ok(()) }
    async fn on_cancel(&mut self)      -> Result<()> { Ok(()) }
    async fn on_trash(&self)           -> Result<()> { Ok(()) }
}

/// Convenience macro — eliminates the two-line boilerplate from every DocType.
/// Provides doc()/doc_mut() plus common shortcut methods.
///
/// Usage: impl_document!(SalesOrder);
#[macro_export]
macro_rules! impl_document {
    ($t:ty) => {
        impl $t {
            pub fn name(&self)         -> &str       { &self.doc.name }
            pub fn is_new(&self)       -> bool       { self.doc.is_new() }
            pub fn is_submitted(&self) -> bool       { self.doc.is_submitted() }
            pub fn is_cancelled(&self) -> bool       { self.doc.is_cancelled() }
            pub fn get(&self, f: &str) -> Option<&serde_json::Value> { self.doc.get(f) }
            pub fn set(&mut self, f: &str, v: impl Into<serde_json::Value>) { self.doc.set(f, v) }
        }

        impl AsRef<frappe_core::document::Document> for $t {
            fn as_ref(&self) -> &frappe_core::document::Document { &self.doc }
        }

        impl AsMut<frappe_core::document::Document> for $t {
            fn as_mut(&mut self) -> &mut frappe_core::document::Document { &mut self.doc }
        }
    };
}
```

### `registry.rs` — Two-Track DocType Registry

Compiled types self-register via `inventory`. Dynamic types load from DB once at startup.

```rust
use std::collections::HashMap;
use inventory;
use serde::de::DeserializeOwned;
use crate::{document::Document, doctype::DocType, error::FrappeError};
use frappe_db::DbAdapter;

/// A self-registering entry. The #[doctype] proc macro generates
/// the inventory::submit! call — app devs never write this manually.
pub struct DocTypeEntry {
    pub name:    &'static str,
    pub factory: fn(Document) -> Box<dyn DocType>,
}

inventory::collect!(DocTypeEntry);

/// Metadata for user-created dynamic doctypes (no Rust struct).
#[derive(Debug, Clone)]
pub struct DynamicMeta {
    pub name:   String,
    pub module: String,
}

pub struct DocTypeRegistry {
    /// Compiled DocTypes — from inventory, zero DB cost
    compiled: HashMap<&'static str, fn(Document) -> Box<dyn DocType>>,

    /// Dynamic DocTypes — loaded from DB at startup, refreshable at runtime
    dynamic: dashmap::DashMap<String, DynamicMeta>,
}

impl DocTypeRegistry {
    pub async fn init(db: &DbAdapter) -> Result<Self, FrappeError> {
        // 1. Collect all compiled types — free, they're already in the binary
        let compiled: HashMap<_, _> = inventory::iter::<DocTypeEntry>
            .into_iter()
            .map(|e| (e.name, e.factory))
            .collect();

        tracing::info!(count = compiled.len(), "Loaded compiled doctypes");

        // 2. Load dynamic doctypes — one query, all at once
        let dynamic = Self::load_dynamic(db).await?;
        tracing::info!(count = dynamic.len(), "Loaded dynamic doctypes");

        Ok(Self { compiled, dynamic })
    }

    pub fn create(&self, doctype: &str, doc: Document) -> Result<Box<dyn DocType>, FrappeError> {
        // Compiled types take priority
        if let Some(factory) = self.compiled.get(doctype) {
            return Ok(factory(doc));
        }
        // Fall back to generic dynamic document
        if self.dynamic.contains_key(doctype) {
            return Ok(Box::new(DynamicDocument(doc)));
        }
        Err(FrappeError::UnknownDocType(doctype.into()))
    }

    /// Called after a new DocType is created at runtime.
    pub fn register_dynamic(&self, meta: DynamicMeta) {
        self.dynamic.insert(meta.name.clone(), meta);
    }

    async fn load_dynamic(db: &DbAdapter) -> Result<dashmap::DashMap<String, DynamicMeta>, FrappeError> {
        let rows = frappe_db::crud::get_list(db, "DocType", Some(&["name", "module"]), None, 10_000, 0).await?;
        let map = dashmap::DashMap::new();
        for row in rows {
            let name = row.get("name").and_then(|v| v.as_str()).unwrap_or_default().to_string();
            if !name.is_empty() {
                map.insert(name.clone(), DynamicMeta {
                    name,
                    module: row.get("module").and_then(|v| v.as_str()).unwrap_or_default().into(),
                });
            }
        }
        Ok(map)
    }
}

/// Generic handler for user-created dynamic doctypes.
/// No hooks — if you need hooks on dynamic types, use Rhai scripts.
pub struct DynamicDocument(pub Document);

#[async_trait::async_trait]
impl DocType for DynamicDocument {
    fn doctype_name() -> &'static str where Self: Sized { "Dynamic" }
    fn doc(&self)         -> &Document     { &self.0 }
    fn doc_mut(&mut self) -> &mut Document { &mut self.0 }
}
```

### `context.rs` — The Global Handle

```rust
use std::sync::{Arc, OnceLock};
use frappe_db::DbAdapter;
use crate::registry::DocTypeRegistry;

pub struct FrappeContext {
    pub db:       Arc<DbAdapter>,
    pub registry: Arc<DocTypeRegistry>,
}

static FRAPPE: OnceLock<FrappeContext> = OnceLock::new();

/// Called once at application startup.
pub async fn init(db_url: &str) -> anyhow::Result<()> {
    let db       = Arc::new(DbAdapter::connect(db_url).await?);
    let registry = Arc::new(DocTypeRegistry::init(&db).await?);

    FRAPPE
        .set(FrappeContext { db, registry })
        .map_err(|_| anyhow::anyhow!("frappe::init() called more than once"))?;

    tracing::info!("Frappe initialized");
    Ok(())
}

/// Panics if init() was not called. Intentional — misconfiguration should crash loudly.
pub fn frappe() -> &'static FrappeContext {
    FRAPPE.get().expect("frappe::init() must be called before using the framework")
}
```

### `lib.rs` — The Public API Surface

This is everything an app developer ever imports. Nothing else is public.

```rust
//! frappe-core public API — mirrors frappe.* Python API.

mod context;
mod document;
mod doctype;
mod registry;
mod naming;
mod children;
pub mod error;

pub use context::init;
pub use document::{DocRow, DocStatus, Document};
pub use doctype::DocType;
pub use registry::{DocTypeEntry, DynamicDocument};

use context::frappe;
use error::FrappeError;
use serde::de::DeserializeOwned;

// ── frappe::get_doc ───────────────────────────────────────────────────────────

/// Untyped get_doc — returns Box<dyn DocType>.
/// Use when the doctype is only known at runtime (e.g. from an HTTP request).
pub async fn get_doc(doctype: &str, name: &str) -> Result<Box<dyn DocType>, FrappeError> {
    let ctx = frappe();
    let doc = frappe_db::crud::get_doc(&ctx.db, doctype, name).await?;
    let doc = children::load(&ctx.db, doc).await?;
    ctx.registry.create(doctype, doc)
}

/// Typed get_doc — returns T directly.
/// Use when the type is known at compile time.
pub async fn get_doc_typed<T>(name: &str) -> Result<T, FrappeError>
where
    T: DocType + DeserializeOwned + Default,
{
    let ctx = frappe();
    let doc = frappe_db::crud::get_doc(&ctx.db, T::doctype_name(), name).await?;
    let doc = children::load(&ctx.db, doc).await?;
    Ok(serde_json::from_value(doc.to_value()?)?)
}

// ── frappe::new_doc ───────────────────────────────────────────────────────────

pub fn new_doc(doctype: &str) -> Result<Box<dyn DocType>, FrappeError> {
    frappe().registry.create(doctype, Document::new(doctype))
}

pub fn new_doc_typed<T: DocType + Default>() -> T {
    T::default()
}

// ── frappe::save ─────────────────────────────────────────────────────────────

pub async fn save(doc: &mut dyn DocType) -> Result<(), FrappeError> {
    doc.validate().await?;
    doc.before_save().await?;

    let is_new = doc.doc().is_new();

    if is_new {
        let name = naming::generate(doc.doc()).await?;
        doc.doc_mut().name     = name;
        doc.doc_mut().creation = Some(chrono::Utc::now());
        doc.before_insert().await?;
        frappe_db::crud::insert_doc(&frappe().db, doc.doc().doctype.as_str(), &doc.doc().to_value()?).await?;
        doc.after_insert().await?;
    } else {
        doc.doc_mut().modified = Some(chrono::Utc::now());
        frappe_db::crud::upsert_doc(
            &frappe().db,
            &doc.doc().doctype.clone(),
            &doc.doc().name.clone(),
            &doc.doc().to_value()?,
        ).await?;
    }

    children::save(&frappe().db, doc.doc()).await?;
    doc.after_save().await?;
    Ok(())
}

// ── frappe::submit / cancel ───────────────────────────────────────────────────

pub async fn submit(doc: &mut dyn DocType) -> Result<(), FrappeError> {
    doc.before_submit().await?;
    frappe_db::docstatus::submit_doc(&frappe().db, &doc.doc().doctype, &doc.doc().name).await?;
    doc.doc_mut().docstatus = DocStatus::Submitted;
    doc.on_submit().await?;
    Ok(())
}

pub async fn cancel(doc: &mut dyn DocType) -> Result<(), FrappeError> {
    frappe_db::docstatus::cancel_doc(&frappe().db, &doc.doc().doctype, &doc.doc().name).await?;
    doc.doc_mut().docstatus = DocStatus::Cancelled;
    doc.on_cancel().await?;
    Ok(())
}

pub async fn delete(doctype: &str, name: &str) -> Result<(), FrappeError> {
    frappe_db::crud::delete_doc(&frappe().db, doctype, name).await?;
    Ok(())
}

// ── frappe::get_list ─────────────────────────────────────────────────────────

pub fn get_list(doctype: &str) -> GetListBuilder {
    GetListBuilder::new(doctype)
}

// ── frappe::get_value ────────────────────────────────────────────────────────

pub async fn get_value(doctype: &str, name: &str, field: &str) -> Result<Option<serde_json::Value>, FrappeError> {
    Ok(frappe_db::crud::get_value(&frappe().db, doctype, name, field).await?)
}

pub async fn set_value(doctype: &str, name: &str, field: &str, value: impl Into<serde_json::Value>) -> Result<(), FrappeError> {
    frappe_db::crud::set_field(&frappe().db, doctype, name, field, value.into()).await?;
    Ok(())
}
```

---

## Layer 3 — The GetListBuilder

A fluent builder so call sites read like English:

```rust
use serde::de::DeserializeOwned;
use serde_json::{json, Value};
use crate::{context::frappe, document::DocRow, error::FrappeError};

pub struct GetListBuilder {
    doctype:  String,
    fields:   Vec<String>,
    filters:  serde_json::Map<String, Value>,
    order_by: Option<String>,
    limit:    usize,
    start:    usize,
}

impl GetListBuilder {
    pub fn new(doctype: &str) -> Self {
        Self {
            doctype:  doctype.into(),
            fields:   vec![],
            filters:  Default::default(),
            order_by: None,
            limit:    20,
            start:    0,
        }
    }

    pub fn filter(mut self, field: &str, op: &str, value: impl Into<Value>) -> Self {
        self.filters.insert(field.into(), json!([op, value.into()]));
        self
    }

    pub fn fields(mut self, fields: &[&str]) -> Self {
        self.fields = fields.iter().map(|s| s.to_string()).collect();
        self
    }

    pub fn order_by(mut self, field: &str, desc: bool) -> Self {
        self.order_by = Some(format!("{field} {}", if desc { "DESC" } else { "ASC" }));
        self
    }

    pub fn limit(mut self, n: usize) -> Self { self.limit = n; self }
    pub fn start(mut self, n: usize) -> Self { self.start = n; self }

    /// Returns untyped rows (IndexMap<String, Value>)
    pub async fn run(self) -> Result<Vec<DocRow>, FrappeError> {
        let filters = if self.filters.is_empty() {
            None
        } else {
            Some(Value::Object(self.filters))
        };
        let fields: Vec<&str> = self.fields.iter().map(|s| s.as_str()).collect();
        let fields_opt = if fields.is_empty() { None } else { Some(fields.as_slice()) };

        Ok(frappe_db::crud::get_list(
            &frappe().db,
            &self.doctype,
            fields_opt,
            filters.as_ref(),
            self.limit,
            self.start,
        ).await?)
    }

    /// Returns typed structs
    pub async fn run_typed<T: DeserializeOwned>(self) -> Result<Vec<T>, FrappeError> {
        self.run()
            .await?
            .into_iter()
            .map(|row| serde_json::from_value(Value::Object(
                row.into_iter().collect()
            )).map_err(FrappeError::Deserialize))
            .collect()
    }
}
```

---

## Layer 4 — App Developer Code

This is what the developer of an application writes. They never touch anything above.

```rust
// apps/your-app/src/doctypes/sales_order.rs

use frappe_core::{DocType, Document, DocStatus, impl_document};
use frappe_core::error::FrappeError;
use serde::{Deserialize, Serialize};
use rust_decimal::Decimal;
use chrono::NaiveDate;
use anyhow::ensure;

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct SalesOrder {
    #[serde(flatten)]
    pub doc: Document,

    pub customer:      String,
    pub customer_name: String,
    pub posting_date:  Option<NaiveDate>,
    pub delivery_date: Option<NaiveDate>,
    pub grand_total:   Decimal,
    pub status:        String,

    #[serde(default)]
    pub items: Vec<SalesOrderItem>,
}

// One line gives you name(), is_new(), get(), set(), AsRef<Document>
impl_document!(SalesOrder);

#[async_trait::async_trait]
impl DocType for SalesOrder {
    fn doctype_name() -> &'static str { "Sales Order" }
    fn doc(&self)         -> &Document     { &self.doc }
    fn doc_mut(&mut self) -> &mut Document { &mut self.doc }

    async fn validate(&self) -> Result<(), FrappeError> {
        ensure!(!self.customer.is_empty(), "Customer is required");
        ensure!(!self.items.is_empty(),    "Items cannot be empty");

        for (i, item) in self.items.iter().enumerate() {
            ensure!(!item.item_code.is_empty(), "Row {}: Item Code required", i + 1);
            ensure!(item.qty > Decimal::ZERO,   "Row {}: Qty must be positive", i + 1);
        }

        if let (Some(delivery), Some(posting)) = (self.delivery_date, self.posting_date) {
            ensure!(delivery >= posting, "Delivery date cannot be before posting date");
        }

        Ok(())
    }

    async fn before_save(&mut self) -> Result<(), FrappeError> {
        for item in &mut self.items {
            item.amount = item.qty * item.rate;
        }
        self.grand_total = self.items.iter().map(|i| i.amount).sum();
        Ok(())
    }

    async fn on_submit(&mut self) -> Result<(), FrappeError> {
        self.status = "To Deliver".into();
        Ok(())
    }

    async fn on_cancel(&mut self) -> Result<(), FrappeError> {
        self.status = "Cancelled".into();
        Ok(())
    }
}

// Self-registration — generated by #[doctype] macro later, written manually now
inventory::submit!(frappe_core::DocTypeEntry {
    name:    "Sales Order",
    factory: |doc| {
        let typed: SalesOrder = serde_json::from_value(doc.to_value().unwrap())
            .unwrap_or_default();
        Box::new(typed)
    },
});
```

### Usage in Application Code

```rust
// apps/your-app/src/main.rs

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    frappe_core::init("ws://localhost:8000").await?;

    // get_doc — typed
    let mut order: SalesOrder = frappe::get_doc_typed("SO-0001").await?;
    order.customer = "CUST-001".into();
    frappe::save(&mut order).await?;

    // get_list — fluent
    let orders: Vec<SalesOrder> = frappe::get_list("Sales Order")
        .filter("customer",  "=", "CUST-001")
        .filter("docstatus", "=", 0)
        .order_by("creation", true)
        .limit(50)
        .run_typed::<SalesOrder>()
        .await?;

    // new doc
    let mut so = frappe::new_doc_typed::<SalesOrder>();
    so.customer = "CUST-001".into();
    so.items.push(SalesOrderItem {
        item_code: "ITEM-001".into(),
        qty:       Decimal::from(5),
        rate:      Decimal::from(100),
        ..Default::default()
    });
    frappe::save(&mut so).await?;
    frappe::submit(&mut so).await?;

    Ok(())
}
```

---

## Error Handling

One error enum per crate. No `anyhow` in library code — only in application code.

```rust
// frappe-db/src/error.rs
#[derive(Debug, thiserror::Error)]
pub enum DbError {
    #[error("Document not found: {doctype}/{name}")]
    NotFound { doctype: String, name: String },

    #[error("{doctype}/{name}: expected docstatus {expected}, got {got}")]
    InvalidDocStatus { doctype: String, name: String, expected: i64, got: i64 },

    #[error("Missing required field: {0}")]
    MissingField(String),

    #[error("SurrealDB error: {0}")]
    Surreal(#[from] surrealdb::Error),
}

// frappe-core/src/error.rs
#[derive(Debug, thiserror::Error)]
pub enum FrappeError {
    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Unknown DocType: {0}")]
    UnknownDocType(String),

    #[error("Deserialization error: {0}")]
    Deserialize(#[from] serde_json::Error),

    #[error(transparent)]
    Db(#[from] frappe_db::error::DbError),
}

// ensure! macro for clean validation errors
#[macro_export]
macro_rules! ensure {
    ($cond:expr, $msg:literal $(, $arg:expr)*) => {
        if !$cond {
            return Err(FrappeError::Validation(format!($msg $(, $arg)*)));
        }
    };
}
```

---

## Testing Strategy

### Unit Tests — No DB Required

```rust
// Test hooks in complete isolation
#[cfg(test)]
mod tests {
    use super::*;

    fn make_order(customer: &str, items: Vec<SalesOrderItem>) -> SalesOrder {
        SalesOrder {
            customer: customer.into(),
            items,
            ..Default::default()
        }
    }

    #[tokio::test]
    async fn validate_rejects_empty_items() {
        let order = make_order("CUST-001", vec![]);
        assert!(order.validate().await.is_err());
    }

    #[tokio::test]
    async fn before_save_calculates_totals() {
        let mut order = make_order("CUST-001", vec![
            SalesOrderItem { qty: dec!(5), rate: dec!(100), ..Default::default() },
            SalesOrderItem { qty: dec!(2), rate: dec!(50),  ..Default::default() },
        ]);
        order.before_save().await.unwrap();
        assert_eq!(order.grand_total, dec!(600));
    }
}
```

### Integration Tests — Real DB

```rust
// tests/integration/sales_order.rs
#[tokio::test]
async fn full_lifecycle() {
    frappe_core::init("ws://localhost:8000").await.unwrap();

    let mut so = frappe::new_doc_typed::<SalesOrder>();
    so.customer = "TEST-CUST".into();
    so.items.push(test_item());

    // Save → Submit → Cancel
    frappe::save(&mut so).await.unwrap();
    assert!(!so.is_new());

    frappe::submit(&mut so).await.unwrap();
    assert!(so.is_submitted());

    frappe::cancel(&mut so).await.unwrap();
    assert!(so.is_cancelled());

    // Cleanup
    frappe::delete("Sales Order", so.name()).await.unwrap();
}
```

---

## The Proc Macro Endgame

Once the framework is stable, a single `#[doctype]` attribute replaces all boilerplate:

```rust
// What you write
#[doctype(module = "Selling", submittable)]
pub struct SalesOrder {
    #[link("Customer")]
    pub customer: String,

    #[child_table]
    pub items: Vec<SalesOrderItem>,

    pub grand_total: Decimal,
}

impl Hooks for SalesOrder {
    async fn validate(&self) -> Result<()> {
        ensure!(!self.items.is_empty(), "Items required");
        Ok(())
    }
}
```

The macro generates:
- `impl DocType for SalesOrder`
- `impl_document!(SalesOrder)`
- `inventory::submit!` registration
- `static META: DocTypeMeta` (field list, types, options)
- Link field validation for `customer`
- SurrealDB `DEFINE TABLE` / `DEFINE FIELD` migration SQL

**400 lines of macro code written once. Zero boilerplate ever again.**

---

## Key Rules — Summary

| Rule | Why |
|---|---|
| `DbAdapter::run()` and `run_one()` are the only places that call SurrealDB | One place to fix DB bugs |
| `frappe-db` has zero knowledge of DocType structs | DB layer is swappable |
| `frappe-core` has zero SurrealDB imports | Core logic is DB-agnostic |
| All hook logic lives in `impl DocType for X` | Testable without a DB |
| `impl_document!` macro on every DocType | No copy-pasted boilerplate |
| `inventory::submit!` for self-registration | No central 500-line registry |
| `thiserror` in libraries, `anyhow` in apps | Clean error boundaries |
| Unit tests test hooks only, integration tests test DB | Fast CI, clear failure signals |
