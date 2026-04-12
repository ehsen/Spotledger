# Getting Started with Spotledger

Spotledger is a Frappe-compatible ERP framework written in Rust, backed by
SurrealDB.  This guide walks you through four topics:

1. [Running your first site](#1-running-your-first-site)
2. [Creating a new DocType in Rust code](#2-creating-a-new-doctype)
3. [Importing a DocType from a Frappe JSON file](#3-importing-a-frappe-json-doctype)
4. [Reference: how the system works end-to-end](#4-architecture-reference)

---

## 1. Running Your First Site

### Prerequisites

| Requirement | Notes |
|---|---|
| Rust toolchain (stable) | `rustup show` |
| SurrealDB v2+ running | `surreal start --bind 127.0.0.1:8001 --user root --pass root memory` |
| `cargo` in PATH | Comes with Rust |

> **Ports**: SurrealDB listens on `:8001`; the Spotledger HTTP server on `:8000`.

### 1.1 Build the binary

```powershell
cd F:\Sources\spotledger
cargo build --release
# binary lands at target/release/spotledger.exe
```

For development you can use `cargo run -- <args>` instead.

### 1.2 Create a new site

```powershell
cargo run -- new-site my-company \
    --db-url         ws://127.0.0.1:8001 \
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
| 3 | Connect to SurrealDB; use namespace `my-company` |
| 4 | Framework tables created: `__Auth`, `tabSessions`, `tabSeries`, `tabSingles` |
| 5 | All compiled Tier 0 DocType schemas synced (`DEFINE TABLE … SCHEMAFULL`) |
| 6 | Seed records inserted: roles, user types, Administrator user |
| 6b| `tabDocumentNamingRule` seeded from every compiled `DocTypeMeta.autoname` |
| 7 | Administrator password written to `__Auth` |

After this completes you have the kernel tables ready in SurrealDB namespace
`my-company`.  Domain DocTypes (Account, Customer, Item, etc.) are seeded in
the next step via `install-app`.

### 1.3 Install apps

Spotledger ships two apps: `frappe` (framework DocTypes, ~272) and `erpnext`
(domain DocTypes, ~489).  Install them in order:

```powershell
cargo run -- install-app frappe my-company --bench .
cargo run -- install-app erpnext my-company --bench .
```

**What `install-app` does for each DocType JSON it finds:**

| Step | What runs |
|---|---|
| 1 | Upserts an `app` graph node and each `module` node |
| 2 | Inserts rows into `tabDocType` + `tabDocField` (the source of truth) |
| 3 | Emits `DEFINE TABLE IF NOT EXISTS <tab> SCHEMALESS` so the table is visible in Surrealist immediately |
| 4 | Registers `fn::pipeline::*` SurrealDB functions on first install |
| 5 | Runs a post-install Link check and prints any unresolved references as warnings |

After both installs `tabDocType` will have 761 rows (272 frappe + 489 erpnext)
and all DocType tables will be visible in the schema.

### 1.4 Set the default site and start

```powershell
cargo run -- use my-company        # writes "my-company" to sites/currentsite
cargo run -- start                 # reads sites/currentsite and starts the server
# listening on 127.0.0.1:8000
```

`spotledger use` is a one-time step per bench — once set, `spotledger start`
knows which site to boot without extra flags.

The server reads every `sites/*/site_config.toml` it finds and registers each
as a virtual site, distinguished by the `Host:` header.

### 1.5 Verify the site is up

```powershell
curl http://127.0.0.1:8000/api/ping -H "Host: my-company"
# {"message":"pong"}
```

### 1.6 Log in

```powershell
curl -X POST http://127.0.0.1:8000/api/method/login \
     -H "Host: my-company" \
     -d "usr=Administrator&pwd=changeme123"
# {"message":"Logged In","full_name":"Administrator","home_page":"/desk"}
# Response includes a Set-Cookie: sid=<token>
```

### 1.7 Who am I?

```powershell
curl http://127.0.0.1:8000/api/method/frappe.auth.get_logged_user \
     -H "Host: my-company" \
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

---

## 2. Creating a New DocType

> **When to use this section**: This approach — defining a `DocTypeMeta` struct
> in Rust — is for **Tier 0 kernel types** that the engine itself must know at
> compile time (User, Role, SystemSettings, etc.).  If you are building a
> domain feature (Customer, SalesOrder, Item, …), define a Frappe-compatible
> JSON file and deploy it via `install-app` instead (see §3).  The binary does
> not need to be recompiled to add a domain DocType.

All Tier 0 DocTypes are defined in Rust code as `DocTypeMeta` structs and
registered with the `inventory` crate.  There is **no JSON file** — the binary
is the single source of truth for their structure.

### 2.1 Decide where the DocType lives

Pick an existing crate (`spotledger-core`, `spotledger-accounting`, etc.) or
create a new app crate under `apps/`.  All crates in the workspace are linked
into the binary, so their `inventory::submit!` calls run automatically.

For this example we'll add an `Item` DocType to `apps/stock`.

### 2.2 Define the meta function

Open (or create) the relevant source file, e.g.
`apps/stock/src/item.rs`:

```rust
use spotledger_core::meta::{DocField, DocTypeMeta, FieldType, Permission};
use spotledger_core::registry::MetaEntry;

pub fn item_meta() -> DocTypeMeta {
    DocTypeMeta {
        name: "Item".into(),
        module: "Stock".into(),

        // structural flags
        is_single:       false,
        is_tree:         false,
        is_child:        false,
        is_submittable:  false,
        track_changes:   true,

        fields: vec![
            // The title field — used as the document name
            DocField::new("item_code", "Item Code", FieldType::Data)
                .required()
                .unique()
                .in_list()
                .in_standard_filter(),
            DocField::new("item_name", "Item Name", FieldType::Data)
                .required()
                .in_list(),
            DocField::new("item_group", "Item Group", FieldType::Link)
                .options("ItemGroup")   // link target DocType
                .required()
                .in_standard_filter(),
            DocField::new("description", "Description", FieldType::Text),
            DocField::new("is_stock_item", "Is Stock Item", FieldType::Check)
                .default_value("1"),
            DocField::new("standard_rate", "Standard Rate", FieldType::Currency),
            // Hidden system fields are added automatically — you do NOT need
            // to declare name/owner/creation/modified/modified_by/docstatus/idx.
        ],

        permissions: vec![
            Permission::full("System Manager"),
            Permission::full("Stock Manager"),
            Permission::read_only("All"),
        ],

        title_field:   Some("item_name".into()),
        search_fields: vec!["item_code".into(), "item_name".into()],
        sort_field:    Some("item_name".into()),
        sort_order:    Some("asc".into()),

        // How are new Item documents named?
        // "field:item_code" → document name = whatever the user enters in item_code
        autoname:       Some("field:item_code".into()),
        naming_series:  None,
    }
}

// Register with the inventory so the binary picks it up automatically.
inventory::submit!(MetaEntry {
    name: "Item",
    meta: item_meta,
});
```

### 2.3 Add the field type cheat sheet

| `FieldType` variant | SurrealDB storage | Notes |
|---|---|---|
| `Data` | `option<string>` | Short text (≤ 140 chars by default) |
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
| `SectionBreak` / `ColumnBreak` / `TabBreak` | *(none — layout only)* | No DB column |

### 2.4 Autoname patterns

The `autoname` field in `DocTypeMeta` is the **seed value** written into
`tabDocumentNamingRule` on `new-site`.  Admins can change it in the Desk
afterwards without touching code.

| Pattern | Effect |
|---|---|
| `"field:item_code"` | Name = value of `item_code` field |
| `"ITEM-.YYYY.-.####"` | Naming series: `ITEM-2024-00001`, `ITEM-2024-00002`, … |
| `"hash"` | Random UUID-style name |
| `"prompt"` | User must enter the name manually |
| `None` | UUID fallback (same as `"hash"`) |

For naming series, the `#` characters control zero-padding width.
`####` → 4 digits, `######` → 6 digits.

### 2.5 Export from your crate's `lib.rs`

```rust
// apps/stock/src/lib.rs
pub mod item;
```

The `inventory::submit!` call in `item.rs` does not need to be re-exported;
the registration happens at link time, not at import time.

### 2.6 Run the schema sync and migrations

After rebuilding the binary, either:

**On an existing site** (additive migration):
```powershell
cargo run -- migrate my-company
# --dry-run shows what would run without touching data
cargo run -- migrate my-company --dry-run
```

This (1) syncs new DDL from the compiled inventory, (2) upserts any new
`tabDocumentNamingRule` rows, and (3) runs any pending data migrations.

**On a fresh site** (everything at once):
```powershell
cargo run -- new-site my-company --db-url ws://127.0.0.1:8001 --admin-password changeme123
```

After either command, `tabItem` exists in SurrealDB with all declared fields
plus the standard system fields.

### 2.7 Verify

```powershell
# POST a new Item
curl -X POST http://127.0.0.1:8000/api/resource/Item \
     -H "Host: my-company" \
     -H "Cookie: sid=<token>" \
     -H "Content-Type: application/json" \
     -d '{"item_code":"WIDGET-001","item_name":"Widget","item_group":"Products"}'

# GET it back
curl http://127.0.0.1:8000/api/resource/Item/WIDGET-001 \
     -H "Host: my-company" \
     -H "Cookie: sid=<token>"
```

---

## 3. Importing a Frappe JSON DocType

If you already have a Frappe/ERPNext instance (or a clone of the Frappe
repository), you can import any DocType JSON directly — no hand-writing
required.

### 3.1 Export from Frappe (or use the repo)

Every DocType in a Frappe app is stored as a JSON file next to the Python
module, e.g.:

```
frappe/
  frappe/
    geo/
      doctype/
        currency/
          currency.json   ← this is the file
        country/
          country.json
```

You can also export from a running Frappe instance:
`Desk → DocType → <name> → Menu → Export`.

### 3.2 Generate the Rust source

```powershell
# Print to stdout first to review
cargo run -- generate --from apps/frappe/frappe/geo/doctype/currency/currency.json

# Write directly to a file
cargo run -- generate \
    --from apps/frappe/frappe/geo/doctype/currency/currency.json \
    --out  crates/spotledger-geo/src/currency.rs
```

The command reads the JSON and emits a complete `.rs` file:

```rust
// generated — edit freely
pub fn currency_meta() -> DocTypeMeta {
    DocTypeMeta {
        name: "Currency".into(),
        module: "Geo".into(),
        fields: vec![
            DocField::new("currency_name", "Currency Name", FieldType::Data)
                .required()
                .unique(),
            DocField::new("symbol", "Symbol", FieldType::Data)
                .in_list(),
            // …
        ],
        autoname: Some("field:currency_name".into()),
        // …
    }
}
inventory::submit!(MetaEntry { name: "Currency", meta: currency_meta });
```

### 3.3 What gets mapped

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
| `fields[].options` | `.options("…")` (Link target or Select choices) |
| `fields[].description` | `.description("…")` |
| `permissions[].role` + flags | `Permission::full` / `read_only` / custom struct |
| `field_order` array | field declaration order in `vec![]` |

Frappe booleans stored as `0`/`1` integers are handled automatically.

### 3.4 Wire the file into a crate

1. Create the crate if it doesn't exist (copy `crates/spotledger-geo/Cargo.toml`
   as a template).
2. Add `pub mod currency;` to `src/lib.rs`.
3. Add the crate to the workspace `Cargo.toml` `[workspace] members` list.
4. Add it as a dependency of the main binary in `crates/spotledger/Cargo.toml`.
5. Rebuild and run `migrate` (or `new-site` for a fresh environment).

**The `spotledger-geo` crate** (Currency + Country) ships with the framework
and shows the complete example:

```
crates/spotledger-geo/
  Cargo.toml
  src/
    lib.rs
    currency.rs   ← generated from frappe/geo/doctype/currency/currency.json
    country.rs    ← generated from frappe/geo/doctype/country/country.json
```

---

## 4. Architecture Reference

### Metadata tiers

```
┌──────────────────────────────────────────────────────────────┐
│ Tier 0 — Compiled into the binary (Rust code only)           │
│                                                              │
│  DocTypeMeta { fields, is_single, is_tree, … }               │
│  ~16 kernel types: DocType, DocField, User, Role, Session, … │
│  Resolved via:  inventory::iter::<MetaEntry>()               │
│  Never stored in tabDocType — the binary IS the meta         │
└──────────────────────────────────────────────────────────────┘
                          │ seeded on install-app
                          ▼
┌──────────────────────────────────────────────────────────────┐
│ Domain DocTypes — JSON-seeded into SurrealDB at install time │
│                                                              │
│  tabDocType   — one row per DocType (name, module,           │
│                 issingle, issubmittable, …)                  │
│  tabDocField  — one row per field  (parent = DocType name)   │
│                                                              │
│  Source of truth for all ~761 Frappe/ERPNext DocTypes.       │
│  Adding a field = edit the JSON + re-run install-app.        │
│  No binary recompile needed.                                 │
└──────────────────────────────────────────────────────────────┘
                          │ created by users/admins
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

Rust owns only auth; all validation, computed fields, and side-effects run
inside SurrealDB via `DEFINE FUNCTION` statements registered by `install-app`:

```
POST /api/resource/Account/ACC-001
  │
  ├─ Rust: session check + permission check  (always in Rust)
  └─ SurrealDB: UPSERT → fn::pipeline::run() fires
                         → validates mandatory fields
                         → checks issubmittable / docstatus rules
                         → fires DEFINE EVENT hooks
```

The `fn::pipeline::run($doctype, $doc, $action)` function reads
`tabDocType.issubmittable` directly — no separate metadata table exists.

### Document naming flow (`naming::resolve_name`)

```
resolve_name(adapter, doctype, fields)
│
├─ 1. fields["name"] is set and non-empty?
│      └─ YES → use it directly (user-supplied or import)
│
├─ 2. fields["naming_series"] is set?
│      └─ YES → next_name(adapter, naming_series_value)
│               (increments tabSeries counter and formats)
│
├─ 3. tabDocumentNamingRule WHERE document_type = doctype?
│      └─ FOUND → read `autoname` field and evaluate:
│           "field:X"  → use fields[X]
│           "SERIES.-…"→ next_name(adapter, pattern)
│           "hash"/"UUID" / unrecognised → UUID fallback
│
└─ 4. UUID fallback  (new-{timestamp-hex})
```

### Framework-internal tables (not DocTypes)

These tables are bootstrapped directly (`bootstrap::run_framework_tables`) and
never appear in the Desk UI:

| Table | Purpose |
|---|---|
| `__Auth` | Password hashes (`doctype`, `name`, `fieldname`, `password`) |
| `tabSessions` | Active login sessions and SID tokens |
| `tabSeries` | Naming-series counters (`SINV-2024-` → 42) |
| `tabSingles` | Key/value store for `is_single` DocTypes |

### SurrealDB record IDs

Spotledger uses the table-prefix convention (`tabUser`, `tabRole`, …) but
**not** Frappe's Python class names.  Record IDs follow SurrealDB syntax:

```
tabUser:Administrator          ← simple alphanumeric
tabRole:⟨System Manager⟩      ← ⟨…⟩ required when name has spaces
tabSessions:⟨abc123def456⟩    ← ⟨…⟩ when name contains non-word chars
```

### Multi-site request routing

The HTTP server reads `Host:` and looks up the matching `SiteState`.  When
multiple sites are loaded you **must** pass the correct `Host:` header:

```
GET /api/resource/Item/WIDGET-001
Host: my-company        ← required when more than one site is loaded

→ router finds SiteState { db: <my-company namespace> }
→ executes query in that namespace
```

> **Dev convenience**: if only one site is in `sites/`, the `Host:` header is
> optional — the single site is used automatically.

All sites share one Axum router process; isolation is at the SurrealDB
namespace level.

### Migration strategy

Migrations are **per-site** by design.  Each site is its own SurrealDB
namespace, so `tabMigration` in namespace `acme` is independent of `tabMigration`
in namespace `beta`.

Two tiers run on every `serve` startup and on `migrate`:

| Tier | Mechanism | Tracking |
|---|---|---|
| DDL (schema) | `ensure_all_schemas` — `DEFINE … IF NOT EXISTS` | None needed — always idempotent |
| Data | `MigrationEntry` registered via `inventory::submit!` | `tabMigration` — one row per applied migration per site |

Data migrations are identified by a sortable name (`"0001_seed_x"`) and
executed in lexicographic order.  Each is recorded in `tabMigration` on success
so it never runs twice against the same site.

To write a data migration in any crate:

```rust
use spotledger_db::migrations::{MigrationEntry, MigrationFuture};
use spotledger_db::adapter::DbAdapter;

fn run_0001(db: DbAdapter) -> MigrationFuture {
    Box::pin(async move {
        db.execute("UPDATE tabFoo SET bar = 'default'", vec![]).await
    })
}

inventory::submit!(MigrationEntry {
    name: "0001_backfill_foo_bar",
    run:  run_0001,
});
```

Commands:
```powershell
# Explicit migration (CI / controlled rollout)
cargo run -- migrate my-company

# Dry-run: see what would run without touching data
cargo run -- migrate my-company --dry-run
```

### Adding a new app crate

**For a domain app (JSON-based — no Rust required):**

1. Create `apps/my-app/my-app/` following the Frappe convention.
2. Add a `{doctype_name}/{doctype_name}.json` file for each DocType under
   the appropriate module directory.
3. Run `cargo run -- install-app my-app <site> --bench .`.

No binary recompile.  `tabDocType`, `tabDocField`, and all `DEFINE TABLE`
statements are written automatically.

**For a Tier 0 compiled crate** (engine types only — rarely needed):

1. Create `apps/my-app/Cargo.toml` (see any existing app for the template).
2. Add it to the workspace `Cargo.toml` under `[workspace] members`.
3. Define DocTypes and register them with `inventory::submit!`.
4. Add `my-app = { path = "apps/my-app" }` to `spotledger/Cargo.toml`
   (the main binary crate) so the app's submitted entries are linked in.
5. Rebuild and run `new-site` or `migrate`.
