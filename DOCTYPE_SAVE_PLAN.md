# DocType Save / Update — Implementation Plan

## Background

DocType is not a regular document. Saving a regular document writes one row to
one table. Saving a DocType orchestrates five distinct concerns atomically:

1. **Parent record** — upsert `tabDocType`
2. **Field records** — full-replace `tabDocField` (position-ordered by `idx`)
3. **Permission records** — full-replace `tabDocPerm` (currently missing entirely)
4. **DDL** — `DEFINE TABLE` (new) or `DEFINE FIELD`/`REMOVE FIELD` (update)
5. **Cache invalidation** — meta cache + boot cache so the UI sees changes immediately

Frappe's approach is the reference: it treats DocType as a special controller
that overrides the standard document save pipeline at `validate()` and
`on_update()`. We replicate the same _principle_, adapted for SurrealDB idioms.

---

## Current State (gaps vs. Frappe)

| Concern | Frappe | Our current `handle_save` | Gap |
|---|---|---|---|
| Save `tabDocType` | `INSERT`/`UPDATE` | `UPSERT type::record(...)` | ✅ correct |
| Save `tabDocField` | DELETE-then-INSERT with `idx` | DELETE + INSERT loop | `idx` not stamped consistently |
| Save `tabDocPerm` | DELETE-then-INSERT | **Not touched** | ❌ missing |
| Default perm on new DocType | No auto-default (editor provides) | No auto-default | ✅ same |
| DDL — new doctype | `CREATE TABLE` | `DEFINE TABLE IF NOT EXISTS` | `IF NOT EXISTS` hides whether it ran |
| DDL — updated doctype | `ALTER TABLE` (add/modify/drop) | `DEFINE FIELD IF NOT EXISTS` only | ❌ no modify, no remove |
| Field validation | 14 checks (name, type, reqd+default clash, etc.) | None | ❌ missing |
| Permission validation | Role exists, perm level range | None | ❌ missing |
| Autoname/naming series validation | validates series string format | None | ❌ missing |
| Atomicity | MySQL implicit via DDL commit | Sequential calls, no rollback | ❌ partial failure leaves orphan rows |
| Cache clear | `frappe.clear_cache(doctype)` + user cache | `meta_cache.invalidate()` only | ❌ incomplete |
| `is_new` detection | `frappe.db.exists()` | None | ❌ missing |
| Graph metadata (Phase 2) | N/A (Frappe has no graph layer) | `ensure_meta_records` called from `ensure_schema` only at compile time | ❌ not called at runtime save |

---

## Phases

### Phase 1 — Correct Save / Update Mechanism

**Goal**: a single `save_doctype()` function in `spotledger-db` that is the
canonical path for both the designer API and any future programmatic callers.
Comparable to Frappe's `DocType.on_update()` + `frappe.db.updatedb()`.

### Phase 2 — Field-Level DDL Evolution

**Goal**: handle field type changes and field removal correctly using
`DEFINE FIELD OVERWRITE` and `REMOVE FIELD`.

### Phase 3 — Validation Layer

**Goal**: a `validate_doctype()` function that mirrors Frappe's `validate()`
checks adapted to our type system and SurrealDB constraints.

### Phase 4 — Test Suite

**Goal**: comprehensive unit + integration tests covering every save path.

### Phase 5 — SurrealDB-Specific Metadata

**Goal**: keep the `doctype`/`docfield`/`has_field` graph in sync at runtime
save time, and integrate pipeline-function linkage into the DocType lifecycle.

---

## Phase 1 — Correct Save / Update Mechanism

### 1.1 Transaction support in `DbAdapter`

SurrealDB supports `BEGIN TRANSACTION` / `COMMIT TRANSACTION` / `CANCEL
TRANSACTION`. We need two new methods on `DbAdapter`:

```rust
/// Execute multiple SQL statements inside one SurrealDB transaction.
/// On any error, CANCEL TRANSACTION is issued before returning.
pub async fn transaction<F, Fut>(&self, f: F) -> Result<(), DbError>
where
    F: FnOnce(DbAdapter) -> Fut,
    Fut: Future<Output = Result<(), DbError>>;
```

Or more pragmatically (since SurrealDB's Rust driver can execute multi-statement
strings), a `begin() → execute_in_tx(sql_vec) → commit_or_cancel()` helper:

```rust
pub async fn run_transaction(&self, statements: Vec<TransactionStatement>) -> Result<(), DbError>;
```

**Important caveat**: SurrealDB DDL (`DEFINE TABLE`, `DEFINE FIELD`) is NOT
transactional in the same way MySQL DDL is — it does not cause an implicit
commit. This is actually better: we can wrap ALL five steps (data + DDL)
in one `BEGIN TRANSACTION … COMMIT TRANSACTION` block and they either all
succeed or all roll back.

### 1.2 `is_new_doctype()` — detect new vs. existing

```rust
async fn is_new_doctype(adapter: &DbAdapter, name: &str) -> Result<bool, DbError>
```

Queries `SELECT count() FROM tabDocType WHERE name = $name GROUP ALL`.
Returns `true` if count = 0. Used to gate `DEFINE TABLE` vs. field-diff DDL.

### 1.3 The `save_doctype()` function signature

Lives in a new file: `crates/spotledger-db/src/doctype_save.rs`

```rust
pub struct DoctypeSaveInput {
    pub doctype:  String,                  // The DocType name
    pub meta:     serde_json::Value,       // tabDocType scalar fields
    pub fields:   Vec<serde_json::Value>,  // ordered list of DocField objects
    pub perms:    Vec<serde_json::Value>,  // ordered list of DocPerm objects
}

pub async fn save_doctype(
    adapter: &DbAdapter,
    meta_cache: &MetaCache,
    input: DoctypeSaveInput,
) -> Result<serde_json::Value, DoctypeSaveError>
```

### 1.4 Sequence inside `save_doctype()`

Mirrors Frappe's insert/update paths:

```
1. validate_doctype_input(&input)           // Phase 3 — errors before any DB write
2. is_new  ←  is_new_doctype(adapter, &doctype)
3. stamp_idx on fields (0-based position)   // Stamp idx = position in array
4. stamp_idx on perms
5. BEGIN TRANSACTION
   ├─ 6.  UPSERT type::record("tabDocType", name) CONTENT meta
   ├─ 7.  DELETE FROM tabDocField WHERE parent = $dt
   ├─ 8.  For each field:
   │       INSERT INTO tabDocField CONTENT field   (with parent/parenttype/parentfield stamped)
   ├─ 9.  DELETE FROM tabDocPerm WHERE parent = $dt
   ├─ 10. For each perm:
   │       INSERT INTO tabDocPerm CONTENT perm     (with parent/parenttype/parentfield stamped)
   └─ COMMIT (or CANCEL on any error)
6. If is_new:
   define_table(adapter, &doctype)         // DEFINE TABLE IF NOT EXISTS `tabXxx` SCHEMAFULL
   define_system_fields(adapter, &table)   // name, doctype, owner, creation, modified, ...
7. define_or_update_fields(adapter, &table, &fields, is_new)
   // Phase 1: DEFINE FIELD IF NOT EXISTS  (new fields only — Phase 2 adds OVERWRITE)
8. meta_cache.invalidate(&doctype).await
9. Return the saved doctype record
```

**Why data + DDL are separated**: SurrealDB transactions cover DML but DDL
behaviour inside transactions may vary by version. We keep DML in the
transaction and DDL outside but after. If DDL fails, the data rows are
committed but the table/field definition is inconsistent — same risk as
Frappe's MySQL path. Phase 2 addresses recovery here.

### 1.5 `tabDocPerm` — default on new DocType

When `perms` is empty and `is_new`, inject one default permission row:
- `role`: `"System Manager"`
- `read`, `write`, `perm_create`, `perm_delete`: `1`
- `submit`, `amend`, `cancel`: `0` (unless `is_submittable` is set)

This matches the behaviour Frappe's UI pre-populates.

### 1.6 `idx` stamping

```rust
fn stamp_idx(rows: &mut Vec<Value>) {
    for (i, row) in rows.iter_mut().enumerate() {
        if let Some(obj) = row.as_object_mut() {
            obj.insert("idx".into(), Value::Number(i.into()));
        }
    }
}
```

`idx` = 0-based position in the array at save time. The UI sends fields in
display order; we trust that order and encode it in `idx`. On read, we always
`ORDER BY idx ASC`.

### 1.7 `handle_save` in `designer.rs` becomes a thin wrapper

```rust
pub async fn handle_save(...) -> Result<Value, SpotError> {
    let input = DoctypeSaveInput {
        doctype: require_string(&params, "doctype")?,
        meta:    params.get("meta").cloned().unwrap_or(json!({})),
        fields:  params.get("fields").and_then(Value::as_array).cloned().unwrap_or_default(),
        perms:   params.get("perms").and_then(Value::as_array).cloned().unwrap_or_default(),
    };
    // Tier-0 guard stays here
    if is_tier_0(&input.doctype) { return Err(...); }

    let result = save_doctype(&site.db, &site.meta_cache, input).await
        .map_err(|e| SpotError::Db(e.to_string()))?;

    Ok(json!({ "ok": true, "doctype": result }))
}
```

---

## Phase 2 — Field-Level DDL Evolution

**Goal**: when saving an *existing* DocType, bring the SurrealDB schema into
sync with the new field definitions.

### 2.1 Field diff

```rust
struct FieldDiff {
    added:    Vec<FieldSpec>,   // in new, not in old
    removed:  Vec<String>,      // fieldnames in old, not in new
    changed:  Vec<FieldSpec>,   // in both, but type/default/options changed
    unchanged: Vec<FieldSpec>,
}

fn diff_fields(old: &[Value], new_fields: &[Value]) -> FieldDiff
```

`old` = fields loaded from `tabDocField` before save.
`new_fields` = fields from the save input.

### 2.2 DDL actions per diff bucket

| Bucket | SurrealDB DDL |
|---|---|
| `added` | `DEFINE FIELD IF NOT EXISTS \`f\` ON TABLE \`t\` TYPE ...` |
| `changed` (type/default) | `DEFINE FIELD OVERWRITE \`f\` ON TABLE \`t\` TYPE ...` |
| `removed` | `REMOVE FIELD \`f\` ON TABLE \`t\`; -- only if safe` |

**Removal safety rule** (mirrors Frappe's "additive only" default):
- A field is only `REMOVE FIELD`'d if it carries `"removable": true` in its
  `tabDocField` record AND there is no data in the column for any existing row.
- Otherwise it is orphaned (ignored on read) and logged as a warning.
- This avoids accidental data loss on field rename or drag-reorder.

### 2.3 `DEFINE FIELD OVERWRITE` — when is it safe?

Safe to OVERWRITE:
- `label`, `description`, `hidden`, `reqd`, `bold`, `in_list_view`, `in_standard_filter` — metadata only
- `default_value` changes
- `options` on Link/Select fields (list of choices)

Requires user confirmation (error by default, unless `force: true`):
- `fieldtype` change (e.g. Data → Int)
- `not_nullable` newly set (existing NULL values would violate the constraint)

### 2.4 Index reconciliation

After field diff:
- Drop indexes for removed/unique-changed fields: `REMOVE INDEX IF EXISTS`
- Add indexes for newly-unique or newly-in_standard_filter fields

---

## Phase 3 — Validation Layer

New file: `crates/spotledger-db/src/doctype_validate.rs`

### 3.1 DocType-level checks

| # | Check | Error condition |
|---|---|---|
| V1 | Name not empty | `name.trim().is_empty()` |
| V2 | Name ≤ 61 chars | `name.len() > 61` |
| V3 | Name is valid identifier | Regex `^[A-Za-z][A-Za-z0-9 _-]*$` |
| V4 | Name not a SurrealDB reserved word | Blocklist check |
| V5 | `module` not empty | Required field |
| V6 | `autoname` format valid | If set, must be one of: `hash`, `Prompt`, `field:<f>`, `format:<tpl>`, `naming_series:<prefix>` |
| V7 | Circular child-table reference | DocType must not be its own child (immediate check; full cycle is Phase 3+) |

### 3.2 DocField-level checks (for each field)

| # | Check | Error condition |
|---|---|---|
| F1 | `fieldname` not empty | Empty string |
| F2 | `fieldname` valid identifier | Regex `^[a-z][a-z0-9_]*$` (lowercase, underscores only) |
| F3 | `fieldname` not a SurrealDB reserved word | Blocklist: `name`, `id`, `type`, `return`, `select`, `create`, `update`, `delete`, ... |
| F4 | `fieldname` unique within DocType | No two fields share the same `fieldname` |
| F5 | `fieldtype` is a known type | Validate against the `FieldType` enum |
| F6 | Link field has `options` | `fieldtype == Link` → `options` must be non-empty (the linked DocType name) |
| F7 | Select/Autocomplete has `options` | Non-empty options list |
| F8 | `reqd=1` fields must not have `hidden=1` | A required hidden field can never be filled |
| F9 | `set_only_once` + `allow_on_submit` conflict | Both cannot be 1 |
| F10 | `depends_on` expression is syntactically valid | Basic bracket-balance check |
| F11 | `permlevel` is 0–9 | Out of range |
| F12 | `fetch_from` format | Must be `OtherDocType.fieldname` if set |
| F13 | No duplicate `fieldname` vs. system fields | Cannot name a field `name`, `doctype`, `creation`, `modified`, `owner`, `modified_by`, `docstatus`, `idx` |

### 3.3 DocPerm-level checks

| # | Check |
|---|---|
| P1 | `role` not empty |
| P2 | `role` exists in `tabRole` (warning, not error, to allow fixture seeding order) |
| P3 | `permlevel` is 0–9 |
| P4 | At least one of `read`/`write`/`perm_create` is set |
| P5 | `submit`/`amend`/`cancel` only allowed if DocType `is_submittable = 1` |

### 3.4 `DoctypeSaveError` enum

```rust
pub enum DoctypeSaveError {
    Validation(Vec<ValidationError>),   // One or more V/F/P failures — no DB write done
    Db(DbError),                        // DB error mid-save (data may be partially written)
    DdlFailed { message: String, data_committed: bool },
}

pub struct ValidationError {
    pub field:   Option<String>,    // None = DocType level, Some("fieldname") = field level
    pub code:    &'static str,      // e.g. "F3"
    pub message: String,
}
```

---

## Phase 4 — Test Suite

New file: `crates/spotledger-db/src/doctype_save_tests.rs` (gated on `#[cfg(test)]`)
Integration tests: `crates/spotledger-db/tests/doctype_integration.rs` (gated on `--features integration`)

### 4.1 Unit tests (no DB needed)

```
test_stamp_idx_assigns_positions
test_stamp_idx_on_empty_list
test_is_valid_fieldname_accepts_snake_case
test_is_valid_fieldname_rejects_uppercase
test_is_valid_fieldname_rejects_reserved_words
test_is_valid_fieldname_rejects_system_fields
test_validate_doctype_rejects_empty_name
test_validate_doctype_rejects_long_name
test_validate_doctype_rejects_invalid_chars
test_validate_field_link_requires_options
test_validate_field_reqd_hidden_conflict
test_validate_field_set_only_once_allow_on_submit_conflict
test_validate_perm_submit_on_non_submittable
test_default_perm_injected_when_perms_empty_and_new
test_field_diff_detects_added_fields
test_field_diff_detects_removed_fields
test_field_diff_detects_type_change
test_field_diff_unchanged_when_identical
```

### 4.2 Integration tests (require live SurrealDB)

```
test_save_new_doctype_creates_tabdoctype_row
test_save_new_doctype_creates_tabdocfield_rows_with_idx
test_save_new_doctype_creates_default_tabdocperm_row
test_save_new_doctype_creates_surreal_table
test_save_new_doctype_defines_surreal_fields
test_save_existing_doctype_updates_tabdoctype_row
test_save_existing_doctype_replaces_fields
test_save_existing_doctype_adds_new_surreal_field
test_save_existing_doctype_cache_invalidated
test_save_tier0_doctype_returns_error
test_save_doctype_with_empty_name_returns_validation_error
test_save_doctype_with_duplicate_fieldname_returns_validation_error
test_save_doctype_with_reserved_fieldname_returns_validation_error
test_save_doctype_link_field_without_options_returns_validation_error
test_save_doctype_with_perms_persists_tabdocperm
test_save_doctype_rollback_on_mid_save_db_error  // inject fault
```

### 4.3 Test helpers

```rust
fn minimal_doctype(name: &str) -> DoctypeSaveInput   // returns valid minimal input
fn with_field(input: &mut DoctypeSaveInput, fieldname: &str, fieldtype: &str)
fn with_perm(input: &mut DoctypeSaveInput, role: &str)
async fn assert_table_exists(adapter: &DbAdapter, table: &str)
async fn assert_field_defined(adapter: &DbAdapter, table: &str, field: &str)
async fn assert_tabdocfield_count(adapter: &DbAdapter, doctype: &str, expected: usize)
async fn assert_tabdocperm_count(adapter: &DbAdapter, doctype: &str, expected: usize)
```

---

## Phase 5 — SurrealDB-Specific Metadata

These are capabilities Frappe has no equivalent for. They are deferred because
Phase 1–4 must be solid first.

### 5.1 Graph node sync at runtime save

Currently `ensure_meta_records()` (in `schema.rs`) is only called from
`ensure_schema()` which runs at compile-time / `migrate`. It must also run
after every designer save so that `meta_cache.get()` (which reads the
`doctype → has_field → docfield` graph) sees the updated field list.

Add to the end of `save_doctype()`:

```rust
ensure_meta_records(adapter, &runtime_meta).await?;
```

Where `runtime_meta` is a `DocTypeMeta` constructed from the saved
`tabDocField` rows — same as `meta_cache.get()` does today.

### 5.2 Pipeline function attachment

A DocType can have pipeline stages and functions (`pipeline_stage`,
`pipeline_node`, `fn_source`, `has_node` edges). These are currently managed
separately via `handle_save_function` / `handle_delete_function`. They do not
need to be part of the atomic DocType save. However:

- On DocType **rename** (future): pipeline nodes reference `doctype` by name —
  they must be updated.
- On DocType **delete** (future): all `pipeline_stage`, `pipeline_node`,
  `fn_source`, and `has_node` rows must be removed.

Phase 5 will define `on_doctype_rename()` and `on_doctype_delete()` hooks that
cascade to the pipeline graph.

### 5.3 SurrealDB DEFINE EVENT integration

SurrealDB `DEFINE EVENT` fires inside the triggering write's transaction.
DocType save should optionally emit a `DEFINE EVENT` for doctypes that declare
automation triggers. This is currently handled by the pipeline but could be
improved to declare the event at save-time rather than at first-run.

### 5.4 `DEFINE FUNCTION` ownership

Pipeline functions (`fn::pipeline::run`) are global in SurrealDB. When a
DocType is deleted, its associated `DEFINE FUNCTION` definitions must be
`REMOVE FUNCTION`'d. The designer's `handle_delete_function` already does this
for individual functions but there is no bulk cleanup on DocType delete.

---

## Implementation Order

```
Phase 1a  Add transaction support to DbAdapter
Phase 1b  Add is_new_doctype(), stamp_idx() helpers
Phase 1c  New doctype_save.rs with save_doctype()
Phase 1d  Refactor handle_save in designer.rs to call save_doctype()
Phase 3   Add doctype_validate.rs, wire into save_doctype()
Phase 4   Write unit tests (no DB) — can be done alongside Phase 3
Phase 2   Add field diff + DDL evolution (OVERWRITE / REMOVE FIELD)
Phase 4+  Add integration tests for Phase 2
Phase 5   Graph sync + pipeline lifecycle hooks
```

---

## Files Affected

| File | Change |
|---|---|
| `crates/spotledger-db/src/adapter.rs` | Add `run_transaction()` |
| `crates/spotledger-db/src/doctype_save.rs` | **New** — `save_doctype()`, `is_new_doctype()`, `stamp_idx()` |
| `crates/spotledger-db/src/doctype_validate.rs` | **New** — `validate_doctype_input()`, `DoctypeSaveError` |
| `crates/spotledger-db/src/schema.rs` | Extract `define_system_fields()` and `define_user_fields()` as public fns callable from `doctype_save.rs` |
| `crates/spotledger-db/src/lib.rs` | Expose new modules |
| `crates/spotledger-http/src/methods/designer.rs` | `handle_save` becomes thin wrapper |
| `crates/spotledger-db/tests/doctype_integration.rs` | **New** — integration test suite |

---

## Non-Goals (explicitly out of scope)

- Renaming a DocType (separate `rename_doctype` operation)
- Deleting a DocType (separate `delete_doctype` operation)
- Custom script / controller file generation (no filesystem access in Rust server)
- Global search sync (no equivalent in our system yet)
- DocType import from JSON fixtures — that path (`seed_doctypes.rs`) is separate
  and already works; we align it with `save_doctype()` in a later cleanup
