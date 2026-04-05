//! `DocTypeRegistry` — two-track DocType discovery.
//!
//! Compiled track: Rust structs self-register via `inventory::submit!`.
//! Dynamic track:  DocTypes created at runtime stored as `DynamicMeta`.
//!
//! Compiled types take priority over dynamic ones with the same name.
//!
//! Schema sync: every compiled DocType also submits a `MetaEntry` so the
//! `schema::ensure_all_schemas` function can emit SurrealDB DDL at startup
//! without needing a live DocType instance.

use std::collections::HashMap;

use dashmap::DashMap;

use crate::document::Document;
use crate::doctype::DocType;
use crate::error::CoreError;
use crate::meta::DocTypeMeta;

// ── MetaEntry — schema inventory ─────────────────────────────────────────────

/// A self-registering schema entry for a compiled Rust DocType.
///
/// Use `inventory::submit!(MetaEntry { name: "MyDocType", meta: my_meta_fn })`
/// alongside `DocTypeEntry` so that `ensure_all_schemas` can drive DB DDL.
pub struct MetaEntry {
    pub name: &'static str,
    /// Returns the `DocTypeMeta` for this DocType.
    pub meta: fn() -> DocTypeMeta,
}

inventory::collect!(MetaEntry);

/// A self-registering entry for a compiled Rust DocType.
pub struct DocTypeEntry {
    pub name:    &'static str,
    pub factory: fn(Document) -> Box<dyn DocType>,
}

inventory::collect!(DocTypeEntry);

// ── DynamicMeta ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct DynamicMeta {
    pub name:   String,
    pub module: String,
}

// ── DocTypeRegistry ───────────────────────────────────────────────────────────

pub struct DocTypeRegistry {
    compiled: HashMap<&'static str, fn(Document) -> Box<dyn DocType>>,
    dynamic:  DashMap<String, DynamicMeta>,
}

impl DocTypeRegistry {
    pub fn new(dynamic_metas: impl IntoIterator<Item = DynamicMeta>) -> Self {
        let compiled: HashMap<_, _> = inventory::iter::<DocTypeEntry>
            .into_iter()
            .map(|e| (e.name, e.factory))
            .collect();

        tracing::info!(count = compiled.len(), "Loaded compiled doctypes");

        let dynamic = DashMap::new();
        let mut dyn_count = 0usize;
        for meta in dynamic_metas {
            dynamic.insert(meta.name.clone(), meta);
            dyn_count += 1;
        }
        tracing::info!(count = dyn_count, "Loaded dynamic doctypes");

        Self { compiled, dynamic }
    }

    /// Instantiate a DocType by name.
    pub fn create(&self, doctype: &str, doc: Document) -> Result<Box<dyn DocType>, CoreError> {
        if let Some(factory) = self.compiled.get(doctype) {
            return Ok(factory(doc));
        }
        if self.dynamic.contains_key(doctype) {
            return Ok(Box::new(DynamicDocument(doc)));
        }
        Err(CoreError::UnknownDocType(doctype.into()))
    }

    pub fn register_dynamic(&self, meta: DynamicMeta) {
        self.dynamic.insert(meta.name.clone(), meta);
    }

    pub fn contains(&self, doctype: &str) -> bool {
        self.compiled.contains_key(doctype) || self.dynamic.contains_key(doctype)
    }
}

// ── DynamicDocument ───────────────────────────────────────────────────────────

pub struct DynamicDocument(pub Document);

#[async_trait::async_trait]
impl DocType for DynamicDocument {
    fn doctype_name() -> &'static str where Self: Sized { "Dynamic" }
    fn doc(&self)         -> &Document     { &self.0 }
    fn doc_mut(&mut self) -> &mut Document { &mut self.0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_registry_returns_error() {
        let reg = DocTypeRegistry::new(std::iter::empty());
        let doc = Document::new("Unknown");
        assert!(reg.create("Unknown", doc).is_err());
    }

    #[test]
    fn dynamic_registration_works() {
        let reg = DocTypeRegistry::new(std::iter::empty());
        reg.register_dynamic(DynamicMeta { name: "My Custom DocType".into(), module: "Custom".into() });
        let doc = Document::new("My Custom DocType");
        assert!(reg.create("My Custom DocType", doc).is_ok());
        assert!(reg.contains("My Custom DocType"));
    }
}
