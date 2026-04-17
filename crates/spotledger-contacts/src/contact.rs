//! `Contact` DocType — generated from Frappe JSON by `spotledger generate doctype`.
//! Edit freely; this file is not regenerated automatically.

use spotledger_core::meta::{DocField, DocTypeMeta, FieldType, Permission};
use spotledger_core::registry::MetaEntry;
use spotledger_core::modules::FM;

pub fn contact_meta() -> DocTypeMeta {
    DocTypeMeta {
        name: "Contact".into(),
        module: FM::CONTACTS.into(),
        is_single:      false,
        is_tree:        false,
        is_child:       false,
        is_submittable: false,
        track_changes:  true,
        fields: vec![
            DocField::new("first_name", "First Name", FieldType::Data),
            DocField::new("middle_name", "Middle Name", FieldType::Data),
            DocField::new("last_name", "Last Name", FieldType::Data),
            DocField::new("full_name", "Full Name", FieldType::Data)
                .in_list(),
            DocField::new("email_id", "Email Address", FieldType::Data)
                .options("Email")
                .in_list(),
            DocField::new("user", "User Id", FieldType::Link)
                .options("User"),
            DocField::new("address", "Address", FieldType::Link)
                .options("Address"),
            DocField::new("status", "Status", FieldType::Select)
                .options("Passive\nOpen\nReplied")
                .default_value("Passive")
                .in_list()
                .in_standard_filter(),
            DocField::new("salutation", "Salutation", FieldType::Link)
                .options("Salutation"),
            DocField::new("gender", "Gender", FieldType::Link)
                .options("Gender"),
            DocField::new("phone", "Phone", FieldType::Data)
                .options("Phone")
                .in_list(),
            DocField::new("mobile_no", "Mobile No", FieldType::Data)
                .options("Phone"),
            DocField::new("company_name", "Company Name", FieldType::Data),
            DocField::new("designation", "Designation", FieldType::Data),
            DocField::new("department", "Department", FieldType::Data),
            DocField::new("is_primary_contact", "Is Primary Contact", FieldType::Check)
                .default_value("0"),
            DocField::new("unsubscribed", "Unsubscribed", FieldType::Check)
                .default_value("0"),
            DocField::new("email_ids", "Email IDs", FieldType::Table)
                .options("Contact Email"),
            DocField::new("phone_nos", "Contact Numbers", FieldType::Table)
                .options("Contact Phone"),
            DocField::new("links", "Links", FieldType::Table)
                .options("Dynamic Link"),
        ],
        permissions: vec![
            Permission {
                role: "Sales User".into(),
                read: true, write: true, create: true, delete: false,
                submit: false, cancel: false, amend: false,
                report: true, import: false, export: false,
                print: true, email: true, share: true,
            },
            Permission {
                role: "Purchase User".into(),
                read: true, write: true, create: true, delete: false,
                submit: false, cancel: false, amend: false,
                report: true, import: false, export: false,
                print: true, email: true, share: true,
            },
            Permission {
                role: "Accounts User".into(),
                read: true, write: true, create: true, delete: false,
                submit: false, cancel: false, amend: false,
                report: true, import: false, export: false,
                print: true, email: true, share: true,
            },
            Permission {
                role: "System Manager".into(),
                read: true, write: true, create: true, delete: true,
                submit: false, cancel: false, amend: false,
                report: true, import: true, export: true,
                print: true, email: true, share: true,
            },
        ],
        title_field:   Some("full_name".into()),
        search_fields: vec!["full_name".into(), "email_id".into(), "phone".into()],
        sort_field:    Some("creation".into()),
        sort_order:    Some("DESC".into()),
        autoname:      Some("format:{first_name} {last_name}".into()),
        naming_series: None,
    }
}

inventory::submit!(MetaEntry {
    name: "Contact",
    meta: contact_meta,
});
