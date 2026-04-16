# Spotledger

> **Frappe reimplemented in Rust — with a fundamental architecture shift.**

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Status: Active Development](https://img.shields.io/badge/Status-Active%20Development-orange.svg)]()
[![Not Production Ready](https://img.shields.io/badge/Production%20Ready-No-red.svg)]()

Spotledger is a from-scratch reimplementation of the [Frappe Framework](https://github.com/frappe/frappe) and a subset of [ERPNext](https://github.com/frappe/erpnext) in Rust, backed by SurrealDB.  It exposes a **wire-compatible API** — the same REST endpoints and JSON shapes that Frappe clients expect — so an existing Frappe/ERPNext frontend works against it without modification.

> ⚠️ **This project is in active development and is not production ready.** APIs, database schemas, and CLI commands may change without notice. Do not use in production.

---

## Why rewrite Frappe?

Frappe is a powerful Python framework that has proven the DocType model works extremely well for enterprise software.  However it comes with significant operational weight: Python, MariaDB, Redis (required), Node.js, a Gunicorn process pool, a worker pool (Celery/RQ), and a reverse proxy — typically 6–9 processes just to serve one site.

Spotledger asks: *what if the same DocType model were built on a database that can do the heavy lifting natively?*

---

## The Architecture Shift

The core design premise is that **enterprise ERP logic is, at its heart, a directed acyclic graph (DAG) of operations**, and modern graph databases can express and execute that DAG natively — without a layer of Python controllers in between.

### What this means in practice

| Concern | Frappe | Spotledger |
|---|---|---|
| Validation | Python controller methods (`validate`, `before_save`) | `DEFINE FIELD … ASSERT` + `DEFINE EVENT validate` in SurrealDB — fires unconditionally for every write from any caller |
| Computed fields | Python setters called in controller chain | `DEFINE FIELD report_type VALUE IF root_type IN […] THEN …` — computed inside SurrealDB |
| GL posting on submit | Python `on_submit` calling `make_gl_entries()` | `DEFINE EVENT on_submit` cascading `INSERT INTO gl_entry` — one atomic SurrealDB transaction |
| Cancel cascade | Python walking child doctypes | Recursive depth-first via `cascade_rule` graph edges |
| Document relationships | Nested-set `lft/rgt` integers on tree doctypes | Native graph edges (`RELATE account:1001 -[child_of]-> account:1000`) |
| Pipeline stages | 15-step Python controller chain | DAG of `pipeline_stage → pipeline_node` records stored in SurrealDB; traversed by `fn::pipeline::run()` |
| Schema introspection | Python `frappe.get_meta()` reading MariaDB | `SELECT ->has_field->docfield.* FROM doctype:Account` — live graph query |
| Schema ownership | `custom_field` Python model | `introduced_by` attribute on `has_field` graph edges — every field traces lineage to its app |

### Why SurrealDB?

- **Graph-native**: DocType relationships (Link fields, child tables, tree hierarchies) map directly to graph edges — no ORM required.
- **Event-driven**: `DEFINE EVENT` triggers replace Python controller hooks.  Validation is table-level and unconditional — it cannot be bypassed.
- **Multi-model**: Documents, graph nodes, key/value, and computed fields coexist in one database.
- **Transactions across events**: A Journal Entry submit → GL entries insert atomically.  If any GL validation throws, the entire transaction rolls back.
- **AI/Agent readiness**: When AI agents become the primary drivers of enterprise workflows, they benefit from a database that can answer graph questions natively ("what accounts are downstream of this parent?", "what pipeline stages does this doctype have?") without round-tripping through an application server.

### The single binary

The entire backend is a single statically-linked Rust binary, currently **~27 MB**.

```
spotledger.exe          ← the entire server + CLI + schema engine
SurrealDB               ← the only external dependency
```

No Python, no Node.js, no Redis, no Celery, no Gunicorn.  For the UI, a separate React application (Spotledger-UI) communicates via the same Frappe-compatible REST API.

---

## Stack at a Glance

| Component | Technology | Notes |
|---|---|---|
| Backend | Rust (stable) | Single binary, async via Tokio + Axum |
| Database | SurrealDB v3 | Graph + document + KV; hosts pipeline logic |
| UI | React + Vite (separate repo) | Frappe-compatible API; shadcn/ui components |
| Plugin system | WASM (wasm32-wasip1) | Domain apps compile to `.wasm`, loaded at runtime |
| Cache (planned) | NATS | Will replace in-process Moka cache for multi-node |
| Build system | `cargo xtask` | Builds host + WASM apps in one command |


---

## Feature Parity with Frappe

### Core Framework

| Feature | Frappe | Spotledger | Notes |
|---|---|---|---|
| DocType metadata model | ✅ | ✅ | `tabDocType` + `tabDocField` in SurrealDB; 761 Frappe/ERPNext DocTypes seeded |
| Document CRUD (`/api/resource/`) | ✅ | ✅ | GET, POST (create/update), DELETE; permission-gated |
| Document list + filters (`get_list`) | ✅ | ✅ | Fields, filters, limit, offset |
| Document naming series | ✅ | ✅ | `SO-.YYYY.-.####` patterns; atomic counter in `tabSeries` |
| `field:X` autoname | ✅ | ✅ | |
| Hash/UUID autoname | ✅ | ✅ | |
| Multi-site support | ✅ | ✅ | Isolated SurrealDB namespaces per site, routed by `Host:` header |
| Session-based auth (cookie `sid`) | ✅ | ✅ | Password hashed with Argon2 |
| Boot info (`frappe.boot.get_bootinfo`) | ✅ | ✅ | |
| Workspace / Desk sidebar | ✅ | ✅ | Seeded from Frappe JSON fixtures |
| Form load (`getdoc`, `getdoctype`) | ✅ | ✅ | Returns doc + meta in one call |
| Form save / cancel / discard | ✅ | ✅ | |
| List view / reportview | ✅ | ✅ | `frappe.desk.reportview.*` endpoints |
| Search (link, widget, awesomebar) | ✅ | ✅ | |
| Comments | ✅ | ✅ | Add, update, set publicity |
| Assignments (`assign_to.*`) | ✅ | ✅ | |
| Linked documents | ✅ | ✅ | |
| Document follow / unfollow | ✅ | ✅ | |
| Notifications | ✅ | ✅ | `get_notification_info`, log, mark-as-read |
| User settings (per-doctype) | ✅ | ✅ | |
| Workflow transitions (API shape) | ✅ | 🟡 Stub | Correct JSON shape returned; no execution yet |
| Rename doc | ✅ | ✅ | |
| Tags | ✅ | 🟡 Stub | API registered; storage not wired |
| Like / share | ✅ | 🟡 Stub | API registered; storage not wired |
| Permission system (role-based) | ✅ | ✅ | Read, Write, Create, Delete, Submit, Cancel per role |
| Custom fields | ✅ | ✅ | `custom_` prefix enforced at API boundary |
| DocType designer (meta edit) | ✅ | 🟡 In progress | `designerApi` exists; UI integration ongoing |
| Child tables (in_list_view) | ✅ | ✅ | Seeded, queryable, rendered in UI |
| `is_single` DocTypes | ✅ | ✅ | Stored in `tabSingles` |
| `is_submittable` + docstatus | ✅ | ✅ | 0/1/2 lifecycle; pipeline checks `tabDocType.issubmittable` |
| Module Def + app graph nodes | ✅ | ✅ | `app → module → doctype` graph in SurrealDB |

### Document Validation & Pipeline

| Feature | Frappe | Spotledger | Notes |
|---|---|---|---|
| Mandatory field validation | ✅ | ✅ | Enforced in Rust (Tier 0) and via `DEFINE FIELD ASSERT` (domain) |
| Select option validation | ✅ | ✅ | |
| Field length validation | ✅ | ✅ | Data: 255, SmallText: 140, LongText: uncapped |
| `update_after_submit` field guard | ✅ | ✅ | Blocks edits on submitted docs except marked fields |
| `read_only_on_submit` | ✅ | ✅ | |
| Graph-compute pipeline (`fn::pipeline::run`) | ❌ | ✅ | Spotledger-native; DAG stages stored in SurrealDB |
| Pipeline wiring (`wiring.surql`) | ❌ | ✅ | |
| Domain `fn::` functions (account, invoice, etc.) | ❌ | 🟡 Partial | Framework is live; domain functions authored for GL entry, Journal Entry, Purchase Invoice |
| `depends_on` field visibility | ✅ | ✅ | Evaluated in React UI at render time |
| `hidden` field filtering | ✅ | ✅ | Filtered in UI layout engine |

### Schema & Seeding

| Feature | Frappe | Spotledger | Notes |
|---|---|---|---|
| 272 Frappe framework DocTypes | ✅ | ✅ | Seeded from Frappe JSON via `install-app frappe` |
| 489 ERPNext DocTypes | ✅ | ✅ | Seeded from ERPNext JSON via `install-app erpnext` |
| `DEFINE TABLE` per DocType | N/A | ✅ | All 761 tables defined at install |
| `field_order` / `idx` sorting | ✅ | ✅ | Fields sorted by `idx` from `tabDocField`; frontend and backend aligned |
| Post-install link check | ✅ | ✅ | Unresolved Link fields reported as warnings |
| Patch log (`tabPatchLog`) | ✅ | ✅ | All patches marked done on install |
| Fixture records (Workspace, Page, Report, etc.) | ✅ | ✅ | Imported during `install-app` |
| Schema change log | ❌ | 🟡 Planned | `schema_change_log` table designed; diff-on-upsert not yet wired |
| `introduced_by` / `app_version` on fields | ❌ | 🟡 Planned | Design complete; stamping not yet applied |

### CLI

| Command | Status | Notes |
|---|---|---|
| `new-site` | ✅ | Creates site dir, config, bootstrap schema, seeds default records |
| `install-app` | ✅ | Seeds DocTypes, modules, fixtures, pipeline, patch log |
| `use` | ✅ | Sets default site (`sites/currentsite`) |
| `start` | ✅ | Reads `currentsite`, starts HTTP server |
| `serve` | ✅ | Explicit-arg equivalent of `start` |
| `migrate` | ✅ | DDL sync + pending data migrations; `--dry-run` supported |
| `wire-app` | ✅ | Re-applies pipeline `wiring.surql` without full reinstall |
| `cleanup` | ✅ | Removes orphaned `tabDocField`/`tabDocPerm` rows; `--apply` required for writes |
| `emit` | ✅ | Writes compiled schemas to `generated/schema.surql` (no DB needed) |
| `generate` | ✅ | Converts a Frappe JSON DocType to a Rust `DocTypeMeta` source file |

### Explicitly Out of Scope (by design)

These Frappe features are not planned because they belong to the Python/MariaDB layer that Spotledger replaces:

| Feature | Reason |
|---|---|
| Redis (session/cache) | Sessions in SurrealDB; NATS planned for pub/sub |
| Celery / RQ background jobs | Will use SurrealDB LIVE SELECT + Rust async tasks |
| MariaDB query builder (Frappe QBL) | Not applicable — SurrealQL is the query language |
| Server Scripts (`safe_exec`) | Replaced by WASM plugin functions |
| Query recorder | MariaDB-specific tooling |
| Nested-set `lft/rgt` on tree types | Replaced by native SurrealDB graph edges |

### Not Yet Implemented

| Feature | Notes |
|---|---|
| Email / SMTP | Designed as outbox pattern (SurrealDB event → `pending_notification` table → Rust LIVE SELECT) |
| Background job scheduler | Architecture designed; not built |
| PDF rendering | Planned via headless browser or WeasyPrint sidecar |
| Kanban board | API stubs registered; storage not wired |
| Dashboard charts / number cards | API stubs registered; SurrealQL aggregation queries not wired |
| Query report execution | Stub registered; SurrealQL runner not wired |
| OAuth2 / 2FA | Not started |
| Full-text / global search | Stub registered; SurrealDB `SEARCH` indexes not wired |
| i18n / translations | Not started |
| Website / Web Form | Not planned for initial release |
| Workflow execution | API shape correct; state machine not implemented |
| `schema_change_log` + app versioning | Designed; not wired |
| NATS integration | Planned for cache bus and pub/sub |
| WASM plugin hot-reload | Plugin loading works; hot-reload on file change not implemented |
| Rhai – Embedded Scripting for Rust - This one is very cirtical, because I think we should add a
a scripting language. We may add Deno/TypeScript too, (we havent decided this on it. I am in favour of Rhai
because its rust native plus i can percisely control what this scring engine can access.)

---

## Repository Structure

```
spotledger/
├── crates/
│   ├── spotledger/          ← CLI binary (new-site, install-app, start, …)
│   ├── spotledger-core/     ← DocTypeMeta, Document, field types, validation
│   ├── spotledger-db/       ← SurrealDB adapter, schema, naming, permissions,
│   │                           save_proxy, pipeline, meta_cache, graph_ops
│   ├── spotledger-http/     ← Axum HTTP server, route handlers, method registry
│   ├── spotledger-desk/     ← Desk-specific helper types
│   ├── spotledger-geo/      ← Currency, Country (Tier 0 examples)
│   ├── spotledger-contacts/ ← Contact-related Tier 0 types
│   ├── spotledger-plugins/  ← WASM plugin host (Extism)
│   ├── spotledger-pdk/      ← Plugin Development Kit (wasm32 target)
│   ├── spotledger-printing/ ← Print format helpers
│   ├── spotledger-automation/← Automation rule stubs
│   └── xtask/               ← Build orchestrator (cargo xtask build/wasm/host/check)
├── apps/
│   ├── frappe/              ← 272 Frappe framework DocType JSON files
│   └── erpnext/             ← 489 ERPNext DocType JSON files
│       └── surql/           ← Graph-compute pipeline SurrealQL for ERPNext
│           ├── framework/   ← DDL schema, runner, universal nodes
│           ├── shared/      ← Universal fn:: (mandatory, party check, …)
│           └── doctypes/    ← Per-doctype functions + wiring
├── sites/                   ← Created at runtime; one subdirectory per site
└── generated/
    └── schema.surql         ← Output of `spotledger emit`
```

---

## Quick Start

### Prerequisites

- Rust stable toolchain (`rustup show`)
- SurrealDB v3 (`surreal start --bind 127.0.0.1:8001 --user root --pass root memory`)

### Setup

```powershell
# Build
cargo xtask build

# Create a site
cargo run -- new-site mysite --db-url ws://127.0.0.1:8001 --admin-password changeme123

# Install Frappe + ERPNext DocTypes
cargo run -- install-app frappe  mysite --bench .
cargo run -- install-app erpnext mysite --bench .

# Set default site and start
cargo run -- use mysite
cargo run -- start
# → http://127.0.0.1:8000
```

See [GETTING_STARTED.md](GETTING_STARTED.md) for the complete guide.

---

## Test Coverage

313 passing unit tests; 15 integration tests (require live SurrealDB, run with `--features integration`).

| Category | Tests | Status |
|---|---|---|
| Document model | 17 | ✅ All pass |
| Meta / schema | 20 | ✅ All pass |
| Validation | 29 | ✅ All pass |
| Utils (strings, dates, passwords) | 29 | ✅ All pass |
| Query builders | 14 | ✅ All pass |
| Schema DDL | 19 | ✅ All pass |
| Hooks | 12 | ✅ All pass |
| Permissions (unit) | 4 | ✅ All pass |
| DB CRUD / Auth / Naming / Graph / Migrations | 17 | Integration (require DB) |

---

## Relationship to Frappe / ERPNext

Spotledger uses the Frappe and ERPNext **DocType JSON files** (the schema definitions) as its source of truth for 761 domain DocTypes.  These JSON files are checked into `apps/frappe/` and `apps/erpnext/` as fixtures and are seeded into SurrealDB at install time — they never run as Python code.

The Frappe Python framework, ERPNext Python codebase, and MariaDB are **not** required or used at runtime.  The name "Frappe-compatible" refers only to the REST API wire format and the DocType JSON schema format.

---

## Contributing

The project is pre-release.  If you want to contribute, open an issue first to discuss the area.  The highest-value areas right now are:

1. Domain `fn::` SurrealQL functions for accounting DocTypes (GL Entry, Journal Entry, Payment Entry)
2. NATS integration for pub/sub and cache invalidation
3. Background job scheduler (LIVE SELECT → Tokio task)
4. Email outbox worker

---

## License

MIT — see [LICENSE](LICENSE).
