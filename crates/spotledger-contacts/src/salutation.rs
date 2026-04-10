//! `Salutation` DocType — generated from `frappe/contacts/doctype/salutation/salutation.json`.

use spotledger_core::meta::{DocField, DocTypeMeta, FieldType, Permission};
use spotledger_core::registry::MetaEntry;

pub fn salutation_meta() -> DocTypeMeta {
    DocTypeMeta {
        name:           "Salutation".into(),
        module:         "Contacts".into(),
        is_single:      false,
        is_tree:        false,
        is_child:       false,
        is_submittable: false,
        track_changes:  false,
        fields: vec![
            DocField::new("salutation", "Salutation", FieldType::Data)
                .required()
                .unique()
                .in_list(),
        ],
        permissions: vec![
            Permission::full("System Manager"),
            Permission::read_only("All"),
        ],
        title_field:   Some("salutation".into()),
        search_fields: vec!["salutation".into()],
        sort_field:    Some("salutation".into()),
        sort_order:    Some("asc".into()),
        autoname:      Some("field:salutation".into()),
        naming_series: None,
    }
}

inventory::submit!(MetaEntry {
    name: "Salutation",
    meta: salutation_meta,
});
