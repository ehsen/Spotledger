//! `Error Log` DocType — generated from `frappe/core/doctype/error_log/error_log.json`.

use crate::meta::{DocField, DocTypeMeta, FieldType, Permission};
use crate::modules::FM;
use crate::registry::MetaEntry;

pub fn error_log_meta() -> DocTypeMeta {
    DocTypeMeta {
        name:           "Error Log".into(),
        module:         FM::CORE.into(),
        is_single:      false,
        is_tree:        false,
        is_child:       false,
        is_submittable: false,
        track_changes:  false,
        fields: vec![
            DocField::new("seen", "Seen", FieldType::Check),
            DocField::new("method", "Method / Title", FieldType::Data)
                .in_list(),
            DocField::new("error", "Error", FieldType::Code)
                .in_list(),
            DocField::new("reference_doctype", "Reference DocType", FieldType::Link)
                .options("DocType"),
            DocField::new("reference_name", "Reference Name", FieldType::Data),
            DocField::new("trace_id", "Trace ID", FieldType::Data),
            DocField::new("metadata", "Metadata", FieldType::Code),
        ],
        permissions: vec![
            Permission::full("System Manager"),
        ],
        title_field:   Some("method".into()),
        search_fields: vec!["method".into()],
        sort_field:    Some("creation".into()),
        sort_order:    Some("DESC".into()),
        autoname:      None,
        naming_series: None,
    }
}

inventory::submit!(MetaEntry {
    name: "Error Log",
    meta: error_log_meta,
});
