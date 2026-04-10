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
            // Sidebar/menu visibility — controls whether this DocType appears
            // in the sidebar under its module.  Default 1 for most types.
            DocField::new("show_in_menu", "Show in Menu", FieldType::Check)
                .default_value("1")
                .description("Show this DocType in the sidebar under its module"),
            // Icon shown next to the DocType in sidebar/command palette
            DocField::new("icon", "Icon", FieldType::Data)
                .description("Lucide icon name or icon URL"),
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
            // ── Linkage (system) ───────────────────────────────────────────
            DocField::new("parent", "Parent", FieldType::Link)
                .options("DocType")
                .hidden(),
            DocField::new("parenttype", "Parent Type", FieldType::Data)
                .hidden(),
            DocField::new("parentfield", "Parent Field", FieldType::Data)
                .hidden(),
            DocField::new("idx", "Index", FieldType::Int)
                .hidden(),

            // ── Identity ──────────────────────────────────────────────────
            DocField::new("fieldname", "Fieldname", FieldType::Data)
                .required(),
            DocField::new("label", "Label", FieldType::Data),
            DocField::new("fieldtype", "Fieldtype", FieldType::Select)
                .select_options("Autocomplete\nAttach\nAttach Image\nBarcode\nButton\nCheck\nCode\nColor\nColumn Break\nCurrency\nData\nDate\nDatetime\nDuration\nDynamic Link\nFloat\nFold\nGeolocation\nHeading\nHTML\nHTML Editor\nIcon\nImage\nInt\nJSON\nLink\nLong Text\nMarkdown Editor\nPassword\nPercent\nPhone\nRead Only\nRating\nSection Break\nSelect\nSignature\nSmall Text\nTab Break\nTable\nTable MultiSelect\nText\nText Editor\nTime"),
            DocField::new("options", "Options", FieldType::SmallText),
            DocField::new("default", "Default", FieldType::SmallText),
            DocField::new("precision", "Precision", FieldType::Int),
            DocField::new("length", "Length", FieldType::Int),

            // ── Filtering / search ────────────────────────────────────────
            DocField::new("search_index", "Search Index", FieldType::Check),
            DocField::new("in_filter", "In Filter", FieldType::Check),
            DocField::new("in_list_view", "In List View", FieldType::Check),
            DocField::new("in_standard_filter", "In Standard Filter", FieldType::Check),
            DocField::new("in_global_search", "In Global Search", FieldType::Check),
            DocField::new("in_preview", "In Preview", FieldType::Check),

            // ── Display ───────────────────────────────────────────────────
            DocField::new("bold", "Bold", FieldType::Check),
            DocField::new("translatable", "Translatable", FieldType::Check),
            DocField::new("collapsible", "Collapsible", FieldType::Check),
            DocField::new("collapsible_depends_on", "Collapsible Depends On", FieldType::Code),
            DocField::new("hide_border", "Hide Border", FieldType::Check),
            DocField::new("hide_days", "Hide Days", FieldType::Check),
            DocField::new("hide_seconds", "Hide Seconds", FieldType::Check),
            DocField::new("max_height", "Max Height", FieldType::Data),
            DocField::new("placeholder", "Placeholder", FieldType::Data),
            DocField::new("alignment", "Alignment", FieldType::Select)
                .select_options("\nLeft\nCenter\nRight"),
            DocField::new("button_color", "Button Color", FieldType::Select)
                .select_options("\nDefault\nPrimary\nInfo\nSuccess\nWarning\nDanger"),
            DocField::new("width", "Width", FieldType::Data),
            DocField::new("columns", "Columns", FieldType::Int),

            // ── Print ─────────────────────────────────────────────────────
            DocField::new("no_copy", "No Copy", FieldType::Check),
            DocField::new("print_hide", "Print Hide", FieldType::Check),
            DocField::new("print_hide_if_no_value", "Print Hide If No Value", FieldType::Check),
            DocField::new("print_width", "Print Width", FieldType::Data),
            DocField::new("report_hide", "Report Hide", FieldType::Check),

            // ── Behaviour / logic ─────────────────────────────────────────
            DocField::new("depends_on", "Depends On", FieldType::Code),
            DocField::new("mandatory_depends_on", "Mandatory Depends On", FieldType::Code),
            DocField::new("read_only_depends_on", "Read Only Depends On", FieldType::Code),
            DocField::new("hidden", "Hidden", FieldType::Check),
            DocField::new("read_only", "Read Only", FieldType::Check),
            DocField::new("reqd", "Required", FieldType::Check),
            DocField::new("unique", "Unique", FieldType::Check),
            DocField::new("set_only_once", "Set Only Once", FieldType::Check),
            DocField::new("allow_on_submit", "Allow On Submit", FieldType::Check),
            DocField::new("allow_bulk_edit", "Allow Bulk Edit", FieldType::Check),
            DocField::new("allow_in_quick_entry", "Allow In Quick Entry", FieldType::Check),
            DocField::new("non_negative", "Non Negative", FieldType::Check),
            DocField::new("not_nullable", "Not Nullable", FieldType::Check),
            DocField::new("is_virtual", "Is Virtual", FieldType::Check),
            DocField::new("sort_options", "Sort Options", FieldType::Check),
            DocField::new("link_filters", "Link Filters", FieldType::Json),

            // ── Permissions ───────────────────────────────────────────────
            DocField::new("permlevel", "Permission Level", FieldType::Int),
            DocField::new("ignore_user_permissions", "Ignore User Permissions", FieldType::Check),
            DocField::new("ignore_xss_filter", "Ignore XSS Filter", FieldType::Check),

            // ── Fetch ─────────────────────────────────────────────────────
            DocField::new("fetch_from", "Fetch From", FieldType::SmallText),
            DocField::new("fetch_if_empty", "Fetch If Empty", FieldType::Check),

            // ── Timeline / dashboard ──────────────────────────────────────
            DocField::new("show_on_timeline", "Show On Timeline", FieldType::Check),
            DocField::new("show_dashboard", "Show Dashboard", FieldType::Check),
            DocField::new("show_description_on_click", "Show Description On Click", FieldType::Check),

            // ── Attachment ────────────────────────────────────────────────
            DocField::new("make_attachment_public", "Make Attachment Public", FieldType::Check),

            // ── Style ─────────────────────────────────────────────────────
            DocField::new("remember_last_selected_value", "Remember Last Selected Value", FieldType::Check),
            DocField::new("sticky", "Sticky", FieldType::Check),
            DocField::new("mask", "Mask", FieldType::Check),

            // ── Misc / meta ───────────────────────────────────────────────
            DocField::new("description", "Description", FieldType::SmallText),
            DocField::new("documentation_url", "Documentation URL", FieldType::Data),
            DocField::new("oldfieldname", "Old Fieldname", FieldType::Data),
            DocField::new("oldfieldtype", "Old Fieldtype", FieldType::Data),
            DocField::new("introduced_by", "Introduced By", FieldType::Data),
            DocField::new("is_custom", "Is Custom", FieldType::Check),
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
            DocField::new("perm_select", "Select", FieldType::Check),  // 'select' is reserved in SurrealDB v3
            DocField::new("read", "Read", FieldType::Check),
            DocField::new("write", "Write", FieldType::Check),
            DocField::new("perm_create", "Create", FieldType::Check),  // 'create' is reserved in SurrealDB v3
            DocField::new("perm_delete", "Delete", FieldType::Check),  // 'delete' is reserved in SurrealDB v3
            DocField::new("submit", "Submit", FieldType::Check),
            DocField::new("perm_cancel", "Cancel", FieldType::Check),  // 'cancel' is reserved in SurrealDB v3
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
