# Getting Started with Spotledger

Spotledger is a Frappe-compatible ERP framework written in Rust, backed by
SurrealDB.  This guide walks you through three topics:

1. [Running your first site](#1-running-your-first-site)
2. [Creating a new DocType in Rust code](#2-creating-a-new-doctype)
3. [Reference: how the system works end-to-end](#3-architecture-reference)

---

## 1. Running Your First Site

### Prerequisites

| Requirement | Notes |
|---|---|
| Rust toolchain (stable) | `rustup show` |
| SurrealDB v2+ running | `surreal start --bind 127.0.0.1:8000 --user root --pass root memory` |
| `cargo` in PATH | Comes with Rust |

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
| 5 | All compiled DocType schemas synced (`DEFINE TABLE … SCHEMAFULL`) |
| 6 | Seed records inserted: roles, user types, Administrator user |
| 6b| `tabDocumentNamingRule` seeded from every compiled `DocTypeMeta.autoname` |
| 7 | Administrator password written to `__Auth` |

After this completes you have a fully functional site in SurrealDB namespace
`my-company`.

### 1.3 Start the HTTP server

```powershell
cargo run -- serve --bench .
# listening on 127.0.0.1:8000
```

The server reads every `sites/*/site_config.toml` it finds and registers each
as a virtual site, distinguished by the `Host:` header.

### 1.4 Verify the site is up

```powershell
curl http://127.0.0.1:8000/api/ping -H "Host: my-company"
# {"message":"pong"}
```

### 1.5 Log in

```powershell
curl -X POST http://127.0.0.1:8000/api/method/login \
     -H "Host: my-company" \
     -d "usr=Administrator&pwd=changeme123"
# {"message":"Logged In","full_name":"Administrator","home_page":"/desk"}
# Response includes a Set-Cookie: sid=<token>
```

### 1.6 Who am I?

```powershell
curl http://127.0.0.1:8000/api/method/frappe.auth.get_logged_user \
     -H "Host: my-company" \
     -H "Cookie: sid=<token from previous step>"
# {"message":"Administrator"}
```

---

## 2. Creating a New DocType

All DocTypes are defined in Rust code as `DocTypeMeta` structs and registered
with the `inventory` crate.  There is **no JSON file** — the binary is the
single source of truth for structure.

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

## 3. Architecture Reference

### Metadata tiers

```
┌──────────────────────────────────────────────────────┐
│ Tier 1 — Compiled into the binary (code only)        │
│                                                      │
│  DocTypeMeta { fields, is_single, is_tree, … }       │
│  Resolved via:  inventory::iter::<MetaEntry>()       │
│  Never stored in tabDocType — the binary IS the meta │
└──────────────────────────────────────────────────────┘
                          │ seeded on new-site
                          ▼
┌──────────────────────────────────────────────────────┐
│ Tier 2 — Seeded to DB, admin-editable at runtime     │
│                                                      │
│  tabDocumentNamingRule  → naming strategy per type   │
│  tabDocPerm             → role permissions per type  │
└──────────────────────────────────────────────────────┘
                          │ created by users/admins
                          ▼
┌──────────────────────────────────────────────────────┐
│ Tier 3 — DB only (never in code)                     │
│                                                      │
│  tabCustomField         → extra fields added at UI   │
│  tabPropertySetter      → field property overrides   │
│  tabUserPermission      → document-level ACL         │
└──────────────────────────────────────────────────────┘
```

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

1. Create `apps/my-app/Cargo.toml` (see any existing app for the template).
2. Add it to the workspace `Cargo.toml` under `[workspace] members`.
3. Define DocTypes and register them with `inventory::submit!`.
4. Add `my-app = { path = "apps/my-app" }` to `spotledger/Cargo.toml`
   (the main binary crate) so the app's submitted entries are linked in.
5. Rebuild and run `new-site` or `migrate`.
