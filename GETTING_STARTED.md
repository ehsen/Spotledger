# Getting Started with Spotledger

Spotledger is a Frappe-compatible ERP framework written in Rust, backed by
SurrealDB.  This guide walks you through:

1. [Running your first site](#1-running-your-first-site)
2. [Tier 0 DocTypes — compiled kernel types](#2-tier-0-doctypes--compiled-kernel-types)
3. [Generating Rust from a Frappe JSON file](#3-generating-rust-from-a-frappe-json-file)
4. [Architecture reference](#4-architecture-reference)

---

## 1. Running Your First Site

### Prerequisites

| Requirement | Notes |
|---|---|
| Rust toolchain (stable) | `rustup show` |
| SurrealDB **v3** running | `surreal start --bind 127.0.0.1:8001 --user root --pass root memory` |
| `cargo` in PATH | Comes with Rust |

> **Ports**: SurrealDB listens on `:8001` (default); the Spotledger HTTP server
> on `:8000` (dev mode).  These are independent processes.

### 1.1 Build the binary

```powershell
cd F:\Sources\spotledger

# Fast build with xtask (builds host workspace + all WASM plugins):
cargo xtask build

# Or release:
cargo xtask build --release

# For quick iteration you can also use cargo run directly:
cargo run -- <args>
```

`cargo xtask` commands:

| Command | What it does |
|---|---|
| `cargo xtask build` | Host workspace + all WASM apps (debug) |
| `cargo xtask build --release` | Both in release mode |
| `cargo xtask wasm` | WASM apps only |
| `cargo xtask host` | Host workspace only |
| `cargo xtask check` | `cargo check --workspace` (fastest feedback) |

### 1.2 Create a new site

```powershell
cargo run -- new-site my-company `
    --db-url         ws://127.0.0.1:8001 `
    --admin-password changeme123
# --bench defaults to "." (current directory)
# --db-user / --db-pass default to "root"
# --db-ns  defaults to the hostname ("my-company")
```

**What happens internally:**

| Step | What runs |
|---|---|
| 1 | `sites/my-company/` directory created on disk |
| 2 | `sites/my-company/site_config.toml` written |
| 3 | Connect to SurrealDB in namespace `my-company` |
| 4 | Framework tables created: `__Auth`, `tabSessions`, `tabSeries`, `tabSingles` |
| 5 | All Tier 0 DocType schemas synced (`DEFINE TABLE … SCHEMAFULL`) |
| 6 | Seed records inserted: default Roles, UserTypes, Administrator user |
| 7 | Administrator password written to `__Auth` |

After this completes, the kernel tables exist in SurrealDB namespace `my-company`.
Domain DocTypes (Account, Customer, Item, etc.) are seeded in the next step.

### 1.3 Install apps

Spotledger ships two apps: `frappe` (framework DocTypes, 272) and `erpnext`
(domain DocTypes, 489).  Install them in order:

```powershell
cargo run -- install-app frappe  my-company --bench .
cargo run -- install-app erpnext my-company --bench .
```

**What `install-app` does:**

| Step | What runs |
|---|---|
| 1 | Upserts `app` and `module` graph nodes in SurrealDB |
| 2 | Seeds rows into `tabDocType` + `tabDocField` from every DocType JSON |
| 3 | Emits `DEFINE TABLE IF NOT EXISTS` for each DocType (visible in Surrealist) |
| 4 | Registers `fn::pipeline::*` SurrealDB functions for the graph-compute pipeline |
| 5 | Applies `wiring.surql` files for each DocType (pipeline stage wiring) |
| 6 | Imports fixture records (Workspace, Page, Report, Print Format, …) |
| 7 | Marks all patches as completed in `tabPatchLog` |
| 8 | Runs a post-install Link check — unresolved references are printed as warnings |

After both installs `tabDocType` will have 761 rows and all DocType tables will
be visible in the schema.

### 1.4 Set the default site and start

```powershell
cargo run -- use my-company   # writes "my-company" to sites/currentsite
cargo run -- start            # reads sites/currentsite; starts HTTP server
# listening on 127.0.0.1:8000
```

`spotledger use` is a one-time step per bench.  Once set, `spotledger start`
boots the default site without additional flags.

### 1.5 Verify the site is up

```powershell
curl http://127.0.0.1:8000/api/ping -H "Host: my-company"
# {"message":"pong"}
```

### 1.6 Log in

```powershell
curl -X POST http://127.0.0.1:8000/api/method/login `
     -H "Host: my-company" `
     -d "usr=Administrator&pwd=changeme123"
# {"message":"Logged In","full_name":"Administrator","home_page":"/desk"}
# Response includes Set-Cookie: sid=<token>
```

### 1.7 Verify the logged-in user

```powershell
curl http://127.0.0.1:8000/api/method/frappe.auth.get_logged_user `
     -H "Host: my-company" `
     -H "Cookie: sid=<token from previous step>"
# {"message":"Administrator"}
```

### Typical daily workflow

```powershell
# First-time setup (once per bench):
cargo run -- new-site my-company --db-url ws://127.0.0.1:8001 --admin-password changeme123
cargo run -- install-app frappe  my-company --bench .
cargo run -- install-app erpnext my-company --bench .
cargo run -- use my-company

# Every subsequent session (SurrealDB must already be running):
cargo run -- start
```

### Other useful commands

```powershell
# Re-apply pipeline wiring after editing a wiring.surql file (no full reinstall):
cargo run -- wire-app erpnext my-company --bench .

# Apply pending migrations:
cargo run -- migrate my-company
cargo run -- migrate my-company --dry-run   # preview only

# Remove orphaned tabDocField/tabDocPerm rows for deleted DocTypes:
cargo run -- cleanup my-company --apply     # omit --apply for a dry-run

# Write all compiled schemas to generated/schema.surql (no DB needed):
cargo run -- emit

# Explicit serve with custom bind (equivalent to start but all flags required):
cargo run -- serve --bind 127.0.0.1:8080 --bench .
```

---

## 2. Tier 0 DocTypes — Compiled Kernel Types

> **When this applies**: Defining a `DocTypeMeta` struct in Rust is for the
> small set of **Tier 0 kernel types** the engine cannot boot without at compile
> time — `DocType`, `DocField`, `DocPerm`, `User`, `Role`, `UserPermission`,
> `ModuleDef`, and a handful more.
>
> For all domain features (Account, Customer, SalesOrder, Item, …) define a
> Frappe-compatible JSON file and deploy it via `install-app` (see §1.3).
> **No binary recompile is needed to add a domain DocType.**

All Tier 0 DocTypes are defined as `DocTypeMeta` structs registered with the
`inventory` crate.  There is no JSON file — the compiled binary is the sole
source of truth for their structure.

### 2.1 Decide where the DocType lives

Pick an existing framework crate (`spotledger-core`, `spotledger-geo`, etc.)
or create a new one.  All crates in the workspace are linked into the binary,
so their `inventory::submit!` calls run automatically.

### 2.2 Define the meta function

```rust
// Example: crates/spotledger-geo/src/currency.rs
use spotledger_core::meta::{DocField, DocTypeMeta, FieldType, Permission};
use spotledger_core::registry::MetaEntry;

pub fn currency_meta() -> DocTypeMeta {
    DocTypeMeta {
        name: "Currency".into(),
        module: "Geo".into(),
        is_single:      false,
        is_tree:        false,
        is_child:       false,
        is_submittable: false,
        track_changes:  false,
        fields: vec![
            DocField::new("currency_name", "Currency Name", FieldType::Data)
                .required()
                .unique()
                .in_list()
                .in_standard_filter(),
            DocField::new("symbol", "Symbol", FieldType::Data)
                .in_list(),
            // System fields (name, owner, creation, modified, docstatus, idx)
            // are added automatically — do NOT declare them here.
        ],
        permissions: vec![
            Permission::full("System Manager"),
            Permission::read_only("All"),
        ],
        title_field:   Some("currency_name".into()),
        search_fields: vec!["currency_name".into()],
        sort_field:    Some("currency_name".into()),
        sort_order:    Some("asc".into()),
        autoname:      Some("field:currency_name".into()),
        naming_series: None,
    }
}

inventory::submit!(MetaEntry { name: "Currency", meta: currency_meta });
```

### 2.3 Field type reference

| `FieldType` variant | SurrealDB storage | Notes |
|---|---|---|
| `Data` | `option<string>` | Short text |
| `Text` / `LongText` / `SmallText` | `option<string>` | Multi-line |
| `Int` | `int` | Integer |
| `Float` / `Currency` / `Percent` | `float` | |
| `Check` | `int` | 0 or 1 |
| `Date` | `option<string>` | `"YYYY-MM-DD"` |
| `Datetime` | `option<datetime>` | SurrealDB native |
| `Select` | `option<string>` | Use `.select_options("A\nB\nC")` |
| `Link` | `option<string>` | Use `.options("TargetDocType")` |
| `Table` | `array<object>` | Child table; use `.options("ChildDocType")` |
| `Password` | `option<string>` | Hashed & stored in `__Auth` |
| `Json` | `any` | Arbitrary JSON blob |
| `SectionBreak` / `ColumnBreak` / `TabBreak` | *(layout only)* | No DB column |

### 2.4 Autoname patterns

| Pattern | Effect |
|---|---|
| `"field:currency_name"` | Name = value of `currency_name` field |
| `"ITEM-.YYYY.-.####"` | Series: `ITEM-2024-00001`, `ITEM-2024-00002`, … |
| `"hash"` | Random UUID-style name |
| `"prompt"` | User must enter the name manually |
| `None` | UUID fallback |

### 2.5 Wire into the workspace

1. Add `pub mod currency;` to the crate's `lib.rs`.
2. If it's a new crate, add it to `[workspace] members` in the root `Cargo.toml`.
3. Add it as a dependency of `crates/spotledger/Cargo.toml` so its `inventory::submit!` entries are linked.
4. Rebuild: `cargo xtask host` (no WASM rebuild needed).
5. Apply to an existing site: `cargo run -- migrate my-company`  
   Or start fresh: `cargo run -- new-site my-company …`

---

## 3. Generating Rust from a Frappe JSON File

The `generate` command converts a Frappe-compatible DocType JSON into a
ready-to-use Rust source file.  This is useful for bootstrapping Tier 0
equivalents of Frappe's framework DocTypes.

Every DocType in a Frappe app is stored as a JSON file, e.g.:

```
frappe/frappe/geo/doctype/currency/currency.json
```

```powershell
# Print to stdout to review
cargo run -- generate --from apps/frappe/frappe/geo/doctype/currency/currency.json

# Write to a file
cargo run -- generate `
    --from apps/frappe/frappe/geo/doctype/currency/currency.json `
    --out  crates/spotledger-geo/src/currency.rs
```

**What gets mapped:**

| Frappe JSON key | Rust output |
|---|---|
| `autoname` | `DocTypeMeta::autoname` |
| `is_single` / `is_child_table` / `is_submittable` | structural flags |
| `fields[].fieldtype` | `FieldType::*` variant (30+ types mapped) |
| `fields[].reqd` | `.required()` |
| `fields[].unique` | `.unique()` |
| `fields[].in_list_view` | `.in_list()` |
| `fields[].in_standard_filter` | `.in_standard_filter()` |
| `fields[].default` | `.default_value("…")` |
| `fields[].options` | `.options("…")` |
| `permissions[].role` + flags | `Permission::full` / `read_only` |
| `field_order` array | field declaration order in `vec![]` |

The `spotledger-geo` crate (Currency + Country) shows the complete example.

---

## 4. Architecture Reference

### Layer split

```
┌────────────────────────────────────────────────────────────┐
│ Rust Host Binary — The Engine                              │
│                                                            │
│  HTTP server, routing, session handling                    │
│  Auth + permission enforcement  ← Rust owns this forever  │
│  Document naming (resolve_name)                            │
│  Save pipeline orchestrator → calls fn::pipeline::run()   │
│  WASM plugin sandbox host                                  │
│  Bootstrap schema (Tier 0 kernel types only)               │
└────────────────────────────────────────────────────────────┘
                          │
                          ▼
┌────────────────────────────────────────────────────────────┐
│ SurrealDB v3 — The Knowledge Base                          │
│                                                            │
│  All DocType/DocField records as graph nodes               │
│  fn::pipeline::run() — graph-compute pipeline              │
│  DEFINE EVENT hooks for cascades and side effects          │
│  All document instance data                                │
│  tabDocType / tabDocField — source of truth for 761        │
│    domain DocTypes (272 frappe + 489 erpnext)              │
└────────────────────────────────────────────────────────────┘
```

### Metadata tiers

```
┌──────────────────────────────────────────────────────────────┐
│ Tier 0 — Compiled into the binary (Rust code only)           │
│                                                              │
│  ~8 kernel types: DocType, DocField, DocPerm, User, Role,    │
│  UserPermission, ModuleDef, Site                             │
│  Registered via inventory::submit!(MetaEntry { … })          │
│  These are the minimum set the engine needs to boot.         │
└──────────────────────────────────────────────────────────────┘
                          │ seeded via install-app
                          ▼
┌──────────────────────────────────────────────────────────────┐
│ Domain DocTypes — JSON-seeded into SurrealDB at install time │
│                                                              │
│  tabDocType   — one row per DocType                          │
│  tabDocField  — one row per field (parent = DocType name)    │
│                                                              │
│  All 761 Frappe/ERPNext DocTypes live here.                  │
│  Adding/editing a field: update the JSON, re-run install-app │
│  No binary recompile needed.                                 │
└──────────────────────────────────────────────────────────────┘
                          │ created by admins at runtime
                          ▼
┌──────────────────────────────────────────────────────────────┐
│ Admin-only — DB only (never in code)                         │
│                                                              │
│  tabCustomField      → extra fields added at UI              │
│  tabPropertySetter   → field property overrides              │
│  tabUserPermission   → document-level ACL                    │
└──────────────────────────────────────────────────────────────┘
```

### Save pipeline

```
POST /api/resource/Account/ACC-001
  │
  ├─ Rust: session check + permission check
  ├─ Rust: resolve_name() → naming series / field name / UUID
  └─ SurrealDB: UPSERT → fn::pipeline::run($doc_id, $doctype, $action)
                           → validates mandatory fields
                           → checks issubmittable / docstatus rules
                           → fires DEFINE EVENT hooks (GL posting, etc.)
```

`fn::pipeline::run()` reads `tabDocType.issubmittable` directly.
If no pipeline stages are registered for a doctype, the call returns
`{status:"skipped"}` and the document is saved as-is (safe default).

### Graph-compute pipeline

Pipeline functions are SurrealQL `fn::` definitions stored in each app's
`surql/` directory and registered by `install-app`:

```
apps/erpnext/surql/
  framework/schema.surql      ← pipeline table definitions
  framework/runner.surql      ← fn::pipeline::run() dispatcher
  shared/*.surql              ← universal fn:: (mandatory, party check, …)
  doctypes/{name}/
    functions/*.surql         ← domain fn:: for this DocType
    meta.surql                ← registers this DocType in the pipeline
    wiring.surql              ← connects stages → nodes
```

After editing a `wiring.surql` file, re-apply it without a full reinstall:

```powershell
cargo run -- wire-app erpnext my-company --bench .
```

### Framework-internal tables

These tables are bootstrapped directly and never appear in the Desk UI:

| Table | Purpose |
|---|---|
| `__Auth` | Password hashes (`doctype`, `name`, `fieldname`, `password`) |
| `tabSessions` | Active login sessions and SID tokens |
| `tabSeries` | Naming-series counters (`SINV-2024-` → 42) |
| `tabSingles` | Key/value store for `is_single` DocTypes |

### SurrealDB record ID conventions

Spotledger uses the `tab<Name>` table-prefix convention.  Record IDs follow
SurrealDB v3 syntax:

```
tabUser:Administrator          ← simple alphanumeric — no escaping needed
tabRole:⟨System Manager⟩      ← ⟨…⟩ required when the name contains spaces
tabSessions:⟨abc123def456⟩    ← ⟨…⟩ when name contains non-word characters
```

### Multi-site request routing

The HTTP server reads `Host:` and routes to the matching site's SurrealDB
namespace.  When more than one site is loaded you **must** pass the header:

```
GET /api/resource/Item/WIDGET-001
Host: my-company
```

> **Dev convenience**: if only one site exists in `sites/`, the `Host:` header
> is optional.

### Document naming flow

```
resolve_name(adapter, doctype, fields)
│
├─ 1. fields["name"] already set and non-empty?  → use it directly
│
├─ 2. fields["naming_series"] set?
│      └─ YES → increment tabSeries counter and format
│
├─ 3. tabDocumentNamingRule WHERE document_type = doctype?
│      └─ FOUND → evaluate autoname:
│           "field:X"   → use fields[X]
│           "PREFIX.-…" → naming series
│           "hash" / unrecognised → UUID fallback
│
└─ 4. UUID fallback  (new-{timestamp-hex})
```

### Migrations

Two tiers run on `migrate` (and at server startup):

| Tier | Mechanism | Tracking |
|---|---|---|
| DDL (schema) | `ensure_all_schemas` — `DEFINE … IF NOT EXISTS` | Idempotent; no tracking needed |
| Data | `MigrationEntry` registered via `inventory::submit!` | `tabMigration` — one row per applied migration per site |

```powershell
cargo run -- migrate my-company
cargo run -- migrate my-company --dry-run
```

### Adding a new domain app (no Rust required)

1. Create `apps/my-app/my-app/` following the Frappe directory convention.
2. Add `{module}/{doctype_name}/doctype/{doctype_name}/{doctype_name}.json`
   for each DocType.
3. Optionally add `surql/` pipeline files.
4. Run `cargo run -- install-app my-app my-company --bench .`.

`tabDocType`, `tabDocField`, and all `DEFINE TABLE` statements are written
automatically.  No binary recompile needed.
