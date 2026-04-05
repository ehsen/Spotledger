//! Tier 0: Core DocType metadata types (DocType, DocField, DocPerm, CustomField, PropertySetter).
//!
//! These are the system DocTypes that define the schema of all other DocTypes.

use crate::meta::{DocField, DocTypeMeta, FieldType, Permission};
use crate::registry::MetaEntry;

// ── DocType ──────────────────────────────────────────────────────────────────

pub fn doctype_meta() -> DocTypeMeta {
    DocTypeMeta {
        name: "DocType".into(),
        module: "Core".into(),
        is_single: false,
        is_tree: false,
        is_child: false,
        is_submittable: false,
        track_changes: false,
        fields: vec![
            DocField::new("name", "Name", FieldType::Data)
                .required()
                .in_list(),
            DocField::new("module", "Module", FieldType::Link)
                .options("Module")
                .required(),
            DocField::new("issingle", "Is Single", FieldType::Check),
            DocField::new("istree", "Is Tree", FieldType::Check),
            DocField::new("issubmittable", "Is Submittable", FieldType::Check),
            DocField::new("custom", "Custom", FieldType::Check)
                .hidden(),
            DocField::new("is_child_table", "Is Child Table", FieldType::Check),
            DocField::new("track_changes", "Track Changes", FieldType::Check),
            DocField::new("title_field", "Title Field", FieldType::Data),
            DocField::new("search_fields", "Search Fields", FieldType::Data)
                .description("Comma-separated list of fields for search"),
            DocField::new("sort_field", "Sort Field", FieldType::Data),
            DocField::new("sort_order", "Sort Order", FieldType::Select)
                .select_options("asc\ndesc"),
            DocField::new("autoname", "Auto Name", FieldType::Data)
                .description("Pattern: 'field:fieldname', 'hash', or expression"),
            DocField::new("naming_series", "Naming Series", FieldType::Data),
            DocField::new("fields", "Fields", FieldType::Table)
                .options("DocField"),
            DocField::new("permissions", "Permissions", FieldType::Table)
                .options("DocPerm"),
        ],
        permissions: vec![
            Permission::full("System Manager"),
            Permission::read_only("All"),
        ],
        title_field: Some("name".into()),
        search_fields: vec!["name".into(), "module".into()],
        sort_field: Some("name".into()),
        sort_order: Some("asc".into()),
        autoname: None,
        naming_series: None,
    }
}

inventory::submit!(MetaEntry {
    name: "DocType",
    meta: doctype_meta,
});

// ── DocField ─────────────────────────────────────────────────────────────────

pub fn docfield_meta() -> DocTypeMeta {
    DocTypeMeta {
        name: "DocField".into(),
        module: "Core".into(),
        is_single: false,
        is_tree: false,
        is_child: true,
        is_submittable: false,
        track_changes: false,
        fields: vec![
            DocField::new("parent", "Parent", FieldType::Link)
                .options("DocType")
                .hidden(),
            DocField::new("parenttype", "Parent Type", FieldType::Data)
                .hidden(),
            DocField::new("parentfield", "Parent Field", FieldType::Data)
                .hidden(),
            DocField::new("idx", "Index", FieldType::Int)
                .hidden(),
            DocField::new("fieldname", "Fieldname", FieldType::Data)
                .required(),
            DocField::new("label", "Label", FieldType::Data)
                .required(),
            DocField::new("fieldtype", "Fieldtype", FieldType::Select)
                .select_options("Data\nLink\nSelect\nTable\nCheck\nInt\nFloat\nCurrency\nDate\nDatetime\nTime\nText\nLongText\nSmallText\nCode\nPassword\nJson\nHtml\nAttach\nAttachImage\nSignature\nColor\nRating"),
            DocField::new("options", "Options", FieldType::Text),
            DocField::new("reqd", "Required", FieldType::Check),
            DocField::new("unique", "Unique", FieldType::Check),
            DocField::new("read_only", "Read Only", FieldType::Check),
            DocField::new("hidden", "Hidden", FieldType::Check),
            DocField::new("in_list_view", "In List View", FieldType::Check),
            DocField::new("in_standard_filter", "In Standard Filter", FieldType::Check),
            DocField::new("bold", "Bold", FieldType::Check),
            DocField::new("default_value", "Default Value", FieldType::Text),
            DocField::new("description", "Description", FieldType::Text),
            DocField::new("set_only_once", "Set Only Once", FieldType::Check),
            DocField::new("allow_on_submit", "Allow On Submit", FieldType::Check),
            DocField::new("ignore_xss_filter", "Ignore XSS Filter", FieldType::Check),
            DocField::new("permlevel", "Permission Level", FieldType::Int),
            DocField::new("fetch_from", "Fetch From", FieldType::Data),
            DocField::new("fetch_if_empty", "Fetch If Empty", FieldType::Check),
            DocField::new("precision", "Precision", FieldType::Int),
            DocField::new("length", "Length", FieldType::Int),
        ],
        permissions: vec![Permission::full("System Manager")],
        title_field: Some("fieldname".into()),
        search_fields: vec!["fieldname".into(), "label".into()],
        sort_field: Some("idx".into()),
        sort_order: Some("asc".into()),
        autoname: None,
        naming_series: None,
    }
}

inventory::submit!(MetaEntry {
    name: "DocField",
    meta: docfield_meta,
});

// ── DocPerm ──────────────────────────────────────────────────────────────────

pub fn docperm_meta() -> DocTypeMeta {
    DocTypeMeta {
        name: "DocPerm".into(),
        module: "Core".into(),
        is_single: false,
        is_tree: false,
        is_child: true,
        is_submittable: false,
        track_changes: false,
        fields: vec![
            DocField::new("parent", "Parent", FieldType::Link)
                .options("DocType")
                .hidden(),
            DocField::new("parenttype", "Parent Type", FieldType::Data)
                .hidden(),
            DocField::new("parentfield", "Parent Field", FieldType::Data)
                .hidden(),
            DocField::new("idx", "Index", FieldType::Int)
                .hidden(),
            DocField::new("role", "Role", FieldType::Link)
                .options("Role")
                .required(),
            DocField::new("permlevel", "Permission Level", FieldType::Int),
            DocField::new("read", "Read", FieldType::Check),
            DocField::new("write", "Write", FieldType::Check),
            DocField::new("create", "Create", FieldType::Check),
            DocField::new("delete", "Delete", FieldType::Check),
            DocField::new("submit", "Submit", FieldType::Check),
            DocField::new("cancel", "Cancel", FieldType::Check),
            DocField::new("amend", "Amend", FieldType::Check),
            DocField::new("report", "Report", FieldType::Check),
            DocField::new("import", "Import", FieldType::Check),
            DocField::new("export", "Export", FieldType::Check),
            DocField::new("print", "Print", FieldType::Check),
            DocField::new("email", "Email", FieldType::Check),
            DocField::new("share", "Share", FieldType::Check),
            DocField::new("if_owner", "If Owner", FieldType::Check),
        ],
        permissions: vec![Permission::full("System Manager")],
        title_field: Some("role".into()),
        search_fields: vec!["role".into()],
        sort_field: Some("permlevel".into()),
        sort_order: Some("asc".into()),
        autoname: None,
        naming_series: None,
    }
}

inventory::submit!(MetaEntry {
    name: "DocPerm",
    meta: docperm_meta,
});

// ── CustomField ──────────────────────────────────────────────────────────────

pub fn customfield_meta() -> DocTypeMeta {
    DocTypeMeta {
        name: "CustomField".into(),
        module: "Core".into(),
        is_single: false,
        is_tree: false,
        is_child: false,
        is_submittable: false,
        track_changes: false,
        fields: vec![
            DocField::new("dt", "DocType", FieldType::Link)
                .options("DocType")
                .required(),
            DocField::new("fieldname", "Fieldname", FieldType::Data)
                .required(),
            DocField::new("label", "Label", FieldType::Data),
            DocField::new("fieldtype", "Fieldtype", FieldType::Select)
                .select_options("Data\nLink\nSelect\nTable\nCheck\nInt\nFloat\nCurrency\nDate\nDatetime\nTime\nText\nLongText\nSmallText"),
            DocField::new("options", "Options", FieldType::Text),
            DocField::new("reqd", "Required", FieldType::Check),
            DocField::new("unique", "Unique", FieldType::Check),
            DocField::new("read_only", "Read Only", FieldType::Check),
            DocField::new("hidden", "Hidden", FieldType::Check),
            DocField::new("in_list_view", "In List View", FieldType::Check),
            DocField::new("default_value", "Default Value", FieldType::Text),
            DocField::new("description", "Description", FieldType::Text),
        ],
        permissions: vec![
            Permission::full("System Manager"),
            Permission::read_only("All"),
        ],
        title_field: Some("fieldname".into()),
        search_fields: vec!["fieldname".into(), "dt".into()],
        sort_field: None,
        sort_order: None,
        autoname: None,
        naming_series: None,
    }
}

inventory::submit!(MetaEntry {
    name: "CustomField",
    meta: customfield_meta,
});

// ── PropertySetter ───────────────────────────────────────────────────────────

pub fn propertysetter_meta() -> DocTypeMeta {
    DocTypeMeta {
        name: "PropertySetter".into(),
        module: "Core".into(),
        is_single: false,
        is_tree: false,
        is_child: false,
        is_submittable: false,
        track_changes: false,
        fields: vec![
            DocField::new("doctype_or_field", "DocType or Field", FieldType::Select)
                .select_options("DocType\nDocField")
                .required(),
            DocField::new("doc_type", "Doc Type", FieldType::Link)
                .options("DocType")
                .required(),
            DocField::new("field_name", "Field Name", FieldType::Data),
            DocField::new("property", "Property", FieldType::Data)
                .required(),
            DocField::new("value", "Value", FieldType::Text)
                .required(),
            DocField::new("old_value", "Old Value", FieldType::Text)
                .hidden(),
        ],
        permissions: vec![Permission::full("System Manager")],
        title_field: Some("doc_type".into()),
        search_fields: vec!["doc_type".into(), "field_name".into()],
        sort_field: None,
        sort_order: None,
        autoname: None,
        naming_series: None,
    }
}

inventory::submit!(MetaEntry {
    name: "PropertySetter",
    meta: propertysetter_meta,
});
