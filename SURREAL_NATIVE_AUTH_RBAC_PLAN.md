# SurrealDB-Native Auth & Graph RBAC — Migration Plan

**Date**: April 18, 2026  
**Status**: Planning  
**Scope**: Move authentication and all RBAC enforcement into SurrealDB. Remove duplicate Rust permission logic. Move workflow state machine into SurrealQL graph functions.

---

## 1. Current State

### Auth pipeline (today)
```
Request → sid cookie → get_session(db, sid) → tabSessions lookup → CurrentUser string
Login   → lookup_user() → get_password_hash() → verify_password() [Rust crypto] → create_session()
```

### Permission pipeline (today)
```
has_permission() → fn::permissions::has() [SurrealQL] ─► returns bool
                 → has_permission_rust()  [Rust fallback] ─► same logic duplicated
get_user_roles() [Rust] → reads tabUser.roles array from DB
build_permission_lists [Rust] → called at boot info time, 5 separate DB calls
```

### Workflow (today)
```
handle_workflow_get_transitions() [Rust] → 4 sequential get_list() DB calls
handle_workflow_apply()           [Rust] → 5 sequential get_list()/upsert_doc() calls
```

### What already exists in SurrealQL (06_permissions.surql)
- `fn::permissions::get_roles($user)` — full role traversal  
- `fn::permissions::has($user, $doctype, $ptype)` — full RBAC check  
- `fn::permissions::get_all($user, $doctype)` — full permission map in one call  

These already encode the complete Frappe RBAC model correctly as graph traversals.

---

## 2. Target State

### Auth pipeline (target)
```
Request → JWT cookie → verify (pure crypto, zero DB) → $auth populated → user known
Login   → SurrealDB SIGNIN [native record access] → DB issues JWT → set cookie
```

### Permission pipeline (target)
```
HTTP route handler → single fn::permissions::has() call [SurrealQL] → allow/deny
DEFINE TABLE PERMISSIONS → engine enforces per-query [defense in depth layer]
```

### Workflow (target)
```
handle_workflow_get_transitions() [Rust] → single fn::workflow::get_transitions() SurrealQL call
handle_workflow_apply()           [Rust] → single fn::workflow::apply() SurrealQL call
                                          (role check + write + docstatus update atomic)
```

### What Rust retains permanently
| Responsibility | Why |
|---|---|
| JWT signature verification | Crypto — must be zero DB round-trip per request |
| `new_password` interceptor in save path | Security boundary — plaintext never reaches DB |
| `print` / `email` / `report` HTTP handlers | Not DB verbs — produce output (PDF, email send) |
| WASM plugin host | Compute boundary |
| HTTP routing + request parsing | Network layer |

---

## 3. What Does NOT Change

- `tabDocPerm` data model — admins still change permissions in the UI with no DDL required
- `fn::permissions::has()` logic — already correct, not rewritten
- `tabSessions` — kept throughout all phases, deprecated only in a future cleanup
- `save_proxy.rs`, `naming.rs`, pipeline — untouched
- All existing HTTP routes and response shapes
- All existing tests must pass at every phase boundary

---

## 4. Migration Phases

---

### Phase 0 — Additive SurrealQL functions (zero Rust changes)

**Goal**: All new SurrealQL logic exists in the DB and is verified correct before Rust switches to use it.

#### 0.1 — Workflow graph functions  
Add to `06_permissions.surql` (or a new `07_workflow.surql`):

**`fn::workflow::get_transitions($user, $doctype, $current_state)`**  
- Traverse: tabWorkflow → tabWorkflowTransition filtered by state + allowed role  
- Gate: user's roles must include the `allowed` role on the transition  
- Returns: array of `{ action, next_state, allow_self_approval }` objects  
- Single DB call, replaces 4 Rust `get_list()` calls  

**`fn::workflow::apply($user, $doctype, $doc_id, $action)`**  
- Verify: a valid transition exists for this user + action from current state  
- THROW if not: `"User X cannot apply action 'Approve' from state 'Draft'"`  
- Write: UPDATE document setting `workflow_state_field` + `docstatus` if the target state has a `doc_status` value  
- Return: the updated document  
- Atomic: the role check and write are in the same DB transaction  

**`fn::workflow::get_boot_transitions($user)`** (optional, for boot info)  
- Returns all (doctype, action) pairs the user can currently perform across all documents  

#### 0.2 — UserPermission row-level function  
Add to `06_permissions.surql`:

**`fn::permissions::check_user_perms($user, $doctype, $record)`**  
- Reads `tabUser_Permission` for this user  
- If no rows: return true (no restrictions)  
- If rows exist: return true only if `$record[allow_field] = for_value` for each restriction  
- Designed to be embedded in `DEFINE TABLE PERMISSIONS FOR select WHERE ...` clauses  

#### 0.3 — Boot permissions function  
Add to `06_permissions.surql`:

**`fn::permissions::get_boot_permissions($user)`**  
- Replaces `build_permission_lists()` in Rust  
- Returns `{ can_read: [...], can_write: [...], can_create: [...], can_delete: [...], can_submit: [...], can_cancel: [...] }`  
- Single DB call, traverses `tabDocType` + `tabHas_Role` + `tabDocPerm` once  
- Administrator: returns all doctypes in all lists  

#### 0.4 — Verify via direct SurrealDB queries  
Before any Rust change, verify all new functions return correct results using direct SurrealDB queries against the running dev site.

**Verifiable milestone**: All new `fn::` functions callable and returning expected results. Zero Rust changes. All existing tests still pass.

---

### Phase 1 — Workflow migrates to SurrealQL (thin Rust wrappers)

**Goal**: `handle_workflow_get_transitions` and `handle_workflow_apply` become single-call wrappers. Behavior is identical.

#### Changes in `crates/spotledger-http/src/methods/desk/model.rs`

**`handle_workflow_get_transitions`**: Replace the 4-call Rust sequence with:
```rust
surql_one("RETURN fn::workflow::get_transitions($user, $doctype, $state)", params)
```

**`handle_workflow_apply`**: Replace the 5-call Rust sequence with:
```rust
surql_one("RETURN fn::workflow::apply($user, $doctype, $id, $action)", params)
```

The response shapes are identical — the SurrealQL functions are designed to match what the Rust code currently returns.

#### Workflow transition `condition` field

Currently the `condition` field on `tabWorkflowTransition` is ignored in Rust. This does not change in Phase 1 — same behavior preserved. The long-term path (deferred, not in this plan) is: `condition` becomes a reference to a named `fn::` function in `fn_source`, evaluated by `fn::workflow::apply()` before the state change.

**Verifiable milestone**: All workflow API calls (`get_transitions`, `apply_workflow`) produce identical responses. Rust `model.rs` workflow functions are now each a single DB call.

---

### Phase 2 — Native SurrealDB Authentication (dual-mode, non-breaking)

**Goal**: SurrealDB handles authentication and issues JWTs. Existing `sid` sessions continue to work in parallel throughout this phase.

#### 2.1 — Add `DEFINE ACCESS` in bootstrap DDL

In `crates/spotledger-db/src/bootstrap.rs` (or the schema init file), add to the startup DDL:

```surql
DEFINE ACCESS user ON DATABASE TYPE RECORD
  SIGNIN (
    SELECT * FROM tabUser
    WHERE (name = $identifier OR email = $identifier)
      AND enabled = true
      AND crypto::argon2::compare(
            (SELECT VALUE password FROM __Auth
             WHERE doctype = 'User' AND name = $identifier
               AND fieldname = 'password' LIMIT 1)[0],
            $password
          )
  )
  WITH JWT ALGORITHM HS512 KEY $env("SURREAL_JWT_SECRET")
  DURATION FOR SESSION 12h;
```

This is **additive** — it defines a new access method but changes nothing about existing `tabSessions` logic.

#### 2.2 — `SURREAL_JWT_SECRET` environment variable

Add to site configuration (`site_config.json` or env): `SURREAL_JWT_SECRET` — a random 64-byte base64 string generated at `new-site` time. Add generation to `new_site.rs`.

#### 2.3 — Login handler: dual-path

Modify the login route handler in `crates/spotledger-http/src/methods/`:

1. Attempt SurrealDB `SIGNIN` with the submitted credentials  
2. If it succeeds: DB returns a JWT. Store it as `token` cookie (HttpOnly, SameSite=Lax). Continue to also create a `tabSessions` row and `sid` cookie as today.  
3. If `DEFINE ACCESS` is not yet live or SIGNIN fails for a non-auth reason: fall back to the current Rust `lookup_user + get_password_hash + verify_password + create_session` path  

This dual-path ensures zero regression during rollout.

#### 2.4 — `resolve_current_user` in middleware: token-first, sid-fallback

Modify `crates/spotledger-http/src/middleware.rs`:

```
1. Check for `token` cookie
2. If present: verify JWT signature locally (pure crypto, zero DB call)
3. Extract `id` claim from JWT → user name
4. If token absent or invalid: fall back to existing `sid` → get_session() path
5. Default: "Guest"
```

JWT verification is a local crypto operation — `jsonwebtoken` crate, single call, no DB. The `sid` path is fully preserved as fallback.

#### 2.5 — `set_user_password` stays in Rust

`set_user_password(adapter, user, plaintext)` continues to hash and write to `__Auth`. The `DEFINE ACCESS SIGNIN` clause reads from `__Auth` directly. No change needed to password storage format — `crypto::argon2::compare` in SurrealDB 2.x accepts the same passlib argon2id format already used.

**Verifiable milestone**:  
- New login: receives both `token` (JWT) and `sid` cookies  
- Both cookies independently authenticate the user  
- Existing users with `sid` cookies continue to work without re-login  
- `SURREAL_TEST_URL` integration test: SIGNIN via SurrealDB SDK returns valid JWT for a seeded test user

---

### Phase 3 — DEFINE TABLE PERMISSIONS (defense in depth layer)

**Goal**: SurrealDB's query engine enforces `fn::permissions::has()` at the table level. No DB query can return rows the user has no `read` permission on, even if Rust code has a bug.

#### 3.1 — Standard permission template

Every installed non-Tier-0 doctype gets:

```surql
DEFINE TABLE tab<DoctypeName> PERMISSIONS
  FOR select WHERE $auth = NONE
    OR fn::permissions::has(<string>$auth.id, '<DoctypeName>', 'read')
  FOR create WHERE $auth != NONE
    AND fn::permissions::has(<string>$auth.id, '<DoctypeName>', 'create')
  FOR update WHERE $auth != NONE
    AND fn::permissions::has(<string>$auth.id, '<DoctypeName>', 'write')
  FOR delete WHERE $auth != NONE
    AND fn::permissions::has(<string>$auth.id, '<DoctypeName>', 'delete');
```

The `$auth = NONE` guard on `select` allows the Rust backend (which connects with root credentials, not record access) to continue reading all data. `$auth` is only populated for record-access connections (JWT sessions). This is the critical design point: **Rust's DB connection uses root credentials and is unaffected by DEFINE TABLE PERMISSIONS**. The enforcement layer applies only to direct/future record-access connections.

#### 3.2 — Where the DDL is applied

- `install_app` generates and executes the `DEFINE TABLE PERMISSIONS` statement for each doctype it installs, using the template above  
- The `tabDoctypeName` must already exist (schema seeded first, then permissions applied)  
- Idempotent: `DEFINE TABLE OVERWRITE` — safe to re-run on upgrade  
- Tier-0 doctypes (DocType, DocField, User, Role, etc.) get a hardcoded permissions block in bootstrap DDL. For Tier-0, the rule is: only System Manager and Administrator can select/create/update/delete.

#### 3.3 — `fn::permissions::check_user_perms` integration (optional, Phase 3b)

For organizations that configure `tabUser_Permission` row-level restrictions, `DEFINE TABLE PERMISSIONS` can be extended:

```surql
FOR select WHERE ($auth = NONE OR fn::permissions::has($auth.id, 'Sales Order', 'read'))
  AND ($auth = NONE OR fn::permissions::check_user_perms($auth.id, 'Sales Order', this))
```

The `this` reference means this clause is evaluated per-row. Only activate for tables where `UserPermission` restrictions are actually configured. Start without this; add in Phase 3b when `check_user_perms` is verified correct.

**Verifiable milestone**:  
- Direct SurrealDB SDK connection authenticated as a non-System-Manager user with `read` permission on Sales Order but not on Purchase Order: `SELECT * FROM tabSalesOrder` returns data, `SELECT * FROM tabPurchaseOrder` returns empty array  
- Rust server requests (root credentials) unaffected  
- All existing API tests pass

---

### Phase 4 — Remove Rust permission redundancy

**Goal**: Delete the Rust logic that duplicates what SurrealQL already does.

Prerequisites: Phase 1 + 2 + 3 stable and verified.

#### 4.1 — `build_permission_lists` → `fn::permissions::get_boot_permissions()`

In `crates/spotledger-http/src/methods/desk/mod.rs`:

- Replace the call to `build_permission_lists(db, user, &roles)` with a single call to `fn::permissions::get_boot_permissions($user)`  
- The function (added in Phase 0.3) returns the same `{ can_read, can_write, ... }` shape  
- The local `build_permission_lists` function and `get_user_roles` helper are deleted  

#### 4.2 — `has_permission_rust` fallback removed

In `crates/spotledger-db/src/permissions.rs`:

- `has_permission()` currently: try SurrealQL → fall back to Rust  
- After this phase: SurrealQL only. Remove `has_permission_rust()` entirely  
- `get_user_roles()` in `permissions.rs` stays (used in tests) but is no longer in the hot path  

#### 4.3 — `get_doc_permissions` simplified

`get_doc_permissions()` currently: try `fn::permissions::get_all()` → fall back to Rust. Remove the Rust fallback. If `fn::permissions::get_all()` is absent, return an error (it will always be present after Phase 0).

#### 4.4 — What stays in `permissions.rs`

Keep the typed structs `PermissionType`, `DocPermission`, `Role`, `IfOwner`, `UserPermission`, `Share` — they are Rust API surface used by tests and WASM plugins. Only the DB query functions lose their Rust fallbacks.

**Verifiable milestone**:  
- `cargo test -p spotledger-db --lib` — all unit tests pass  
- All integration tests pass  
- Server behavior identical to pre-Phase-4

---

### Phase 5 — Boot info uses `$auth`, `sid` sessions deprecated (future)

This phase is explicitly deferred. It requires validating Phase 1–4 in production first.

When done:
- Login no longer creates `tabSessions` rows  
- `resolve_current_user` only checks JWT, `sid` path removed  
- `tabSessions` table kept but empty (not dropped — could be re-used later)  
- `get_session()` / `create_session()` functions in `auth.rs` marked deprecated  

---

## 5. Files Affected Per Phase

| Phase | Files Changed |
|---|---|
| 0 | `apps/erpnext/erpnext/surql/framework/06_permissions.surql` (new functions added), optionally new `07_workflow.surql` |
| 1 | `crates/spotledger-http/src/methods/desk/model.rs` (workflow handlers) |
| 2 | `crates/spotledger-db/src/bootstrap.rs` (DEFINE ACCESS DDL), `crates/spotledger-http/src/middleware.rs` (token-first), login route handler, `new_site.rs` (JWT secret gen) |
| 3 | `install_app.rs` (DEFINE TABLE PERMISSIONS generation), `crates/spotledger-db/src/bootstrap.rs` (Tier-0 tables) |
| 4 | `crates/spotledger-http/src/methods/desk/mod.rs` (remove `build_permission_lists`, `get_user_roles`), `crates/spotledger-db/src/permissions.rs` (remove Rust fallbacks) |
| 5 | `crates/spotledger-http/src/middleware.rs`, `crates/spotledger-db/src/auth.rs` (deferred) |

---

## 6. Non-Goals / Explicitly Deferred

- **`condition` field on workflow transitions**: Currently ignored in Rust and will remain ignored through all phases. The long-term path is: `condition` becomes a reference to a named `fn::` function. Addressed when workflow authoring in the Designer is built.
- **Removing `tabSessions`**: Deferred to Phase 5.
- **Row-level UserPermission in DEFINE TABLE** (Phase 3b): Deferred until `fn::permissions::check_user_perms()` is proven correct under load.
- **Frontend changes**: Out of scope. Auth is cookie-based — frontend behavior is unchanged.
- **Multi-tenant or multi-DB auth scoping**: Out of scope per the DB_NATIVE_APP_PLATFORM.md invariant.

---

## 7. Invariants Across All Phases

1. **Rust DB connection uses root credentials**. `DEFINE TABLE PERMISSIONS` only restricts record-access (JWT) connections. The Rust backend is never subject to table-level permission filtering — it enforces permissions at the application layer before issuing queries.

2. **`tabDocPerm` remains the permission source of truth**. `fn::permissions::has()` reads it live. No DDL change is needed when an admin modifies permissions.

3. **`set_user_password` stays in Rust**. Plaintext passwords never reach the DB in any form other than the hash.

4. **All phases are independently deployable**. Each phase leaves the system in a fully functional state. Phases do not need to be done together.

5. **No response shape changes**. HTTP API responses are byte-for-byte identical before and after each phase.
