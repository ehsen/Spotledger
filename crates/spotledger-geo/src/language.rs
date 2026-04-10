//! `Language` DocType — generated from Frappe JSON by `spotledger generate doctype`.
//! Edit freely; this file is not regenerated automatically.

use spotledger_core::meta::{DocField, DocTypeMeta, FieldType, Permission};
use spotledger_core::registry::MetaEntry;

pub fn language_meta() -> DocTypeMeta {
    DocTypeMeta {
        name: "Language".into(),
        module: "Core".into(),
        is_single:      false,
        is_tree:        false,
        is_child:       false,
        is_submittable: false,
        track_changes:  true,
        fields: vec![
            DocField::new("language_code", "Language Code", FieldType::Data)
                .required()
                .unique()
                .in_list(),
            DocField::new("language_name", "Language Name", FieldType::Data)
                .required()
                .in_list(),
            DocField::new("flag", "Flag", FieldType::Data)
                .in_list(),
            DocField::new("based_on", "Based On", FieldType::Link)
                .options("Language"),
            DocField::new("enabled", "Enabled", FieldType::Check)
                .default_value("1"),
            DocField::new("date_format", "Date Format", FieldType::Select)
                .options("\nyyyy-mm-dd\ndd-mm-yyyy\ndd/mm/yyyy\ndd.mm.yyyy\nmm/dd/yyyy\nmm-dd-yyyy"),
            DocField::new("time_format", "Time Format", FieldType::Select)
                .options("\nHH:mm:ss\nHH:mm"),
            DocField::new("number_format", "Number Format", FieldType::Select)
                .options("\n#,###.##\n#.###,##\n# ###.##\n# ###,##\n#'###.##\n#, ###.##\n#,##,###.##\n#,###.###\n#.###\n#,###"),
            DocField::new("first_day_of_the_week", "First Day of the Week", FieldType::Select)
                .options("\nSunday\nMonday\nTuesday\nWednesday\nThursday\nFriday\nSaturday"),
        ],
        permissions: vec![
            Permission {
                role: "System Manager".into(),
                read: true, write: true, create: true, delete: true,
                submit: false, cancel: false, amend: false,
                report: false, import: false, export: false,
                print: false, email: false, share: false,
            },
            Permission::read_only("All"),
        ],
        title_field:   Some("language_name".into()),
        search_fields: vec!["language_name".into()],
        sort_field:    Some("creation".into()),
        sort_order:    Some("DESC".into()),
        autoname:      Some("field:language_code".into()),
        naming_series: None,
    }
}

inventory::submit!(MetaEntry {
    name: "Language",
    meta: language_meta,
});
