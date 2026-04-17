//! `Country` DocType — generated from Frappe JSON by `spotledger generate doctype`.
//! Edit freely; this file is not regenerated automatically.

use spotledger_core::meta::{DocField, DocTypeMeta, FieldType, Permission};
use spotledger_core::registry::MetaEntry;
use spotledger_core::modules::FM;

pub fn country_meta() -> DocTypeMeta {
    DocTypeMeta {
        name: "Country".into(),
        module: FM::GEO.into(),
        is_single:      false,
        is_tree:        false,
        is_child:       false,
        is_submittable: false,
        track_changes:  true,
        fields: vec![
            DocField::new("country_name", "Country Name", FieldType::Data)
                .required()
                .unique()
                .in_list(),
            DocField::new("code", "Code", FieldType::Data)
                .required()
                .in_list()
                .description("The country's ISO 3166 ALPHA-2 code.")
                .length(2),
            DocField::new("date_format", "Date Format", FieldType::Data)
                .in_list(),
            DocField::new("time_format", "Time format", FieldType::Data)
                .in_list()
                .default_value("HH:mm:ss"),
            DocField::new("time_zones", "Time Zones", FieldType::Text)
                .in_list(),
        ],
        permissions: vec![
            Permission {
                role: "System Manager".into(),
                read: true,
                write: true,
                create: true,
                delete: false,
                submit: false,
                cancel: false,
                amend: false,
                report: true,
                import: true,
                export: true,
                print: true,
                email: true,
                share: true,
            },
            Permission::read_only("All"),
        ],
        title_field:   None,
        search_fields: vec![],
        sort_field:    Some("country_name".into()),
        sort_order:    Some("ASC".into()),
        autoname:      Some("field:country_name".into()),
        naming_series: None,
    }
}

inventory::submit!(MetaEntry {
    name: "Country",
    meta: country_meta,
});
