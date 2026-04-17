//! `Print Heading` DocType — generated from Frappe JSON by `spotledger generate doctype`.
//! Edit freely; this file is not regenerated automatically.

use spotledger_core::meta::{DocField, DocTypeMeta, FieldType, Permission};
use spotledger_core::registry::MetaEntry;
use spotledger_core::modules::FM;

pub fn print_heading_meta() -> DocTypeMeta {
    DocTypeMeta {
        name: "Print Heading".into(),
        module: FM::PRINTING.into(),
        is_single:      false,
        is_tree:        false,
        is_child:       false,
        is_submittable: false,
        track_changes:  false,
        fields: vec![
            DocField::new("print_heading", "Print Heading", FieldType::Data)
                .required()
                .unique()
                .in_list(),
            DocField::new("description", "Description", FieldType::SmallText)
                .in_list(),
        ],
        permissions: vec![
            Permission {
                role: "System Manager".into(),
                read: true, write: true, create: true, delete: true,
                submit: false, cancel: false, amend: false,
                report: true, import: false, export: false,
                print: true, email: true, share: true,
            },
            Permission::read_only("Desk User"),
        ],
        title_field:   None,
        search_fields: vec!["print_heading".into()],
        sort_field:    Some("creation".into()),
        sort_order:    Some("DESC".into()),
        autoname:      Some("field:print_heading".into()),
        naming_series: None,
    }
}

inventory::submit!(MetaEntry {
    name: "Print Heading",
    meta: print_heading_meta,
});
