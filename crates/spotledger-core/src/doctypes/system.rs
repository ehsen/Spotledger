//! Tier 0: System configuration types (SystemSettings, DefaultValue).

use crate::meta::{DocField, DocTypeMeta, FieldType, Permission};
use crate::registry::MetaEntry;

// ── SystemSettings ────────────────────────────────────────────────────────────

pub fn systemsettings_meta() -> DocTypeMeta {
    DocTypeMeta {
        name: "SystemSettings".into(),
        module: "Core".into(),
        is_single: true,
        is_tree: false,
        is_child: false,
        is_submittable: false,
        track_changes: false,
        fields: vec![
            DocField::new("setup_complete", "Setup Complete", FieldType::Check),
            DocField::new("system_default_locale", "System Default Locale", FieldType::Link)
                .options("Language"),
            DocField::new("country", "Country", FieldType::Data),
            DocField::new("timezone", "Timezone", FieldType::Data)
                .default_value("UTC"),
            DocField::new("enable_password_user_creation", "Enable Password User Creation", FieldType::Check),
            DocField::new("disable_user_import", "Disable User Import", FieldType::Check),
            DocField::new("allow_on_submit", "Allow On Submit", FieldType::Check),
            DocField::new("incomplete_days", "Incomplete Days", FieldType::Int),
            DocField::new("setup_wizard_redirect", "Setup Wizard Redirect", FieldType::Check)
                .set_only_once(),
            DocField::new("mail_server", "Mail Server", FieldType::Link)
                .options("Email Account"),
            DocField::new("mail_port", "Mail Port", FieldType::Int),
            DocField::new("use_ssl", "Use SSL", FieldType::Check),
            DocField::new("mail_login", "Mail Login", FieldType::Data),
            DocField::new("mail_password", "Mail Password", FieldType::Password),
            DocField::new("auto_email_id", "Auto Email ID", FieldType::Data),
            DocField::new("default_company", "Default Company", FieldType::Link)
                .options("Company"),
            DocField::new("default_currency", "Default Currency", FieldType::Link)
                .options("Currency"),
            DocField::new("default_doc_title", "Default Doc Title", FieldType::Data),
            DocField::new("enable_two_factor_auth", "Enable Two Factor Auth", FieldType::Check),
            DocField::new("file_upload_size_limit", "File Upload Size Limit", FieldType::Int)
                .description("Size in MB"),
            DocField::new("session_expiry_timeout", "Session Expiry Timeout", FieldType::Int)
                .description("Minutes"),
        ],
        permissions: vec![Permission::full("System Manager")],
        title_field: None,
        search_fields: vec![],
        sort_field: None,
        sort_order: None,
        autoname: Some("SystemSettings".into()),
        naming_series: None,
    }
}

inventory::submit!(MetaEntry {
    name: "SystemSettings",
    meta: systemsettings_meta,
});

// ── DefaultValue ──────────────────────────────────────────────────────────────

pub fn defaultvalue_meta() -> DocTypeMeta {
    DocTypeMeta {
        name: "DefaultValue".into(),
        module: "Core".into(),
        is_single: false,
        is_tree: false,
        is_child: false,
        is_submittable: false,
        track_changes: false,
        fields: vec![
            DocField::new("parenttype", "DocType", FieldType::Link)
                .options("DocType")
                .required(),
            DocField::new("fieldname", "Fieldname", FieldType::Data)
                .required(),
            DocField::new("value", "Value", FieldType::Text),
            DocField::new("defvalue", "Default Value", FieldType::Text),
            DocField::new("user", "User", FieldType::Link)
                .options("User"),
            DocField::new("is_standard", "Is Standard", FieldType::Check)
                .hidden(),
        ],
        permissions: vec![
            Permission::full("System Manager"),
            Permission::read_only("All"),
        ],
        title_field: Some("fieldname".into()),
        search_fields: vec!["parenttype".into(), "fieldname".into()],
        sort_field: None,
        sort_order: None,
        autoname: None,
        naming_series: None,
    }
}

inventory::submit!(MetaEntry {
    name: "DefaultValue",
    meta: defaultvalue_meta,
});
