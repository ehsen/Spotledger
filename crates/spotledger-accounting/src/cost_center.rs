//! `Cost Center` DocType — generated from Frappe JSON by `spotledger generate doctype`.
//! Edit freely; this file is not regenerated automatically.

use spotledger_core::meta::{DocField, DocTypeMeta, FieldType, Permission};
use spotledger_core::registry::MetaEntry;
use spotledger_core::modules::FM;

pub fn cost_center_meta() -> DocTypeMeta {
    DocTypeMeta {
        name: "Cost Center".into(),
        module: FM::ACCOUNTS.into(),
        is_single:      false,
        is_tree:        true,
        is_child:       false,
        is_submittable: false,
        track_changes:  false,
        fields: vec![
            DocField::new("sb0", "sb0", FieldType::SectionBreak),
            DocField::new("cost_center_name", "Cost Center Name", FieldType::Data)
                .required()
                .in_list(),
            DocField::new("cost_center_number", "Cost Center Number", FieldType::Data)
                .in_list()
                .in_standard_filter(),
            DocField::new("parent_cost_center", "Parent Cost Center", FieldType::Link)
                .options("Cost Center")
                .required()
                .in_list(),
            DocField::new("company", "Company", FieldType::Link)
                .options("Company")
                .required()
                .in_list()
                .in_standard_filter(),
            DocField::new("cb0", "cb0", FieldType::ColumnBreak),
            DocField::new("is_group", "Is Group", FieldType::Check)
                .default_value("0"),
            DocField::new("disabled", "Disabled", FieldType::Check)
                .default_value("0"),
            DocField::new("lft", "lft", FieldType::Int)
                .hidden(),
            DocField::new("rgt", "rgt", FieldType::Int)
                .hidden(),
            DocField::new("old_parent", "old_parent", FieldType::Link)
                .options("Cost Center")
                .hidden(),
        ],
        permissions: vec![
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
                export: false,
                print: true,
                email: true,
                share: true,
            },
            Permission::read_only("Auditor"),
            Permission::read_only("Accounts User"),
            Permission::read_only("Sales User"),
            Permission::read_only("Purchase User"),
            Permission {
                role: "Employee".into(),
                read: false,
                write: false,
                create: false,
                delete: false,
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
        ],
        title_field:   None,
        search_fields: vec![],
        sort_field:    Some("creation".into()),
        sort_order:    Some("ASC".into()),
        autoname:      None,
        naming_series: None,
    }
}

inventory::submit!(MetaEntry {
    name: "Cost Center",
    meta: cost_center_meta,
});
