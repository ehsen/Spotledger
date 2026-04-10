//! `Print Format` DocType — generated from `frappe/printing/doctype/print_format/print_format.json`.

use spotledger_core::meta::{DocField, DocTypeMeta, FieldType, Permission};
use spotledger_core::registry::MetaEntry;

pub fn print_format_meta() -> DocTypeMeta {
    DocTypeMeta {
        name:           "Print Format".into(),
        module:         "Printing".into(),
        is_single:      false,
        is_tree:        false,
        is_child:       false,
        is_submittable: false,
        track_changes:  false,
        fields: vec![
            DocField::new("doc_type", "Document Type", FieldType::Link)
                .options("DocType")
                .in_list(),
            DocField::new("module", "Module", FieldType::Link)
                .options("Module Def"),
            DocField::new("disabled", "Disabled", FieldType::Check),
            DocField::new("standard", "Standard", FieldType::Select)
                .select_options("No\nYes")
                .required(),
            DocField::new("custom_format", "Custom Format", FieldType::Check),
            DocField::new("print_format_type", "Print Format Type", FieldType::Select)
                .select_options("Jinja\nServer Script"),
            DocField::new("raw_printing", "Raw Printing", FieldType::Check),
            DocField::new("html", "HTML", FieldType::Code),
            DocField::new("raw_commands", "Raw Commands", FieldType::Code),
            DocField::new("align_labels_right", "Align Labels Right", FieldType::Check),
            DocField::new("show_section_headings", "Show Section Headings", FieldType::Check),
            DocField::new("default_print_language", "Default Print Language", FieldType::Link)
                .options("Language"),
            DocField::new("font", "Font", FieldType::Data),
            DocField::new("css", "CSS", FieldType::Code),
            DocField::new("format_data", "Format Data", FieldType::Code),
            DocField::new("absolute_value", "Show Absolute Value", FieldType::Check),
        ],
        permissions: vec![
            Permission::full("System Manager"),
            Permission::read_only("All"),
        ],
        title_field:   Some("name".into()),
        search_fields: vec!["name".into(), "doc_type".into()],
        sort_field:    Some("name".into()),
        sort_order:    Some("asc".into()),
        autoname:      None,
        naming_series: None,
    }
}

inventory::submit!(MetaEntry {
    name: "Print Format",
    meta: print_format_meta,
});
