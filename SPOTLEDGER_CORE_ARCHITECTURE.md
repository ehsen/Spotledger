# Spotledger Core — Framework Architecture

> **Audience**: engineers and LLMs reviewing the codebase.  
> **Scope**: framework layer only — not application-domain logic (Finance, Accounting, etc.).  
> **Status as of**: April 2026

---

## 1. What Spotledger Is

Spotledger is a **Frappe like framwork ** rewritten in Rust, using SurrealDB as the database. It is not a wrapper around Frappe — it is a ground-up re-implementation that speaks the same HTTP API to a certain degree.

**Importtant Note**
We are not a replica of frappe or something its major architectureal 
shift, the only thing that we pick from frappe is meta data based rendering
and frappe metadata (even though that will be greatly enhanced)

-- Its an APP platform (which on surface is low code, AI Native, but you 
can technically write anything in it using Rust too)

The guiding principle is a strict **engine / knowledge-base** split:

| Rust (the Engine) | SurrealDB (the Knowledge Base) |
|---|---|
| HTTP routing, auth, session | All document data |
| Generic save pipeline (auth → name → write) | All doctype/field metadata |
| Permission enforcement | All domain validation and compute logic |
| Schema DDL at startup | App/module/field provenance graph |
| Naming-series counter calls | All `fn::` business logic |

Everything that Frappe encodes in Python controller classes lives in SurrealDB as `DEFINE FUNCTION` / `DEFINE EVENT` / graph-stored pipeline stages.

---

## 2. Repository Layout

```
spotledger/                     ← workspace root
├── crates/
│   ├── spotledger/             ← CLI binary (main.rs, all commands)
│   ├── spotledger-core/        ← pure-Rust types: Document, DocTypeMeta, FieldType, utils
│   ├── spotledger-db/          ← database layer: DbAdapter, save pipeline, permissions, naming
│   ├── spotledger-http/        ← Axum HTTP server, routes, middleware, method registry
│   ├── spotledger-pdk/         ← plugin development kit (Extism guest API)
│   ├── spotledger-plugins/     ← compiled host-side plugin loader
│   └── xtask/                  ← build automation (cargo xtask build/check/host/wasm)
└── apps/
    ├── spotledger-core/        ← DB-native app: framework DocTypes + surql pipeline
    │   └── spotledger-core/
    │       └── surql/
    │           ├── framework/  ← 01_schema … 07_workflow.surql
    │           └── shared/     ← universal fn:: definitions
    └── spotledger-finance/     ← DB-native app: Finance module DocTypes + surql
        └── spotledger-finance/
            └── surql/doctypes/ ← per-doctype functions.surql + wiring.surql
```

### Crate dependency tree (simplified)

```
spotledger (CLI)
    ├── spotledger-http
    │       ├── spotledger-db
    │       │       └── spotledger-core
    │       └── spotledger-core
    └── spotledger-db
            └── spotledger-core
```

`spotledger-core` has **zero** dependencies on Rust async or SurrealDB — it is plain data types and pure functions.

---

## 3. Crate Responsibilities

### 3.1 `spotledger-core`

**Pure types and utilities. No I/O.**

| Module | Contents |
|---|---|
| `meta.rs` | `DocTypeMeta`, `DocField`, `FieldType` enum, `LayoutKind`, `Permission` — compiled schema descriptors |
| `document.rs` | `Document` struct (name + `IndexMap<String, Value>` fields), `DocStatus`, `DocRow` |
| `registry.rs` | `DocTypeRegistry` — two-track: compiled `inventory::submit!` entries + runtime `DashMap` of dynamic types |
| `modules.rs` | `FM` struct — single source of truth for all module name constants (`FM::CORE`, `FM::ACCOUNTS`, …) |
| `doctype.rs` | `DocType` trait — `validate()`, `before_save()`, `after_save()` implemented by Tier 0 controllers |
| `validation.rs` | Generic validators: mandatory fields, select options, length, XSS sanitize, numeric coercion |
| `config.rs` | `SiteConfig`, `DatabaseConfig`, `SiteInfo` — deserialized from `site_config.toml` |
| `utils/` | Frappe-compatible utility functions: date math, money formatting, password hash, HTML strip |
| `doctypes/` | Tier 0 DocType controller implementations: `DocType`, `User`, `Role`, `DocPerm`, `NamingRule` |

Tier 0 types register themselves at compile time via `inventory::submit!(MetaEntry { ... })` and `inventory::submit!(DocTypeEntry { ... })`. The inventory is iterated at startup to produce DDL and to handle save routing for those types.

### 3.2 `spotledger-db`

**All SurrealDB interaction.** No Axum, no HTTP concerns.

| Module | Responsibility |
|---|---|
| `adapter.rs` | `DbAdapter` — the single chokepoint. All SQL goes through `run`, `run_one`, or `execute`. Wraps `Surreal<Client>`. |
| `connection.rs` | Opens a SurrealDB WebSocket connection; selects namespace/database. |
| `bootstrap.rs` | `run_framework_tables()` — emits DDL for `__Auth`, `tabSessions`, `tabSeries`, `tabSingles`, `tabMigration`, graph node tables, `installed_app`. Called once per `new-site`. |
| `schema.rs` | `ensure_all_schemas()` — iterates `inventory::iter::<MetaEntry>` and emits `DEFINE TABLE / DEFINE FIELD` for all Tier 0 types. Also `ensure_schema(meta)` for one type (two-phase: DDL + graph node upsert). |
| `document.rs` | `get_doc`, `get_list`, `get_value`, `insert_doc`, `upsert_doc`, `delete_doc` — thin SQL helpers. |
| `controller.rs` | Tier 0 save pipeline (15 ordered steps: numeric coercion → naming → old-doc fetch → validation → hooks → DB write → after-hooks). Used only for Tier 0 compiled types. |
| `save_proxy.rs` | Thin save handler for all runtime-seeded (non-Tier 0) DocTypes. Steps: auth check → `custom_` prefix enforcement → naming (via `fn::naming::resolve`) → SurrealDB write → `fn::pipeline::run`. |
| `naming.rs` | Rust-side naming utilities: `resolve_name`, `next_name` (series counter increment against `tabSeries`). |
| `permissions.rs` | `has_permission(adapter, user, doctype, PermissionType)` — queries `tabHas_Role` + `tabDocPerm`. `PermissionType` enum covers all 14 Frappe permission actions. |
| `meta_cache.rs` | `MetaCache` — moka async LRU cache (1 024 entries, 5-minute TTL). Checks compiled inventory first; on miss queries SurrealDB graph (`doctype` / `docfield` / `has_field`). Returns `DynDocTypeMeta`. |
| `pipeline.rs` | `apply_pipeline_functions(adapter, app_root)` — loads all `.surql` files in the correct 10-step order at install time. `run_pipeline(adapter, doc_id, doctype, action)` — calls `fn::pipeline::run` from Rust. `apply_pipeline_functions_multi` — loads multiple apps into a single combined registry. |
| `apply_surql.rs` | `apply_surql_file`, `apply_surql_dir`, `apply_surql_dir_named`, `collect_fn_names` — file-based SurrealQL loading utilities. |
| `graph_ops.rs` | `upsert_app_node`, `upsert_module_node`, `relate_module_contains_doctype`, `upsert_docfield_graph`, `log_schema_change`. |
| `hooks.rs` | `HookRegistry` — in-process Rust hook dispatch (before/after save/insert/submit). Used by Tier 0 controller only. |
| `auth.rs` | `create_session`, `validate_session`, `set_user_password`, `check_user_password`. |
| `jwt.rs` | JWT-based session tokens (sign, verify). |
| `migrations.rs` | Sequential numbered migration runner (stored in `tabMigration`). |

### 3.3 `spotledger-http`

**Axum HTTP server.** No direct SurrealDB access — delegates everything to `spotledger-db`.

| Module | Responsibility |
|---|---|
| `server.rs` | `serve(args)` — entrypoint: logging, site loading, router construction, bind. |
| `routes.rs` | REST resource handlers: `GET/POST /api/resource/{doctype}`, `GET/PUT /api/resource/{doctype}/{name}`, `POST/DELETE` with submit/cancel routes. Permission-gated. |
| `middleware.rs` | `site_middleware` — host-header routing: extracts `SiteState` from `AppState` per request. JWT session validation → `CurrentUser` extension. |
| `state.rs` | `SiteState` (db, doc_cache, search_cache, method_registry, hook_registry, meta_cache) + `AppState` (`DashMap<hostname, SiteState>`). |
| `methods/mod.rs` | `MethodRegistry` + `call_method` handler for `/api/method/{*path}`. |
| `methods/auth.rs` | Login/logout handlers. |
| `methods/desk/` | Frappe Desk-compatible method handlers: `getdoctype`, `get_doc`, `get_list`, `search_link`, list-view helpers, notifications. |
| `methods/designer.rs` | DocType Designer API (`get_meta`, `save_doctype`). |
| `methods/apps.rs` | Boot info (`frappe.boot.get_bootinfo`), installed apps list. |
| `methods/client.rs` | `frappe.client.*` method group. |

**Route map (key paths)**:

```
GET  /api/ping
GET  /api/resource/{doctype}                    → resource_list
POST /api/resource/{doctype}                    → resource_create
GET  /api/resource/{doctype}/{name}             → resource_get
PUT  /api/resource/{doctype}/{name}             → resource_update
POST /api/resource/{doctype}/{name}/submit      → resource_submit
POST /api/resource/{doctype}/{name}/cancel      → resource_cancel
POST /api/method/{*path}                        → call_method dispatcher
GET  /api/method/frappe.client.get              → getdoc_handler
GET  /api/method/frappe.client.get_list         → getdoc_handler (list)
POST /api/method/login                          → login_handler
POST /api/method/logout                         → logout_handler
GET  /api/method/frappe.desk.form.load.getdoctype → getdoctype_handler
GET  /api/method/frappe.boot.get_bootinfo       → boot handler
```

### 3.4 `spotledger` (CLI)

**`spotledger` binary** — all operator commands.

| Command | File | What it does |
|---|---|---|
| `new-site` | `new_site.rs` | Create site dir, write `site_config.toml`, bootstrap framework tables, run `ensure_all_schemas`, seed default records, hash admin password. Auto-installs apps with `"auto_install": true`. |
| `install-app` | `install_app.rs` | Seed DocType JSONs, Module Def records, importable fixtures, patch log entries, pipeline framework + wiring. |
| `start` / `serve` | `start.rs` | Load site(s), start Axum server. |
| `migrate` | `migrate.rs` | Run pending numbered migrations. |
| `use` | `use_site.rs` | Write `sites/currentsite` (plain text). |
| `cleanup` | `cleanup.rs` | Report or purge orphaned data. |
| `emit` | `emit.rs` | Code generation output. |
| `generate` | `generate.rs` | Generate Rust from Frappe JSON. |

### 3.5 `spotledger-pdk`

Plugin Development Kit for **Extism** WASM guest plugins. Provides `sl_get_doc`, `sl_save_doc` and other host-function wrappers for plugin authors writing logic in Rust compiled to `wasm32-wasip1`.

---

## 4. The Document Model

### 4.1 `Document` struct

```rust
pub struct Document {
    pub name:    String,
    pub doctype: String,
    pub fields:  IndexMap<String, Value>,   // ordered, serde_json::Value
}
```

`name` is stored separately from `fields` because `fields` does not include `name` — it is only in `doc.name`. Tests and handlers that look up the name must check `doc.name`, not `doc.fields.get("name")`.

### 4.2 Tier 0 vs. Runtime DocTypes

| Tier | What | How handled |
|---|---|---|
| **Tier 0** | `DocType`, `DocField`, `DocPerm`, `User`, `Role`, `ModuleDef`, `NamingRule` | Compiled into host binary via `inventory::submit!`. Use the full 15-step controller in `controller.rs`. Cannot be removed. |
| **Runtime** | All other DocTypes (Account, Journal Entry, Customer, …) | Seeded from JSON by `install_app`. Metadata lives in SurrealDB `tabDocType` / `tabDocField`. Handled by `save_proxy.rs`. |

Tier 0 types are the types the engine literally cannot boot without. Everything else is runtime data.

---

## 5. The Save Pipeline

Two paths exist, chosen by whether the target doctype is Tier 0 compiled or runtime-seeded.

### 5.1 Tier 0 — `controller::save_doc`

```
fix_numeric_types
    → resolve_name          (fn::naming::resolve via SurrealDB, UUID fallback)
    → fetch_old_doc         (existing records only)
    → validate_constants    (set_only_once fields)
    → validate_update_submit
    → set_user_and_timestamp
    → set_parent_in_children
    → mandatory check
    → validate_selects
    → validate_length
    → sanitize_content      (XSS strip)
    → validate hook         (HookRegistry)
    → before_save hook
    → DB write              (insert_doc / upsert_doc)
    → after_save hook
```

Returns the persisted `Document`.

### 5.2 Runtime — `save_proxy::save_doc_proxy`

```
1. Permission check         (has_permission → tabHas_Role + tabDocPerm)
2. custom_ prefix enforce   (meta_cache lookup → rewrite is_custom fieldnames)
3. User password intercept  (strip __new_password, store in __Auth after write)
4. Naming                   (fn::naming::resolve($doctype, $doc) in SurrealDB)
5. UPSERT to SurrealDB      ← DEFINE EVENT / DEFINE FIELD ASSERT fire here automatically
6. fn::pipeline::run        (if doctype is registered in doctype_meta)
7. Re-read persisted doc    (if pipeline returned "ok")
```

The SurrealDB write at step 5 is the boundary: every `DEFINE EVENT` and `DEFINE FIELD VALUE` expression fires unconditionally for **every caller**, including the SurrealDB console and other events — not just Rust. This is the fundamental difference from Frappe where bypassing the Python controller bypasses validation.

---

## 6. The Graph-Compute Pipeline

This is the core innovation. Domain logic — validation, computed fields, GL posting, cancel cascades — lives in SurrealDB as a **data-driven DAG** rather than compiled controller code.

### 6.1 Pipeline Tables (defined in `01_schema.surql`)

| Table | Purpose |
|---|---|
| `doctype_meta` | Opt-in registry: presence = doctype participates in pipeline. Stores `is_submittable`. |
| `pipeline_stage` | One record per `(doctype, action, stage_name)` with `ord` for sequencing. |
| `pipeline_node` | Global function registry: one record per `fn_name`. Shared across doctypes. |
| `has_node` | Relation edge: `pipeline_stage → pipeline_node`. Carries `ord` and `config` (per-edge params). |
| `cascade_rule` | Cancel dependency: which child doctypes to cancel when a parent is cancelled. |
| `pipeline_run` | Audit log written by Rust around every `fn::pipeline::run` call. |
| `child_of` | SCHEMAFULL relation for tree doctypes (Account, Cost Center). Replaces `lft/rgt` nested sets. |

### 6.2 Runner — `fn::pipeline::run`

Defined in `02_runner.surql`. Called by `save_proxy.rs` after the DB write.

```
fn::pipeline::run($doc_id: record, $doctype: string, $action: string)
    ├── lookup tabDocType.issubmittable
    ├── guard: submit/cancel/amend require issubmittable
    ├── fetch pipeline_stage WHERE doctype = $doctype AND action = $action ORDER BY ord
    ├── if no stages → return { status: "skipped", reason: "no_stages" }
    └── for each stage:
            for each has_node edge (ordered by ord):
                fn::registry::dispatch(fn_name, $doc_id, config)
                if result.error ≠ NONE → THROW   ← rolls back all mutations
    └── if action = "cancel" → fn::pipeline::cascade_cancel($doc_id, $doctype)
    └── return { status: "ok" }
```

**THROW semantics**: because `fn::pipeline::run` executes as a single SurrealDB statement, a `THROW` rolls back **all** mutations made within that call — GL inserts, computed UPDATEs, status changes. A soft `RETURN { error }` is converted to `THROW` by the runner so that naively-written domain functions are still safe.

### 6.3 Registry — `fn::registry::dispatch`

Auto-generated by `apply_pipeline_functions` (step 4 of install). Collects all `DEFINE FUNCTION` names from shared and domain `.surql` files via `collect_fn_names`, then generates a `MATCH`-style dispatcher. Rust never hand-codes which functions exist.

### 6.4 App-level `surql/` layout

```
apps/{app}/{app}/surql/
    framework/
        01_schema.surql        ← pipeline tables DDL
        02_runner.surql        ← fn::pipeline::run
        03_universal_nodes.surql ← shared pipeline_node records
        04_wire_generic.surql  ← fn::pipeline::wire_generic helper
        05_naming.surql        ← fn::naming::* (resolve, next_series, apply_rule)
        06_permissions.surql   ← fn::permissions::has, get_roles
        07_workflow.surql      ← workflow state machine support
    shared/
        fn_validate_mandatory.surql
        fn_validate_party.surql
        fn_validate_cancel.surql
        fn_on_submit_mark_submitted.surql
        fn_on_cancel_mark_cancelled.surql
    doctypes/{name}/
        functions.surql        ← DEFINE FUNCTION fn::*  (domain logic)
        wiring.surql           ← UPSERT pipeline_stage + RELATE has_node edges
```

`wiring.surql` is pure data — it calls `fn::pipeline::wire_generic(...)` to upsert a stage and connect nodes. No Rust code encodes the stage graph topology.

### 6.5 Loading order (`apply_pipeline_functions`)

1. `01_schema.surql` — DDL
2. `shared/*.surql` — universal fn:: definitions
3. `doctypes/*/functions.surql` — domain fn:: definitions
4. Auto-generate + apply `fn::registry::dispatch`
5. `02_runner.surql` — needs registry
6. `03_universal_nodes.surql`
7. `04_wire_generic.surql`
8. `doctypes/*/wiring.surql` — stage + edge upserts
9. `05_naming.surql`
10. `06_permissions.surql`

---

## 7. Naming System

Naming is **fully in SurrealDB** via `fn::naming::resolve($doctype, $doc)`.

Priority order:
1. Explicit `name` field in document → use as-is
2. `naming_series` field in document → `fn::naming::next_series(template)`
3. Admin override in `tabDocumentNamingRule`
4. `tabDocType.autoname` default pattern
5. UUID fallback (`fn::naming::uuid_name()`)

Supported autoname patterns (Frappe-compatible):
- `"field:fieldname"` — value of a document field
- `"naming_series:"` — use `naming_series` field value as template
- `"format:TEMPLATE"` — series template
- `"SO-.YYYY.-.####"` — direct series template (expands YYYY/YY/MM/DD tokens, increments `tabSeries` counter)
- `"Prompt"` / `"hash"` / `"UUID"` — UUID fallback

Series counters live in `tabSeries` and are incremented with an atomic `UPDATE ... SET current = current + 1 RETURN AFTER`.

Rust still has `naming.rs` with `resolve_name` and `next_name` as utilities used by the Tier 0 controller. For runtime DocTypes, naming is done entirely in SurrealDB.

---

## 8. Permission System

### 8.1 Rust layer (`permissions.rs`)

```rust
has_permission(adapter, user, doctype, PermissionType) -> Result<bool>
```

Queries `tabHas_Role` + `tabDocPerm` directly via SQL. `PermissionType` is a typed Rust enum (14 variants: Select, Read, Write, Create, Delete, Submit, Cancel, Amend, Print, Email, Report, Import, Export, Share).

Hierarchy:
1. `Administrator` → always allowed
2. `System Manager` role → always allowed (doctype-level)
3. Role-based `tabDocPerm` match
4. `if_owner` logic (document owner special permissions)
5. User Permissions (link field restrictions)
6. Document Sharing

### 8.2 SurrealDB layer (`06_permissions.surql`)

`fn::permissions::has($user, $doctype, $ptype)` — same logic as a single SurrealDB graph traversal. Used when permission checks are needed inside pipeline functions without a Rust round-trip.

### 8.3 Storage tables

| Table | Purpose |
|---|---|
| `tabUser` | Users: name, email, enabled |
| `tabRole` | Roles: name, disabled |
| `tabHas_Role` | Adjacency list: user → role |
| `tabDocPerm` | Doctype role permissions (perm_create, perm_delete, perm_cancel, perm_select use prefixed names due to SurrealDB v3 reserved-keyword constraint) |
| `tabUser_Permission` | Link-field restrictions per user |
| `tabShared` | Explicit document-level sharing |

---

## 9. Metadata as a Graph

Every DocType, DocField, Module, and App is a graph node in SurrealDB. Relationships are edges.

```
app:spotledger-finance  -[provides_module]->  module:accounts
module:accounts         -[contains]->         doctype:journal_entry
doctype:journal_entry   -[has_field { idx:0, introduced_by:"spotledger-finance" }]->  docfield:journal_entry_posting_date
```

### 9.1 Node tables

| Table | Type | Contents |
|---|---|---|
| `app` | SCHEMALESS | name, title, version, logo_url |
| `module` | SCHEMALESS | name, label, app |
| `doctype` | SCHEMALESS | name, module, is_submittable, is_child, … |
| `docfield` | SCHEMALESS | name, fieldtype, label, options, reqd, … |

### 9.2 Edge tables

| Edge | From → To | Payload |
|---|---|---|
| `provides_module` | app → module | — |
| `contains` | module → doctype | — |
| `has_field` | doctype → docfield | `idx`, `introduced_by`, `app_version`, `is_custom` |
| `child_of` | record → record | — (SCHEMAFULL, used for tree doctypes) |

`has_field.introduced_by` is the provenance key: it records which app added a field. This powers the 3-stage uninstall lifecycle: remove edges (automatic), orphan report, purge (explicit confirmation required).

### 9.3 Schema change log

`schema_change_log` — one row per field change at install/upgrade time, carrying `target_type`, `target_name`, `change_type` (added/modified/removed), `app`, `app_version`, `diff`. Enables no-migration-file schema history.

### 9.4 Meta cache

`MetaCache` (moka async LRU, 1 024 entries, 5-min TTL) wraps the graph query. For Tier 0 compiled types it returns the compiled `DocTypeMeta` without hitting SurrealDB. For all other types it queries `doctype / docfield / has_field` and returns `DynDocTypeMeta`. Call `MetaCache::invalidate(doctype)` after any schema-modifying operation.

---

## 10. Site Lifecycle

### 10.1 `new-site`

```
1. Create sites/{hostname}/ on disk
2. Write site_config.toml
3. Connect to SurrealDB (ws://..., define namespace+database if absent)
4. run_framework_tables() — __Auth, tabSessions, tabSeries, tabSingles, tabMigration, graph tables, installed_app
5. ensure_all_schemas()   — DDL for all inventory::submit! Tier 0 types
6. seed_default_records() — Administrator user, default Roles, UserTypes
7. set_user_password()    — store hashed admin password in __Auth
8. run_pending_migrations()
9. Auto-install apps where app.json["auto_install"] = true
```

No `.surql` files are read at `new-site` time — the Tier 0 schema is emitted entirely from compiled Rust.

### 10.2 `install-app`

```
1. Read app.json manifest → upsert app graph node
2. seed_doctypes_for_app() — walk apps/{app}/**/doctype/{name}/{name}.json
                              UPSERT tabDocType + tabDocField + tabDocPerm rows
                              upsert doctype/docfield graph nodes + has_field edges
                              log schema changes
3. Seed Module Def records from modules.txt
4. upsert_module_node + relate_module_contains_doctype edges
5. Seed importable fixtures (Workspace, Page, Report, Print Format, …)
6. Seed patch log entries from patches.txt
7. apply_pipeline_functions(app_root) — load all surql/ in correct order
8. Upsert installed_app record
```

Re-running `install-app` is fully idempotent — all UPSERT/MERGE operations and `IF NOT EXISTS` DDL guards make every step a no-op when the data is already present.

### 10.3 `start` / `serve`

```
1. Scan sites/ for site_config.toml files
2. For each site:
   a. connect to SurrealDB
   b. ensure_all_schemas() — apply any new DDL from recompiled binary
   c. sync_user_doctype_schemas() — additive DDL for user-created DocTypes
   d. apply_naming_functions()
   e. apply_permissions_functions()
   f. apply_pipeline_bootstrap()
   g. apply_pipeline_functions_multi() — reload all installed app surql/
   h. Build SiteState (db, caches, method registry, hook registry, meta_cache)
3. Register site in AppState (keyed by hostname)
4. Bind Axum router, start serving
```

---

## 11. HTTP Request Flow

```
Client request
    │
    ▼ Tower middleware stack:
    ├── TraceLayer          (request/response logging)
    ├── CorsLayer           (CORS headers)
    ├── CompressionLayer    (gzip/brotli)
    └── site_middleware     (hostname → SiteState, JWT session → CurrentUser)
    │
    ▼ Axum router:
    ├── /api/resource/*     → routes.rs  (permission check → DB helpers)
    └── /api/method/*       → methods/mod.rs::call_method → MethodRegistry dispatch
    │
    ▼ DB layer (spotledger-db):
    ├── Tier 0 types: controller::save_doc (15-step pipeline)
    └── Runtime types: save_proxy::save_doc_proxy (auth → naming → upsert → pipeline)
    │
    ▼ SurrealDB:
    ├── DEFINE EVENT handlers (validation, compute, cascades)
    └── fn::pipeline::run (stage → node → fn dispatch)
```

---

## 12. Framework-Internal Tables

These tables are not DocTypes and never appear in the Desk UI.

| Table | Purpose |
|---|---|
| `__Auth` | Password hashes keyed by `(doctype, name, fieldname)`. Intentionally invisible. |
| `tabSessions` | Active login sessions (sid, user, status, lastupdate). |
| `tabSeries` | Naming-series counters (`prefix → current`). Atomically incremented. |
| `tabSingles` | Key/value store for Single DocTypes (`doctype, field, value`). |
| `tabMigration` | Applied migration log (name, applied_at, batch). |
| `installed_app` | Installed app registry (name, version, installed_at, app_path). |
| `doctype` | Graph node — one per DocType. |
| `docfield` | Graph node — one per DocField. |
| `has_field` | Relation edge: doctype → docfield with provenance. |
| `schema_change_log` | Audit log of every field-level schema change at install/upgrade time. |

---

## 13. Multi-Site Architecture

One Spotledger process serves multiple sites simultaneously. Each site is:
- A directory under `sites/{hostname}/` on disk
- A separate SurrealDB namespace+database
- A `SiteState` instance in `AppState`'s `DashMap<hostname, SiteState>`

The `site_middleware` extracts the `Host` header from each request and looks up the matching `SiteState`. If no site is found, the request is rejected with 404. Sites are fully isolated at the database level.

---

## 14. Key Design Decisions and Constraints

### SurrealDB v3 reserved-keyword fields
Fields named `create`, `select`, `cancel`, `delete` in `tabDocPerm` are stored as `perm_create`, `perm_select`, `perm_cancel`, `perm_delete` because the SurrealDB v3 SDK fails to deserialize responses whose field keys are SQL keywords.

### `type::record` not `type::thing`
SurrealDB v3 syntax: `type::record("table", $var)`. `type::thing` is completely removed. `type::record` cannot be used directly in RELATE/UPSERT source positions — must be assigned to a `LET` variable first.

### `THROW` not `RETURN { error }`
Domain functions that detect errors must `THROW`. A soft return leaves mutations committed. The pipeline runner converts soft errors to `THROW` as a safety net.

### Naming `fn::naming::resolve` returns "NONE" guard
`<string>(NONE)` in SurrealDB evaluates to the string `"NONE"`, not the absence of a value. Naming functions check the raw field value (`$raw_val != NONE`) before casting to string to avoid silently producing the literal string `"NONE"` as a document name.

### `custom_` prefix enforced at API boundary
Fields where `is_custom = true` on the `docfield` graph node must have fieldnames starting with `custom_`. `save_proxy.rs` rewrites the fieldname at the API boundary if missing. This prevents custom fields from shadowing standard fields.

### Additive-only schema
`ensure_schema` only adds tables and fields — it never drops or renames anything automatically. Removing a field from `DocTypeMeta` leaves the DB column as orphaned data the engine ignores. Deliberate removal requires the 3-stage uninstall lifecycle.

---

## 15. Build and Development

### Cargo xtask commands
```
cargo xtask build    ← host + all WASM apps (wasm32-wasip1) → target/plugins/
cargo xtask wasm     ← WASM apps only
cargo xtask host     ← host workspace only
cargo xtask check    ← fast cargo check --workspace
```

WASM apps are auto-discovered by scanning `apps/*/Cargo.toml`.

### Site commands
```
spotledger new-site <hostname> --bench . --db-url ws://127.0.0.1:8000 --admin-password admin
spotledger install-app spotledger-core <hostname> --bench .
spotledger install-app spotledger-finance <hostname> --bench .
spotledger use <hostname>
spotledger start
```

`sites/currentsite` (plain text file, no extension) is written by `use` and read by `start` as the default site.

---

## 16. What Rust Owns Forever

These responsibilities will never move to SurrealDB:

1. **Auth enforcement** — Frappe-style role × doctype × permission-type matrix cannot be expressed in SurrealDB's built-in permission model.
2. **Naming** — Rust assigns the record ID before the pipeline runs. The document needs an ID to exist in SurrealDB at all.
3. **`custom_` prefix enforcement** — API boundary rule; must happen before the write reaches SurrealDB.
4. **HTTP routing and session handling** — web server concerns.
5. **WASM plugin sandbox** — plugin host process must be in Rust.

Everything else — validation, computed fields, GL posting, cancel cascades, workflow transitions — lives in or is moving to SurrealDB.
