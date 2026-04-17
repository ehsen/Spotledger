//! `Address` DocType — generated from Frappe JSON by `spotledger generate doctype`.
//! Edit freely; this file is not regenerated automatically.

use spotledger_core::meta::{DocField, DocTypeMeta, FieldType, Permission};
use spotledger_core::registry::MetaEntry;
use spotledger_core::modules::FM;

pub fn address_meta() -> DocTypeMeta {
    DocTypeMeta {
        name: "Address".into(),
        module: FM::CONTACTS.into(),
        is_single:      false,
        is_tree:        false,
        is_child:       false,
        is_submittable: false,
        track_changes:  true,
        fields: vec![
            DocField::new("address_title", "Address Title", FieldType::Data),
            DocField::new("address_type", "Address Type", FieldType::Select)
                .options("Billing\nShipping\nOffice\nPersonal\nPlant\nPostal\nShop\nSubsidiary\nWarehouse\nCurrent\nPermanent\nOther")
                .required()
                .in_list(),
            DocField::new("address_line1", "Address Line 1", FieldType::Data)
                .required(),
            DocField::new("address_line2", "Address Line 2", FieldType::Data),
            DocField::new("city", "City/Town", FieldType::Data)
                .required()
                .in_list(),
            DocField::new("county", "County", FieldType::Data),
            DocField::new("state", "State/Province", FieldType::Data),
            DocField::new("country", "Country", FieldType::Link)
                .options("Country")
                .required()
                .in_standard_filter(),
            DocField::new("pincode", "Postal Code", FieldType::Data),
            DocField::new("email_id", "Email Address", FieldType::Data)
                .options("Email"),
            DocField::new("phone", "Phone", FieldType::Data)
                .options("Phone"),
            DocField::new("fax", "Fax", FieldType::Data),
            DocField::new("is_primary_address", "Preferred Billing Address", FieldType::Check)
                .default_value("0"),
            DocField::new("is_shipping_address", "Preferred Shipping Address", FieldType::Check)
                .default_value("0"),
            DocField::new("disabled", "Disabled", FieldType::Check)
                .default_value("0"),
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
        title_field:   Some("address_title".into()),
        search_fields: vec!["address_title".into(), "city".into(), "country".into()],
        sort_field:    Some("creation".into()),
        sort_order:    Some("DESC".into()),
        autoname:      Some("format:{address_title}-{address_type}".into()),
        naming_series: None,
    }
}

inventory::submit!(MetaEntry {
    name: "Address",
    meta: address_meta,
});
