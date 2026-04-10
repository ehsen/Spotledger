//! `Email Template` DocType — generated from `frappe/email/doctype/email_template/email_template.json`.

use spotledger_core::meta::{DocField, DocTypeMeta, FieldType, Permission};
use spotledger_core::registry::MetaEntry;

pub fn email_template_meta() -> DocTypeMeta {
    DocTypeMeta {
        name:           "Email Template".into(),
        module:         "Email".into(),
        is_single:      false,
        is_tree:        false,
        is_child:       false,
        is_submittable: false,
        track_changes:  false,
        fields: vec![
            DocField::new("subject", "Subject", FieldType::Data)
                .required()
                .in_list(),
            DocField::new("response", "Response", FieldType::LongText),
            DocField::new("use_html", "Use HTML", FieldType::Check),
            DocField::new("response_html", "Response HTML", FieldType::Code),
        ],
        permissions: vec![
            Permission::full("System Manager"),
            Permission::read_only("All"),
        ],
        title_field:   Some("name".into()),
        search_fields: vec!["name".into(), "subject".into()],
        sort_field:    Some("name".into()),
        sort_order:    Some("asc".into()),
        autoname:      None, // Prompt
        naming_series: None,
    }
}

inventory::submit!(MetaEntry {
    name: "Email Template",
    meta: email_template_meta,
});
