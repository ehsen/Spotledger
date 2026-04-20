# DB-Native App Platform — Architecture & Implementation Plan

**Date**: April 18, 2026  
**Status**: Planning + Execution In Progress  
**Scope**: Introduces a second app type (DB-native), migrates non-Tier-0 framework DocTypes out of Rust, establishes Spotledger-Core, and lays the foundation for the app store.

---

## 1. Vision in One Sentence

Spotledger becomes a **general-purpose enterprise app development platform** where apps are created, designed, and published entirely from within the platform — no Rust compilation, no WASM, no local tooling required — and WASM is reserved only for heavy compute use-cases.

> **Authoring principle**: The platform is the IDE. You open the browser, create an app, add DocTypes, write SurrealQL logic in the Designer, and publish — all without a terminal. The CLI (`export-app`, `pack-app`) exists for CI/CD pipelines and advanced users who want git-based version control, but it is never a prerequisite.

---

## 2. The Two App Types

| Dimension | WASM App (existing) | DB-Native App (new) |
|---|---|---|
| Stored as | `.wasm` binary | directory / `.slpkg` archive |
| Compiled by | `cargo xtask wasm` | never — runs directly |
| DocType definitions | Rust `DocTypeMeta` struct | JSON (same format already used for frappe/erpnext) |
| Logic (validate, save hooks) | Rust → WASM | SurrealQL `fn::` functions |
| Schema seeding | `inventory::submit!` + `ensure_all_schemas` | `install_app` reads JSON, upserts to SurrealDB |
| Versioning | Cargo semver + build | `app.json` manifest + `.slpkg` package |
| Use case | CPU-heavy computation, external integrations | all ERP doctypes, workflows, automation, extensions |
| Who authors | Rust dev | anyone — from within the platform UI |

**WASM apps are not deprecated.** They remain the right choice for PDF rendering, heavy math, external API clients, anything CPU-bound. DB-native apps cover 95% of ERP use-cases.

---

## 3. What This Is NOT

- This is not a multi-tenant SaaS redesign.
- This does not change the HTTP server, auth, or save pipeline.
- Frontend changes are out of scope here (separate discussion).
- Distribution / app store UI is deferred (noted in plan but not implemented now).

---

## 4. Current State Audit

### What is currently compiled into Rust (must migrate)

| Rust crate | Module | DocTypes | Has logic? |
|---|---|---|---|
| `spotledger-contacts` | Contacts | Address, Contact, Gender, Salutation | No |
| `spotledger-automation` | Automation | Webhook, Scheduled Job, Server Script, Auto Repeat | No (stubs only) |
| `spotledger-geo` | Geo | Country, State, Currency, Language, Timezone | No |
| `spotledger-desk` | Desk | Workspace, Page, Dashboard, Note | No |
| `spotledger-printing` | Printing | Print Format, Letter Head, Print Settings | No |
| `spotledger-accounting` | Accounts | (already removed) | N/A |

None of these crates contain any business logic — they are pure `DocTypeMeta` struct definitions with `inventory::submit!`. They exist only to drive DDL at startup. This is the entire migration problem: **they do not need to be in Rust at all**.

### What must stay in Rust (Tier 0 — cannot ever be DB-native)

| Crate | Reason |
|---|---|
| `spotledger-core` (doctypes: DocType, DocField, DocPerm) | Meta-schema: cannot be loaded from DB if DB schema doesn't exist yet |
| `spotledger-core` (User, Role, UserPermission) | Auth is a Rust responsibility forever |
| `spotledger-core` (Site, ModuleDef) | Server startup scaffolding |
| `spotledger-db` (schema.rs, naming.rs, save_proxy.rs) | Engine internals |
| `spotledger-http` | HTTP server, routing, session |

The `TIER_0_DOCTYPES` constant in `designer.rs` is already correct. It will not change.

---

## 5. Spotledger-Core App

The **Spotledger-Core** app is a DB-native app that ships with the Spotledger binary and is auto-installed on `new-site`. It replaces the compiled `spotledger-contacts`, `spotledger-automation`, `spotledger-geo`, `spotledger-desk`, and `spotledger-printing` crates.

### Location

```
apps/
  frappe/          ← existing (761 DocTypes, will eventually migrate too)
  erpnext/         ← existing WASM app
  spotledger-core/ ← NEW DB-native app
    app.json
    modules/
      Contacts/
        doctypes/
          Address/
            Address.json
          Contact/
            Contact.json
          Gender/
            Gender.json
          Salutation/
            Salutation.json
      Automation/
        doctypes/
          Webhook/
          Scheduled Job Type/
          Server Script/
          Auto Repeat/
        surql/
          webhook_dispatch.surql
      Geo/
        doctypes/
          Country/
          Currency/
          Language/
          Timezone/
          State/
        fixtures/
          countries.json
          currencies.json
          languages.json
      Desk/
        doctypes/
          Workspace/
          Notification/
          Note/
          Tag/
      Printing/
        doctypes/
          Print Format/
          Letter Head/
          Print Settings/
      Setup/
        doctypes/
          System Settings/
          Global Defaults/
          Naming Series/
```

### `app.json` manifest

```json
{
  "name": "spotledger-core",
  "title": "Spotledger Core",
  "version": "1.0.0",
  "description": "Framework DocTypes — the foundation every other app builds on",
  "author": "Spotledger",
  "license": "AGPL-3.0",
  "depends_on": [],
  "modules": ["Contacts", "Automation", "Geo", "Desk", "Printing", "Setup"],
  "auto_install": true
}
```

`auto_install: true` tells `new-site` to install this app automatically, just like the current hard-coded module seeding.

---

## 6. DB-Native App Format (`.slpkg`)

A DB-native app is a **directory** during development and a **`.slpkg` archive** (gzip'd tar) for distribution.

### Directory structure

```
my-app/
  app.json                   ← manifest (required)
  modules/
    <ModuleName>/
      doctypes/
        <DoctypeName>/
          <DoctypeName>.json ← standard Frappe doctype JSON format
          fixtures/          ← optional seed records (array of JSON objects)
            <DoctypeName>.json
      surql/                 ← optional SurrealQL logic files
        <function_name>.surql
  fixtures/                  ← module-level fixtures
  surql/                     ← app-level SurrealQL (runs once at install)
    install.surql             ← one-time setup (create graph edges, seed data)
  patches/
    v1_0_1_<description>.surql  ← migration patches (run once, recorded in patch log)
```

### DocType JSON format

**Unchanged** — same Frappe-compatible JSON already consumed by `seed_doctypes.rs`. No new format needed.

### SurrealQL files — naming convention

```
validate_<doctype>.surql   → fn::<app>::validate_<doctype>($doc_id)
before_save_<field>.surql  → fn::<app>::before_save_<field>($doc_id)
on_submit_<doctype>.surql  → fn::<app>::on_submit_<doctype>($doc_id)
```

The file name determines the function name registered in `fn_source` and wired into the pipeline. The `save_function` designer endpoint already handles this — `install_app` will call the same code path.

---

## 7. App Manifest Schema (`app.json`)

```typescript
interface AppManifest {
  name: string;           // kebab-case, unique identifier
  title: string;          // display name
  version: string;        // semver
  description?: string;
  author?: string;
  license?: string;
  depends_on: string[];   // ["spotledger-core", "spotledger-entities"]
  modules: string[];      // module names this app provides
  auto_install?: boolean; // install with new-site? (only for bundled apps)
  min_core_version?: string; // minimum Spotledger version required
}
```

### Dependency resolution

At `install_app` time:
1. Parse `app.json`
2. For each `depends_on`: verify that app is already installed in `installed_app` table
3. If not, error with clear message: `"spotledger-core must be installed before my-app"`
4. Dependencies are never auto-installed (explicit intent only)

---

## 8. Platform App Management (Primary)

### 8.1 In-Platform App Lifecycle

The **normal** app authoring workflow is entirely within the Spotledger UI:

```
Apps panel (new sidebar entry)
  ├── Installed Apps list
  │     name · version · module count · author
  ├── [+ New App] button → "New App" form
  │     name (kebab), title, version, description, author, license, depends_on
  │     → creates row in `installed_app` with status = "development"
  │     → creates Module Def rows for declared modules
  ├── App detail view
  │     Overview | Modules | DocTypes | Functions | Publish
  │     [Add Module]  [Export .slpkg]  [Publish]
  └── Publish workflow
        bump version field → validate → generate .slpkg download or push to registry
```

The Designer's **App picker** (already implemented) lets the user assign any existing DocType (or new one) to an app. Creating a new app from the Apps panel immediately makes it available as an option in the Designer's app dropdown.

### 8.2 Backend API Methods (In-Platform)

| Method | Purpose |
|---|---|
| `spotledger.apps.list` | List all installed apps with metadata |
| `spotledger.apps.create` | Create a new DB-native app (name, title, version, depends_on, modules) |
| `spotledger.apps.get` | Fetch app manifest + stats (doctype count, fn count) |
| `spotledger.apps.update` | Update manifest fields (title, version, description) |
| `spotledger.apps.export` | Generate and return a `.slpkg` binary stream |
| `spotledger.apps.publish` | Push to registry (future) |

All of these write to / read from `installed_app` in SurrealDB. No filesystem involvement until `export`.

### 8.3 CLI Commands (Secondary — for CI/CD and advanced users)

The CLI is an optional complement, not the primary path:

```
spotledger new-app <name>                           # scaffold app.json + directory (offline bootstrap)
spotledger new-module <module> --app <app>          # add module directory
spotledger new-doctype <doctype> --app <app> --module <module>  # scaffold JSON
spotledger pack-app <app-dir>                       # → <name>-<version>.slpkg
spotledger install-app <app-dir-or-slpkg> <site>    # install from file/dir
spotledger upgrade-app <app-name> <new-slpkg> <site>
spotledger uninstall-app <app-name> <site>          # 3-stage safe uninstall
spotledger export-app <app-name> <site>             # DB state → filesystem (for git)
```

`export-app` closes the loop: design everything in the platform → export to files → commit to git → CI runs `install-app` on production.

---

## 9. App Versioning Strategy

### During development: filesystem + git

The app directory is the source of truth during development. The user puts it in a git repo.

```
git init my-app
# make changes in Designer → export-app → commit
git add . && git commit -m "feat: add Contact notes field"
```

No special tooling needed. The directory format is designed to be git-friendly (one file per DocType = clear diffs).

### Why git is the right answer here

- **No lock-in** — standard VCS workflow every developer already knows
- **AI-friendly** — AI generates diffs, not binary blobs
- **Works offline** — no central registry dependency during development
- **Private repos** — GitHub/GitLab private repos for proprietary apps
- **Branch-based review** — PR reviews for schema changes (major for enterprise)

Compared to alternatives:
- A custom registry: more infrastructure, no benefit during dev phase
- Database-native versioning: `schema_change_log` covers the audit trail but not branching/collaboration
- ZIP-only: no branching, terrible diffs

### Release artifacts

`spotledger pack-app` produces a `.slpkg` file (gzip'd tar with a SHA-256 manifest). This is the unit of distribution — post to GitHub Releases, app store, or share directly.

### Version tracking in DB

Two new tables (extend existing `schema_change_log` architecture):

```surql
DEFINE TABLE installed_app SCHEMAFULL;
DEFINE FIELD name        ON installed_app TYPE string;   -- app name (PK)
DEFINE FIELD version     ON installed_app TYPE string;   -- semver
DEFINE FIELD installed_at ON installed_app TYPE datetime;
DEFINE FIELD app_path    ON installed_app TYPE option<string>; -- local dev path (null for installed .slpkg)
DEFINE FIELD manifest    ON installed_app TYPE object;  -- full app.json snapshot

DEFINE TABLE app_patch_log SCHEMAFULL;
DEFINE FIELD app         ON app_patch_log TYPE string;
DEFINE FIELD patch_file  ON app_patch_log TYPE string;
DEFINE FIELD applied_at  ON app_patch_log TYPE datetime;
```

`schema_change_log` already handles field-level diffs — no change needed there.

---

## 10. Designer Integration

The DocDesigner already saves to `tabDocType` / `tabDocField` / `fn_source` / `pipeline_node`. No change to how it works internally.

Three designer capabilities are needed:

### 10.1 App/Module context in Designer (Done)

When creating a DocType, the user picks:
- **App** — which installed DB-native app owns this DocType (or `__custom__` for site-local types)
- **Module** — which module within that app

The App dropdown is populated from `installed_app` via `frappe.desk.desktop.get_installed_apps`. Already implemented.

### 10.2 Apps Panel (primary create/manage flow)

A new top-level **Apps** panel in the sidebar (alongside Designer, Users, etc.) is the main place to:

- See all installed apps with status badges (`bundled` / `development` / `installed`)
- Create a new app (form: name, title, version, description, author, license, depends_on modules)
- View an app's DocTypes and functions
- Trigger export / publish

Creating an app here writes a new `installed_app` row in SurrealDB and immediately makes the app available in the Designer's app picker dropdown — no CLI step needed.

### 10.3 Export from Designer / Apps Panel

A **"Export .slpkg"** button on the app detail view — calls `spotledger.apps.export`:
- Reads all DocTypes where `app = <selected_app>` from `tabDocType`
- Reads their fields, permissions, SurrealQL functions
- Returns a `.slpkg` binary stream (download in browser)

This is the "design in platform, ship as file" loop. The CLI `export-app` does the same thing but writes to the filesystem instead of returning a download.

---

## 11. Migration Plan — Rust Crates → DB-Native

This is the mechanical work. Zero behavior change; just moving definitions from Rust to JSON.

### Phase 1 — Spotledger-Core app scaffold (no Rust changes yet)

1. Create `apps/spotledger-core/app.json`
2. Convert each compiled DocType to JSON:
   - Run `spotledger generate doctype` for each (already exists)
   - Or write JSON by hand from existing `*_meta()` functions — they have all the info
   - Place in `apps/spotledger-core/modules/<Module>/doctypes/<Name>/<Name>.json`
3. Verify `install_app` (unchanged) can install `spotledger-core` cleanly
4. Add `spotledger-core` to `auto_install` list in `new-site`

### Phase 2 — Remove compiled crates

After Phase 1 is verified working end-to-end:

5. Remove `inventory::submit!` from `spotledger-contacts`, `spotledger-geo`, `spotledger-desk`, `spotledger-printing`, `spotledger-automation`
   - These crates can be **deleted entirely** (no logic, no value without the inventory)
   - Or kept as empty stubs with a big comment — prefer deletion for clarity
6. Remove them from workspace `Cargo.toml`
7. Remove `spotledger_contacts::name()` etc. force-link calls from `main.rs`
8. Remove their module names from `FM::FRAMEWORK_MODULES` (now seeded by spotledger-core install)
9. Update `ensure_all_schemas` — it only processes Tier-0 types now (DocType, DocField, etc.)

**What does NOT change in step 6-9**:
- DDL generation (`schema.rs`) still runs for Tier-0 types at startup
- `save_proxy.rs`, `naming.rs`, pipeline code — untouched
- `designer.rs`, `TIER_0_DOCTYPES` guard — untouched
- HTTP routes — untouched

### Phase 3 — `new-site` auto-install spotledger-core

10. In `new_site.rs`: after creating the site, call `install_app("spotledger-core", site)` if `auto_install` is true in its manifest
11. Remove hard-coded `seed_framework_modules()` call from `server.rs` startup — modules are now **install-only**, with no server-start fallback seeding

### Phase 4 — `new-app` CLI command

12. Implement `spotledger new-app <name>` — creates directory with `app.json` template
13. Implement `spotledger new-doctype` — scaffolds a minimal JSON from a template
14. Implement `spotledger export-app <app-name> <site>` — DB → filesystem export

### Phase 5 — Designer "App" picker

15. Add `app` field to `tabDocType` (already in `installed_app` table; link to it) — Done
16. Designer `save` endpoint: write `app` to `tabDocType` record — Done (passed through `meta`/`extra_meta` in save path)
17. Designer UI: dropdown showing installed DB-native apps + `__custom__` — Done

---

## 12. What Rust Retains After Migration

After this migration, the Rust binary's responsibilities are:

| Responsibility | Crate | Notes |
|---|---|---|
| HTTP server + routing | `spotledger-http` | unchanged |
| Session auth + permission checks | `spotledger-db/auth.rs`, `spotledger-db/permissions.rs` | unchanged |
| Save proxy (auth → naming → pipeline call) | `spotledger-db/save_proxy.rs` | unchanged |
| Naming (UUID, hash, series, by-field) | `spotledger-db/naming.rs`, `apps/*/surql/naming/05_naming.surql` | unchanged |
| Tier-0 DDL at startup | `spotledger-db/schema.rs` | only DocType, DocField, DocPerm, User, Role, UserPermission, Site, ModuleDef |
| DB-native app install/export | `spotledger/install_app.rs` (extended) | reads app.json + JSON doctypes |
| WASM plugin host | `spotledger-plugins` | unchanged |
| Printing (PDF generation) | `spotledger-printing` (Rust stays for PDF work) | only the Rust PDF engine; Print Format doctype moves to spotledger-core |
| CLI (new-site, install-app, pack-app, export-app) | `spotledger/cli.rs` | extended |

The binary becomes: **auth + naming + HTTP + WASM host + DB-native app installer**. Nothing else.

---

## 13. App Store Foundation (Deferred — Noted Only)

The `.slpkg` format + `installed_app` table are the technical foundation. The app store layer (registry, discovery, trust/signing, billing) is explicitly out of scope for this phase. The following DB structures will make it straightforward to add later:

```surql
-- Future: app registry entry
DEFINE TABLE app_registry SCHEMAFULL;
DEFINE FIELD app_name    ON app_registry TYPE string;
DEFINE FIELD publisher   ON app_registry TYPE string;
DEFINE FIELD versions    ON app_registry TYPE array<object>;
DEFINE FIELD signature   ON app_registry TYPE option<string>; -- Ed25519 sig over .slpkg
```

For now: distribute `.slpkg` files via GitHub Releases, direct download, or email. The `install_app` command already accepts a file path — it will be extended to accept a URL too.

---

## 14. Example: Building a "Payroll" App

To make the vision concrete — **everything starts in the platform**:

### In the browser (primary workflow)

```
1. Sidebar → Apps → [+ New App]
   Fill: name=payroll, title=Payroll, version=1.0.0
   depends_on: [spotledger-core]
   modules: [Payroll]
   → Click Create
   
2. Designer → [New DocType]
   Name: Employee, App: payroll, Module: Payroll
   → Add fields in the Designer UI
   → Write validate/save SurrealQL in the Pipeline tab
   → Save
   
3. Repeat for Salary Slip, Salary Component, Leave Allocation…

4. Apps panel → payroll → [Export .slpkg]
   → Downloads payroll-1.0.0.slpkg
   
5. Install on production:
   Admin → Apps → [Install from file] → upload payroll-1.0.0.slpkg
```

### Optional: git-based version control (advanced)

```bash
# Export DB state to filesystem for git
spotledger export-app payroll my-dev-site
git add apps/payroll/ && git commit -m "feat: add Employee leave_balance field"

# Package from files (CI/CD)
spotledger pack-app apps/payroll/
# → payroll-1.0.0.slpkg

# Deploy to production
spotledger install-app payroll-1.0.0.slpkg prod-site
```

The CLI round-trip is purely optional. The `.slpkg` produced by the browser export and the CLI are identical.

The app dependency:
```json
{
  "name": "payroll",
  "depends_on": ["spotledger-core"]
}
```

`spotledger-core` provides Address, Contact, Country, Currency and other base types. A future `spotledger-entities` app will provide Item, Customer, Supplier, Company as a shared base.

---

## 15. Implementation Phases & Priority

### Phase 1 — Spotledger-Core app (HIGH — unblocks everything)

| Task | File(s) affected | Effort |
|---|---|---|
| Create `apps/spotledger-core/app.json` | new file | tiny |
| Convert compiled Rust DocTypes → JSON | new files in `apps/spotledger-core/` | medium |
| Extend `install_app` to read `app.json` and mark `installed_app` | `install_app.rs` | small |
| Add `auto_install` logic to `new_site.rs` | `new_site.rs` | small |
| Remove non-Tier-0 `inventory::submit!` crates | 5 crates | small (delete code) |
| Update `FM::FRAMEWORK_MODULES` for Tier-0 only | `modules.rs` | tiny |

**Verifiable milestone**: `spotledger new-site test --...` installs cleanly with Contacts, Geo, Automation DocTypes coming from DB, not Rust.

### Phase 2 — In-Platform App Management (HIGH — core UX, enables self-service authoring)

This is the primary authoring entry point. Users must be able to create and manage apps without any CLI.

| Task | File(s) affected | Effort |
|---|---|---|
| `spotledger.apps.list` method — list installed apps from `installed_app` | new `methods/apps.rs` | small |
| `spotledger.apps.create` method — create row in `installed_app`, seed Module Def rows | `methods/apps.rs` | small |
| `spotledger.apps.get` method — manifest + doctype/fn counts | `methods/apps.rs` | small |
| `spotledger.apps.update` method — update title/version/description | `methods/apps.rs` | tiny |
| `spotledger.apps.export` method — generate `.slpkg` binary in-memory, return as download | `methods/apps.rs`, `export_app.rs` | medium |
| Apps panel UI — list, create form, detail view | `Spotledger-ui/src/app/panels/Apps/` | medium |
| Sidebar entry for Apps panel | `Sidebar.tsx` | tiny |
| "Install from file" upload in Apps panel | `Apps/` panel | small |

**Verifiable milestone**: Open browser → Apps panel → create app → open Designer → new DocType picks the app from dropdown → export .slpkg → reinstall via Apps panel upload.

### Phase 3 — CLI Tools (MEDIUM — for CI/CD and power users, not required for basic authoring)

| Task | File(s) affected |
|---|---|
| `spotledger new-app` command | `cli.rs`, new `new_app.rs` |
| `spotledger export-app` command | new `export_app.rs` |
| `spotledger pack-app` (.slpkg format) | new `pack_app.rs` |
| `install_app` accepts `.slpkg` files | `install_app.rs` |

### Phase 4 — Designer "App" context (Done)

| Task | Status |
|---|---|
| `app` field on `tabDocType` | Done |
| Designer save writes `app` via `extra_meta` | Done |
| Designer UI: app picker dropdown | Done |
| "Export App" button in Apps panel | Phase 2 |

---

## 16. Invariants (Non-Negotiables)

1. **Tier-0 types are never DB-native**. DocType, DocField, DocPerm, User, Role, UserPermission, Site, ModuleDef must exist in Rust and are bootstrapped before any DB read. The `TIER_0_DOCTYPES` guard in `designer.rs` stays forever.

2. **DB-native apps do not change the save pipeline**. `save_proxy.rs` does not care whether a doctype came from a WASM app or a DB-native app. The pipeline is doctype-agnostic.

3. **install_app is always the right path**. There is no second way to "load" an app. DB-native apps go through `install_app` exactly like WASM apps (minus the WASM loading step).

4. **Doctype JSON format is unchanged**. The Frappe-compatible JSON format that `seed_doctypes.rs` already parses is the canonical format. No new format. DB-native apps use the exact same JSON.

5. **schema_change_log records all diffs**. Every install/upgrade writes change records. This is the audit trail and the rollback mechanism.

6. **WASM apps are additive**. A WASM app can still carry DocType JSONs alongside its `.wasm`. That path is untouched.

---

## 17. Execution Snapshot (April 18, 2026)

This section captures what is already implemented in code versus what remains to close Phase 1.

### 17.1 Implemented

- `apps/spotledger-core/app.json` exists and is loadable.
- Spotledger-Core DocTypes are present in app files under `apps/spotledger-core/spotledger-core/*/doctype/*`.
- `new-site` already auto-installs `spotledger-core` when the app directory exists.
- `install_app` reads `app.json` and records installs into `installed_app`.
- `install_app` enforces `depends_on` dependencies from `app.json` against `installed_app`.
- `new-site` auto-install now honors `app.json.auto_install` (no manifest flag, no auto-install).
- `install-app` now accepts `.slpkg` archives and installs directly from the packaged app contents.
- CLI scaffolding commands exist: `new-app`, `new-module`, `new-doctype`, `export-app`, `pack-app`.
- `export-app` now includes `permissions` from `tabDocPerm` and exports function/wiring metadata from `fn_source` + pipeline tables.
- Fresh-site smoke test verified on `verify_install_only_rebuilt`: `installed_app = [spotledger-core]`, `Module Def` search returns 9 modules from install flow, and `Workspace` metadata loads successfully via `getdoctype`.
- Non-Tier-0 framework crates are already removed from the workspace (migration largely completed at crate level).

### 17.2 Remaining to Close Phase 1

- None. Phase 1 verification is complete.

### 17.3 Phase 1 Close-Out Checklist (Definition of Done)

1. `spotledger new-site <site>` results in `spotledger-core` installed only when `app.json.auto_install = true`.
2. `spotledger install-app <app>` fails with a clear message when any manifest dependency is missing.
3. `Module Def` rows for non-Tier-0 modules are created by install flow, not server startup fallback.
4. `spotledger pack-app` followed by `spotledger install-app <file.slpkg>` works end-to-end.
5. Fresh site smoke test passes:
  - login works
  - `search_link` for `Module Def` returns expected modules
  - Spotledger-Core doctypes are available from `tabDocType`/`getdoctype`

---

## 18. User Management — Making Users Creatable

### 18.1 Problem Statement

User, Role, HasRole, UserPermission, and UserType are **Tier-0 DocTypes** compiled into Rust. They exist as schema definitions and their seed records (`System Manager`, `System User`, `Website User`, `Administrator`) are already written to SurrealDB at `new-site` time (in `bootstrap.rs: SEED_RECORDS`). However, **there is currently no way to create or manage users through the UI** because:

1. The User form has special field types (`Password`, `HTML`) that the generic `FieldRenderer` does not handle
2. The roles child table renders as a raw data grid — unusable without a role-picker widget
3. Password changes must be intercepted before DB write (hash before store) — the generic save path does not do this
4. No dedicated "Users" panel exists in `Spotledger-ui`

### 18.2 What Frappe Does (for reference)

Frappe's `User` is a **standard DocType form** (not a completely custom page). The standard form renderer handles layout and fields. The special behaviour is layered on top via `user.js` client scripts:

- `roles_html` — an `HTML` field whose wrapper div is populated by a `frappe.ui.RoleEditor` widget (a custom JavaScript class that renders a searchable role grid)
- `modules_html` — similarly populated by a `frappe.ui.ModuleEditor`
- `new_password` — a `Password` field that is intercepted by the save path and routed to `set_user_password()`, never stored in the main doctype table

The `user_list.js` adds a custom list indicator (Active/Disabled badge). There is no separate "Users" page — it is just `/desk#List/User` and `/desk#Form/User/<name>`.

### 18.3 Our Approach — Dedicated Users Panel + Patched Generic Form

We will do **both**:

**Path A — Dedicated "Users" panel** (primary UX, immediate value)  
A purpose-built React panel, not the generic FormView. It handles all User-specific complexities in a clean UI. This is what regular admins use.

**Path B — Generic FormView works for User** (secondary, for developers)  
Fix the generic form renderer so a User form opened via `FormView` (e.g. from the DocType list or command palette) is at minimum functional. This requires the `Password` field type fix.

### 18.4 Backend: What Already Works

| Capability | Status |
|---|---|
| `tabRole`, `tabUserType` schema | ✅ Tier-0, compiled in Rust |
| Seed records (System Manager, System User, etc.) | ✅ `SEED_RECORDS` in `bootstrap.rs` |
| Password hashing (`hash_password`, `set_user_password`) | ✅ `crates/spotledger-db/src/auth.rs` |
| `verify_password` (pbkdf2-sha256 + argon2) | ✅ same file |
| `GET /api/resource/User` (list) | ✅ generic resource handler |
| `GET /api/resource/User/<name>` (single) | ✅ generic resource handler |
| Email validation on User save (`validate_user`) | ✅ `doctypes/user.rs` |

### 18.5 Backend: What Needs to Be Added

Status update (April 18, 2026): the password interceptor is now implemented in the compiled Tier-0 save/controller path for `User`, with the runtime proxy path aligned to the same behavior.

#### A. Password interceptor in save path

When `POST /api/resource/User` or `PUT /api/resource/User/<name>` receives a payload containing `new_password` or `password`:

1. Extract and remove the password field from the document before it reaches `save_proxy`
2. Validate: minimum length (8 chars), non-empty
3. Call `set_user_password(db, user_name, password)` which hashes and writes to `__Auth`
4. Never write a plaintext password to `tabUser`

**Where**: the compiled Tier-0 save path for `User` must intercept this before persistence. In practice this belongs in the shared Rust save/controller layer, not only in `save_proxy.rs`, because `User` is a compiled DocType and does not use the runtime proxy path.

```rust
// In save_proxy.rs — before calling upsert_doc
if doctype == "User" {
    if let Some(pw) = doc.fields.remove("new_password").or_else(|| doc.fields.remove("password")) {
        if let Some(pw_str) = pw.as_str() {
            if !pw_str.is_empty() {
                set_user_password(adapter, &doc_name, pw_str).await?;
            }
        }
    }
}
```

#### B. `full_name` computed field

`full_name = first_name + " " + last_name` must be computed before saving. Options:
- SurrealDB `DEFINE FIELD full_name ON tabUser VALUE string::concat($value.first_name, " ", $value.last_name)` — preferred (runs automatically on every write)
- OR Rust interceptor in the User save path (simpler short-term)

This is already present in bootstrap DDL.

#### C. `generate_keys` button endpoint

`POST /api/method/frappe.core.doctype.user.user.generate_keys` — generates API key + secret for the user. Low priority; add as a named method handler when needed.

#### D. Seed `Guest` role

`Guest` role seeding is already present in bootstrap.

### 18.6 Frontend: Password Field Type in FieldRenderer

Status update (April 18, 2026): implemented. `ControlPassword.tsx` now handles `Password` fields, and `FormMain` routes edits through `new_password` so the generic `User` form is functional without exposing stored hashes.

In `src/app/panels/FormView/controls/`, add a `ControlPassword.tsx`:

```tsx
// Never pre-populates the field value from the document (security: never show hashed value)
// Shows a masked input
// On change, writes to formState values under the key "new_password"
// Empty value on submit means "don't change password" (backend interceptor honours this)
```

In `FieldRenderer.tsx`, add a case for `fieldtype === "Password"` → `<ControlPassword />`.

This makes the generic User form usable (password change works, field doesn't show a hash).

### 18.7 Frontend: Dedicated Users Panel

Status update (April 18, 2026): implemented in `Spotledger-ui` as a dedicated `Users` panel with `UserList.tsx`, `UserForm.tsx`, and `RoleSelector.tsx`, opened from the sidebar Administration section.

Location: `src/app/panels/Users/`

```
src/app/panels/Users/
  index.ts
  UserList.tsx        ← list all users; columns: avatar, full_name, email, user_type, enabled badge
  UserForm.tsx        ← create/edit user; wraps the generic FormView or builds its own layout
  RoleSelector.tsx    ← multi-select role checkboxes (replaces raw HasRole child table)
  useUserForm.ts      ← form state hook; handles password field separately
```

#### UserList — matches Frappe's list view

- Fetches `GET /api/resource/User?fields=["name","full_name","email","user_type","enabled","user_image"]`
- Shows `Active` (green) / `Disabled` (grey) badge based on `enabled`
- "New User" button opens UserForm in create mode
- Click row → UserForm in edit mode

#### UserForm — simplified Frappe-style user form

Sections mirroring Frappe's User form:

1. **Basic Info** — `email`, `first_name`, `last_name`, `username`, `user_image`
2. **Roles** — `RoleSelector` component (checkbox list of all Roles, pre-checked based on `roles` child table)
3. **Change Password** — `new_password` input (never pre-filled)
4. **Settings** — `enabled`, `user_type`, `language`

On save:
- Build `roles` array as `[{ role: "System Manager" }, ...]` from checked boxes
- Include `new_password` only if the field was touched
- `POST /api/resource/User` (create) or `PUT /api/resource/User/<name>` (update)

#### RoleSelector — replaces Frappe's RoleEditor widget

```tsx
// Fetches all Roles via GET /api/resource/Role?fields=["name","desk_access"]
// Renders as a two-column checklist grouped by: "Desk Roles" (desk_access=1) / "Portal Roles"
// Selected roles are stored as HasRole child rows
```

This is a pure React component — no custom JS framework like Frappe's. Simpler, more maintainable.

### 18.8 Tier-0 DocType → tabDocType seeding (currently missing)

Tier-0 DocTypes (User, Role, HasRole, etc.) have their schema compiled in Rust, but their **tabDocType and tabDocField rows** must also exist in the DB so that:
- The Designer can look them up (it reads from `tabDocType`)
- `search_link?doctype=Role` works (it queries `tabRole` but needs the doctype meta to know the title field)
- Permission checks can find meta for these doctypes

`seed_framework_doctypes()` in `bootstrap.rs` handles this — it iterates `MetaEntry` inventory and upserts `tabDocType` + `tabDocField` rows. **Verify** this is being called at `new-site` and covers all Tier-0 types including User, Role, HasRole, UserPermission, UserType.

Currently in `new_site.rs`:
```rust
seed_framework_doctypes(&db)    // ← must cover User, Role, HasRole, UserPermission, UserType
```

If any are missing from the inventory (`inventory::submit!` not called), add them.

### 18.9 The Missing Tier-0 DocTypes vs spotledger-core App

The key distinction for the plan:

| DocType | Location | Reason |
|---|---|---|
| User, Role, HasRole, UserPermission | Tier-0 (Rust + `inventory::submit!`) | Auth cannot be dynamic |
| UserType | Tier-0 (Rust + seeded in `SEED_RECORDS`) | Required before any user can log in |
| RoleProfile, ModuleProfile | **spotledger-core app** (DB-native JSON) | Pure configuration, no auth coupling |
| UserPermission (standalone, not child table) | Tier-0 | Core permission matrix |

`RoleProfile` and `ModuleProfile` are Frappe doctypes that allow grouping roles for bulk assignment — they are configuration helpers, not core auth. They belong in `apps/spotledger-core/`, not in Rust.

### 18.10 Implementation Order (Phase 1.5 — before full spotledger-core)

These can be done immediately, before the main Rust crate migration:

| Task | File | Effort |
|---|---|---|
| Password interceptor in compiled User save path | `controller.rs` / save layer | Done |
| `full_name` computed DEFINE FIELD in bootstrap DDL | `bootstrap.rs` | Done |
| `ControlPassword.tsx` component | `Spotledger-ui` | Done |
| `UserList.tsx` + `UserForm.tsx` + `RoleSelector.tsx` | `Spotledger-ui` | Done |
| Verify `seed_framework_doctypes` covers all Tier-0 types | `bootstrap.rs` | Done (inventory coverage tests added) |
| Seed `Guest` role in `SEED_RECORDS` | `bootstrap.rs` | Done |

**Verifiable milestone**: Admin can log in → navigate to Users panel → create a new user with roles → new user can log in. This flow is now covered by an HTTP-level integration test that verifies password hashing into `__Auth`, no plaintext in `tabUser`, embedded roles persistence, and successful login with the new credentials.

---

## 19. Questions Explicitly Deferred

1. **Frontend modification without touching core** — separate document, separate discussion.
2. **App store UI / registry / signing** — deferred until Phase 1-3 are done.
3. **Multi-site app sharing** — the `.slpkg` + `install_app` mechanism handles this; the registry layer is deferred.
4. **AI-driven app authoring** — will work naturally once the Apps panel and Designer are fully wired. The AI generates DocTypes + SurrealQL in-platform; the human reviews and exports.
5. **ERPNext migration** — currently ERPNext is a WASM app with 489 DocType JSONs. Migrating it to DB-native is feasible but deferred (it has no WASM logic yet — it's just JSON + future SurrealQL).

---

## 20. In-Platform App Authoring — Detailed Design

### 20.1 The Gap This Closes

The current plan mentions `new-app` and `export-app` CLI commands as if they are the entry point. They are not. The entry point is the **Apps panel** inside the running platform. CLI tools are an export/CI path for users who want filesystem artifacts and git workflows. First-time users, non-developers, and AI agents never need a terminal.

### 20.2 Apps Panel — Full Specification

**Location**: `src/app/panels/Apps/` — new panel, opened from sidebar under Administration.

**Sidebar entry**: Same Administration section as Users, alongside "DocType Designer".

#### List view (`AppList.tsx`)

| Column | Notes |
|---|---|
| App name (link) | kebab-case identifier |
| Title | display name |
| Version | semver badge |
| Status | `bundled` (came with server), `installed` (from .slpkg), `development` (created in-platform) |
| DocTypes | count |
| [Export] button | downloads .slpkg |
| [⋮] menu | Update manifest / Uninstall (non-bundled only) |

"**+ New App**" button opens the create form.

#### Create form (`AppForm.tsx`)

```
App ID *            [ my-payroll           ] (kebab-case, validated)
Title  *            [ My Payroll App       ]
Version             [ 1.0.0               ]
Description         [ ...                  ]
Author              [ ...                  ]
License             [ MIT / AGPL-3.0 / ... ]
Depends on          [ spotledger-core  ×  ] (multi-select from installed apps)
Initial modules     [ Payroll          ×  ] (comma-separated or chip input)
```

On **Create**:
1. POST `spotledger.apps.create` with the form data
2. Backend: insert `installed_app:<app-id>` with `status = "development"` + `manifest` JSON
3. Backend: create `Module Def` rows for each declared module (same as `install_app` does for JSON apps)
4. Frontend: navigate to the App detail view
5. The new app **immediately appears** in the Designer's App dropdown

#### Detail view (`AppDetail.tsx`)

Tabs:
- **Overview** — manifest fields (editable inline), depends_on badges
- **DocTypes** — list of `tabDocType WHERE app = <name>`, each linking to Designer
- **Functions** — list of `fn_source` entries for this app's doctypes
- **Export / Publish** — version bump + [Download .slpkg] + future publish-to-registry button

### 20.3 Backend: `spotledger.apps.*` Method Handlers

New file: `crates/spotledger-http/src/methods/apps.rs`

```rust
pub async fn handle_list_apps(site, params) -> Result<Value, SpotError>
// SELECT * FROM installed_app ORDER BY name ASC

pub async fn handle_create_app(site, params) -> Result<Value, SpotError>
// Validates: name is kebab-case, not already in installed_app, depends_on all installed
// Inserts installed_app:<name> SET manifest = {...}, status = "development", installed_at = time::now()
// Creates Module Def rows via existing seed_module_defs helper
// Returns: { ok: true, app: <name> }

pub async fn handle_get_app(site, params) -> Result<Value, SpotError>
// Returns manifest + SELECT count() FROM tabDocType WHERE app = $name GROUP ALL
//                   + SELECT count() FROM fn_source WHERE doctype IN (SELECT name FROM tabDocType WHERE app = $name) GROUP ALL

pub async fn handle_update_app(site, params) -> Result<Value, SpotError>
// UPDATE installed_app:<name> SET manifest.title = ..., manifest.version = ..., etc.

pub async fn handle_export_app(site, params) -> Result<Value, SpotError>
// Builds .slpkg in memory (same logic as CLI export-app)
// Returns base64-encoded bytes + filename, or streams directly
```

Routes wired in `routes.rs` under `spotledger.apps.*`.

### 20.4 Authoring Loop (Complete, Zero CLI)

```
1. Sidebar → Apps → [+ New App]
   → fills in name, title, module(s)
   → app created in DB immediately

2. Sidebar → Designer → [New DocType]
   → picks App = "my-payroll" from dropdown  ← populated from installed_app
   → picks Module = "Payroll"
   → designs fields, sets autoname, permissions
   → Saves → tabDocType row written with app = "my-payroll"

3. Designer → Pipeline tab
   → writes validate/before_save SurrealQL
   → Saves → fn_source row written, wired to pipeline

4. Sidebar → Apps → my-payroll → Export tab
   → [Download .slpkg]  ← server assembles archive in memory, browser downloads

5. On another site / for production:
   → Sidebar → Apps → [Install from file]
   → uploads payroll-1.0.0.slpkg
   → backend runs install_app() with in-memory archive
   → all DocTypes, functions, fixtures appear immediately
```

### 20.5 What Makes This Different from Frappe

Frappe requires:
- Python environment locally
- `bench get-app` / `bench install-app` from terminal
- `bench migrate` for schema changes
- File system access to write `.json` files

Spotledger requires:
- A browser tab

The filesystem is **optional** (for git/CI users who want it), never mandatory.

### 20.6 Implementation Order for Phase 2

Priority order within Phase 2:

1. `methods/apps.rs` — `list`, `create`, `get` (read/create path first, no export yet)
2. `AppList.tsx` + `AppForm.tsx` — create and list apps in-platform
3. Sidebar entry for Apps panel
4. `handle_export_app` — in-memory `.slpkg` assembly + download
5. `AppDetail.tsx` with DocTypes/Functions tabs and Export tab
6. "Install from file" upload handler in Apps panel (calls `install_app` with uploaded bytes)
