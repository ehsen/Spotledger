//! Canonical DocType save / update orchestration.
//!
//! `save_doctype()` is the single entry point for all DocType persistence —
//! both the designer API and any future programmatic callers use this function.
//!
//! ## Sequence (mirrors Frappe's `DocType.on_update()` + `frappe.db.updatedb()`)
//!
//! 1. **Validate** — run all `doctype_validate` checks; abort on any failure.
//! 2. **Detect** — determine `is_new` by querying `tabDocType`.
//! 3. **Stamp idx** — assign 0-based position to each field and permission row.
//! 4. **DML transaction** — inside `BEGIN TRANSACTION … COMMIT TRANSACTION`:
//!    - Upsert `tabDocType` row (all scalar meta fields).
//!    - Delete + re-insert `tabDocField` rows (designer is authoritative).
//!    - Delete + re-insert `tabDocPerm` rows.
//! 5. **DDL** — outside the transaction (SurrealDB DDL is schema-level, not row-level):
//!    - `DEFINE TABLE` if new; field defines for new/changed fields.
//!    - Phase 2: `DEFINE FIELD OVERWRITE` / `REMOVE FIELD` for field evolution.
//! 6. **Invalidate** — bust the meta cache so the next read sees fresh metadata.

use std::collections::{HashMap, HashSet};

use serde_json::{json, Value};

use crate::adapter::{DbAdapter, TransactionStatement};
use crate::document::doctype_to_table;
use crate::error::DbError;
use crate::meta_cache::MetaCache;
use crate::schema::{CHILD_FIELDS, SYSTEM_FIELDS};

pub use crate::doctype_validate::ValidationError;

// ── Input types ───────────────────────────────────────────────────────────────

/// All scalar flags/attributes of a single DocField, fully typed.
#[derive(Debug, Clone)]
pub struct DocFieldInput {
    pub fieldname:          String,
    pub label:              String,
    pub fieldtype:          String,
    pub options:            Option<String>,
    pub reqd:               bool,
    pub hidden:             bool,
    pub bold:               bool,
    pub in_list_view:       bool,
    pub in_standard_filter: bool,
    pub read_only:          bool,
    pub set_only_once:      bool,
    pub allow_on_submit:    bool,
    pub permlevel:          u8,
    pub unique:             bool,
    pub not_nullable:       bool,
    pub depends_on:         Option<String>,
    pub fetch_from:         Option<String>,
    pub default_value:      Option<String>,
    pub description:        Option<String>,
    /// 0-based display position — stamped by `save_doctype`, not the caller.
    pub idx:                usize,
    /// Any extra attributes from the UI payload that we store verbatim.
    pub extra_attrs:        HashMap<String, Value>,
}

/// All scalar flags/attributes of a single DocPerm row, fully typed.
#[derive(Debug, Clone)]
pub struct DocPermInput {
    pub role:        String,
    pub permlevel:   u8,
    pub read:        bool,
    pub write:       bool,
    pub perm_create: bool,
    pub perm_delete: bool,
    pub perm_select: bool,
    pub perm_cancel: bool,
    pub submit:      bool,
    pub amend:       bool,
    pub report:      bool,
    pub import:      bool,
    pub export:      bool,
    pub print:       bool,
    pub email:       bool,
    pub share:       bool,
    pub if_owner:    bool,
    /// 0-based position — stamped by `save_doctype`.
    pub idx:         usize,
}

/// The complete, typed input to `save_doctype`.
#[derive(Debug, Clone)]
pub struct DoctypeSaveInput {
    // ── Required scalar meta ──────────────────────────────────────────────────
    pub doctype:        String,
    pub module:         String,
    pub autoname:       String,

    // ── Boolean flags ─────────────────────────────────────────────────────────
    pub is_child:       bool,
    pub is_single:      bool,
    pub is_submittable: bool,
    pub is_tree:        bool,
    pub custom:         bool,

    // ── Child rows ────────────────────────────────────────────────────────────
    pub fields: Vec<DocFieldInput>,
    pub perms:  Vec<DocPermInput>,

    /// The current authenticated user — stamped as owner/modified_by.
    pub user: String,

    /// Extra top-level meta fields from the UI (title_field, sort_field, etc.)
    /// stored verbatim in tabDocType without interpretation.
    pub extra_meta: HashMap<String, Value>,
}

// ── Error type ────────────────────────────────────────────────────────────────

#[derive(Debug, thiserror::Error)]
pub enum DoctypeSaveError {
    #[error("Validation failed: {0:?}")]
    Validation(Vec<ValidationError>),

    #[error("Database error: {0}")]
    Db(#[from] DbError),

    #[error("DDL failed after data was committed: {message}")]
    DdlFailed { message: String, data_committed: bool },
}

// ── Default permission ────────────────────────────────────────────────────────

fn default_system_manager_perm() -> DocPermInput {
    DocPermInput {
        role:        "System Manager".into(),
        permlevel:   0,
        read:        true,
        write:       true,
        perm_create: true,
        perm_delete: true,
        perm_select: false,
        perm_cancel: false,
        submit:      false,
        amend:       false,
        report:      true,
        import:      false,
        export:      true,
        print:       true,
        email:       false,
        share:       false,
        if_owner:    false,
        idx:         0,
    }
}

// ── idx stamping ──────────────────────────────────────────────────────────────

/// Stamp 0-based `idx` on every field from its array position.
pub fn stamp_field_idx(fields: &mut Vec<DocFieldInput>) {
    for (i, f) in fields.iter_mut().enumerate() {
        f.idx = i;
    }
}

/// Stamp 0-based `idx` on every permission row from its array position.
pub fn stamp_perm_idx(perms: &mut Vec<DocPermInput>) {
    for (i, p) in perms.iter_mut().enumerate() {
        p.idx = i;
    }
}

// ── Existence check ───────────────────────────────────────────────────────────

/// Returns `true` when no `tabDocType` row with the given name exists.
pub async fn is_new_doctype(adapter: &DbAdapter, name: &str) -> Result<bool, DbError> {
    let rows = adapter
        .run(
            "SELECT count() FROM tabDocType WHERE name = $name GROUP ALL",
            vec![("name".into(), Value::String(name.to_owned()))],
        )
        .await?;
    let count = rows
        .first()
        .and_then(|v| v.get("count"))
        .and_then(Value::as_u64)
        .unwrap_or(0);
    Ok(count == 0)
}

// ── Value builders ────────────────────────────────────────────────────────────

/// Build the `tabDocType` content object from a `DoctypeSaveInput`.
fn build_doctype_content(input: &DoctypeSaveInput, user: &str, is_new: bool) -> Value {
    let mut obj = serde_json::Map::new();

    obj.insert("name".into(),         Value::String(input.doctype.clone()));
    obj.insert("doctype".into(),      Value::String("DocType".into()));
    obj.insert("module".into(),       Value::String(input.module.clone()));
    obj.insert("autoname".into(),     Value::String(input.autoname.clone()));
    obj.insert("istable".into(),      Value::Number((input.is_child as u8).into()));
    obj.insert("issingle".into(),     Value::Number((input.is_single as u8).into()));
    obj.insert("is_submittable".into(), Value::Number((input.is_submittable as u8).into()));
    obj.insert("is_tree".into(),      Value::Number((input.is_tree as u8).into()));
    obj.insert("custom".into(),       Value::Number((input.custom as u8).into()));
    // Scalar system fields — docstatus/idx have no VALUE clause so must be explicit.
    // creation/modified are handled by SurrealDB VALUE expressions in the schema.
    obj.insert("docstatus".into(),    Value::Number(0i64.into()));
    obj.insert("idx".into(),          Value::Number(0i64.into()));
    obj.insert("modified_by".into(),  Value::String(user.to_owned()));
    // Always include owner — UPSERT CONTENT replaces the whole document.
    // For new docs: set to current user.
    // For existing docs: preserve the original owner if it was passed in extra_meta,
    // otherwise fall back to current user so the field is never NONE.
    let owner = if is_new {
        user.to_owned()
    } else {
        input.extra_meta
            .get("owner")
            .and_then(Value::as_str)
            .unwrap_or(user)
            .to_owned()
    };
    obj.insert("owner".into(), Value::String(owner));
    // Always store empty arrays — child data lives in tabDocField/tabDocPerm
    obj.insert("fields".into(),       Value::Array(vec![]));
    obj.insert("permissions".into(),  Value::Array(vec![]));

    // Merge any extra top-level meta fields from the UI
    for (k, v) in &input.extra_meta {
        obj.insert(k.clone(), v.clone());
    }

    Value::Object(obj)
}

/// Build the `tabDocField` content object for one field, stamping parent info.
fn build_docfield_content(f: &DocFieldInput, doctype: &str, user: &str, is_new_field: bool) -> Value {
    let mut obj = serde_json::Map::new();

    // Deterministic name matching Frappe convention: "DocTypeName-fieldname"
    let row_name = format!("{doctype}-{}", f.fieldname);
    obj.insert("name".into(),        Value::String(row_name));
    obj.insert("doctype".into(),     Value::String("DocField".into()));
    obj.insert("parent".into(),      Value::String(doctype.to_owned()));
    obj.insert("parenttype".into(),  Value::String("DocType".into()));
    obj.insert("parentfield".into(), Value::String("fields".into()));
    obj.insert("idx".into(),         Value::Number(f.idx.into()));
    obj.insert("docstatus".into(),   Value::Number(0i64.into()));
    // creation/modified handled by SurrealDB VALUE expressions.
    obj.insert("modified_by".into(), Value::String(user.to_owned()));
    // Always include owner — UPSERT CONTENT replaces the whole document so
    // owner must always be set. Preserve it via extra_attrs on updates.
    let field_owner = if is_new_field {
        user.to_owned()
    } else {
        f.extra_attrs
            .get("owner")
            .and_then(Value::as_str)
            .unwrap_or(user)
            .to_owned()
    };
    obj.insert("owner".into(), Value::String(field_owner));
    obj.insert("fieldname".into(),   Value::String(f.fieldname.clone()));
    obj.insert("label".into(),       Value::String(f.label.clone()));
    obj.insert("fieldtype".into(),   Value::String(f.fieldtype.clone()));
    obj.insert("reqd".into(),        Value::Number((f.reqd as u8).into()));
    obj.insert("hidden".into(),      Value::Number((f.hidden as u8).into()));
    obj.insert("bold".into(),        Value::Number((f.bold as u8).into()));
    obj.insert("in_list_view".into(),         Value::Number((f.in_list_view as u8).into()));
    obj.insert("in_standard_filter".into(),   Value::Number((f.in_standard_filter as u8).into()));
    obj.insert("read_only".into(),            Value::Number((f.read_only as u8).into()));
    obj.insert("set_only_once".into(),        Value::Number((f.set_only_once as u8).into()));
    obj.insert("allow_on_submit".into(),      Value::Number((f.allow_on_submit as u8).into()));
    obj.insert("permlevel".into(),            Value::Number(f.permlevel.into()));
    obj.insert("unique".into(),               Value::Number((f.unique as u8).into()));
    obj.insert("not_nullable".into(),         Value::Number((f.not_nullable as u8).into()));

    if let Some(ref v) = f.options       { obj.insert("options".into(),       Value::String(v.clone())); }
    if let Some(ref v) = f.depends_on    { obj.insert("depends_on".into(),    Value::String(v.clone())); }
    if let Some(ref v) = f.fetch_from    { obj.insert("fetch_from".into(),    Value::String(v.clone())); }
    if let Some(ref v) = f.default_value { obj.insert("default_value".into(), Value::String(v.clone())); }
    if let Some(ref v) = f.description   { obj.insert("description".into(),   Value::String(v.clone())); }

    // Extra attributes from the UI payload
    for (k, v) in &f.extra_attrs {
        obj.entry(k.clone()).or_insert_with(|| v.clone());
    }

    Value::Object(obj)
}

/// Build the `tabDocPerm` content object for one permission row.
fn build_docperm_content(p: &DocPermInput, doctype: &str, user: &str) -> Value {
    // Deterministic name: "DocTypeName-Role-idx"
    let row_name = format!("{doctype}-{}-{}", p.role, p.idx);
    json!({
        "name":         row_name,
        "doctype":      "DocPerm",
        "parent":       doctype,
        "parenttype":   "DocType",
        "parentfield":  "permissions",
        "idx":          p.idx,
        "docstatus":    0i64,
        "owner":        user,
        // creation/modified handled by SurrealDB VALUE expressions.
        "modified_by":  user,
        "role":         p.role,
        "permlevel":    p.permlevel,
        "read":         p.read as u8,
        "write":        p.write as u8,
        "perm_create":  p.perm_create as u8,
        "perm_delete":  p.perm_delete as u8,
        "perm_select":  p.perm_select as u8,
        "perm_cancel":  p.perm_cancel as u8,
        "submit":       p.submit as u8,
        "amend":        p.amend as u8,
        "report":       p.report as u8,
        "import":       p.import as u8,
        "export":       p.export as u8,
        "print":        p.print as u8,
        "email":        p.email as u8,
        "share":        p.share as u8,
        "if_owner":     p.if_owner as u8,
    })
}

// ── SurrealDB type string ─────────────────────────────────────────────────────

/// Map a Frappe field type string to a SurrealDB type declaration.
/// Returns `None` for layout-only fields.
fn surql_type_for_str(fieldtype: &str) -> Option<&'static str> {
    match fieldtype.to_lowercase().replace('-', " ").as_str() {
        "check" | "int"                                      => Some("none | int"),
        "float" | "currency" | "percent" | "rating"         => Some("float"),
        "datetime"                                           => Some("option<datetime>"),
        "date" | "time"                                      => Some("option<string>"),
        "table" | "table multiselect"                        => Some("array<any>"),
        "json"                                               => Some("any"),
        "section break" | "column break" | "tab break"      => None,
        _                                                    => Some("option<string>"),
    }
}

fn is_layout_type(fieldtype: &str) -> bool {
    surql_type_for_str(fieldtype).is_none()
}

fn is_numeric_type(fieldtype: &str) -> bool {
    matches!(
        fieldtype.to_lowercase().as_str(),
        "check" | "int" | "float" | "currency" | "percent" | "rating"
    )
}

// ── DDL helpers ───────────────────────────────────────────────────────────────

async fn define_table(adapter: &DbAdapter, table: &str, doctype_name: &str) -> Result<(), DbError> {
    let sql = format!(
        "DEFINE TABLE IF NOT EXISTS `{table}` SCHEMAFULL \
         COMMENT 'SpotLedger DocType: {doctype_name}';"
    );
    adapter.execute(&sql, vec![]).await
}

async fn define_system_fields(adapter: &DbAdapter, table: &str, is_child: bool) -> Result<(), DbError> {
    for (fieldname, ty) in SYSTEM_FIELDS {
        let sql = format!(
            "DEFINE FIELD IF NOT EXISTS `{fieldname}` ON TABLE `{table}` TYPE {ty} PERMISSIONS FULL;"
        );
        adapter.execute(&sql, vec![]).await?;
    }
    if is_child {
        for (fieldname, ty) in CHILD_FIELDS {
            let sql = format!(
                "DEFINE FIELD IF NOT EXISTS `{fieldname}` ON TABLE `{table}` TYPE {ty} PERMISSIONS FULL;"
            );
            adapter.execute(&sql, vec![]).await?;
        }
    }
    Ok(())
}

async fn define_user_field(
    adapter: &DbAdapter,
    table: &str,
    f: &DocFieldInput,
    overwrite: bool,
) -> Result<(), DbError> {
    let Some(ty) = surql_type_for_str(&f.fieldtype) else { return Ok(()) };

    // Apply not_nullable: strip option<> wrapper if set
    let effective_type = if f.not_nullable && ty.starts_with("option<") {
        ty[7..ty.len() - 1].to_owned()
    } else {
        ty.to_owned()
    };

    let kw = if overwrite { "OVERWRITE" } else { "IF NOT EXISTS" };

    let mut sql = format!(
        "DEFINE FIELD {kw} `{}` ON TABLE `{table}` TYPE {effective_type} PERMISSIONS FULL",
        f.fieldname
    );

    if let Some(ref dv) = f.default_value {
        if !dv.is_empty() {
            if is_numeric_type(&f.fieldtype) {
                sql.push_str(&format!(" DEFAULT {dv}"));
            } else {
                let escaped = dv.replace('\'', "\\'");
                sql.push_str(&format!(" DEFAULT '{escaped}'"));
            }
        }
    } else if f.fieldtype.eq_ignore_ascii_case("Check") {
        sql.push_str(" DEFAULT 0");
    }

    sql.push(';');
    adapter.execute(&sql, vec![]).await
}

async fn define_unique_name_index(adapter: &DbAdapter, table: &str, safe_name: &str) -> Result<(), DbError> {
    let sql = format!(
        "DEFINE INDEX IF NOT EXISTS idx_{safe_name}_name ON TABLE `{table}` FIELDS name UNIQUE;"
    );
    adapter.execute(&sql, vec![]).await
}

async fn define_field_indexes(
    adapter: &DbAdapter,
    table: &str,
    safe_name: &str,
    f: &DocFieldInput,
) -> Result<(), DbError> {
    if f.unique {
        let idx_name = format!("idx_{safe_name}_{}_uq", f.fieldname);
        let sql = format!(
            "DEFINE INDEX IF NOT EXISTS `{idx_name}` ON TABLE `{table}` FIELDS `{}` UNIQUE;",
            f.fieldname
        );
        adapter.execute(&sql, vec![]).await?;
    }
    if f.in_standard_filter {
        let idx_name = format!("idx_{safe_name}_{}", f.fieldname);
        let sql = format!(
            "DEFINE INDEX IF NOT EXISTS `{idx_name}` ON TABLE `{table}` FIELDS `{}`;",
            f.fieldname
        );
        adapter.execute(&sql, vec![]).await?;
    }
    Ok(())
}

// ── Field diff (Phase 2) ──────────────────────────────────────────────────────

/// Load existing fieldnames for a DocType from `tabDocField`.
async fn load_existing_fieldnames(
    adapter: &DbAdapter,
    doctype: &str,
) -> Result<HashSet<String>, DbError> {
    let rows = adapter
        .run(
            "SELECT fieldname FROM tabDocField WHERE parent = $dt AND parenttype = 'DocType'",
            vec![("dt".into(), Value::String(doctype.to_owned()))],
        )
        .await?;

    Ok(rows
        .into_iter()
        .filter_map(|v| v.get("fieldname").and_then(Value::as_str).map(str::to_owned))
        .collect())
}

/// Compute per-field DDL action needed.
#[derive(Debug, PartialEq, Eq)]
enum FieldDdlAction {
    /// Brand-new field — `DEFINE FIELD IF NOT EXISTS`.
    Add,
    /// Type, default, or not_nullable changed — `DEFINE FIELD OVERWRITE`.
    Overwrite,
    /// Field unchanged — no DDL.
    Unchanged,
}

/// Decide the DDL action for each field based on old vs. new state.
///
/// We detect a change by serialising the type+default+not_nullable tuple; full
/// schema diffing is left to Phase 2 follow-on work.
async fn classify_fields<'a>(
    adapter: &DbAdapter,
    doctype: &str,
    new_fields: &'a [DocFieldInput],
    is_new_dt: bool,
) -> Result<Vec<(&'a DocFieldInput, FieldDdlAction)>, DbError> {
    if is_new_dt {
        return Ok(new_fields.iter().map(|f| (f, FieldDdlAction::Add)).collect());
    }

    // Fetch existing fieldnames from tabDocField
    let rows = adapter
        .run(
            "SELECT fieldname \
             FROM tabDocField WHERE parent = $dt AND parenttype = 'DocType'",
            vec![("dt".into(), Value::String(doctype.to_owned()))],
        )
        .await?;

    // Build a set of existing fieldnames
    let existing: std::collections::HashSet<String> = rows
        .into_iter()
        .filter_map(|row| row.get("fieldname").and_then(Value::as_str).map(str::to_owned))
        .collect();

    let mut result = Vec::with_capacity(new_fields.len());
    for f in new_fields {
        if is_layout_type(&f.fieldtype) {
            continue;
        }
        // Always use Overwrite for existing fields so that a resave always
        // re-applies DEFINE FIELD even when the tabDocField metadata hasn't
        // changed.  This repairs tables where the initial DDL was silently
        // swallowed (e.g. due to an earlier bug in execute not calling check()).
        let action = match existing.get(&f.fieldname) {
            None => FieldDdlAction::Add,
            Some(_) => FieldDdlAction::Overwrite,
        };
        result.push((f, action));
    }

    Ok(result)
}

// ── Removed-field handling ────────────────────────────────────────────────────

/// For fields that existed before but are no longer in the new payload, emit
/// `REMOVE FIELD` only when there is provably no data in that column.
/// Otherwise the column is orphaned (silently ignored by SurrealDB SCHEMAFULL
/// reads but not dropped).
async fn remove_dropped_fields(
    adapter: &DbAdapter,
    table: &str,
    old_names: &HashSet<String>,
    new_names: &HashSet<&str>,
) -> Result<(), DbError> {
    for fn_ in old_names {
        if new_names.contains(fn_.as_str()) || is_layout_type(fn_) {
            continue;
        }
        // Check if any row has a non-null value in this column
        let check_sql = format!(
            "SELECT count() FROM `{table}` WHERE `{fn_}` IS NOT NONE GROUP ALL"
        );
        let rows = adapter.run(&check_sql, vec![]).await?;
        let count = rows
            .first()
            .and_then(|v| v.get("count"))
            .and_then(Value::as_u64)
            .unwrap_or(0);
        if count == 0 {
            let rm_sql = format!("REMOVE FIELD IF EXISTS `{fn_}` ON TABLE `{table}`;");
            adapter.execute(&rm_sql, vec![]).await?;
            tracing::info!(table = %table, field = %fn_, "Removed empty dropped field");
        } else {
            tracing::warn!(
                table = %table, field = %fn_, rows = count,
                "Field dropped from schema but has data — column orphaned, not removed"
            );
        }
    }
    Ok(())
}

// ── Main save function ────────────────────────────────────────────────────────

/// Save or update a DocType atomically.
///
/// This is the canonical entry point. All callers (designer API, future
/// programmatic use) must go through here.
pub async fn save_doctype(
    adapter: &DbAdapter,
    meta_cache: &MetaCache,
    mut input: DoctypeSaveInput,
) -> Result<Value, DoctypeSaveError> {
    // ── 1. Validate ───────────────────────────────────────────────────────────
    crate::doctype_validate::validate_doctype_input(&input)
        .map_err(DoctypeSaveError::Validation)?;

    // ── 2. Detect is_new ──────────────────────────────────────────────────────
    let is_new = is_new_doctype(adapter, &input.doctype).await?;

    // ── 3. Stamp idx ──────────────────────────────────────────────────────────
    stamp_field_idx(&mut input.fields);
    stamp_perm_idx(&mut input.perms);

    // Inject default System Manager permission for new doctypes with no perms
    if is_new && input.perms.is_empty() {
        input.perms.push(default_system_manager_perm());
        tracing::debug!(
            doctype = %input.doctype,
            "Injected default System Manager permission for new DocType"
        );
    }

    // ── 4. Load old field names (for Phase 2 diff) ────────────────────────────
    let old_fieldnames = if !is_new {
        load_existing_fieldnames(adapter, &input.doctype).await?
    } else {
        HashSet::new()
    };

    // ── 5. DML transaction ────────────────────────────────────────────────────
    let user = if input.user.is_empty() { "Administrator" } else { &input.user };

    let doctype_content = build_doctype_content(&input, user, is_new);

    let mut stmts: Vec<TransactionStatement> = Vec::new();

    // 5a. Upsert tabDocType
    stmts.push(TransactionStatement::new(
        "UPSERT type::record($table, $name) CONTENT $content",
        vec![
            ("table".into(), Value::String("tabDocType".into())),
            ("name".into(),  Value::String(input.doctype.clone())),
            ("content".into(), doctype_content),
        ],
    ));

    // 5b. Delete old tabDocField rows
    stmts.push(TransactionStatement::new(
        "DELETE FROM tabDocField WHERE parent = $dt AND parenttype = 'DocType'",
        vec![("dt".into(), Value::String(input.doctype.clone()))],
    ));

    // 5c. Insert each field row
    for f in &input.fields {
        let row_name    = format!("{}-{}", input.doctype, f.fieldname);
        let is_new_field = is_new || !old_fieldnames.contains(&f.fieldname);
        let content     = build_docfield_content(f, &input.doctype, user, is_new_field);
        stmts.push(TransactionStatement::new(
            "UPSERT type::record($table, $name) CONTENT $content",
            vec![
                ("table".into(),   Value::String("tabDocField".into())),
                ("name".into(),    Value::String(row_name)),
                ("content".into(), content),
            ],
        ));
    }

    // 5d. Delete old tabDocPerm rows
    stmts.push(TransactionStatement::new(
        "DELETE FROM tabDocPerm WHERE parent = $dt AND parenttype = 'DocType'",
        vec![("dt".into(), Value::String(input.doctype.clone()))],
    ));

    // 5e. Insert each permission row
    for p in &input.perms {
        let row_name = format!("{}-{}-{}", input.doctype, p.role, p.idx);
        let content  = build_docperm_content(p, &input.doctype, user);
        stmts.push(TransactionStatement::new(
            "UPSERT type::record($table, $name) CONTENT $content",
            vec![
                ("table".into(),   Value::String("tabDocPerm".into())),
                ("name".into(),    Value::String(row_name)),
                ("content".into(), content),
            ],
        ));
    }

    adapter.run_transaction(stmts).await?;

    tracing::info!(
        doctype = %input.doctype,
        is_new = is_new,
        field_count = input.fields.len(),
        perm_count = input.perms.len(),
        "DML transaction committed"
    );

    // ── 6. DDL ────────────────────────────────────────────────────────────────
    let table     = doctype_to_table(&input.doctype);
    let safe_name = input.doctype.replace([' ', '-'], "_").to_lowercase();

    // 6a. Define table (idempotent)
    define_table(adapter, &table, &input.doctype)
        .await
        .map_err(|e| DoctypeSaveError::DdlFailed {
            message: format!("DEFINE TABLE failed: {e}"),
            data_committed: true,
        })?;

    // 6b. System fields (idempotent)
    define_system_fields(adapter, &table, input.is_child)
        .await
        .map_err(|e| DoctypeSaveError::DdlFailed {
            message: format!("System field DDL failed: {e}"),
            data_committed: true,
        })?;

    // 6c. Classify fields: add vs overwrite vs unchanged
    let classified = classify_fields(adapter, &input.doctype, &input.fields, is_new).await?;

    for (f, action) in &classified {
        match action {
            FieldDdlAction::Unchanged => {}
            FieldDdlAction::Add => {
                define_user_field(adapter, &table, f, false)
                    .await
                    .map_err(|e| DoctypeSaveError::DdlFailed {
                        message: format!("DEFINE FIELD `{}` failed: {e}", f.fieldname),
                        data_committed: true,
                    })?;
            }
            FieldDdlAction::Overwrite => {
                define_user_field(adapter, &table, f, true)
                    .await
                    .map_err(|e| DoctypeSaveError::DdlFailed {
                        message: format!("DEFINE FIELD OVERWRITE `{}` failed: {e}", f.fieldname),
                        data_committed: true,
                    })?;
            }
        }
        define_field_indexes(adapter, &table, &safe_name, f)
            .await
            .map_err(|e| DoctypeSaveError::DdlFailed {
                message: format!("Index DDL for `{}` failed: {e}", f.fieldname),
                data_committed: true,
            })?;
    }

    // 6d. Unique index on `name` for non-child tables
    if !input.is_child {
        define_unique_name_index(adapter, &table, &safe_name)
            .await
            .map_err(|e| DoctypeSaveError::DdlFailed {
                message: format!("Name index DDL failed: {e}"),
                data_committed: true,
            })?;
    }

    // 6e. Remove dropped fields (Phase 2 — safe only when no data exists)
    if !is_new {
        let new_fieldnames: HashSet<&str> = input
            .fields
            .iter()
            .filter(|f| !is_layout_type(&f.fieldtype))
            .map(|f| f.fieldname.as_str())
            .collect();
        remove_dropped_fields(adapter, &table, &old_fieldnames, &new_fieldnames)
            .await
            .map_err(|e| DoctypeSaveError::DdlFailed {
                message: format!("REMOVE FIELD failed: {e}"),
                data_committed: true,
            })?;
    }

    tracing::info!(doctype = %input.doctype, is_new = is_new, "DDL sync complete");

    // ── 7. Invalidate meta cache ──────────────────────────────────────────────
    meta_cache.invalidate(&input.doctype).await;

    // ── 8. Return the saved tabDocType record ─────────────────────────────────
    let saved_rows = adapter
        .run(
            "SELECT * FROM tabDocType WHERE name = $name LIMIT 1",
            vec![("name".into(), Value::String(input.doctype.clone()))],
        )
        .await?;

    let saved = saved_rows
        .into_iter()
        .next()
        .ok_or_else(|| DoctypeSaveError::Db(DbError::NotFound {
            doctype: "DocType".into(),
            name: input.doctype.clone(),
        }))?;

    Ok(saved)
}

// ── Utility ───────────────────────────────────────────────────────────────────

fn bool_field(row: &Value, key: &str) -> bool {
    match row.get(key) {
        Some(Value::Bool(b)) => *b,
        Some(Value::Number(n)) => n.as_u64().unwrap_or(0) != 0,
        _ => false,
    }
}

// ── Input parsing from serde_json::Value (for HTTP layer) ────────────────────

/// Parse a `serde_json::Value` object into a `DocFieldInput`.
///
/// The HTTP designer API sends JSON — this is the one place where we cross
/// from `Value` to typed structs.  No `unwrap_or_default` fallbacks for
/// required fields; missing required fields surface as a validation error later.
pub fn parse_docfield_from_value(v: &Value) -> DocFieldInput {
    let obj = v.as_object().cloned().unwrap_or_default();

    let mut extra_attrs = HashMap::new();
    let known_keys: HashSet<&str> = [
        "fieldname", "label", "fieldtype", "options", "reqd", "hidden",
        "bold", "in_list_view", "in_standard_filter", "read_only",
        "set_only_once", "allow_on_submit", "permlevel", "unique",
        "not_nullable", "depends_on", "fetch_from", "default_value",
        "description", "idx", "name", "doctype", "parent", "parenttype",
        "parentfield",
    ]
    .into();

    for (k, val) in &obj {
        if !known_keys.contains(k.as_str()) {
            extra_attrs.insert(k.clone(), val.clone());
        }
    }

    DocFieldInput {
        fieldname:          str_field(&obj, "fieldname"),
        label:              str_field(&obj, "label"),
        fieldtype:          str_field_or(&obj, "fieldtype", "Data"),
        options:            opt_str_field(&obj, "options"),
        reqd:               bool_from_obj(&obj, "reqd"),
        hidden:             bool_from_obj(&obj, "hidden"),
        bold:               bool_from_obj(&obj, "bold"),
        in_list_view:       bool_from_obj(&obj, "in_list_view"),
        in_standard_filter: bool_from_obj(&obj, "in_standard_filter"),
        read_only:          bool_from_obj(&obj, "read_only"),
        set_only_once:      bool_from_obj(&obj, "set_only_once"),
        allow_on_submit:    bool_from_obj(&obj, "allow_on_submit"),
        permlevel:          u8_field(&obj, "permlevel"),
        unique:             bool_from_obj(&obj, "unique"),
        not_nullable:       bool_from_obj(&obj, "not_nullable"),
        depends_on:         opt_str_field(&obj, "depends_on"),
        fetch_from:         opt_str_field(&obj, "fetch_from"),
        default_value:      opt_str_field(&obj, "default_value"),
        description:        opt_str_field(&obj, "description"),
        idx:                0, // will be stamped
        extra_attrs,
    }
}

/// Parse a `serde_json::Value` object into a `DocPermInput`.
pub fn parse_docperm_from_value(v: &Value) -> DocPermInput {
    let obj = v.as_object().cloned().unwrap_or_default();
    DocPermInput {
        role:        str_field(&obj, "role"),
        permlevel:   u8_field(&obj, "permlevel"),
        read:        bool_from_obj(&obj, "read"),
        write:       bool_from_obj(&obj, "write"),
        perm_create: bool_from_obj(&obj, "perm_create"),
        perm_delete: bool_from_obj(&obj, "perm_delete"),
        perm_select: bool_from_obj(&obj, "perm_select"),
        perm_cancel: bool_from_obj(&obj, "perm_cancel"),
        submit:      bool_from_obj(&obj, "submit"),
        amend:       bool_from_obj(&obj, "amend"),
        report:      bool_from_obj(&obj, "report"),
        import:      bool_from_obj(&obj, "import"),
        export:      bool_from_obj(&obj, "export"),
        print:       bool_from_obj(&obj, "print"),
        email:       bool_from_obj(&obj, "email"),
        share:       bool_from_obj(&obj, "share"),
        if_owner:    bool_from_obj(&obj, "if_owner"),
        idx:         0, // will be stamped
    }
}

// ── Field extract helpers (no fallback for required fields) ───────────────────

fn str_field(obj: &serde_json::Map<String, Value>, key: &str) -> String {
    obj.get(key)
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_owned()
}

fn str_field_or<'a>(obj: &'a serde_json::Map<String, Value>, key: &str, default: &'a str) -> String {
    obj.get(key)
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .unwrap_or(default)
        .to_owned()
}

fn opt_str_field(obj: &serde_json::Map<String, Value>, key: &str) -> Option<String> {
    obj.get(key)
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
}

fn bool_from_obj(obj: &serde_json::Map<String, Value>, key: &str) -> bool {
    match obj.get(key) {
        Some(Value::Bool(b)) => *b,
        Some(Value::Number(n)) => n.as_u64().unwrap_or(0) != 0,
        Some(Value::String(s)) => s == "1" || s.eq_ignore_ascii_case("true"),
        _ => false,
    }
}

fn u8_field(obj: &serde_json::Map<String, Value>, key: &str) -> u8 {
    obj.get(key)
        .and_then(Value::as_u64)
        .unwrap_or(0) as u8
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── stamp_field_idx ───────────────────────────────────────────────────────
    #[test]
    fn test_stamp_idx_assigns_positions() {
        let mut fields = vec![
            DocFieldInput {
                fieldname: "a".into(), label: "A".into(), fieldtype: "Data".into(),
                options: None, reqd: false, hidden: false, bold: false,
                in_list_view: false, in_standard_filter: false, read_only: false,
                set_only_once: false, allow_on_submit: false, permlevel: 0,
                unique: false, not_nullable: false, depends_on: None, fetch_from: None,
                default_value: None, description: None, idx: 99, extra_attrs: Default::default(),
            },
            DocFieldInput {
                fieldname: "b".into(), label: "B".into(), fieldtype: "Data".into(),
                options: None, reqd: false, hidden: false, bold: false,
                in_list_view: false, in_standard_filter: false, read_only: false,
                set_only_once: false, allow_on_submit: false, permlevel: 0,
                unique: false, not_nullable: false, depends_on: None, fetch_from: None,
                default_value: None, description: None, idx: 99, extra_attrs: Default::default(),
            },
        ];
        stamp_field_idx(&mut fields);
        assert_eq!(fields[0].idx, 0);
        assert_eq!(fields[1].idx, 1);
    }

    #[test]
    fn test_stamp_idx_on_empty_list() {
        let mut fields: Vec<DocFieldInput> = vec![];
        stamp_field_idx(&mut fields); // must not panic
        assert!(fields.is_empty());
    }

    // ── surql_type_for_str ────────────────────────────────────────────────────
    #[test]
    fn test_surql_type_mapping() {
        assert_eq!(surql_type_for_str("Check"),            Some("none | int"));
        assert_eq!(surql_type_for_str("Int"),              Some("none | int"));
        assert_eq!(surql_type_for_str("Float"),            Some("float"));
        assert_eq!(surql_type_for_str("Currency"),         Some("float"));
        assert_eq!(surql_type_for_str("Datetime"),         Some("option<datetime>"));
        assert_eq!(surql_type_for_str("Date"),             Some("option<string>"));
        assert_eq!(surql_type_for_str("Time"),             Some("option<string>"));
        assert_eq!(surql_type_for_str("Table"),            Some("array<any>"));
        assert_eq!(surql_type_for_str("Table MultiSelect"), Some("array<any>"));
        assert_eq!(surql_type_for_str("JSON"),             Some("any"));
        assert_eq!(surql_type_for_str("Data"),             Some("option<string>"));
        assert_eq!(surql_type_for_str("Section Break"),    None);
        assert_eq!(surql_type_for_str("Column Break"),     None);
        assert_eq!(surql_type_for_str("Tab Break"),        None);
    }

    // ── build_docfield_content ────────────────────────────────────────────────
    #[test]
    fn test_build_docfield_content_stamps_parent_info() {
        let f = DocFieldInput {
            fieldname: "airline_name".into(), label: "Airline Name".into(),
            fieldtype: "Data".into(), options: None, reqd: true, hidden: false,
            bold: false, in_list_view: true, in_standard_filter: false, read_only: false,
            set_only_once: false, allow_on_submit: false, permlevel: 0,
            unique: false, not_nullable: false, depends_on: None, fetch_from: None,
            default_value: None, description: None, idx: 2,
            extra_attrs: Default::default(),
        };
        let content = build_docfield_content(&f, "Airlines", "Administrator", true);
        let obj = content.as_object().unwrap();
        assert_eq!(obj["name"].as_str().unwrap(),        "Airlines-airline_name");
        assert_eq!(obj["parent"].as_str().unwrap(),      "Airlines");
        assert_eq!(obj["parenttype"].as_str().unwrap(),  "DocType");
        assert_eq!(obj["parentfield"].as_str().unwrap(), "fields");
        assert_eq!(obj["idx"].as_u64().unwrap(),         2);
        assert_eq!(obj["reqd"].as_u64().unwrap(),        1);
    }

    // ── build_docperm_content ─────────────────────────────────────────────────
    #[test]
    fn test_build_docperm_content_deterministic_name() {
        let p = DocPermInput {
            role: "System Manager".into(), permlevel: 0,
            read: true, write: true, perm_create: true, perm_delete: false,
            perm_select: false, perm_cancel: false,
            submit: false, amend: false,
            report: true, import: false, export: true, print: true,
            email: false, share: false, if_owner: false, idx: 0,
        };
        let content = build_docperm_content(&p, "Airlines", "Administrator");
        let obj = content.as_object().unwrap();
        assert_eq!(obj["name"].as_str().unwrap(),       "Airlines-System Manager-0");
        assert_eq!(obj["parent"].as_str().unwrap(),     "Airlines");
        assert_eq!(obj["parenttype"].as_str().unwrap(), "DocType");
        assert_eq!(obj["read"].as_u64().unwrap(),       1);
        assert_eq!(obj["submit"].as_u64().unwrap(),     0);
    }

    // ── default_system_manager_perm ───────────────────────────────────────────
    #[test]
    fn test_default_perm_has_read_write_create() {
        let p = default_system_manager_perm();
        assert_eq!(p.role, "System Manager");
        assert!(p.read);
        assert!(p.write);
        assert!(p.perm_create);
        assert!(!p.submit);
    }

    // ── is_layout_type ────────────────────────────────────────────────────────
    #[test]
    fn test_is_layout_type() {
        assert!(is_layout_type("Section Break"));
        assert!(is_layout_type("Column Break"));
        assert!(is_layout_type("Tab Break"));
        assert!(!is_layout_type("Data"));
        assert!(!is_layout_type("Link"));
    }

    // ── parse_docfield_from_value ─────────────────────────────────────────────
    #[test]
    fn test_parse_docfield_from_value_reads_all_known_fields() {
        let v = json!({
            "fieldname": "iata_code",
            "label": "IATA Code",
            "fieldtype": "Data",
            "reqd": 1,
            "unique": 1,
            "permlevel": 0,
            "in_list_view": 1,
            "custom_attr": "extra"
        });
        let f = parse_docfield_from_value(&v);
        assert_eq!(f.fieldname, "iata_code");
        assert_eq!(f.label, "IATA Code");
        assert!(f.reqd);
        assert!(f.unique);
        assert!(f.in_list_view);
        assert_eq!(f.extra_attrs.get("custom_attr").and_then(Value::as_str), Some("extra"));
    }

    // ── build_doctype_content ─────────────────────────────────────────────────
    #[test]
    fn test_build_doctype_content_structure() {
        let input = DoctypeSaveInput {
            doctype:        "Airlines".into(),
            module:         "Custom".into(),
            autoname:       "hash".into(),
            is_child:       false,
            is_single:      false,
            is_submittable: false,
            is_tree:        false,
            custom:         true,
            fields:         vec![],
            perms:          vec![],
            user:           "Administrator".into(),
            extra_meta:     Default::default(),
        };
        let content = build_doctype_content(&input, "Administrator", false);
        let obj = content.as_object().unwrap();
        assert_eq!(obj["name"].as_str().unwrap(),    "Airlines");
        assert_eq!(obj["module"].as_str().unwrap(),  "Custom");
        assert_eq!(obj["autoname"].as_str().unwrap(), "hash");
        assert_eq!(obj["istable"].as_u64().unwrap(),  0);
        assert_eq!(obj["custom"].as_u64().unwrap(),   1);
        assert!(obj["fields"].as_array().unwrap().is_empty());
        assert!(obj["permissions"].as_array().unwrap().is_empty());
    }
}
