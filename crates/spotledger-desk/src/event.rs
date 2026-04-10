//! `Event` DocType — generated from `frappe/desk/doctype/event/event.json`.

use spotledger_core::meta::{DocField, DocTypeMeta, FieldType, Permission};
use spotledger_core::registry::MetaEntry;

pub fn event_meta() -> DocTypeMeta {
    DocTypeMeta {
        name:           "Event".into(),
        module:         "Desk".into(),
        is_single:      false,
        is_tree:        false,
        is_child:       false,
        is_submittable: false,
        track_changes:  false,
        fields: vec![
            DocField::new("subject", "Subject", FieldType::SmallText)
                .required()
                .in_list(),
            DocField::new("event_category", "Event Category", FieldType::Select)
                .select_options("\nEvent\nMeeting\nCall\nSent/Received Email\nOther"),
            DocField::new("event_type", "Event Type", FieldType::Select)
                .select_options("Private\nPublic\nConfidential")
                .required(),
            DocField::new("starts_on", "Starts On", FieldType::Datetime)
                .required()
                .in_list(),
            DocField::new("ends_on", "Ends On", FieldType::Datetime),
            DocField::new("all_day", "All Day", FieldType::Check),
            DocField::new("color", "Color", FieldType::Color),
            DocField::new("description", "Description", FieldType::LongText),
            DocField::new("send_reminder", "Send Reminder", FieldType::Check),
            DocField::new("status", "Status", FieldType::Select)
                .select_options("Open\nClosed\nCancelled"),
            DocField::new("repeat_this_event", "Repeat This Event", FieldType::Check),
            DocField::new("repeat_on", "Repeat On", FieldType::Select)
                .select_options("\nDaily\nWeekly\nMonthly\nYearly"),
            DocField::new("repeat_till", "Repeat Till", FieldType::Date),
            DocField::new("monday", "Monday", FieldType::Check),
            DocField::new("tuesday", "Tuesday", FieldType::Check),
            DocField::new("wednesday", "Wednesday", FieldType::Check),
            DocField::new("thursday", "Thursday", FieldType::Check),
            DocField::new("friday", "Friday", FieldType::Check),
            DocField::new("saturday", "Saturday", FieldType::Check),
            DocField::new("sunday", "Sunday", FieldType::Check),
            DocField::new("event_participants", "Event Participants", FieldType::Table)
                .options("Event Participants"),
            DocField::new("sync_with_google_calendar", "Sync With Google Calendar", FieldType::Check),
            DocField::new("google_calendar", "Google Calendar", FieldType::Link)
                .options("Google Calendar"),
            DocField::new("reference_doctype", "Reference DocType", FieldType::Link)
                .options("DocType"),
            DocField::new("reference_docname", "Reference Document", FieldType::DynamicLink)
                .options("reference_doctype"),
        ],
        permissions: vec![
            Permission::full("System Manager"),
            Permission::read_only("All"),
        ],
        title_field:   Some("subject".into()),
        search_fields: vec!["subject".into()],
        sort_field:    Some("starts_on".into()),
        sort_order:    Some("DESC".into()),
        autoname:      Some("EV.#####".into()),
        naming_series: None,
    }
}

inventory::submit!(MetaEntry {
    name: "Event",
    meta: event_meta,
});
