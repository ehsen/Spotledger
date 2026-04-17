//! `Fiscal Year Company` DocType — generated from Frappe JSON by `spotledger generate doctype`.
//! Edit freely; this file is not regenerated automatically.

use spotledger_core::meta::{DocField, DocTypeMeta, FieldType, Permission};
use spotledger_core::registry::MetaEntry;
use spotledger_core::modules::FM;

pub fn fiscal_year_company_meta() -> DocTypeMeta {
    DocTypeMeta {
        name: "Fiscal Year Company".into(),
        module: FM::ACCOUNTS.into(),
        is_single:      false,
        is_tree:        false,
        is_child:       false,
        is_submittable: false,
        track_changes:  true,
        fields: vec![
            DocField::new("company", "Company", FieldType::Link)
                .options("Company")
                .required()
                .in_list(),
        ],
        permissions: vec![
            Permission::full("System Manager"),
        ],
        title_field:   None,
        search_fields: vec![],
        sort_field:    Some("creation".into()),
        sort_order:    Some("DESC".into()),
        autoname:      None,
        naming_series: None,
    }
}

inventory::submit!(MetaEntry {
    name: "Fiscal Year Company",
    meta: fiscal_year_company_meta,
});
