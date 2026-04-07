//! Tier 0: Document naming types (DocumentNamingRule, DocumentNamingSettings).
//!
//! ## How naming works
//!
//! Each non-singleton DocType may have a row in `tabDocumentNamingRule` keyed
//! by the DocType name.  `naming::resolve_name` consults this table (step 3) to
//! determine how to auto-generate document names.
//!
//! The `autoname` field uses Frappe-compatible patterns:
//!
//! | Pattern              | Meaning                                           |
//! |----------------------|---------------------------------------------------|
//! | `field:fieldname`    | Use the value of `fieldname` as the document name |
//! | `hash`               | UUID/hash (default when no rule exists)           |
//! | `prompt`             | User enters the name manually                     |
//! | `SO-.YYYY.-.####`    | Naming series (auto-increment counter)            |
//!
//! On `new-site`, `bootstrap::seed_naming_rules` iterates all compiled
//! `DocTypeMeta.autoname` values and inserts one row per non-singleton DocType
//! that has a non-None autoname.  Admins may then edit these rows in the Desk
//! without recompiling.

use crate::meta::{DocField, DocTypeMeta, FieldType, Permission};
use crate::registry::MetaEntry;

// ── DocumentNamingRule ────────────────────────────────────────────────────────

/// One row controls the naming strategy for one DocType.
///
/// The `name` of each record equals the `document_type` value (e.g., the rule
/// for `Sales Invoice` is stored as `tabDocumentNamingRule:⟨Sales Invoice⟩`),
/// which guarantees at most one default rule per DocType.
pub fn documentnamingrule_meta() -> DocTypeMeta {
    DocTypeMeta {
        name: "DocumentNamingRule".into(),
        module: "Core".into(),
        is_single: false,
        is_tree: false,
        is_child: false,
        is_submittable: false,
        track_changes: true,
        fields: vec![
            DocField::new("document_type", "Document Type", FieldType::Link)
                .options("DocType")
                .required()
                .unique()
                .in_list()
                .in_standard_filter(),
            DocField::new("autoname", "Auto Name", FieldType::Data)
                .description(
                    "Naming pattern: 'field:fieldname' | 'hash' | 'prompt' | \
                     series like 'SO-.YYYY.-.####'"
                )
                .in_list(),
            DocField::new("naming_series_options", "Naming Series Options", FieldType::Text)
                .description(
                    "Newline-separated series patterns shown in the Naming \
                     Series dropdown on forms (e.g. 'SINV-.YYYY.-.####\\nSINV-RETURN-.YYYY.-.')"
                ),
            DocField::new("is_standard", "Is Standard", FieldType::Check)
                .read_only()
                .hidden(),
        ],
        permissions: vec![Permission::full("System Manager")],
        title_field: Some("document_type".into()),
        search_fields: vec!["document_type".into()],
        sort_field: Some("document_type".into()),
        sort_order: Some("asc".into()),
        // Name of each rule = the document_type it controls.
        autoname: Some("field:document_type".into()),
        naming_series: None,
    }
}

inventory::submit!(MetaEntry {
    name: "DocumentNamingRule",
    meta: documentnamingrule_meta,
});

// ── DocumentNamingSettings ────────────────────────────────────────────────────

pub fn documentnamingsettings_meta() -> DocTypeMeta {
    DocTypeMeta {
        name: "DocumentNamingSettings".into(),
        module: "Core".into(),
        is_single: true,
        is_tree: false,
        is_child: false,
        is_submittable: false,
        track_changes: false,
        fields: vec![
            DocField::new("prefix_series_based_on_document_field", "Prefix Series Based On Document Field", FieldType::Check),
            DocField::new("apply_series_on_submit_only", "Apply Series On Submit Only", FieldType::Check)
                .description("If checked, naming series are applied only on submit"),
            DocField::new("allow_custom_naming", "Allow Custom Naming", FieldType::Check)
                .description("Allow users to supply a custom name during import"),
            DocField::new("allow_duplicate_series", "Allow Duplicate Series", FieldType::Check)
                .description("Skip uniqueness enforcement on naming series counters"),
        ],
        permissions: vec![Permission::full("System Manager")],
        title_field: None,
        search_fields: vec![],
        sort_field: None,
        sort_order: None,
        autoname: Some("DocumentNamingSettings".into()),
        naming_series: None,
    }
}

inventory::submit!(MetaEntry {
    name: "DocumentNamingSettings",
    meta: documentnamingsettings_meta,
});
