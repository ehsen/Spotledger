//! Tier 0: User management types (User, Role, HasRole, UserPermission, UserGroup, UserType).

use crate::meta::{DocField, DocTypeMeta, FieldType, Permission};
use crate::registry::MetaEntry;

// ── User ──────────────────────────────────────────────────────────────────────

pub fn user_meta() -> DocTypeMeta {
    DocTypeMeta {
        name: "User".into(),
        module: "Core".into(),
        is_single: false,
        is_tree: false,
        is_child: false,
        is_submittable: false,
        track_changes: true,
        fields: vec![
            DocField::new("email", "Email", FieldType::Data)
                .required()
                .unique()
                .in_list(),
            DocField::new("first_name", "First Name", FieldType::Data),
            DocField::new("last_name", "Last Name", FieldType::Data),
            DocField::new("full_name", "Full Name", FieldType::Data)
                .read_only()
                .in_list(),
            DocField::new("user_type", "User Type", FieldType::Link)
                .options("UserType")
                .required(),
            DocField::new("roles", "Roles", FieldType::Table)
                .options("HasRole"),
            DocField::new("permissions", "Permissions", FieldType::Table)
                .options("UserPermission"),
            DocField::new("enabled", "Enabled", FieldType::Check)
                .required(),
            DocField::new("password", "Password", FieldType::Password)
                .description("Password will be sent to user via email if not already set"),
            DocField::new("phone", "Phone", FieldType::Data),
            DocField::new("mobile_no", "Mobile No", FieldType::Data),
            DocField::new("bio", "Bio", FieldType::Text),
            DocField::new("mute_sounds", "Mute Sounds", FieldType::Check),
            DocField::new("send_welcome_email", "Send Welcome Email", FieldType::Check),
            DocField::new("owner", "Owner", FieldType::Data)
                .hidden(),
            DocField::new("creation", "Creation", FieldType::Datetime)
                .hidden(),
            DocField::new("modified", "Modified", FieldType::Datetime)
                .hidden(),
            DocField::new("modified_by", "Modified By", FieldType::Data)
                .hidden(),
        ],
        permissions: vec![
            Permission::full("System Manager"),
            Permission {
                role: "User".into(),
                read: true,
                write: true,
                create: true,
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
        title_field: Some("full_name".into()),
        search_fields: vec!["email".into(), "first_name".into(), "last_name".into()],
        sort_field: Some("full_name".into()),
        sort_order: Some("asc".into()),
        autoname: Some("email".into()),
        naming_series: None,
    }
}

inventory::submit!(MetaEntry {
    name: "User",
    meta: user_meta,
});

// ── Role ──────────────────────────────────────────────────────────────────────

pub fn role_meta() -> DocTypeMeta {
    DocTypeMeta {
        name: "Role".into(),
        module: "Core".into(),
        is_single: false,
        is_tree: false,
        is_child: false,
        is_submittable: false,
        track_changes: false,
        fields: vec![
            DocField::new("role_name", "Role Name", FieldType::Data)
                .required()
                .unique()
                .in_list(),
            DocField::new("disabled", "Disabled", FieldType::Check),
            DocField::new("desk_access", "Desk Access", FieldType::Check),
            DocField::new("description", "Description", FieldType::Text),
            DocField::new("role_type", "Role Type", FieldType::Select)
                .select_options("System\nCustom"),
        ],
        permissions: vec![Permission::full("System Manager")],
        title_field: Some("role_name".into()),
        search_fields: vec!["role_name".into()],
        sort_field: Some("role_name".into()),
        sort_order: Some("asc".into()),
        autoname: Some("role_name".into()),
        naming_series: None,
    }
}

inventory::submit!(MetaEntry {
    name: "Role",
    meta: role_meta,
});

// ── HasRole ───────────────────────────────────────────────────────────────────

pub fn hasrole_meta() -> DocTypeMeta {
    DocTypeMeta {
        name: "HasRole".into(),
        module: "Core".into(),
        is_single: false,
        is_tree: false,
        is_child: true,
        is_submittable: false,
        track_changes: false,
        fields: vec![
            DocField::new("parent", "Parent", FieldType::Link)
                .options("User")
                .hidden(),
            DocField::new("parenttype", "Parent Type", FieldType::Data)
                .hidden(),
            DocField::new("parentfield", "Parent Field", FieldType::Data)
                .hidden(),
            DocField::new("idx", "Index", FieldType::Int)
                .hidden(),
            DocField::new("role", "Role", FieldType::Link)
                .options("Role")
                .required(),
        ],
        permissions: vec![Permission::full("System Manager")],
        title_field: Some("role".into()),
        search_fields: vec!["role".into()],
        sort_field: None,
        sort_order: None,
        autoname: None,
        naming_series: None,
    }
}

inventory::submit!(MetaEntry {
    name: "HasRole",
    meta: hasrole_meta,
});

// ── UserPermission ────────────────────────────────────────────────────────────

pub fn userpermission_meta() -> DocTypeMeta {
    DocTypeMeta {
        name: "UserPermission".into(),
        module: "Core".into(),
        is_single: false,
        is_tree: false,
        is_child: true,
        is_submittable: false,
        track_changes: false,
        fields: vec![
            DocField::new("parent", "Parent", FieldType::Link)
                .options("User")
                .hidden(),
            DocField::new("parenttype", "Parent Type", FieldType::Data)
                .hidden(),
            DocField::new("parentfield", "Parent Field", FieldType::Data)
                .hidden(),
            DocField::new("idx", "Index", FieldType::Int)
                .hidden(),
            DocField::new("user", "User", FieldType::Link)
                .options("User")
                .required(),
            DocField::new("ptype", "Permission Type", FieldType::Select)
                .select_options("User\nShare")
                .required(),
            DocField::new("doc", "Document", FieldType::Link)
                .options("User")
                .required(),
            DocField::new("when_shared", "When Shared", FieldType::Datetime),
        ],
        permissions: vec![Permission::full("System Manager")],
        title_field: Some("user".into()),
        search_fields: vec!["user".into(), "doc".into()],
        sort_field: None,
        sort_order: None,
        autoname: None,
        naming_series: None,
    }
}

inventory::submit!(MetaEntry {
    name: "UserPermission",
    meta: userpermission_meta,
});

// ── UserGroup ─────────────────────────────────────────────────────────────────

pub fn usergroup_meta() -> DocTypeMeta {
    DocTypeMeta {
        name: "UserGroup".into(),
        module: "Core".into(),
        is_single: false,
        is_tree: false,
        is_child: false,
        is_submittable: false,
        track_changes: false,
        fields: vec![
            DocField::new("name", "Name", FieldType::Data)
                .required()
                .unique()
                .in_list(),
            DocField::new("description", "Description", FieldType::Text),
            DocField::new("users", "Users", FieldType::Table)
                .options("UserGroupMember"),
        ],
        permissions: vec![Permission::full("System Manager")],
        title_field: Some("name".into()),
        search_fields: vec!["name".into()],
        sort_field: Some("name".into()),
        sort_order: Some("asc".into()),
        autoname: Some("name".into()),
        naming_series: None,
    }
}

inventory::submit!(MetaEntry {
    name: "UserGroup",
    meta: usergroup_meta,
});

// ── UserGroupMember ──────────────────────────────────────────────────────────

pub fn usergroupmember_meta() -> DocTypeMeta {
    DocTypeMeta {
        name: "UserGroupMember".into(),
        module: "Core".into(),
        is_single: false,
        is_tree: false,
        is_child: true,
        is_submittable: false,
        track_changes: false,
        fields: vec![
            DocField::new("parent", "Parent", FieldType::Link)
                .options("UserGroup")
                .hidden(),
            DocField::new("parenttype", "Parent Type", FieldType::Data)
                .hidden(),
            DocField::new("parentfield", "Parent Field", FieldType::Data)
                .hidden(),
            DocField::new("idx", "Index", FieldType::Int)
                .hidden(),
            DocField::new("user", "User", FieldType::Link)
                .options("User")
                .required(),
        ],
        permissions: vec![Permission::full("System Manager")],
        title_field: Some("user".into()),
        search_fields: vec!["user".into()],
        sort_field: None,
        sort_order: None,
        autoname: None,
        naming_series: None,
    }
}

inventory::submit!(MetaEntry {
    name: "UserGroupMember",
    meta: usergroupmember_meta,
});

// ── UserType ──────────────────────────────────────────────────────────────────

pub fn usertype_meta() -> DocTypeMeta {
    DocTypeMeta {
        name: "UserType".into(),
        module: "Core".into(),
        is_single: false,
        is_tree: false,
        is_child: false,
        is_submittable: false,
        track_changes: false,
        fields: vec![
            DocField::new("name", "Name", FieldType::Data)
                .required()
                .unique()
                .in_list(),
            DocField::new("description", "Description", FieldType::Text),
            DocField::new("user_type_name", "User Type Name", FieldType::Data)
                .required(),
        ],
        permissions: vec![Permission::full("System Manager")],
        title_field: Some("name".into()),
        search_fields: vec!["name".into()],
        sort_field: Some("name".into()),
        sort_order: Some("asc".into()),
        autoname: Some("name".into()),
        naming_series: None,
    }
}

inventory::submit!(MetaEntry {
    name: "UserType",
    meta: usertype_meta,
});
