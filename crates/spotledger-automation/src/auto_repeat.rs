//! `Auto Repeat` DocType — generated from Frappe JSON by `spotledger generate doctype`.
//! Edit freely; this file is not regenerated automatically.

use spotledger_core::meta::{DocField, DocTypeMeta, FieldType, Permission};
use spotledger_core::registry::MetaEntry;
use spotledger_core::modules::FM;

pub fn auto_repeat_meta() -> DocTypeMeta {
    DocTypeMeta {
        name: "Auto Repeat".into(),
        module: FM::AUTOMATION.into(),
        is_single:      false,
        is_tree:        false,
        is_child:       false,
        is_submittable: false,
        track_changes:  false,
        fields: vec![
            DocField::new("reference_doctype", "Reference Document Type", FieldType::Link)
                .options("DocType")
                .required()
                .in_list(),
            DocField::new("reference_document", "Reference Document", FieldType::DynamicLink)
                .options("reference_doctype")
                .required(),
            DocField::new("submit_on_creation", "Submit on Creation", FieldType::Check)
                .default_value("0"),
            DocField::new("start_date", "Start Date", FieldType::Date)
                .required()
                .in_list()
                .default_value("Today"),
            DocField::new("end_date", "End Date", FieldType::Date)
                .in_list(),
            DocField::new("disabled", "Disabled", FieldType::Check)
                .default_value("0"),
            DocField::new("frequency", "Frequency", FieldType::Select)
                .options("\nDaily\nWeekly\nFortnightly\nMonthly\nQuarterly\nHalf-yearly\nYearly")
                .required()
                .in_list(),
            DocField::new("repeat_on_day", "Repeat on Day", FieldType::Int),
            DocField::new("repeat_on_last_day", "Repeat on Last Day of the Month", FieldType::Check)
                .default_value("0"),
            DocField::new("next_schedule_date", "Next Schedule Date", FieldType::Date),
            DocField::new("notify_by_email", "Notify by Email", FieldType::Check)
                .default_value("0"),
            DocField::new("recipients", "Recipients", FieldType::SmallText),
            DocField::new("template", "Template", FieldType::Link)
                .options("Email Template"),
            DocField::new("subject", "Subject", FieldType::Data),
            DocField::new("message", "Message", FieldType::Text)
                .default_value("Please find attached {{ doc.doctype }} #{{ doc.name }}"),
            DocField::new("print_format", "Print Format", FieldType::Link)
                .options("Print Format"),
            DocField::new("status", "Status", FieldType::Select)
                .options("\nActive\nDisabled\nCompleted")
                .in_list(),
            DocField::new("repeat_on_days", "Repeat on Days", FieldType::Table)
                .options("Auto Repeat Day"),
        ],
        permissions: vec![
            Permission {
                role: "System Manager".into(),
                read: true, write: true, create: true, delete: true,
                submit: false, cancel: false, amend: false,
                report: true, import: false, export: false,
                print: true, email: true, share: true,
            },
            Permission {
                role: "All".into(),
                read: true, write: true, create: true, delete: false,
                submit: false, cancel: false, amend: false,
                report: false, import: false, export: false,
                print: false, email: false, share: false,
            },
        ],
        title_field:   Some("reference_document".into()),
        search_fields: vec!["reference_doctype".into(), "reference_document".into()],
        sort_field:    Some("creation".into()),
        sort_order:    Some("DESC".into()),
        autoname:      Some("format:AUT-AR-{#####}".into()),
        naming_series: None,
    }
}

inventory::submit!(MetaEntry {
    name: "Auto Repeat",
    meta: auto_repeat_meta,
});
