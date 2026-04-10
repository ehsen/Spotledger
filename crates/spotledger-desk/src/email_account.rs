//! `Email Account` DocType — generated from `frappe/email/doctype/email_account/email_account.json`.

use spotledger_core::meta::{DocField, DocTypeMeta, FieldType, Permission};
use spotledger_core::registry::MetaEntry;

pub fn email_account_meta() -> DocTypeMeta {
    DocTypeMeta {
        name:           "Email Account".into(),
        module:         "Email".into(),
        is_single:      false,
        is_tree:        false,
        is_child:       false,
        is_submittable: false,
        track_changes:  false,
        fields: vec![
            DocField::new("email_id", "Email ID", FieldType::Data)
                .required()
                .unique()
                .in_list(),
            DocField::new("email_account_name", "Email Account Name", FieldType::Data)
                .required()
                .in_list(),
            DocField::new("service", "Service", FieldType::Select)
                .select_options("\nGMail\nSendGrid\nSparkPost\nYahoo Mail\nOutlook\nOffice 365\nSendmail\nCustom"),
            DocField::new("domain", "Email Domain", FieldType::Link)
                .options("Email Domain"),
            DocField::new("login_id", "Login ID", FieldType::Data),
            DocField::new("password", "Password", FieldType::Password),
            DocField::new("enable_incoming", "Enable Incoming", FieldType::Check),
            DocField::new("email_server", "Incoming Mail Server", FieldType::Data),
            DocField::new("use_imap", "Use IMAP", FieldType::Check),
            DocField::new("use_ssl", "Use SSL", FieldType::Check),
            DocField::new("append_to", "Append To", FieldType::Link)
                .options("DocType"),
            DocField::new("default_incoming", "Default Incoming", FieldType::Check),
            DocField::new("enable_outgoing", "Enable Outgoing", FieldType::Check),
            DocField::new("smtp_server", "SMTP Server", FieldType::Data),
            DocField::new("use_tls", "Use TLS", FieldType::Check),
            DocField::new("smtp_port", "SMTP Port", FieldType::Data),
            DocField::new("default_outgoing", "Default Outgoing", FieldType::Check),
            DocField::new("send_unsubscribe_message", "Send Unsubscribe Message", FieldType::Check),
            DocField::new("track_email_status", "Track Email Status", FieldType::Check),
            DocField::new("add_signature", "Add Signature", FieldType::Check),
            DocField::new("signature", "Signature", FieldType::LongText),
            DocField::new("enable_auto_reply", "Enable Auto Reply", FieldType::Check),
            DocField::new("auto_reply_message", "Auto Reply Message", FieldType::LongText),
            DocField::new("auth_method", "Auth Method", FieldType::Select)
                .select_options("Basic\nOAuth"),
            DocField::new("connected_app", "Connected App", FieldType::Link)
                .options("Connected App"),
            DocField::new("connected_user", "Connected User", FieldType::Link)
                .options("User"),
        ],
        permissions: vec![
            Permission::full("System Manager"),
        ],
        title_field:   Some("email_account_name".into()),
        search_fields: vec!["email_id".into(), "email_account_name".into()],
        sort_field:    Some("email_account_name".into()),
        sort_order:    Some("asc".into()),
        autoname:      Some("field:email_account_name".into()),
        naming_series: None,
    }
}

inventory::submit!(MetaEntry {
    name: "Email Account",
    meta: email_account_meta,
});
