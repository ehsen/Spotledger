//! Tests for `spotledger_db::permissions` — PermissionType, Role, PermissionLevel, User.
//!
//! ## Frappe parity
//! | Frappe test | This test |
//! |---|---|
//! | `test_permissions.py::test_permission_type_list` | `test_standard_rights_count` |
//! | `test_permissions.py::test_permission_db_field` | `test_permission_type_db_field` |
//! | `test_permissions.py::test_permission_from_str` | `test_permission_type_from_str` |
//! | `test_permissions.py::test_administrator_always_allowed` | `test_role_administrator` |
//! | `test_permissions.py::test_system_manager_always_allowed` | `test_role_system_manager` |
//! | `test_permissions.py::test_role_equality` | `test_role_equality` |
//! | `test_permissions.py::test_permlevel` | `test_permission_level_numeric` |
//! | `test_custom_docperm.py::test_perm_level` | `test_permission_level_child_table` |
//! | `test_permissions.py::test_user_has_roles` | `test_user_with_roles` |
//! | `test_permissions.py::test_user_is_admin` | `test_user_admin_role` |

use spotledger_db::permissions::{PermissionLevel, PermissionType, Role, User};

// ── PermissionType ────────────────────────────────────────────────────────────

/// Frappe parity: Frappe defines 14 permission types — we must have the same set.
#[test]
fn test_standard_rights_count() {
    assert_eq!(PermissionType::STANDARD_RIGHTS.len(), 14);
}

/// Frappe parity: `perms.py::test_permission_db_field`
/// The special keyword collision fix: `delete` → `perm_delete` in SurrealDB.
#[test]
fn test_permission_type_db_field_delete_renamed() {
    // SurrealDB v3 reserved keyword workaround: `delete` field renamed to `perm_delete`
    assert_eq!(PermissionType::Delete.as_db_field(), "perm_delete");
}

/// Non-renamed fields map to their lowercase names.
#[test]
fn test_permission_type_db_field() {
    assert_eq!(PermissionType::Read.as_db_field(),   "read");
    assert_eq!(PermissionType::Write.as_db_field(),  "write");
    assert_eq!(PermissionType::Create.as_db_field(), "create");
    assert_eq!(PermissionType::Submit.as_db_field(), "submit");
    assert_eq!(PermissionType::Cancel.as_db_field(), "cancel");
    assert_eq!(PermissionType::Amend.as_db_field(),  "amend");
    assert_eq!(PermissionType::Print.as_db_field(),  "print");
    assert_eq!(PermissionType::Email.as_db_field(),  "email");
    assert_eq!(PermissionType::Report.as_db_field(), "report");
    assert_eq!(PermissionType::Import.as_db_field(), "import");
    assert_eq!(PermissionType::Export.as_db_field(), "export");
    assert_eq!(PermissionType::Share.as_db_field(),  "share");
    assert_eq!(PermissionType::Select.as_db_field(), "select");
}

/// Frappe parity: `test_permissions.py::test_permission_from_str`
/// All standard permission strings round-trip through `from_str`.
#[test]
fn test_permission_type_from_str_roundtrip() {
    let cases = [
        ("select", PermissionType::Select),
        ("read",   PermissionType::Read),
        ("write",  PermissionType::Write),
        ("create", PermissionType::Create),
        ("delete", PermissionType::Delete),
        ("submit", PermissionType::Submit),
        ("cancel", PermissionType::Cancel),
        ("amend",  PermissionType::Amend),
        ("print",  PermissionType::Print),
        ("email",  PermissionType::Email),
        ("report", PermissionType::Report),
        ("import", PermissionType::Import),
        ("export", PermissionType::Export),
        ("share",  PermissionType::Share),
    ];
    for (s, expected) in cases {
        let parsed = PermissionType::from_str(s);
        assert_eq!(parsed, Some(expected), "from_str({s:?}) must return {expected:?}");
    }
}

/// Unknown permission string returns None.
#[test]
fn test_permission_type_from_str_unknown() {
    assert_eq!(PermissionType::from_str("nonexistent"), None);
    assert_eq!(PermissionType::from_str(""), None);
    assert_eq!(PermissionType::from_str("Read"), None, "case-sensitive");
}

/// `display_name()` returns a human-readable label.
#[test]
fn test_permission_type_display_name() {
    assert_eq!(PermissionType::Read.display_name(),   "Read");
    assert_eq!(PermissionType::Write.display_name(),  "Write");
    assert_eq!(PermissionType::Delete.display_name(), "Delete");
    assert_eq!(PermissionType::Submit.display_name(), "Submit");
}

/// PermissionType equality and hash.
#[test]
fn test_permission_type_equality() {
    assert_eq!(PermissionType::Read, PermissionType::Read);
    assert_ne!(PermissionType::Read, PermissionType::Write);
}

/// All STANDARD_RIGHTS are distinct.
#[test]
fn test_standard_rights_all_distinct() {
    use std::collections::HashSet;
    let set: HashSet<PermissionType> = PermissionType::STANDARD_RIGHTS
        .iter()
        .copied()
        .collect();
    assert_eq!(set.len(), PermissionType::STANDARD_RIGHTS.len(), "all standard rights must be unique");
}

// ── Role ──────────────────────────────────────────────────────────────────────

/// Frappe parity: administrator role constant.
#[test]
fn test_role_administrator() {
    let r = Role::administrator();
    assert_eq!(r.as_str(), "Administrator");
}

/// Frappe parity: system manager role constant.
#[test]
fn test_role_system_manager() {
    let r = Role::system_manager();
    assert_eq!(r.as_str(), "System Manager");
}

/// Frappe parity: `test_permissions.py::test_role_equality`
/// Roles with the same name are equal.
#[test]
fn test_role_equality() {
    let r1 = Role::new("Sales User");
    let r2 = Role::new("Sales User");
    let r3 = Role::new("Accounts User");
    assert_eq!(r1, r2);
    assert_ne!(r1, r3);
}

/// Role can be used in a HashSet (implements Hash).
#[test]
fn test_role_hashable() {
    use std::collections::HashSet;
    let mut set = HashSet::new();
    set.insert(Role::new("Sales User"));
    set.insert(Role::new("Sales User")); // duplicate
    set.insert(Role::new("Accounts User"));
    assert_eq!(set.len(), 2, "duplicate role must be deduplicated in HashSet");
}

/// Roles are ordered by name (BTreeSet support).
#[test]
fn test_role_ordered() {
    use std::collections::BTreeSet;
    let mut set = BTreeSet::new();
    set.insert(Role::new("Z Role"));
    set.insert(Role::new("A Role"));
    let roles: Vec<_> = set.iter().collect();
    assert_eq!(roles[0].as_str(), "A Role");
    assert_eq!(roles[1].as_str(), "Z Role");
}

// ── PermissionLevel ───────────────────────────────────────────────────────────

/// Frappe parity: `test_permissions.py::test_permlevel`
/// Level 0 = DocType.
#[test]
fn test_permission_level_numeric() {
    assert_eq!(PermissionLevel::DocType.as_u32(), 0);
    assert_eq!(PermissionLevel::from_u32(0), PermissionLevel::DocType);
}

/// Frappe parity: `test_custom_docperm.py::test_perm_level`
/// Levels 1+ are ChildTable variants.
#[test]
fn test_permission_level_child_table() {
    assert_eq!(PermissionLevel::ChildTable(1).as_u32(), 1);
    assert_eq!(PermissionLevel::ChildTable(2).as_u32(), 2);
    assert_eq!(PermissionLevel::from_u32(1), PermissionLevel::ChildTable(1));
    assert_eq!(PermissionLevel::from_u32(3), PermissionLevel::ChildTable(3));
}

#[test]
fn test_permission_level_equality() {
    assert_eq!(PermissionLevel::DocType, PermissionLevel::DocType);
    assert_ne!(PermissionLevel::DocType, PermissionLevel::ChildTable(1));
    assert_ne!(PermissionLevel::ChildTable(1), PermissionLevel::ChildTable(2));
}

// ── User ──────────────────────────────────────────────────────────────────────

/// Frappe parity: `test_permissions.py::test_user_has_roles`
/// User can carry a list of roles.
#[test]
fn test_user_with_roles() {
    let user = User::new("test@example.com", vec![
        Role::new("Sales User"),
        Role::new("Accounts User"),
    ]);
    assert_eq!(user.name, "test@example.com");
    assert_eq!(user.roles.len(), 2);
    assert!(user.roles.contains(&Role::new("Sales User")));
    assert!(user.roles.contains(&Role::new("Accounts User")));
}

/// Frappe parity: `test_permissions.py::test_user_is_admin`
/// A user with the Administrator role is detected.
#[test]
fn test_user_admin_role() {
    let admin = User::new("Administrator", vec![Role::administrator()]);
    assert!(admin.roles.contains(&Role::administrator()));
}

/// User with no roles.
#[test]
fn test_user_no_roles() {
    let user = User::new("nobody@example.com", vec![]);
    assert!(user.roles.is_empty());
}
