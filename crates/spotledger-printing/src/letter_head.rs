//! `Letter Head` DocType — generated from Frappe JSON by `spotledger generate doctype`.
//! Edit freely; this file is not regenerated automatically.

use spotledger_core::meta::{DocField, DocTypeMeta, FieldType, Permission};
use spotledger_core::registry::MetaEntry;

pub fn letter_head_meta() -> DocTypeMeta {
    DocTypeMeta {
        name: "Letter Head".into(),
        module: "Printing".into(),
        is_single:      false,
        is_tree:        false,
        is_child:       false,
        is_submittable: false,
        track_changes:  false,
        fields: vec![
            DocField::new("letter_head_name", "Letter Head Name", FieldType::Data)
                .required()
                .unique()
                .in_list(),
            DocField::new("source", "Letter Head Based On", FieldType::Select)
                .options("Image\nHTML"),
            DocField::new("footer_source", "Footer Based On", FieldType::Select)
                .options("Image\nHTML"),
            DocField::new("disabled", "Disabled", FieldType::Check)
                .default_value("0")
                .in_list(),
            DocField::new("is_default", "Default Letter Head", FieldType::Check)
                .default_value("0")
                .in_list(),
            DocField::new("image", "Image", FieldType::AttachImage),
            DocField::new("image_height", "Image Height (px)", FieldType::Float),
            DocField::new("image_width", "Image Width (px)", FieldType::Float),
            DocField::new("align", "Align", FieldType::Select)
                .options("Left\nRight\nCenter")
                .default_value("Left"),
            DocField::new("content", "Header HTML", FieldType::Html),
            DocField::new("footer", "Footer HTML", FieldType::Html),
            DocField::new("footer_image", "Footer Image", FieldType::AttachImage),
            DocField::new("footer_image_height", "Footer Image Height (px)", FieldType::Float),
            DocField::new("footer_image_width", "Footer Image Width (px)", FieldType::Float),
            DocField::new("footer_align", "Footer Align", FieldType::Select)
                .options("Left\nRight\nCenter")
                .default_value("Left"),
            DocField::new("header_script", "Header Script", FieldType::Code),
            DocField::new("footer_script", "Footer Script", FieldType::Code),
        ],
        permissions: vec![
            Permission {
                role: "System Manager".into(),
                read: true, write: true, create: true, delete: true,
                submit: false, cancel: false, amend: false,
                report: true, import: false, export: false,
                print: true, email: true, share: true,
            },
        ],
        title_field:   Some("letter_head_name".into()),
        search_fields: vec![],
        sort_field:    Some("creation".into()),
        sort_order:    Some("DESC".into()),
        autoname:      Some("field:letter_head_name".into()),
        naming_series: None,
    }
}

inventory::submit!(MetaEntry {
    name: "Letter Head",
    meta: letter_head_meta,
});
