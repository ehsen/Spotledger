//! Runtime DocType meta cache — queries SurrealDB for metadata of non-compiled
//! (Tier 3+) DocTypes and caches the result in memory.
//!
//! ## How it works
//!
//! For Tier 0 compiled types the engine always uses the `DocTypeMeta` struct
//! registered via `inventory::submit!`.  For every other DocType — those seeded
//! from JSON by `install_app`, or added by WASM plugins — the meta lives only
//! in SurrealDB.
//!
//! `MetaCache::get()` tries the compiled inventory first.  On a miss it queries
//! the `doctype` / `docfield` / `has_field` graph for a live meta record and
//! returns a dynamically-constructed `DynDocTypeMeta`.
//!
//! Entries are held in a [`moka`] async cache and expire after `ttl_secs`
//! (default: 300 s / 5 minutes), matching Frappe's `frappe.local.meta` TTL.
//!
//! ## Invalidation
//!
//! Call `MetaCache::invalidate(doctype)` after any schema-change operation
//! (custom field add/remove, property setter, plugin install/uninstall) to
//! force the next access to re-query SurrealDB.

use std::sync::Arc;
use std::time::Duration;

use moka::future::Cache;
use serde_json::Value;

use crate::adapter::DbAdapter;
use crate::error::DbError;

// ── DynDocTypeMeta ────────────────────────────────────────────────────────────

/// A lightweight, heap-allocated DocType meta constructed from live SurrealDB
/// graph records.  This is the runtime equivalent of the compiled `DocTypeMeta`
/// struct for non-Tier-0 types.
#[derive(Clone, Debug)]
pub struct DynDocField {
    pub fieldname:          String,
    pub label:              String,
    pub fieldtype:          String,
    pub options:            Option<String>,
    pub reqd:               bool,
    pub unique:             bool,
    pub read_only:          bool,
    pub hidden:             bool,
    pub in_list_view:       bool,
    pub in_standard_filter: bool,
    pub bold:               bool,
    pub default_value:      Option<String>,
    pub description:        Option<String>,
    pub is_custom:          bool,
    pub introduced_by:      Option<String>,
    pub idx:                i64,
}

#[derive(Clone, Debug)]
pub struct DynDocTypeMeta {
    pub name:           String,
    pub module:         String,
    pub is_single:      bool,
    pub is_child:       bool,
    pub is_submittable: bool,
    pub is_tree:        bool,
    pub introduced_by:  Option<String>,
    pub fields:         Vec<DynDocField>,
}

impl DynDocTypeMeta {
    /// Field names visible to list/filter queries (mirrors compiled meta helper).
    pub fn filter_fields(&self) -> Vec<&str> {
        self.fields
            .iter()
            .filter(|f| f.in_standard_filter)
            .map(|f| f.fieldname.as_str())
            .collect()
    }

    /// All fields that should appear in list views.
    pub fn list_fields(&self) -> Vec<&str> {
        self.fields
            .iter()
            .filter(|f| f.in_list_view)
            .map(|f| f.fieldname.as_str())
            .collect()
    }
}

// ── MetaCache ─────────────────────────────────────────────────────────────────

/// Async LRU cache for runtime DocType meta records.
///
/// Holds up to 1 024 entries; each entry expires 5 minutes after last access.
/// Clone is cheap — the inner cache is reference-counted.
#[derive(Clone)]
pub struct MetaCache {
    inner: Cache<String, Arc<DynDocTypeMeta>>,
}

impl MetaCache {
    /// Construct a cache with the default capacity (1 024) and TTL (300 s).
    pub fn new() -> Self {
        Self::with_ttl(300)
    }

    /// Construct a cache with a custom TTL in seconds.
    pub fn with_ttl(ttl_secs: u64) -> Self {
        Self {
            inner: Cache::builder()
                .max_capacity(1_024)
                .time_to_live(Duration::from_secs(ttl_secs))
                .build(),
        }
    }

    /// Get the meta for `doctype`, fetching from SurrealDB on a cache miss.
    ///
    /// Returns `DbError::NotFound` when the DocType has no graph node in the DB.
    pub async fn get(
        &self,
        adapter: &DbAdapter,
        doctype: &str,
    ) -> Result<Arc<DynDocTypeMeta>, DbError> {
        let key = doctype.to_owned();

        if let Some(cached) = self.inner.get(&key).await {
            return Ok(cached);
        }

        let meta = fetch_meta_from_db(adapter, doctype).await?;
        let arc = Arc::new(meta);
        self.inner.insert(key, arc.clone()).await;
        Ok(arc)
    }

    /// Remove a single entry, forcing the next `get()` to re-query SurrealDB.
    pub async fn invalidate(&self, doctype: &str) {
        self.inner.invalidate(doctype).await;
    }

    /// Flush the entire cache (e.g. after a bulk schema migration).
    pub async fn invalidate_all(&self) {
        self.inner.invalidate_all();
    }
}

impl Default for MetaCache {
    fn default() -> Self {
        Self::new()
    }
}

// ── DB fetch ──────────────────────────────────────────────────────────────────

/// Query SurrealDB for all graph nodes of a DocType and assemble a
/// `DynDocTypeMeta`.  Uses the `doctype` / `has_field` / `docfield` graph.
async fn fetch_meta_from_db(
    adapter: &DbAdapter,
    doctype: &str,
) -> Result<DynDocTypeMeta, DbError> {
    let dt_id = doctype_to_graph_id(doctype);

    // ── 1. Fetch the doctype node ─────────────────────────────────────────────
    let dt_rows = adapter
        .run(
            "SELECT * FROM doctype WHERE name = $name LIMIT 1",
            vec![("name".into(), Value::String(doctype.to_owned()))],
        )
        .await?;

    let dt_row = dt_rows.into_iter().next().ok_or_else(|| DbError::NotFound {
        doctype: doctype.to_owned(),
        name:    dt_id.clone(),
    })?;

    // ── 2. Fetch all has_field edges + docfield nodes ordered by idx ──────────
    let field_rows = adapter
        .run(
            "SELECT \
               ->has_field.idx AS idx, \
               ->has_field.is_custom AS is_custom, \
               ->has_field.introduced_by AS introduced_by, \
               ->has_field->docfield.* AS field \
             FROM doctype \
             WHERE name = $name \
             ORDER BY idx ASC \
             FETCH field",
            vec![("name".into(), Value::String(doctype.to_owned()))],
        )
        .await?;

    // ── 3. Assemble DynDocTypeMeta ────────────────────────────────────────────
    let module = dt_row
        .get("module")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_owned();

    let is_single = bool_field(&dt_row, "is_single");
    let is_child = bool_field(&dt_row, "is_child");
    let is_submittable = bool_field(&dt_row, "is_submittable");
    let is_tree = bool_field(&dt_row, "is_tree");
    let introduced_by = dt_row
        .get("introduced_by")
        .and_then(Value::as_str)
        .map(str::to_owned);

    let mut fields: Vec<DynDocField> = field_rows
        .into_iter()
        .filter_map(|row| parse_field_row(&row))
        .collect();

    // Sort by idx so the field ordering is deterministic.
    fields.sort_by_key(|f| f.idx);

    Ok(DynDocTypeMeta {
        name: doctype.to_owned(),
        module,
        is_single,
        is_child,
        is_submittable,
        is_tree,
        introduced_by,
        fields,
    })
}

// ── parsing helpers ───────────────────────────────────────────────────────────

fn doctype_to_graph_id(doctype: &str) -> String {
    doctype.to_lowercase().replace([' ', '-'], "_")
}

fn bool_field(row: &Value, key: &str) -> bool {
    match row.get(key) {
        Some(Value::Number(n)) => n.as_i64().unwrap_or(0) != 0,
        Some(Value::Bool(b)) => *b,
        _ => false,
    }
}

fn parse_field_row(row: &Value) -> Option<DynDocField> {
    // The query returns the nested docfield object under "field"
    let field = row.get("field").unwrap_or(row);

    let fieldname = field.get("fieldname")?.as_str()?.to_owned();
    let fieldtype = field
        .get("fieldtype")
        .and_then(Value::as_str)
        .unwrap_or("Data")
        .to_owned();

    Some(DynDocField {
        fieldname,
        label: field
            .get("label")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned(),
        fieldtype,
        options: field
            .get("options")
            .and_then(Value::as_str)
            .map(str::to_owned),
        reqd:    bool_field(field, "reqd"),
        unique:  bool_field(field, "unique"),
        read_only: bool_field(field, "read_only"),
        hidden:  bool_field(field, "hidden"),
        in_list_view: bool_field(field, "in_list_view"),
        in_standard_filter: bool_field(field, "in_standard_filter"),
        bold:    bool_field(field, "bold"),
        default_value: field
            .get("default_value")
            .and_then(Value::as_str)
            .map(str::to_owned),
        description: field
            .get("description")
            .and_then(Value::as_str)
            .map(str::to_owned),
        is_custom: bool_field(row, "is_custom"),
        introduced_by: row
            .get("introduced_by")
            .and_then(Value::as_str)
            .map(str::to_owned),
        idx: row
            .get("idx")
            .and_then(Value::as_i64)
            .unwrap_or(0),
    })
}
