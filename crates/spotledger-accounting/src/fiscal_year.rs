//! `Fiscal Year` DocType — generated from Frappe JSON by `spotledger generate doctype`.
//! Edit freely; this file is not regenerated automatically.

use spotledger_core::meta::{DocField, DocTypeMeta, FieldType, Permission};
use spotledger_core::registry::MetaEntry;

pub fn fiscal_year_meta() -> DocTypeMeta {
    DocTypeMeta {
        name: "Fiscal Year".into(),
        module: "Accounts".into(),
        is_single:      false,
        is_tree:        false,
        is_child:       false,
        is_submittable: false,
        track_changes:  false,
        fields: vec![
            DocField::new("year", "Year Name", FieldType::Data)
                .required()
                .unique()
                .in_list()
                .description("For e.g. 2012, 2012-13"),
            DocField::new("disabled", "Disabled", FieldType::Check)
                .default_value("0"),
            DocField::new("is_short_year", "Is Short/Long Year", FieldType::Check)
                .set_only_once()
                .default_value("0")
                .description("More/Less than 12 months."),
            DocField::new("year_start_date", "Year Start Date", FieldType::Date)
                .required()
                .in_list()
                .set_only_once(),
            DocField::new("year_end_date", "Year End Date", FieldType::Date)
                .required()
                .in_list()
                .set_only_once(),
            DocField::new("companies", "Companies", FieldType::Table)
                .options("Fiscal Year Company"),
            DocField::new("auto_created", "Auto Created", FieldType::Check)
                .read_only()
                .hidden()
                .default_value("0"),
        ],
        permissions: vec![
            Permission {
                role: "System Manager".into(),
                read: true,
                write: true,
                create: true,
                delete: true,
                submit: false,
                cancel: false,
                amend: false,
                report: true,
                import: false,
                export: false,
                print: true,
                email: true,
                share: true,
            },
            Permission::read_only("Sales User"),
            Permission::read_only("Purchase User"),
            Permission::read_only("Accounts User"),
            Permission::read_only("Stock User"),
            Permission::read_only("Employee"),
            Permission::read_only("Accounts Manager"),
            Permission::read_only("Stock Manager"),
            Permission::read_only("Auditor"),
        ],
        title_field:   None,
        search_fields: vec![],
        sort_field:    Some("name".into()),
        sort_order:    Some("DESC".into()),
        autoname:      Some("field:year".into()),
        naming_series: None,
    }
}

inventory::submit!(MetaEntry {
    name: "Fiscal Year",
    meta: fiscal_year_meta,
});
