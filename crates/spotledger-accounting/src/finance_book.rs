//! `Finance Book` DocType — generated from Frappe JSON by `spotledger generate doctype`.
//! Edit freely; this file is not regenerated automatically.

use spotledger_core::meta::{DocField, DocTypeMeta, FieldType, Permission};
use spotledger_core::registry::MetaEntry;

pub fn finance_book_meta() -> DocTypeMeta {
    DocTypeMeta {
        name: "Finance Book".into(),
        module: "Accounts".into(),
        is_single:      false,
        is_tree:        false,
        is_child:       false,
        is_submittable: false,
        track_changes:  true,
        fields: vec![
            DocField::new("finance_book_name", "Name", FieldType::Data)
                .unique(),
        ],
        permissions: vec![
            Permission::read_only("Accounts User"),
            Permission {
                role: "Accounts Manager".into(),
                read: true,
                write: true,
                create: true,
                delete: true,
                submit: false,
                cancel: false,
                amend: false,
                report: true,
                import: false,
                export: true,
                print: true,
                email: true,
                share: true,
            },
            Permission::read_only("Auditor"),
        ],
        title_field:   None,
        search_fields: vec![],
        sort_field:    Some("creation".into()),
        sort_order:    Some("DESC".into()),
        autoname:      Some("field:finance_book_name".into()),
        naming_series: None,
    }
}

inventory::submit!(MetaEntry {
    name: "Finance Book",
    meta: finance_book_meta,
});
