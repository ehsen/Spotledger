# DB-Native App Platform — Architecture & Implementation Plan

**Date**: April 18, 2026  
**Status**: Planning  
**Scope**: Introduces a second app type (DB-native), migrates non-Tier-0 framework DocTypes out of Rust, establishes Spotledger-Core, and lays the foundation for the app store.

---

## 1. Vision in One Sentence

Spotledger becomes a **general-purpose enterprise app development platform** where most apps are pure SurrealDB — no Rust compilation, no WASM, no deploys — and WASM is reserved for heavy lifting only.

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
| Who authors | Rust dev | AI / SurrealQL dev / anyone |

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

## 8. New CLI Commands

### App scaffolding
```
spotledger new-app <name>                           # scaffold app.json + directory
spotledger new-module <module> --app <app>          # add module directory
spotledger new-doctype <doctype> --app <app> --module <module>  # scaffold JSON
```

### Package & distribute
```
spotledger pack-app <app-dir>                       # → <name>-<version>.slpkg
spotledger install-app <app-dir-or-slpkg> <site>    # existing command, extended
spotledger upgrade-app <app-name> <new-slpkg> <site>
spotledger uninstall-app <app-name> <site>          # 3-stage safe uninstall
```

### Export from Designer
```
spotledger export-app <app-name> <site>             # dump DB state → app directory
```

`export-app` is the key command: it reads `tabDocType`, `tabDocField`, `fn_source`, `pipeline_node` from the DB and writes them back to the filesystem in the canonical app format. This closes the loop — design in DB, export to files, version control, share.

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

Two new designer capabilities are needed:

### 10.1 App/Module context in Designer

When creating a DocType, the user picks:
- **App** — which installed DB-native app owns this DocType (or `__custom__` for site-local types)
- **Module** — which module within that app

The `introduced_by` field on `tabDocField` records captures this. Already defined in architecture.

### 10.2 Export from Designer

A new button/command: **"Export App"** — calls `spotledger.designer.export_app`:
- Reads all DocTypes where `app = <selected_app>` from `tabDocType`
- Reads their fields, permissions, SurrealQL functions
- Returns a zip stream or writes to a file path

This is the "design in DB, ship as files" loop.

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
11. Remove hard-coded `seed_framework_modules()` call from `server.rs` startup — those modules now come from the spotledger-core install
    - **Exception**: keep seeding `Core` module (Tier-0, owned by Rust, not by spotledger-core)

### Phase 4 — `new-app` CLI command

12. Implement `spotledger new-app <name>` — creates directory with `app.json` template
13. Implement `spotledger new-doctype` — scaffolds a minimal JSON from a template
14. Implement `spotledger export-app <app-name> <site>` — DB → filesystem export

### Phase 5 — Designer "App" picker

15. Add `app` field to `tabDocType` (already in `installed_app` table; link to it)
16. Designer `save` endpoint: write `app` to `tabDocType` record
17. Designer UI: dropdown showing installed DB-native apps + `__custom__`

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

To make the vision concrete:

```bash
# 1. Scaffold
spotledger new-app payroll
# Creates apps/payroll/app.json, apps/payroll/modules/

# 2. Add module
spotledger new-module "Payroll" --app payroll

# 3. Add DocTypes (in Designer OR via JSON files)
spotledger new-doctype "Employee" --app payroll --module Payroll
spotledger new-doctype "Salary Slip" --app payroll --module Payroll
spotledger new-doctype "Salary Component" --app payroll --module Payroll

# 4. Install on dev site (reads from apps/payroll/)
spotledger install-app payroll my-dev-site --bench .

# 5. Open Designer → design fields, write SurrealQL logic
# All goes to DB. Changes tracked in schema_change_log.

# 6. Export changes back to files (for git commit)
spotledger export-app payroll my-dev-site --bench .
git add apps/payroll/ && git commit -m "feat: add Employee leave_balance field"

# 7. Package for release
spotledger pack-app apps/payroll/
# → payroll-1.0.0.slpkg

# 8. Install on production
spotledger install-app payroll-1.0.0.slpkg prod-site --bench .
```

The app has a dependency:
```json
{
  "name": "payroll",
  "depends_on": ["spotledger-core", "spotledger-entities"]
}
```

`spotledger-entities` is a future app providing Item, Customer, Supplier, Company as a shared base — the "Core Entity App" mentioned in the vision.

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

### Phase 2 — `new-app` / `export-app` CLI (MEDIUM — enables developer workflow)

| Task | File(s) affected |
|---|---|
| `spotledger new-app` command | `cli.rs`, new `new_app.rs` |
| `spotledger export-app` command | new `export_app.rs` |
| `spotledger pack-app` (.slpkg format) | new `pack_app.rs` |
| `install_app` accepts `.slpkg` files | `install_app.rs` |

### Phase 3 — Designer "App" context (LOW — polish, not blocking)

| Task |
|---|
| `app` field on `tabDocType` |
| Designer save writes `app` + `module` |
| Designer UI: app/module picker dropdown |
| "Export App" button in Designer |

---

## 16. Invariants (Non-Negotiables)

1. **Tier-0 types are never DB-native**. DocType, DocField, DocPerm, User, Role, UserPermission, Site, ModuleDef must exist in Rust and are bootstrapped before any DB read. The `TIER_0_DOCTYPES` guard in `designer.rs` stays forever.

2. **DB-native apps do not change the save pipeline**. `save_proxy.rs` does not care whether a doctype came from a WASM app or a DB-native app. The pipeline is doctype-agnostic.

3. **install_app is always the right path**. There is no second way to "load" an app. DB-native apps go through `install_app` exactly like WASM apps (minus the WASM loading step).

4. **Doctype JSON format is unchanged**. The Frappe-compatible JSON format that `seed_doctypes.rs` already parses is the canonical format. No new format. DB-native apps use the exact same JSON.

5. **schema_change_log records all diffs**. Every install/upgrade writes change records. This is the audit trail and the rollback mechanism.

6. **WASM apps are additive**. A WASM app can still carry DocType JSONs alongside its `.wasm`. That path is untouched.

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

#### A. Password interceptor in save path

When `POST /api/resource/User` or `PUT /api/resource/User/<name>` receives a payload containing `new_password` or `password`:

1. Extract and remove the password field from the document before it reaches `save_proxy`
2. Validate: minimum length (8 chars), non-empty
3. Call `set_user_password(db, user_name, password)` which hashes and writes to `__Auth`
4. Never write a plaintext password to `tabUser`

**Where**: `save_proxy.rs` — add a pre-write interceptor for doctype `"User"`. This is the only doctype-specific logic that Rust needs to own (because password storage is a security boundary that cannot move to SurrealDB events).

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

Add the `DEFINE FIELD` to `SEED_RECORDS` / bootstrap DDL so it runs at startup.

#### C. `generate_keys` button endpoint

`POST /api/method/frappe.core.doctype.user.user.generate_keys` — generates API key + secret for the user. Low priority; add as a named method handler when needed.

#### D. Seed `Guest` role

`bootstrap.rs` seeds Administrator, System Manager, and All. Add `Guest` role (used by anonymous/unauthenticated requests in Frappe).

### 18.6 Frontend: Password Field Type in FieldRenderer

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
| Password interceptor in save path | `save_proxy.rs` | Small |
| `full_name` computed DEFINE FIELD in bootstrap DDL | `bootstrap.rs` | Tiny |
| `ControlPassword.tsx` component | `Spotledger-ui` | Small |
| `UserList.tsx` + `UserForm.tsx` + `RoleSelector.tsx` | `Spotledger-ui` | Medium |
| Verify `seed_framework_doctypes` covers all Tier-0 types | `bootstrap.rs` | Tiny |
| Seed `Guest` role in `SEED_RECORDS` | `bootstrap.rs` | Tiny |

**Verifiable milestone**: Admin can log in → navigate to Users panel → create a new user with roles → new user can log in.

---

## 17. Questions Explicitly Deferred

1. **Frontend modification without touching core** — separate document, separate discussion.
2. **App store UI / registry / signing** — deferred until Phase 1-3 are done.
3. **Multi-site app sharing** — the `.slpkg` + `install_app` mechanism handles this; the registry layer is deferred.
4. **AI-driven app authoring** — will work naturally once `new-doctype` CLI and Designer export exist. The AI generates JSON + SurrealQL files; the human reviews and commits.
5. **ERPNext migration** — currently ERPNext is a WASM app with 489 DocType JSONs. Migrating it to DB-native is feasible but deferred (it has no WASM logic yet — it's just JSON + future SurrealQL).
