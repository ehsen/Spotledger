//! UTM DocTypes — generated from `frappe/website/doctype/utm_*/utm_*.json`.
//! Covers: UTM Campaign, UTM Medium, UTM Source.

use spotledger_core::meta::{DocField, DocTypeMeta, FieldType, Permission};
use spotledger_core::registry::MetaEntry;

// ── UTM Campaign ──────────────────────────────────────────────────────────────

pub fn utm_campaign_meta() -> DocTypeMeta {
    DocTypeMeta {
        name:           "UTM Campaign".into(),
        module:         "Website".into(),
        is_single:      false,
        is_tree:        false,
        is_child:       false,
        is_submittable: false,
        track_changes:  false,
        fields: vec![
            DocField::new("campaign_description", "Campaign Description", FieldType::SmallText)
                .in_list(),
            DocField::new("slug", "Slug", FieldType::Data),
        ],
        permissions: vec![
            Permission::full("System Manager"),
            Permission::read_only("All"),
        ],
        title_field:   Some("name".into()),
        search_fields: vec!["name".into()],
        sort_field:    Some("name".into()),
        sort_order:    Some("asc".into()),
        autoname:      None, // Prompt
        naming_series: None,
    }
}

inventory::submit!(MetaEntry {
    name: "UTM Campaign",
    meta: utm_campaign_meta,
});

// ── UTM Medium ────────────────────────────────────────────────────────────────

pub fn utm_medium_meta() -> DocTypeMeta {
    DocTypeMeta {
        name:           "UTM Medium".into(),
        module:         "Website".into(),
        is_single:      false,
        is_tree:        false,
        is_child:       false,
        is_submittable: false,
        track_changes:  false,
        fields: vec![
            DocField::new("description", "Description", FieldType::SmallText)
                .in_list(),
            DocField::new("slug", "Slug", FieldType::Data),
        ],
        permissions: vec![
            Permission::full("System Manager"),
            Permission::read_only("All"),
        ],
        title_field:   Some("name".into()),
        search_fields: vec!["name".into()],
        sort_field:    Some("name".into()),
        sort_order:    Some("asc".into()),
        autoname:      None, // Prompt
        naming_series: None,
    }
}

inventory::submit!(MetaEntry {
    name: "UTM Medium",
    meta: utm_medium_meta,
});

// ── UTM Source ────────────────────────────────────────────────────────────────

pub fn utm_source_meta() -> DocTypeMeta {
    DocTypeMeta {
        name:           "UTM Source".into(),
        module:         "Website".into(),
        is_single:      false,
        is_tree:        false,
        is_child:       false,
        is_submittable: false,
        track_changes:  false,
        fields: vec![
            DocField::new("description", "Description", FieldType::SmallText)
                .in_list(),
            DocField::new("slug", "Slug", FieldType::Data),
        ],
        permissions: vec![
            Permission::full("System Manager"),
            Permission::read_only("All"),
        ],
        title_field:   Some("name".into()),
        search_fields: vec!["name".into()],
        sort_field:    Some("name".into()),
        sort_order:    Some("asc".into()),
        autoname:      None, // Prompt
        naming_series: None,
    }
}

inventory::submit!(MetaEntry {
    name: "UTM Source",
    meta: utm_source_meta,
});
