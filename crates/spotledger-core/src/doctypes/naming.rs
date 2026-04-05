//! Tier 0: Document naming types (DocumentNamingRule, DocumentNamingSettings).

use crate::meta::{DocField, DocTypeMeta, FieldType, Permission};
use crate::registry::MetaEntry;

// ── DocumentNamingRule ────────────────────────────────────────────────────────

pub fn documentnamingrule_meta() -> DocTypeMeta {
    DocTypeMeta {
        name: "DocumentNamingRule".into(),
        module: "Core".into(),
        is_single: false,
        is_tree: false,
        is_child: false,
        is_submittable: true,
        track_changes: false,
        fields: vec![
            DocField::new("name", "Name", FieldType::Data)
                .required()
                .in_list(),
            DocField::new("doctype", "DocType", FieldType::Link)
                .options("DocType")
                .required(),
            DocField::new("rule_type", "Rule Type", FieldType::Select)
                .select_options("Hash\nField\nExpression\nSeries")
                .required(),
            DocField::new("field_value", "Field Value", FieldType::Data),
            DocField::new("expression", "Expression", FieldType::Code)
                .description("Python expression to generate name"),
            DocField::new("series_pattern", "Series Pattern", FieldType::Data)
                .description("Pattern like SINV-{YY}-{MM}-{seq}"),
            DocField::new("next_id", "Next ID", FieldType::Int)
                .hidden(),
            DocField::new("is_default", "Is Default", FieldType::Check),
            DocField::new("is_standard", "Is Standard", FieldType::Check)
                .hidden(),
        ],
        permissions: vec![Permission::full("System Manager")],
        title_field: Some("doctype".into()),
        search_fields: vec!["doctype".into()],
        sort_field: Some("doctype".into()),
        sort_order: Some("asc".into()),
        autoname: None,
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
                .description("If checked series will be applied only on submit"),
            DocField::new("allow_custom_naming", "Allow Custom Naming", FieldType::Check)
                .description("Allow user to set custom name/id during import"),
            DocField::new("allow_duplicate_series", "Allow Duplicate Series", FieldType::Check)
                .description("Allow duplicate series"),
            DocField::new("naming_rules", "Naming Rules", FieldType::Table)
                .options("DocumentNamingRule"),
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
