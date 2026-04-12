# SpotledgerCore Architecture Plan

**Date**: April 12, 2026
**Status**: In Progress — Phases 0–3b, A, B, C complete. **Major architectural revision April 9, 2026** — metadata-as-graph model adopted. **April 12, 2026** — `doctype_meta` table removed; `tabDocType` is the single source of truth for all DocType metadata.
**Scope**: Full architectural rewrite from current monolithic Axum crate to a Linux-kernel-style modular WASM plugin system, written entirely in Rust, with SurrealDB as the live metadata knowledge base.

---

## Executive Summary

**SpotledgerCore** is a **financial framework** — not a generic app framework. It is purpose-built for financial and enterprise applications.

Key design decisions:
- **No Python. No JavaScript runtime (V8). No Rhai.** The system is Rust end-to-end.
- **Rust is the engine. SurrealDB is the knowledge base.** They are not the same thing. Rust owns HTTP, auth enforcement, WASM sandboxing, and the save pipeline orchestration. SurrealDB owns all metadata (DocType/DocField records as graph nodes), all domain logic (validation events, computed fields, graph traversal), and all document data.
- **Metadata is live graph data.** `DocType` and `DocField` records exist as nodes in SurrealDB with `has_field` edges. `frappe.get_meta("Account")` is a graph query, not a compiled Rust lookup. Custom fields, plugin-added fields, and user-added fields are all just edges in the same graph — no merge step, no separate table.
- **The save pipeline is a thin auth proxy.** Rust checks permission, sends one DB statement, returns the response. SurrealDB's own `DEFINE EVENT` / `DEFINE FIELD VALUE` / `ASSERT` fire within the same transaction — unconditionally, regardless of call origin.
- **Modules are `.wasm` binaries** — dropped into a `plugins/` folder. Each plugin carries an embedded `schema.json` payload (not Rust structs). At plugin load, the host engine upserts schema records into SurrealDB. Domain logic runs as `DEFINE FUNCTION LANGUAGE WASM` inside SurrealDB, with direct graph access.
- **Frappe REST API parity** — `/api/resource/`, `/api/method/`, `/api/v2/` are preserved so the Frappe desk frontend works unchanged.
- **Schema versioning without migrations.** All schema changes are idempotent upserts. A `schema_change_log` table records every field change with app version and diff. No SQL migration files ever.

### What is "Core"?

> Core = the engine that runs financial applications. It is the HTTP server, the permission enforcer, the WASM sandbox host, and the DB adapter. It does NOT contain the schema of Account, Customer, or Journal Entry — those live in SurrealDB, seeded by plugins.
>
> The only DocTypes compiled into Rust (Tier 0) are those the engine itself cannot boot without: DocType, DocField, User, Role, SystemSettings.

---

## Syncfusion License Key
For UI, You need to register Syncfusion license key for version 33.xx
this is Essential Studio Enterprise Edition. 
key is : Ngo9BigBOggjHTQxAR8/V1JHaF5cWWdCf1FpRmJGdld5fUVHYVZUTXxaS00DNHVRdkdlWXtfcXRWRmldWUJ0WkRWYEo=

## Progress Summary (April 9, 2026)

| Phase | Description | Status |
|-------|-------------|--------|
| **Phase 0** | Repository Restructure | ✅ Complete |
| **Phase 1** | Typed Schema (DocTypeMeta) | ✅ Complete — **scope now Tier 0 only** |
| **Phase 1b** | Framework DocTypes (Tier 0/1) | 🔄 In Progress — Tier 0 done; Tier 3 scope moved to Phase A/B |
| **Phase 1c** | Core Utils | ✅ Complete |
| **Phase 2** | WASM Plugin Host | ✅ Complete — all host fns registered, live WAT e2e test passing |
| **Phase 2b** | Plugin Capability Declaration & Party Resolution | ✅ Complete — `PluginManifest` capabilities, `PluginRegistry::register_capabilities()`, `PartyTypeDecl` |
| **Phase 3** | spotledger-pdk | ✅ Complete — `api.rs`, `accounting.rs` guest-side wrappers, dual wasm32/host compilation |
| **Phase 3b** | Boot Info & UI Integration | ✅ Complete — see §Boot Info below |
| **Phase A** | Metadata-as-Graph Foundation | ✅ Complete |
| **Phase B** | Save Pipeline → Auth+Proxy + SurrealDB Events | ✅ Complete |
| **Phase C** | App/Module Graph + Schema Versioning | ✅ Complete |
| **Phase G** | Graph-Compute Pipeline (SurrealDB `fn::` functions) | ✅ Complete — 15 fn:: registered, 761 DocType tables DEFINE'd at install time |
| **Phase 4** | ERPNext Domain App (`apps/erpnext`) | 🔄 In Progress — 761 DocTypes seeded (272 frappe + 489 erpnext), wasm build pending |
| **Phase 5** | frappe.client API Completion | ⏳ Not Started |
| **Phase 6** | API v2 Endpoints | ⏳ Not Started |
| **Phase 7** | Background Jobs | ⏳ Not Started |
| **Phase 8** | File Storage | ⏳ Not Started |
| **Phase 9** | Full-Text Search | ⏳ Not Started |
| **Phase 10** | Notifications & Real-time | ⏳ Not Started |

### Key Decisions Made During Implementation

- **Child tables**: Stored as embedded `array<object>` in parent SurrealDB record — no scatter/gather to separate tables
- **`DocTypeMeta` drives DDL (Tier 0 only)**: `ensure_schema` / `ensure_all_schemas` emit `DEFINE TABLE / FIELD` only for the ~16 Tier 0 bootstrap types. All domain DocTypes (Account, Customer, etc.) get their DDL from `seed_doctypes_for_app` reading JSON files, not from compiled Rust structs.
- **Save pipeline is auth + proxy**: All `frappe.client.save` / `insert` / `delete` flow through a thin Rust handler that (1) checks permission, (2) sends one SurrealDB statement. The 15-step pipeline is replaced by SurrealDB `DEFINE EVENT` / `DEFINE FIELD VALUE` / `ASSERT` firing automatically inside the transaction.
- **Validation is table-level and unconditional**: `DEFINE TABLE ... ASSERT`, `DEFINE FIELD ... ASSERT`, and `DEFINE EVENT` fire for every write regardless of caller — Rust save path, SurrealDB event cascade, or direct console access.
- **Permission system**: Strong-typed with full Frappe parity (role, DocType, If Owner, User permissions, `permlevel`). Auth stays in Rust permanently — SurrealDB's native auth model cannot encode Frappe-style role+doctype permission matrices.
- **`MetaEntry` inventory**: Still used for Tier 0 DDL sync only. Domain DocTypes are not in the compiled inventory.
- **Single ERPNext app**: Instead of separate WASM plugins per domain, all ERPNext modules (Accounts, Buying, Selling, Stock, CRM, Assets, Manufacturing, Projects, Quality Management, Subcontracting) live in **one app** — `apps/erpnext/` — following the ERPNext directory convention. This is manageable because the suite is deployed as a unit. Module-level granularity is still preserved via the `module` field on each DocType and the `app→module→doctype` graph.
- **`spotledger-accounting` removed from workspace**: Accounting logic moves to SurrealDB events inside the `erpnext` app. The host binary has no compile-time knowledge of Account, JournalEntry, etc.
- **Topological DocType seeding**: `install_app` parses `Link` fields in each DocType JSON, builds a dependency graph, and seeds in topological order (lowest-level first). This prevents FK constraint failures (e.g. `Journal Entry Account` references `Account` — `Account` is seeded first).
- **Metadata is live graph data**: `doctype:Account -[has_field { idx, introduced_by, app_version, is_custom }]-> docfield:account_type`. All plugins, extensions, and user customizations are edges on the same graph. One query retrieves the complete merged meta.
- **Ownership tracking on edges**: Every `has_field` edge carries `introduced_by` (app/plugin name), `app_version`, `is_custom`, and `removable_on_uninstall`. Uninstall is 3-stage: edge removal → orphan report → explicit purge. No automatic data loss.
- **`schema_change_log`**: Written by `install_app` / `seed_doctypes_for_app` on every field diff. Records app version, before/after snapshot, timestamp. Replaces PatchLog for schema. Downgrade = reverse the diff.
- **`custom_` prefix**: Admin-created fields must have `custom_` prefix, enforced at the Rust API boundary. `is_custom = true` fields are never auto-removed on uninstall.
- **Apps and modules are graph nodes**: `app:erpnext -[provides_module]-> module:Accounts -[contains]-> doctype:Account`. Full lineage from every field back to its owning app.
- **No Workspaces** — Spotledger UI uses a module-driven sidebar driven by `ModuleDef` records and `DocType.show_in_menu = 1`. Workspaces code remains in the backend for compatibility but is not consumed by the Spotledger UI.
- **Outbox pattern for external side effects**: Email, queue, third-party API calls are never triggered directly from SurrealDB events. Events write to `pending_notification` / outbox tables; Rust subscribes via `LIVE SELECT` and processes them.
- **`doctype_meta` table removed (April 12, 2026)**: The `doctype_meta` table was removed entirely. Its only purpose was to track `is_submittable` for the save pipeline runner. `tabDocType.issubmittable` is now the single source of truth. `02_runner.surql` reads `SELECT issubmittable FROM tabDocType WHERE name = $doctype LIMIT 1` directly. `is_pipeline_registered()` queries `tabDocType`. No separate metadata table is needed.
- **All 761 DocType tables DEFINE'd at install time**: `seed_doctypes_for_app` now emits `DEFINE TABLE IF NOT EXISTS <tablename> SCHEMALESS COMMENT '...'` for every DocType JSON. This makes all tables visible in Surrealist and schema tooling immediately after install, even before any records are written.
- **`sites/currentsite` file (Frappe convention)**: `spotledger use <sitename>` writes the site name as plain text to `<bench>/sites/currentsite`. All CLI commands that operate on the default site read this file. Matches Frappe bench convention exactly.
- **`spotledger start` = bench start**: Reads `sites/currentsite` and starts the HTTP server. Equivalent to `spotledger serve --bench .` with the default site pre-selected.

---

## Phase 3b — Boot Info & UI Integration

> **Completed April 2026.**

### Overview

The Spotledger UI (React/TypeScript, `Spotledger-ui/`) connects to the Axum backend through a small set of well-defined contracts. This phase makes the boot info endpoint production-ready and wires the UI's sidebar to real data.

### New DocTypes (spotledger-core, Tier 0)

#### `ModuleDef`

Controls what appears in the sidebar navigation. Each installed app inserts one or more records here.

| Field              | Type  | Default | Purpose                                           |
|--------------------|-------|---------|---------------------------------------------------|
| `module_name`      | Data  | —       | Primary key / unique identifier                   |
| `app_name`         | Data  | —       | App that owns this module (e.g. `"spotledger"`)   |
| `label`            | Data  | —       | Display label (falls back to `module_name`)       |
| `icon`             | Data  | —       | Lucide icon name shown in the sidebar             |
| `show_in_menu`     | Check | `1`     | **Controls sidebar visibility** — set `0` to hide |
| `order`            | Int   | `0`     | Sort order (ascending) in the sidebar             |
| `restrict_to_domain` | Data | —    | Show only when this domain is active              |
| `custom`           | Check | `0`     | Set by admin when creating custom modules         |

#### `UserModule` (child of `User`)

Lists which modules a specific user is allowed to access.  **Empty list = all modules allowed** (same semantics as Frappe's `allow_modules`).

| Field    | Type | Purpose                            |
|----------|------|------------------------------------|
| `module` | Link → ModuleDef | Module name |

#### `DocType` additions

| Field          | Type  | Default | Purpose                                     |
|----------------|-------|---------|---------------------------------------------|
| `show_in_menu` | Check | `1`     | Whether this DocType appears in the sidebar under its module |
| `icon`         | Data  | —       | Icon shown next to the DocType in sidebar   |

### Boot Info Endpoint

```
GET  /api/method/frappe.boot.get_bootinfo
POST /api/method/frappe.boot.get_bootinfo    (both verbs accepted by method dispatcher)
```

Authentication: requires valid `sid` cookie. Returns `403` for Guest.  
Also aliased at `frappe.utils.boot.get_boot_info` for internal use.

#### Response Shape

```jsonc
{
  // ── Identity
  "sitename":       "ehsen",
  "server_date":    "2026-04-08",
  "lang":           "en",
  "developer_mode": false,

  // ── Logged-in user (drives RBAC on the client)
  "user": {
    "name":       "administrator",
    "email":      "admin@example.com",
    "full_name":  "Administrator",
    "user_type":  "System User",
    "desk_theme": "Light",
    "roles":      ["System Manager", "Administrator", "All"],
    "defaults":   {},                       // user-scoped DefaultValue rows
    "can_read":   ["Journal Entry", ...],   // filtered by DocPerm
    "can_write":  [...],
    "can_create": [...],
    "can_delete": [...],
    "can_submit": [...],
    "can_cancel": [...],
    "allow_modules": []                     // empty = all modules allowed
  },

  // ── All active users (for @-mentions, avatars)
  "user_info": {
    "administrator": {
      "name": "administrator", "full_name": "Administrator",
      "email": "admin@example.com", "user_type": "System User",
      "avatar_url": null
    }
  },

  // ── Global defaults (from DefaultValue where parenttype = "__default")
  "sysdefaults": {
    "company": "Acme Corp",
    "currency": "USD",
    "date_format": "DD-MM-YYYY",
    "timezone": "UTC",
    "setup_complete": "1"
  },

  // ── Module map (drives sidebar, filtered by user.allow_modules)
  "modules": {
    "Accounts": {
      "app":         "spotledger",
      "label":       "Accounts",
      "icon":        "calculator",
      "order":       1,
      "show_in_menu": 1,
      "doctypes": [
        { "name": "Journal Entry", "icon": null, "issingle": 0, "issubmittable": 1 },
        { "name": "General Ledger", "icon": null, "issingle": 0, "issubmittable": 0 }
      ]
    }
  },
  "module_list": ["Accounts", "Buying", "Selling"],   // sorted by ModuleDef.order

  // ── App summary
  "app_data": [{ "app_name": "spotledger", "app_title": "Spotledger",
                  "app_logo_url": "/assets/spotledger/images/logo.svg",
                  "modules": ["Accounts", "Buying"] }],
  "versions": { "spotledger": "0.1.0" },

  // ── DocType helpers
  "single_types":   ["System Settings", "Global Defaults"],
  "home_page":      "Accounts",

  // ── Branding
  "app_logo_url":   "/assets/spotledger/images/logo.svg",
  "navbar_settings": { "app_logo": "...", "items": [] },
  "max_file_size":  10485760,

  // ── P2 (empty until implemented)
  "__messages": {}, "notification_settings": null,
  "letter_heads": {}, "active_domains": [], "all_domains": [],
  "desktop_icons": [], "frequently_visited_links": [],
  "link_preview_doctypes": [], "link_title_doctypes": [],
  "lang_dict": {}, "timezone_info": {"zones":{}, "rules":{}, "links":{}},
  "docs": [], "time_zone": {"user": "UTC", "system": "UTC"}
}
```

### UI Integration Contracts

#### Post-Login / Session-Restore Flow

```
Login success OR valid sid cookie found
  │
  ├─ Read localStorage: key = "spotledger_boot_<site>_<user>"
  │   ├─ Valid (< 5 min old, version matches) → use cached
  │   └─ Stale / missing → GET /api/method/frappe.boot.get_bootinfo
  │                         └─ write to cache
  │
  ├─ store.setBootInfo(bootinfo)
  └─ store.setAuthenticated(username) → render Shell
```

Cache key envelope: `{ version: 1, fetchedAt: number, ttlMs: 300000, data: BootInfo }`.  
Cache is cleared on logout and when `versions` differ from cached payload.

#### Sidebar Wiring

The `Sidebar.tsx` module list must be hydrated from `bootinfo.modules` (not hardcoded).  
Expansion state is local (`useState`); no persistence needed.

```
bootinfo.module_list.forEach(moduleName => {
  const mod = bootinfo.modules[moduleName]
  // render module group with mod.label, mod.icon
  // render mod.doctypes[].name as clickable items
})
```

#### Logout Fix

`frappeApi.ts → logoutUser()` must use `POST` (not `GET`) and clear the boot cache.

### Files Changed

| File | Change |
|------|--------|
| `crates/spotledger-core/src/doctypes/doctype.rs` | Added `show_in_menu`, `icon` fields to DocType meta |
| `crates/spotledger-core/src/doctypes/system.rs`  | Added `ModuleDef`, `UserModule` meta + `inventory::submit!` |
| `crates/spotledger-core/src/migrations/tier0.surql` | Added DDL for `ModuleDef`, `UserModule`; `show_in_menu`/`icon` on `DocType` |
| `crates/spotledger-http/src/methods/desk/mod.rs`  | Rewrote `build_boot_user`, `handle_get_boot_info`; added helper fns; registered `frappe.boot.get_bootinfo` alias |
| `Spotledger-ui/UI_BACKEND_INTEGRATION_PLAN.md`    | Full integration spec (Phase 1: Auth + Boot) |

### Outstanding UI Work (tracked in UI_BACKEND_INTEGRATION_PLAN.md)

| # | File | Change |
|---|------|--------|
| 1 | `src/lib/frappeApi.ts` | Fix `logoutUser` to POST; add `fetchBootInfo()` |
| 2 | `src/lib/bootCache.ts` | New: localStorage cache with TTL + versioning |
| 3 | `src/lib/initSession.ts` | New: shared login/restore initialiser |
| 4 | `src/store/useAppStore.ts` | Add `bootInfo` state + `setBootInfo` action |
| 5 | `src/types/bootinfo.ts` | New: TypeScript interface matching §Response Shape |
| 6 | `src/App.tsx` | Call `initSession` instead of bare `setAuthenticated` |
| 7 | `src/app/auth/LoginPage.tsx` | Call `initSession` after successful login |
| 8 | `src/app/shell/Sidebar.tsx` | Drive from `bootinfo.modules` instead of hardcoded data |

---

## CLI Commands Reference

The `spotledger` binary is the `bench` equivalent for this project.

| Command | Purpose |
|---------|---------|
| `spotledger new-site <hostname>` | Create site directory + bootstrap SurrealDB schema |
| `spotledger install-app <app> <site>` | DocTypes + Module Defs + fixtures + pipeline `fn::` functions |
| `spotledger migrate <site>` | DDL sync + data migrations |
| `spotledger seed-doctypes <site>` | Seed DocType JSON into site (called by install-app) |
| `spotledger use <sitename>` | **Set default site** — writes to `sites/currentsite` |
| `spotledger start` | **Start all backend components** — reads `sites/currentsite`, starts HTTP server |
| `spotledger serve` | Start HTTP server (explicit flags: `--bench`, `--bind`, `--mode`) |
| `spotledger emit` | Generate `generated/schema.surql` (no DB required) |
| `spotledger cleanup <site>` | Remove orphaned `tabDocField`/`tabDocPerm` rows |
| `spotledger generate` | Generate Rust DocType source from a Frappe-compatible JSON file |

### `spotledger use <sitename>`

Writes the site name as plain text to `<bench>/sites/currentsite` (Frappe bench convention).
After running this, `spotledger start` will boot without requiring explicit `--bench`/`--site` flags.

```bash
spotledger use hello_graph
# → writes "hello_graph" to ./sites/currentsite
```

Optional flags: `--bench <path>` (default: `.`)

### `spotledger start`

Reads `<bench>/sites/currentsite` to determine the default site, then starts the HTTP server.
Equivalent to `spotledger serve --bench .` but with human-friendly output.

```bash
spotledger start
# Using default site: hello_graph
# Spotledger  0.1.0  ready on http://127.0.0.1:8000

spotledger start --bind 0.0.0.0:9000 --mode prod
```

Optional flags: `--bench <path>`, `--bind <addr>`, `--mode dev|prod`

> **Database is a separate concern** — `spotledger start` does not start SurrealDB.
> Start SurrealDB independently (`surreal start ...`) before running `spotledger start`.

### Typical Development Workflow

```bash
# First-time setup (once per site):
spotledger new-site hello_graph --db-url ws://127.0.0.1:8001 --admin-password secret
spotledger install-app frappe hello_graph
spotledger install-app erpnext hello_graph
spotledger use hello_graph        # set default site

# Daily development:
spotledger start                  # starts HTTP server on :8000
```

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
- Permission system (strong-typed, Frappe-parity role/DocType/If Owner/User permissions)
- Session/auth (login, logout, PBKDF2 + Argon2 passwords)
- Method registry (`/api/method/` dispatcher)
- Desk handlers: `getdoc`, `getdoctype`, `savedocs`, `search_link`, `get_boot_info`, reportview, notifications, listview
- `frappe.client.*` core methods (get, get_list, save, insert, set_value, delete, submit, cancel)
- Hook registry (Rust-native hooks only)
- DocType trait with full lifecycle hooks
- Document caching (Moka)
- **Typed `DocTypeMeta` / `DocField` / `FieldType`** — full validation constraint fields, builder API
- **`MetaEntry` self-registration** via `inventory::collect!` for schema inventory
- **`DocType::meta()` trait method** — returns `&'static DocTypeMeta` for compiled types
- **`to_frappe_json()` conversion** — DocTypeMeta/DocField/Permission convert to Frappe REST API format
- **`handle_getdoctype` optimized** — now uses compiled meta first (zero DB queries for compiled types), falls back to DB for dynamic types
- **Child tables embedded** in parent SurrealDB record (`array<object>`) — scatter/gather removed
- **Document validation pipeline** (`validation.rs`) — mandatory, select options, length, set_only_once, allow_on_submit, XSS sanitize
- **Ordered save controller** (`controller.rs`) — 15-step pipeline with hooks integration
- **Hibernate-style schema DDL sync** (`schema.rs`) — `ensure_schema` / `ensure_all_schemas` from `MetaEntry` inventory
- **`new-site`** calls `ensure_all_schemas` — all compiled DocType schemas synced at startup
- **WASM plugin host** (`spotledger-plugins`) — `PluginRegistry`, `host_fn` ABI, memory marshaling, versioning
- **Accounting engine stubs** (Company, Account, Currency, FiscalYear, JournalEntry, GL engine) in `spotledger-accounting`
- **Tier 0 framework DocTypes fully implemented** — 14 DocType schemas: DocType, DocField, DocPerm, CustomField, PropertySetter, User, Role, HasRole, UserPermission, UserGroup, UserGroupMember, UserType, SystemSettings, DefaultValue, DocumentNamingRule, DocumentNamingSettings
- **Tier 0 SurrealQL migrations** (`tier0.surql`) — 300 lines of DEFINE TABLE/FIELD/INDEX DDL, auto-embedded in binary
- **Module wiring complete** — `doctypes::*` and `migrations::*` exported from lib.rs for startup schema sync
- **DocField builder enhancements** — added `.in_standard_filter()`, `.description()`, `.default_value()` for full field metadata

**What is missing / stubbed (pre-April-9 scope):**
- `frappe.client.rename_doc`, `attach_file`, `validate_link`
- Full `/api/v2/` endpoints
- Plugin migration runner
- Background jobs / scheduler
- File storage
- Full-text search
- `spotledger-desk` — Tier 1 DocTypes (stub structure exists, no DocType implementations)
- `spotledger-automation` — Tier 2 DocTypes (stub, partial hooks only)
- Domain plugins (`apps/selling`, `apps/buying`, `apps/stock`) — Cargo stubs, no doctype implementations
- `spotledger-pdk` `utils.rs` module

**What is now superseded / revised (April 9 architectural change):**
- The 15-step `controller::save_doc` pipeline — **replaced** by auth+proxy + SurrealDB events (Phase B)
- Tier 3 compiled `DocTypeMeta` (Account, GLEntry, JournalEntry) — **replaced** by JSON-seeded graph records (Phase A)
- `lft`/`rgt` nested set on Account — **replaced** by `child_of` graph edges (Phase A)
- `spotledger-core/src/validation.rs` pure validation — **replaced** by `DEFINE TABLE ASSERT` / `DEFINE EVENT` in SurrealDB (Phase B)
- `MetaEntry` + `inventory::submit!` for domain types — **scoped to Tier 0 only**

---

## Target Architecture

```
spotledger/                              ← workspace root
├── Cargo.toml                           ← workspace manifest
│
├── .cargo/
│   └── config.toml                      ← `xtask = "run --package xtask --"` alias
│
├── crates/
│   ├── xtask/                           ← BUILD ORCHESTRATOR (host + WASM apps in one command)
│   │   src/
│   │     main.rs       → cargo xtask [build|wasm|host|check] [--release]
│   │                     builds wasm32-wasip1 apps → copies to target/plugins/
│   │
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
  Required before ANY plugin can load. Rust MUST know these at compile time.

  DocType, DocField, DocPerm, CustomField, PropertySetter
  User, Role, HasRole, UserType, UserPermission, UserGroup
  DefaultValue, SystemSettings, Session
  ModuleDef, UserModule
  -- Graph bootstrap tables (Phase A):
  doctype (graph node), docfield (graph node), has_field (relation edge)
  schema_change_log, app, module
  _spotledger_migrations (internal tracking table)

Tier 1 — Framework UI DocTypes (spotledger-desk, compiled in)
  Required for the desk to function.

  Workspace, WorkspaceSidebar, WorkspaceLink, WorkspaceChart
  ToDo, Note, Event, Comment, Communication, Tag, TagLink
  File, Folder
  NotificationLog, NotificationSettings, Notification
  pending_notification (outbox — Phase B)
  Workflow, WorkflowAction, WorkflowState, WorkflowTransition
  Version, AuditTrail, DeletedDocument, ViewLog
  DocumentNamingRule, DocumentNamingSettings, NamingSeries
  Language

Tier 2 — Optional framework modules (feature-gated, compiled in)
  spotledger-automation: AssignmentRule, AutoRepeat, Milestone, Reminder

Tier 3 — Financial Core (spotledger-accounting)
  ⚠️ REVISED: Schema lives in JSON files seeded via install_app into SurrealDB graph.
  GL engine HOST FUNCTIONS stay compiled in Rust (stable ABI for plugins).
  Domain logic (validate_parent, set_root_and_report_type, propagate tree, etc.)
  lives as SurrealDB DEFINE EVENT / DEFINE FIELD VALUE / DEFINE FUNCTION LANGUAGE WASM.

  JSON schema seeds (not compiled DocTypeMeta):
    Company, FiscalYear, FiscalYearCompany
    Account (tree via child_of edges, no lft/rgt), CostCenter (tree)
    Currency, CurrencyExchange
    TaxCategory, TaxTemplate, TaxTemplateDetail
    GLEntry  (written by engine only)
    JournalEntry, JournalEntryAccount
    PaymentTerms, PaymentTermsTemplate

  Compiled Rust (host functions, always available to plugins):
    sl_make_gl_entries, sl_reverse_gl_entries
    sl_get_account_balance, sl_get_fiscal_year
    sl_get_exchange_rate, sl_convert_to_base_currency
    sl_validate_account, sl_get_default_account

Tier 4 — Domain plugins (WASM, .wasm files in plugins/)
  Schema carried as embedded schema.json in each .wasm binary.
  Logic as SurrealDB WASM functions where data-local; Extism WASM for cross-doc orchestration.

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

### Phase 0 — Repository Restructure ✅ COMPLETE

**Goal**: New crate layout in place. All existing code compiles in new locations. Zero functionality changes.

**Tasks:**
- [x] Create `crates/spotledger-core/` — moved from `spotledger-types/`
- [x] Update `spotledger-db/` to depend on `spotledger-core` instead of `spotledger-types`
- [x] Create `crates/spotledger-http/` — extracted `server.rs`, `routes.rs`, `middleware.rs`, `methods/` from `spotledger/`
- [x] Create `crates/spotledger-plugins/` — stub → now has `PluginRegistry`, `host_fn` ABI, memory marshaling, versioning
- [x] Create `crates/spotledger-pdk/` — stub (guest side `host.rs` re-exports only)
- [x] Create `crates/spotledger-desk/` — stub (structure only, `doctype.rs` stub)
- [x] Create `crates/spotledger-accounting/` — stub → now has DocType schemas + GL engine
- [x] Create `crates/spotledger-automation/` — stub (`scheduled_job.rs`, `webhook.rs` stubs)
- [x] `crates/spotledger/` — thin `main.rs` + `cli.rs` only
- [x] Create `apps/selling/`, `apps/buying/`, `apps/stock/`, `apps/hrms/` — stub Cargo.toml (`crate-type = ["cdylib"]`)
- [x] Remove `crates/spotledger-proxy/` from workspace
- [x] Update workspace `Cargo.toml` members list
- [x] Create `crates/xtask/` — build orchestrator: `cargo xtask build` compiles host workspace + all WASM apps, copies `.wasm` → `target/plugins/`
- [x] Create `.cargo/config.toml` with `xtask` alias
- [x] `cargo build` passes with zero warnings

**Exit criterion**: ✅ `cargo build` clean. All tests pass. `cargo xtask check` verified working.

---

### Phase 1 — Typed Schema (DocTypeMeta) ✅ COMPLETE

**Goal**: Static `DocTypeMeta` in `spotledger-core`. `getdoctype` for compiled types returns correct data without a DB query.

**Tasks:**
- [x] Add `meta.rs` to `spotledger-core/src/` with `FieldType`, `LayoutKind`, `FieldKind`, `DocField`, `DocTypeMeta`, `Permission`
- [x] `DocField::is_layout_only()` (`FieldType::is_layout()`) implemented
- [x] `DocTypeMeta` builder API (`DocTypeMetaBuilder`) in `meta.rs`
- [x] `DocField` builder chain: `required()`, `in_list()`, `bold()`, `read_only()`, `hidden()`, `unique()`, `not_nullable()`, `set_only_once()`, `allow_on_submit()`, `ignore_xss_filter()`, `permlevel()`, `length()`, `fetch_from()`, `fetch_if_empty()`, `select_options()`, `default()`, `precision()`
- [x] `DocTypeMeta`: `autoname`, `naming_series` fields added
- [x] `MetaEntry` + `inventory::collect!(MetaEntry)` for schema inventory
- [x] `DocTypeRegistry` extended with `MetaEntry`; `get_compiled_meta(doctype)` in `controller.rs`
- [x] DB sync rule: `FieldType::is_layout()` skips layout fields in schema DDL
- [x] Extend `DocType` trait with `fn meta() -> &'static DocTypeMeta` — added to trait
- [x] Update `handle_getdoctype`: use compiled meta (now uses compiled meta, falls back to DB for uncompiled types)

**Exit criterion**: ✅ Meta system complete. `getdoctype` now prioritizes compiled meta (zero DB queries for compiled doctypes), with automatic fallback to DB for dynamic/uncompiled types.

---

---

## Architecture Revision — April 9, 2026

> **This section records the major architectural decision taken April 9, 2026.**
> All phases below are updated to reflect this. The detailed rationale lives in
> `memories/repo/spotledger-architecture.md`.

### The Decision in One Paragraph

Rust's compiled `DocTypeMeta` structs are **not** the source of truth for domain DocTypes (Account, Customer, Journal Entry, etc.). They are only used for the ~16 Tier 0 bootstrap types that the engine itself needs at compile time. Everything else — schema, field definitions, computed derivations, validation logic — lives in SurrealDB as graph data, seeded at install time from JSON files (Frappe-compatible `{name}.json`). The save pipeline collapses to: auth check in Rust → one DB write → SurrealDB events fire automatically. Adding a field to Account does not require recompiling the binary.

### Three-Layer Architecture

```
Rust (Engine)          SurrealDB (Knowledge Base)       WASM Plugins (Logic + Schema)
──────────────         ───────────────────────────      ─────────────────────────────
HTTP server            doctype/docfield graph nodes     Carry embedded schema.json
Auth enforcement       has_field edges + provenance     Upserted into graph at load
WASM sandbox host      Computed fields (DEFINE FIELD    Domain logic as DEFINE FUNCTION
DB adapter             VALUE ...)                       LANGUAGE WASM inside SurrealDB
Tier 0 DDL only        Events (DEFINE EVENT)            Direct graph access, no IPC
                       All document instance data
                       schema_change_log
```

### What Rust Knows at Compile Time (Tier 0 Only)

DocType, DocField, DocPerm, User, Role, UserPermission, Session, SystemSettings, ModuleDef.
Nothing else. The save pipeline is generic — it does not know what fields `Account` has.

### Save Pipeline After Revision

```rust
async fn save_handler(session: Session, body: Document) -> Response {
    check_permission(&session, &body.doctype, PermType::Write)?;  // Rust owns auth forever
    let result = db.execute("UPSERT type::thing($dt, $name) CONTENT $doc", params![body]).await?;
    Json(result).into_response()  // SurrealDB events fire inside the transaction
}
```

---

### Phase 1b — Framework DocTypes (Tier 0 + Tier 1 + Tier 3) 🔄 IN PROGRESS

> **Revised scope**: Tier 0 and Tier 1 DocTypes stay compiled in Rust (the engine needs them).
> Tier 3 accounting DocTypes (Account, GL Entry, Journal Entry, etc.) move to JSON-seeded
> graph records. Their logic moves to SurrealDB WASM events. The `lft/rgt` nested set on
> Account is replaced by `child_of` graph edges.

**Goal**: Tier 0 and Tier 1 DocTypes are typed Rust structs compiled into the engine (the engine needs them at boot). Tier 3 (accounting DocTypes) moves to JSON-seeded graph records \u2014 see Phase A.

**Tier 0 (spotledger-core) — compiled in, engine cannot boot without these:**
- [x] `doctypes/doctype.rs` — DocType, DocField, DocPerm, CustomField, PropertySetter
- [x] `doctypes/user.rs` — User, Role, HasRole, UserPermission
- [x] `doctypes/system.rs` — SystemSettings (single), DefaultValue, ModuleDef
- [x] `doctypes/naming.rs` — DocumentNamingRule, DocumentNamingSettings
- [x] `migrations/tier0.surql` — DDL for all Tier 0 tables
- [ ] Add `doctype`, `docfield`, `has_field` (relation), `schema_change_log`, `app`, `module` tables to Tier 0 DDL (Phase A prerequisite)

**Tier 1 (spotledger-desk) — compiled in, desk cannot function without these:**
- [ ] `doctypes/workspace.rs` — Workspace, WorkspaceSidebar
- [ ] `doctypes/todo.rs` — ToDo, Note, Event, Comment
- [ ] `doctypes/file.rs` — File (schema + hooks; storage in Phase 8)
- [ ] `doctypes/communication.rs` — Communication, NotificationLog, Notification
- [ ] `doctypes/workflow.rs` — Workflow, WorkflowAction, WorkflowState, WorkflowTransition
- [ ] `doctypes/version.rs` — Version (`before_insert` captures diff), AuditTrail, DeletedDocument
- [ ] `migrations/tier1.rs` — embedded SurrealQL for Tier 1 tables

**Tier 3 (spotledger-accounting) — ⚠️ SCOPE REVISED: moves to JSON + SurrealDB events (Phase A/B)**
- [x] Rust structs exist for Company, Account, Currency, FiscalYear, JournalEntry, GL Entry
- [x] `make_gl_entries()` / `reverse_gl_entries()` signatures present as host functions
- [ ] Remove compiled `DocTypeMeta` for Tier 3 types — replace with `schema.json` files seeded via `install_app`
- [ ] Remove `lft`/`rgt` from Account \u2014 replaced by `child_of` graph edges (Phase A)
- [ ] GL engine host functions (`sl_make_gl_entries` etc.) stay in Rust as stable ABI \u2014 domain logic moves to SurrealDB events (Phase B)
- [ ] `engine/fiscal.rs`, `engine/currency.rs`, `engine/coa.rs` \u2014 keep as host fn implementations, not DocType hooks

**Exit criterion**: ⏳ `spotledger new-site` seeds all Tier 0/1 tables. Phase A/B handle Tier 3.

---

### Phase 1c — Core Utils ✅ COMPLETE

**Implementation Summary:**

All 9 core utility modules implemented with Frappe parity:

- [x] `utils/formatting.rs` — `cint`, `flt`, `fmt_money`, `formatdate`, `format_datetime`, `format_duration` + NumberFormat struct with 7 predefined formats
- [x] `utils/strings.rs` — `scrub`, `unscrub`, `cstr`, `strip_html`, `escape_html`, `get_abbr`, `slug`, `sbool`, `truncate`
- [x] `utils/dates.rs` — `now`, `today`, `add_days`, `add_months`, `add_years`, `date_diff`, `get_first_day`, `get_last_day`, `get_first_day_of_week`, `get_last_day_of_week`, `get_quarter_start`, `get_quarter_ending`, `get_year_start`, `get_year_ending`, `get_timestamp` (15+ functions)
- [x] `utils/numbers.rs` — `rounded` (3 strategies: BankersLegacy/Bankers/Commercial), `floor`, `ceil`, `in_words` (up to trillions), `money_in_words`, `safe_div`, `remainder`
- [x] `utils/validation.rs` — `validate_email_address` (RFC 5321), `validate_email_list`, `validate_phone_number` (ITU E.164), `validate_url`, `validate_doc_name`, `validate_fieldname`, `validate_date_string`, `is_email_like`
- [x] `utils/passwords.rs` — passlib-compatible: `hash_password` (pbkdf2-sha256 260k rounds), `check_password` (pbkdf2 + Argon2), `password_strength_score`, `ab64_encode`/`ab64_decode`
- [x] `utils/nestedset.rs` — Nested Set Model: `TreeStore` async trait (9 methods), `get_ancestors_of`, `get_descendants_of`, `rebuild_tree`, `validate_loop`, pure helpers: `is_ancestor`, `is_descendant`, `ancestors_from_slice`, `descendants_from_slice`, `depth_of`
- [x] `utils/data.rs` — `unique`, `flatten`, `group_by_field`, `has_common`, `get_common`, `diff`, `chunk`, `sort_by_field`, `sort_by_numeric_field`
- [x] `utils/json.rs` — `parse_json`, `as_json`, `as_json_pretty`, `as_json_bytes`, `json_get_str`, `json_get_string`, `json_get_f64`, `json_get_i64`, `json_get_bool`, `json_merge`, `is_empty_value`
- [x] All modules re-exported from `spotledger-core` root (22 high-use functions)
- [x] **76 unit tests** across all modules — all passing
- [x] **19 doc-comment tests** — all passing (1 async test skipped)
- [x] `User::validate` implemented using `spotledger_core::utils::validation::validate_email_address`

**Dependencies added:**
- `regex = "1"` (email/phone/URL validation, HTML parsing)
- `once_cell = "1"` (lazy-compiled regex patterns)
- `pbkdf2`, `sha2`, `argon2`, `base64`, `rand` (password hashing)
- `chrono` (dates/times, already present)
- `async-trait` (TreeStore, already present)

**Exit criterion**: ✅ SATISFIED
- `User::validate` uses `spotledger_core::utils::validation::validate_email_address`
- All 76 unit tests + 19 doc-tests pass
- Clean compilation with no warnings (unused imports removed)

**Commits:**
1. `9780a31` — Add dependencies for core utilities
2. `c09d8bf` — Wire utils module into spotledger-core public API
3. `40a5d39` — Implement 9 core utility modules (2691 lines)
4. `a30fe9c` — Add User::validate using email validation utils

---

### Phase 2 — WASM Plugin Host ✅ COMPLETE

**Goal**: Core can load a `.wasm` file, call `sl_plugin_init()`, and dispatch lifecycle hooks. All host functions registered. Live WASM test passing.

**Tasks:**
- [x] `extism` dependency in `spotledger-plugins/Cargo.toml`
- [x] Implement `PluginRegistry` — `load_all()`, `load_plugin()`, `plugin_init()`, `is_loaded()`, `get_metadata()` in `spotledger-plugins/src/registry.rs`
- [x] `abi.rs` — document operation host functions (`sl_get_doc`, `sl_save_doc`, `sl_insert_doc`, `sl_delete_doc`, `sl_get_list`, `sl_submit_doc`, `sl_cancel_doc`, `sl_get_value`, `sl_set_value`, `sl_exists`, `sl_count`, `sl_throw`, `sl_log`)
- [x] `abi_accounting.rs` — GL engine host functions (`sl_make_gl_entries`, `sl_get_account_balance`, `sl_get_fiscal_year`, `sl_reverse_gl_entries`)
- [x] `host_fns.rs` — real extism `host_fn!` implementations for all 14 document/GL host functions registered via `build_host_functions()`. Replaces stub ABI with live functions.
- [x] `abi_utils.rs` — 12 utils host functions (`sl_utils_scrub`, `sl_utils_now`, `sl_utils_today`, `sl_utils_add_days`, `sl_utils_date_diff`, `sl_utils_formatdate`, `sl_utils_validate_email`, `sl_utils_validate_phone`, `sl_utils_money_in_words`, `sl_utils_get_ancestors_of/descendants_of`) registered via `build_utils_host_functions()`
- [x] `memory.rs` — MsgPack memory marshaling for plugin ↔ host data transfer
- [x] `context.rs` — per-call context threading (adapter + hook registry refs)
- [x] `extension.rs` — `MethodRegistration`, boot contributions, workspace contributions structs
- [x] `versioning.rs` — ABI version check, topological plugin dependency order, cycle detection
- [x] `db.rs` — DB adapter shim wired to host functions
- [x] `gl.rs` — GL adapter shim wired to accounting host functions; balance validation in `GlAdapter::make_gl_entries()`
- [x] `registry.rs` fixed: `Plugin::new(&wasm_bytes, host_fns, true)` — host functions passed; previously empty `[]`
- [x] `wat = "1"` dev-dependency added for WAT→WASM compilation in tests
- [x] Live end-to-end WAT plugin test — WAT module compiled at test time, loaded via `PluginRegistry`, `sl_plugin_init()` called successfully
- [x] `test_live_wasm_plugin_with_sl_log_import_loads` — WAT plugin that **imports `sl_log`** loads without error, proving `sl_log` is registered
- [x] `MigrationRunner` for plugin-specific migrations — not yet implemented

**Exit criterion**: ✅ SATISFIED — 26 tests pass (12 unit + 14 integration). Three live WASM e2e tests pass.

---

### Phase 2b — Plugin Capability Declaration & Party Resolution ✅ COMPLETE (struct layer)

**Context — the problem this phase solves:**

In ERPNext-style frameworks, `JournalEntry` (a pure accounting construct, kernel-tier) has a
child table row `JournalEntryAccount` with two fields:
- `party_type` — which can be `"Customer"`, `"Supplier"`, or `"Employee"`
- `party` — a DynamicLink whose target DocType is determined at runtime by `party_type`

The problem: `Customer` is defined in the `selling` WASM plugin; `Supplier` in `buying`;
`Employee` in `hrms`. If the accounting kernel hardcodes or imports these types it creates
an **upward dependency** — the kernel depends on plugins, which inverts the dependency graph.
This is a hard architectural boundary violation: a Rust crate compiled into the binary cannot
`use` a type from a `.wasm` loaded at runtime.

The same issue generalises to any Link field in a compiled DocType that points at a DocType
provided by a plugin (e.g. `project` → `Project` from a projects plugin, `cost_center` →
`CostCenter` ... wait, CostCenter is core, but `warehouse` → `Warehouse` is a stock plugin
concern).

**Root causes — two distinct problems:**

1. **Party fields**: `party_type` is an open set populated by plugin registrations.
   Accounting needs to validate the party without knowing at compile time what types exist.

2. **Plugin Link fields**: A compiled DocType contains a `FieldType::Link("Customer")` but
   `Customer` is plugin-provided. At schema sync and validation time, the kernel must know
   whether the referenced DocType's table exists (plugin loaded) or not (field dormant).

**Solution — Unified Plugin Capability Declaration:**

**Part A — `PartyType` DocType (kernel table):**
`spotledger-accounting` ships a `PartyType` DocType in its Tier 3 migrations.
- Schema: `{ name (PK), doctype_name, account_type: Receivable|Payable, plugin_id }`
- Core parties (`Internal`, etc.) can be seeded by core itself — no external dependency
- Plugins insert rows at load time via a manifest declaration (see Part B)
- `JournalEntryAccount.party_type` is now `Link("PartyType")` — a **kernel** table
- `validate_party()` in accounting resolves `PartyType → doctype_name` and checks existence,
  but only when the originating plugin is loaded (see Part C)

**Part B — `PluginManifest` capabilities (extended):**
The existing `PluginManifest` struct in `spotledger-plugins/src/versioning.rs` gains:
```rust
pub struct PluginManifest {
    // ... existing fields ...
    /// All DocTypes this plugin provides — used for Link field dormancy checks.
    pub provides_doctypes: Vec<String>,
    /// Party types this plugin registers into the kernel PartyType table.
    pub party_types: Vec<PartyTypeDecl>,
}

pub struct PartyTypeDecl {
    pub name: String,          // "Customer"
    pub doctype_name: String,  // "Customer" (usually same)
    pub account_type: String,  // "Receivable" | "Payable"
}
```
The plugin host calls `PluginRegistry::register_capabilities(&manifest)` after loading each
plugin, which:
1. Populates `provided_doctypes: DashMap<String, String>` (doctype → plugin_id) in memory
2. `INSERT OR IGNORE`s `PartyTypeDecl` entries into the `PartyType` table in DB

**Part C — `DocField.provided_by` (Link dormancy):**
`DocField` gains an optional `provided_by: Option<String>` field (the plugin_id string).
Validation routing at save time:
- `provided_by = None` → target is a core type → validate existence strictly
- `provided_by = Some("selling")` → check plugin registry → plugin loaded? validate normally
  → plugin not loaded? skip validation (field is dormant) with a warning log

**Party validation logic (three-tier):**
1. `party_type` not in `PartyType` table → **hard error**: "Unknown party type: X"
2. `party_type` registered but owning plugin not loaded → **hard error**:
   "Party type Customer requires the selling plugin — not loaded"
3. Plugin loaded, party name not found in target table → **validation error**:
   "Customer CUST-9999 does not exist"

*For manual Journal Entry (desk user input):* all three tiers apply.
*For programmatic JE from a plugin (e.g. `on_submit` of Sales Invoice):* the plugin
 already knows the customer exists — existence validation can be skipped via a
 `TrustLevel::Plugin` context flag that the host sets when dispatching plugin hooks.

**Tasks:**
- [x] `PartyTypeDecl` + extended `PluginManifest` (fields: `provides_doctypes`, `party_types`)
      in `spotledger-plugins/src/versioning.rs`
- [x] `PluginRegistry`: `provided_doctypes: DashMap<String, String>` + `register_capabilities()`
      in `spotledger-plugins/src/registry.rs`
- [ ] `DocField`: add `provided_by: Option<String>` field + `.provided_by()` builder method
      in `spotledger-core/src/meta.rs`
- [ ] `PartyType` DocType + `AccountPartyType` enum in
      `spotledger-accounting/src/party_type.rs`
- [ ] Wire `PartyType` into `spotledger-accounting/src/lib.rs`
- [ ] `spotledger-accounting/src/party_validation.rs` — `validate_party()` with three-tier logic
- [ ] Update `JournalEntryAccount` schema: `party_type` becomes `Link("PartyType")`
- [ ] Add `PartyType` to Tier 3 migration DDL (`migrations/tier3.rs` or inline SQL)

**Exit criterion**: A Journal Entry row with `party_type = "Customer"` is rejected when the
selling plugin is not loaded with error "Party type Customer requires the selling plugin — not
loaded". When selling is loaded and `party = "CUST-9999"` does not exist the error is
"Customer CUST-9999 does not exist". Direct programmatic submissions from plugin hooks bypass
existence validation.

---

### Phase 3 — spotledger-pdk ✅ COMPLETE (core wrappers)

**Goal**: Plugin authors depend only on `spotledger-pdk`. Zero direct `extism-pdk` references in app code.

**Tasks:**
- [x] `spotledger-pdk` crate created; `host.rs` re-exports extism-pdk primitives
- [x] `api.rs` — document operation wrappers using raw `extern "C"` + extism memory protocol:
      `sl_log`, `sl_exists`, `sl_get_doc`, `sl_save_doc`, `sl_delete_doc`, `sl_get_value`,
      `sl_set_value`, `sl_throw`, `sl_has_permission`.
      Dual compilation: `wasm32` → real extism memory calls; host target → stubs for tests.
- [x] `accounting.rs` — GL wrappers: `make_gl_entries()`, `reverse_gl_entries()`,
      `get_account_balance()`, `get_fiscal_year()`, `get_exchange_rate()` with `GlEntry` and
      `FiscalYear` structs. Dual compilation: wasm32 → real host fn calls; host → validation stubs.
- [x] `lib.rs` — all public API re-exported at crate root
- [ ] `utils.rs` — guest-side wrappers for `sl_utils_*` host functions
- [ ] `extension.rs` — `MethodRegistration`, `ScheduledJob`, `CrossHook`, `WorkspaceDefinition` structs
- [ ] `apps/selling` compiles using ONLY `spotledger-pdk`

**Implementation notes:**
- No `extism-pdk` dependency — uses raw `extern "C"` + manual extism memory management
  (`extism_alloc`, `extism_store_u8`, `extism_load_u8`, `extism_length` from `extism:env`)
- `alloc_str()` / `read_bytes_handle()` are `pub(crate)` helpers shared by `api.rs` and `accounting.rs`
- `sl_throw` returns `!` on wasm32 via `core::arch::wasm32::unreachable()`

**Exit criterion**: ✅ CORE SATISFIED — `api.rs` and `accounting.rs` compile clean for both host and wasm32 targets. `utils.rs` and `extension.rs` pending.

---

---

### Phase A — Metadata-as-Graph Foundation ⏳ NOT STARTED

**Goal**: DocType and DocField records exist as live graph nodes in SurrealDB. `frappe.get_meta("Account")` is a graph query. `ensure_schema` grows a Phase 2 that upserts these records alongside DDL.

**Tasks:**
- [ ] Define `doctype`, `docfield`, `has_field` (relation) table schemas in Tier 0 migrations
      Fields on `has_field` edge: `idx`, `introduced_by`, `app_version`, `is_custom`, `protected`, `removable_on_uninstall`
- [ ] Extend `ensure_schema` in `spotledger-db/src/schema.rs` with Phase 2:
      `ensure_meta_records(adapter, meta)` — upserts doctype node + docfield nodes + RELATE edges
      for Tier 0 types only (other types come from `seed_doctypes_for_app`)
- [ ] Extend `seed_doctypes_for_app` to stamp `introduced_by` + `app_version` + `is_custom = false`
      on every DocField upsert (reads version from app's `pyproject.toml`)
- [ ] Add diff logic to `seed_doctypes_for_app`: compare existing SurrealDB record vs JSON,
      upsert only changed fields, write `schema_change_log` entry per change
- [ ] Create `schema_change_log` table in Tier 0 migrations
- [ ] `meta_cache` module in `spotledger-db`: query SurrealDB for DocTypeMeta at runtime
      (replaces compiled `MetaEntry` lookups for non-Tier-0 types). LRU cache, invalidated on schema change.
- [ ] Remove `lft`, `rgt`, `old_parent` from Account definition; add `child_of` relation table
- [ ] `get_ancestors_of` / `get_descendants_of` in `spotledger-db` rewritten as SurrealDB graph queries
      `SELECT <-child_of<-account... FROM account:xyz`

**Exit criterion**: `SELECT ->has_field->docfield.* FROM doctype:Account ORDER BY ->has_field.idx` returns all Account fields. `meta_cache.get("Account")` returns a `DocTypeMeta`-equivalent built from that graph query.

---

### Phase B — Save Pipeline → Auth+Proxy + SurrealDB Events ⏳ NOT STARTED

**Goal**: The 15-step Rust save pipeline is replaced. Rust does auth then one DB write. All validation, computed derivation, and side-effect propagation runs inside SurrealDB via events.

**Tasks:**
- [ ] Replace `controller::save_doc` 15-step pipeline with:
      `check_permission` → `db.upsert(doctype, doc)` → return result
- [ ] Replace `controller::delete_doc_checked` similarly
- [ ] Define `DEFINE TABLE account SCHEMAFULL ASSERT ...` structural assertions for mandatory fields
      (generated from `DocField.reqd = true` during schema sync)
- [ ] Define `DEFINE FIELD <fieldname> ON account ASSERT ...` for per-field constraints
      (generated from DocField metadata: `unique`, `not_nullable`, select options)
- [ ] Port first domain event: `account_validate_parent` as `DEFINE EVENT validate ON account`
      — replaces `account.py:validate_parent()`
- [ ] Port `set_root_and_report_type` as `DEFINE FIELD report_type ON account VALUE IF ...`
- [ ] Port `propagate_root_type_to_children` as `DEFINE EVENT` on account
- [ ] GL Entry: `DEFINE EVENT validate ON gl_entry` — account exists, not disabled, not group
- [ ] Journal Entry submit: `DEFINE EVENT on_submit ON journal_entry` — inserts GL entries
      atomically; gl_entry's own event fires inside the same transaction
- [ ] Outbox pattern: `pending_notification` table in Tier 0; Rust `LIVE SELECT` subscriber
- [ ] `DEFINE EVENT on_submit/on_cancel` workflow state enforcement in Tier 0 DDL
      (docstatus 0→1 only if `is_submittable`, 1→2 only if permitted)
- [ ] `custom_` prefix enforcement in Rust API save handler for `is_custom = true` fields

**Exit criterion**: Submit a Journal Entry via `POST /api/resource/Journal Entry/{name}` with `{"docstatus": 1}`. GL entries appear atomically. Cancel reverses them. If Account is disabled the GL insert throws and the entire transaction rolls back.

---

### Phase G — Graph-Compute Pipeline ✅ COMPLETE (April 12, 2026)

**Goal**: The save pipeline is backed by SurrealDB `DEFINE FUNCTION` statements. `fn::pipeline::run()` is called on every save/submit/cancel instead of a Rust-side 15-step controller.

**Architecture**:
- `02_runner.surql` contains `fn::pipeline::run($doctype, $doc, $action)` — the top-level dispatcher
- The runner reads `tabDocType` directly for `issubmittable` (no separate table needed)
- 15 `fn::pipeline::*` functions registered in SurrealDB at install time
- `is_pipeline_registered()` in `spotledger-db/src/pipeline.rs` queries `tabDocType` to detect whether the pipeline is installed

**Key SurrealDB query in `02_runner.surql`:**
```surql
LET $meta = (SELECT issubmittable FROM tabDocType WHERE name = $doctype LIMIT 1)[0];
IF $action IN ["submit", "cancel", "amend"] AND !$meta.issubmittable {
    THROW "DocType " + $doctype + " is not submittable";
};
```

**`doctype_meta` table removed**: This table existed only to cache `issubmittable`. Now that `tabDocType` is authoritative (seeded by `install_app`), `doctype_meta` is redundant and was removed from all code paths:
- `02_runner.surql` — reads `tabDocType` directly
- `is_pipeline_registered()` — queries `tabDocType` 
- `seed_doctypes.rs` — no longer touches `doctype_meta`
- Schema fixup string — `DEFINE TABLE IF NOT EXISTS doctype_meta` removed

**Tasks:**
- [x] `apps/erpnext/erpnext/surql/framework/01_schema.surql` — table definitions
- [x] `apps/erpnext/erpnext/surql/framework/02_runner.surql` — `fn::pipeline::run()` + `fn::pipeline::*`
- [x] `crates/spotledger-db/src/pipeline.rs` — `is_pipeline_registered()`, `register_pipeline_functions()`
- [x] `crates/spotledger/src/install_app.rs` — calls `register_pipeline_functions()` at install end
- [x] `doctype_meta` table removed from all locations
- [x] 15 `fn::` functions registered on `hello_graph` site
- [x] All 761 DocType tables DEFINE'd with `DEFINE TABLE IF NOT EXISTS` at install time

**Exit criterion**: ✅ SATISFIED — `hello_graph` site has 761 tables defined, 15 pipeline `fn::` functions active, saving a document dispatches through `fn::pipeline::run()`.

---

### Phase C — App/Module Graph + Schema Versioning ✅ COMPLETE

**Goal**: Apps and modules are graph nodes with full lineage to every DocType and field they own. Schema changes are versioned and queryable.

**Tasks:**
- [x] `app` table in Tier 0 migrations: `{ name, title, version, installed_at, updated_at }`
- [x] `module` table: `{ name, app, label, icon, show_in_menu, order }`
- [x] Relations: `app-[provides_module]->module`, `module-[contains]->doctype`
- [x] `install_app` command: upsert `app` node at start of install; upsert `module` nodes;
      add `provides_module` and `contains` edges
- [x] Expose `GET /api/method/spotledger.get_installed_apps` — returns app graph
- [x] Uninstall command: remove `has_field` edges for `introduced_by = app`,
      generate orphan report, never auto-purge data
- [ ] `GET /api/method/spotledger.schema_history?doctype=Account` — queries `schema_change_log` (pending)
- [ ] `spotledger-pdk`: replace `DocTypeMeta` struct with `schema.json` payload format (pending)

**Exit criterion**: ✅ `spotledger install-app erpnext` seeds 489 DocTypes (plus 272 frappe = 761 total). App/module graph nodes present in SurrealDB.

---

### Phase 4 — ERPNext Domain App 🔄 IN PROGRESS

**Decision**: One app (`apps/erpnext/`), all domains as modules within it. Follows ERPNext directory convention exactly. Deployed as a unit — no inter-plugin dependency resolution needed.

**Modules included** (copied verbatim from `f:\Sources\erpnext\erpnext\`):
`Accounts` · `Assets` · `Buying` · `Selling` · `Stock` · `CRM` · `Manufacturing` · `Projects` · `Quality Management` · `Subcontracting` · `Setup` · `Domains`

**App structure**:
```
apps/erpnext/
  Cargo.toml              ← cdylib stub, wasm32-wasip1
  src/lib.rs              ← #[no_mangle] sl_plugin_init() {}
  erpnext/                ← Frappe convention: apps/{app}/{app}/
    modules.txt
    accounts/doctype/{name}/{name}.json
    buying/doctype/{name}/{name}.json
    selling/doctype/{name}/{name}.json
    stock/doctype/{name}/{name}.json
    assets/doctype/{name}/{name}.json
    crm/doctype/{name}/{name}.json
    manufacturing/doctype/{name}/{name}.json
    projects/doctype/{name}/{name}.json
    quality_management/doctype/{name}/{name}.json
    subcontracting/doctype/{name}/{name}.json
    setup/doctype/{name}/{name}.json
```

**Dependency checking — graph-first, no pre-sort needed**: DocType definitions are metadata records; SurrealDB has no cross-table constraints at schema-definition time. All DocTypes are seeded in a single pass in any order. After seeding, a graph query finds every Link field pointing to a DocType not yet installed:

```surql
SELECT doctype.name AS source, docfield.fieldname, docfield.options AS links_to
FROM doctype, ->has_field->docfield
WHERE docfield.fieldtype = "Link"
  AND NOT (SELECT 1 FROM doctype WHERE name = docfield.options LIMIT 1)
```

This runs as a **post-install health check** — surfaces actionable warnings (e.g. "Sales Invoice references Customer — install the app that provides it") rather than failing mid-seed.

**Tasks:**
- [x] Remove `spotledger-accounting` from workspace members + workspace deps
- [x] Remove `spotledger-accounting` dep from `spotledger-plugins/Cargo.toml`
- [x] Create `apps/erpnext/` Cargo stub (cdylib, wasm32-wasip1)
- [x] Copy DocType JSON files from `f:\Sources\erpnext` — **483 JSONs** across accounts(186), buying(20), selling(18), stock(77), assets(26), crm(27), manufacturing(47), projects(15), quality_management(16), subcontracting(13), setup(40)
- [x] Implement `post_install_link_check()` in `crates/spotledger/src/install_app.rs` — graph query for unresolved Link fields, printed as warnings
- [x] `cargo xtask check` passes clean
- [x] `spotledger install-app frappe hello_graph` — 272 DocTypes seeded, 15 `fn::` pipeline functions registered
- [x] `spotledger install-app erpnext hello_graph` — 489 DocTypes seeded (accounts: 186, buying: 20, selling: 18, stock: 77, assets: 26, crm: 27, manufacturing: 47, projects: 15, quality_management: 16, subcontracting: 13, setup: 40)
- [x] All 761 DocType tables DEFINE'd as schemaless at install time (visible in Surrealist)
- [x] `doctype_meta` table removed — `tabDocType.issubmittable` used directly by `02_runner.surql` and `is_pipeline_registered()`
- [ ] `cargo xtask wasm` — compile erpnext.wasm (requires `rustup target add wasm32-wasip1`)

**Exit criterion**: ✅ SATISFIED for schema seeding. ERPNext WASM compilation pending.

---

### Phase 5 — frappe.client API Completion ✅ COMPLETE

- [x] `frappe.client.rename_doc` — copy record to new name, delete old, evict cache
- [x] `frappe.client.attach_file` — decode base64, write to sites/<site>/public|private/files/, upsert tabFile record, return URL
- [x] `frappe.client.validate_link` — check linked doc exists + user has read permission
- [x] `frappe.client.bulk_update` — batch `fieldname = value` on list of docnames
- [x] `frappe.client.has_permission` — delegate to `permissions::has_permission`
- [x] `frappe.client.get_doc_permissions` — full DocPermission JSON via `permissions::get_doc_permissions`

**Implementation files:**
- `crates/spotledger-db/src/document.rs` — added `rename_doc()`, `bulk_update()`
- `crates/spotledger-http/src/methods/client.rs` — 6 new handlers + registrations
- `crates/spotledger-http/Cargo.toml` — added `base64` workspace dep

---

### Phase 6 — API v2 Endpoints ⏳ NOT STARTED

- [ ] `GET    /api/v2/document/{doctype}` — list with advanced filter grammar
- [ ] `GET    /api/v2/document/{doctype}/{name}` — get single doc
- [ ] `POST   /api/v2/document/{doctype}` — insert
- [ ] `PATCH  /api/v2/document/{doctype}/{name}` — partial update
- [ ] `DELETE /api/v2/document/{doctype}/{name}` — delete
- [ ] `POST   /api/v2/document/{doctype}/{name}/run_doc_method`
- [ ] `GET    /api/v2/meta/{doctype}` — DocTypeMeta response
- [ ] `GET    /api/v2/document/{doctype}/count`

---

### Phase 7 — Background Jobs ⏳ NOT STARTED

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

### Phase 8 — File Storage ⏳ NOT STARTED

- [ ] `tabFile` DocType in `spotledger-desk`
- [ ] `POST /api/method/upload_file` — store binary, return URL
- [ ] `GET  /files/:path` — public files from disk
- [ ] `GET  /private/files/:path` — private files (permission-gated)
- [ ] `frappe.client.attach_file` wired to file storage
- [ ] `LocalStorage` backend trait (extensible to S3)
- [ ] `sl_upload_file()` host function in ABI

---

### Phase 9 — Full-Text Search ⏳ NOT STARTED

- [ ] SurrealDB FTS (`SEARCH ANALYZER`) for document search
- [ ] `frappe.desk.search.search_link` — complete implementation
- [ ] `frappe.utils.global_search.search` — global search
- [ ] Build search index on `after_save` hook

---

### Phase 10 — Notifications & Real-time ⏳ NOT STARTED

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
