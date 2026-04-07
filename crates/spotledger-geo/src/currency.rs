//! `Currency` DocType — generated from Frappe JSON by `spotledger generate doctype`.
//! Edit freely; this file is not regenerated automatically.

use spotledger_core::meta::{DocField, DocTypeMeta, FieldType, Permission};
use spotledger_core::registry::MetaEntry;

pub fn currency_meta() -> DocTypeMeta {
    DocTypeMeta {
        name: "Currency".into(),
        module: "Geo".into(),
        is_single:      false,
        is_tree:        false,
        is_child:       false,
        is_submittable: false,
        track_changes:  true,
        fields: vec![
            DocField::new("currency_name", "Currency Name", FieldType::Data)
                .required()
                .unique(),
            DocField::new("enabled", "Enabled", FieldType::Check)
                .in_list()
                .default_value("0"),
            DocField::new("fraction", "Fraction", FieldType::Data)
                .in_list()
                .description("Sub-currency. For e.g. \"Cent\""),
            DocField::new("fraction_units", "Fraction Units", FieldType::Int)
                .in_list()
                .description("1 Currency = [?] Fraction For e.g. 1 USD = 100 Cent"),
            DocField::new("smallest_currency_fraction_value", "Smallest Currency Fraction Value", FieldType::Currency)
                .description("Smallest circulating fraction unit (coin). For e.g. 1 cent for USD and it should be entered as 0.01"),
            DocField::new("symbol", "Symbol", FieldType::Data)
                .in_list()
                .description("A symbol for this currency. For e.g. $"),
            DocField::new("symbol_on_right", "Show Currency Symbol on Right Side", FieldType::Check)
                .default_value("0"),
            DocField::new("number_format", "Number Format", FieldType::Select)
                .options("\n#,###.##\n#.###,##\n# ###.##\n# ###,##\n#'###.##\n#, ###.##\n#,##,###.##\n#,###.###\n#.###\n#,###")
                .in_list()
                .description("How should this currency be formatted? If not set, will use system defaults"),
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
                import: true,
                export: true,
                print: true,
                email: true,
                share: true,
            },
            Permission::read_only("Accounts Manager"),
            Permission::read_only("Accounts User"),
            Permission::read_only("Sales User"),
            Permission::read_only("Purchase User"),
        ],
        title_field:   None,
        search_fields: vec![],
        sort_field:    Some("creation".into()),
        sort_order:    Some("DESC".into()),
        autoname:      Some("field:currency_name".into()),
        naming_series: None,
    }
}

inventory::submit!(MetaEntry {
    name: "Currency",
    meta: currency_meta,
});
