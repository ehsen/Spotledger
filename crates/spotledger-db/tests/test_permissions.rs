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

// ── Integration: SurrealDB fn::permissions — RBAC graph traversal ─────────────
//
// Requires a live SurrealDB instance at SURREAL_TEST_URL (default 8500).
// Run with: cargo test -p spotledger-db --features integration --test test_permissions

#[cfg(feature = "integration")]
mod integration {
    use serde_json::json;
    use spotledger_core::config::DatabaseConfig;
    use spotledger_db::permissions::{
        get_doc_permissions, get_user_roles, has_permission, PermissionType, Role,
    };
    use spotledger_db::schema::apply_permissions_functions;
    use spotledger_db::DbAdapter;

    async fn test_adapter(db_suffix: &str) -> DbAdapter {
        let url = std::env::var("SURREAL_TEST_URL")
            .unwrap_or_else(|_| "ws://127.0.0.1:8500".to_string());
        let cfg = DatabaseConfig {
            url,
            user: "root".into(),
            pass: "root".into(),
            ns: "test_ns".into(),
            db: format!("test_perms_{db_suffix}"),
        };
        DbAdapter::connect(&cfg).await.expect("connect to test SurrealDB")
    }

    async fn setup_tables(adapter: &DbAdapter) {
        adapter
            .execute(
                "DEFINE TABLE IF NOT EXISTS tabUser     SCHEMALESS;
                 DEFINE TABLE IF NOT EXISTS tabRole     SCHEMALESS;
                 DEFINE TABLE IF NOT EXISTS tabHas_Role SCHEMALESS;
                 DEFINE TABLE IF NOT EXISTS tabDocPerm  SCHEMALESS;",
                vec![],
            )
            .await
            .expect("setup tables");
        apply_permissions_functions(adapter)
            .await
            .expect("apply permission functions");
    }

    async fn create_role(adapter: &DbAdapter, role_name: &str) {
        adapter
            .execute(
                &format!("UPSERT tabRole:`{role_name}` SET name = '{role_name}', disabled = 0;"),
                vec![],
            )
            .await
            .expect("create role");
    }

    async fn create_user(adapter: &DbAdapter, email: &str) {
        adapter
            .execute(
                &format!(
                    "UPSERT tabUser:`{email}` SET name = '{email}', email = '{email}', enabled = 1;"
                ),
                vec![],
            )
            .await
            .expect("create user");
    }

    async fn assign_role(adapter: &DbAdapter, user: &str, role: &str) {
        let id = format!("{user}-{role}").replace(' ', "_");
        adapter
            .execute(
                &format!(
                    "UPSERT tabHas_Role:`{id}` SET \
                     name = '{id}', parent = '{user}', role = '{role}', parenttype = 'User';"
                ),
                vec![],
            )
            .await
            .expect("assign role");
    }

    async fn create_docperm(
        adapter: &DbAdapter,
        doctype: &str,
        role: &str,
        read: bool,
        write: bool,
        create: bool,
        delete: bool,
    ) {
        let id = format!("{doctype}-{role}").replace(' ', "_");
        let r = read as i32;
        let w = write as i32;
        let c = create as i32;
        let d = delete as i32;
        adapter
            .execute(
                &format!(
                    "UPSERT tabDocPerm:`{id}` SET \
                     name = '{id}', parent = '{doctype}', role = '{role}', permlevel = 0, \
                     read = {r}, write = {w}, perm_create = {c}, perm_delete = {d}, \
                     submit = 0, perm_cancel = 0, amend = 0, print = {r}, email = {r}, \
                     report = {r}, import = 0, export = 0, share = 0, perm_select = {r};"
                ),
                vec![],
            )
            .await
            .expect("create docperm");
    }

    // ── Tests ─────────────────────────────────────────────────────────────────

    /// Administrator always has permission — no DB records needed.
    #[tokio::test]
    async fn test_administrator_bypass() {
        let adapter = test_adapter("admin").await;
        setup_tables(&adapter).await;

        for ptype in PermissionType::STANDARD_RIGHTS {
            let ok = has_permission(&adapter, "Administrator", "Employee", *ptype)
                .await
                .expect("has_permission");
            assert!(ok, "Administrator must have {:?}", ptype.as_db_field());
        }
    }

    /// System Manager role grants all doctype-level permissions.
    #[tokio::test]
    async fn test_system_manager_bypass() {
        let adapter = test_adapter("sysmgr").await;
        setup_tables(&adapter).await;

        let user = "sysmgr@test.example.com";
        create_user(&adapter, user).await;
        create_role(&adapter, "System Manager").await;
        assign_role(&adapter, user, "System Manager").await;

        for ptype in PermissionType::STANDARD_RIGHTS {
            let ok = has_permission(&adapter, user, "Employee", *ptype)
                .await
                .expect("has_permission");
            assert!(ok, "System Manager must have {:?}", ptype.as_db_field());
        }
    }

    /// Read-only user can read but is blocked from write/create/delete.
    #[tokio::test]
    async fn test_readonly_user_read_only() {
        let adapter = test_adapter("ro").await;
        setup_tables(&adapter).await;

        let user = "emp.reader@test.example.com";
        create_user(&adapter, user).await;
        create_role(&adapter, "Employee Reader").await;
        assign_role(&adapter, user, "Employee Reader").await;
        create_docperm(&adapter, "Employee", "Employee Reader", true, false, false, false).await;

        assert!(
            has_permission(&adapter, user, "Employee", PermissionType::Read).await.unwrap(),
            "read must be allowed"
        );
        assert!(
            !has_permission(&adapter, user, "Employee", PermissionType::Write).await.unwrap(),
            "write must be denied"
        );
        assert!(
            !has_permission(&adapter, user, "Employee", PermissionType::Create).await.unwrap(),
            "create must be denied"
        );
        assert!(
            !has_permission(&adapter, user, "Employee", PermissionType::Delete).await.unwrap(),
            "delete must be denied"
        );
    }

    /// User with no roles is denied all access.
    #[tokio::test]
    async fn test_no_roles_user_denied() {
        let adapter = test_adapter("noroles").await;
        setup_tables(&adapter).await;

        let user = "noroles@test.example.com";
        create_user(&adapter, user).await;

        for ptype in [PermissionType::Read, PermissionType::Write, PermissionType::Create] {
            let ok = has_permission(&adapter, user, "Employee", ptype).await.unwrap();
            assert!(!ok, "No-role user must be denied {:?}", ptype.as_db_field());
        }
    }

    /// Permissions from multiple roles are OR-ed together.
    #[tokio::test]
    async fn test_multiple_roles_or_logic() {
        let adapter = test_adapter("multi").await;
        setup_tables(&adapter).await;

        let user = "multi@test.example.com";
        create_user(&adapter, user).await;
        create_role(&adapter, "Read Only").await;
        create_role(&adapter, "Employee Writer").await;
        assign_role(&adapter, user, "Read Only").await;
        assign_role(&adapter, user, "Employee Writer").await;
        create_docperm(&adapter, "Employee", "Read Only",       true, false, false, false).await;
        create_docperm(&adapter, "Employee", "Employee Writer", false, true, false, false).await;

        assert!(
            has_permission(&adapter, user, "Employee", PermissionType::Read).await.unwrap(),
            "should have read from Read Only role"
        );
        assert!(
            has_permission(&adapter, user, "Employee", PermissionType::Write).await.unwrap(),
            "should have write from Employee Writer role"
        );
        assert!(
            !has_permission(&adapter, user, "Employee", PermissionType::Create).await.unwrap(),
            "should NOT have create"
        );
    }

    /// get_doc_permissions() returns correct full bitmap.
    #[tokio::test]
    async fn test_get_all_returns_correct_map() {
        let adapter = test_adapter("getall").await;
        setup_tables(&adapter).await;

        let user = "getall@test.example.com";
        create_user(&adapter, user).await;
        create_role(&adapter, "EmpViewer").await;
        assign_role(&adapter, user, "EmpViewer").await;
        create_docperm(&adapter, "Employee", "EmpViewer", true, false, false, false).await;

        let perms = get_doc_permissions(&adapter, user, "Employee")
            .await
            .expect("get_doc_permissions");

        assert!(perms.read,    "read must be true");
        assert!(!perms.write,  "write must be false");
        assert!(!perms.create, "create must be false");
        assert!(!perms.delete, "delete must be false");
        assert!(perms.print,   "print follows read");
        assert!(perms.email,   "email follows read");
    }

    /// get_user_roles() returns the assigned roles.
    #[tokio::test]
    async fn test_get_roles_returns_user_roles() {
        let adapter = test_adapter("getroles").await;
        setup_tables(&adapter).await;

        let user = "getroles@test.example.com";
        create_user(&adapter, user).await;
        create_role(&adapter, "HR Manager").await;
        create_role(&adapter, "Employee Reader").await;
        assign_role(&adapter, user, "HR Manager").await;
        assign_role(&adapter, user, "Employee Reader").await;

        let roles = get_user_roles(&adapter, user).await.expect("get_user_roles");
        let names: Vec<&str> = roles.iter().map(Role::as_str).collect();

        assert!(names.contains(&"HR Manager"),     "HR Manager should be in roles");
        assert!(names.contains(&"Employee Reader"), "Employee Reader should be in roles");
        assert_eq!(roles.len(), 2);
    }

    /// fn::permissions::has() SurrealDB function — direct call.
    #[tokio::test]
    async fn test_surreal_fn_has() {
        let adapter = test_adapter("fn_has").await;
        setup_tables(&adapter).await;

        let user = "fn.has@test.example.com";
        create_user(&adapter, user).await;
        create_role(&adapter, "FnReader").await;
        assign_role(&adapter, user, "FnReader").await;
        create_docperm(&adapter, "Employee", "FnReader", true, false, false, false).await;

        let check = |ptype: &str| {
            let ptype = ptype.to_owned();
            let adapter = &adapter;
            let user = user.to_owned();
            async move {
                adapter
                    .run(
                        "RETURN fn::permissions::has($user, $doctype, $ptype);",
                        vec![
                            ("user".into(),    json!(user)),
                            ("doctype".into(), json!("Employee")),
                            ("ptype".into(),   json!(ptype)),
                        ],
                    )
                    .await
                    .expect("fn::permissions::has")
                    .into_iter()
                    .next()
                    .and_then(|v| v.as_bool().or_else(|| v.as_i64().map(|n| n != 0)))
                    .unwrap_or(false)
            }
        };

        assert!(check("read").await,   "fn has: read should be true");
        assert!(!check("write").await, "fn has: write should be false");
    }

    /// fn::permissions::get_all() returns a complete permission object.
    #[tokio::test]
    async fn test_surreal_fn_get_all() {
        let adapter = test_adapter("fn_all").await;
        setup_tables(&adapter).await;

        let user = "fn.all@test.example.com";
        create_user(&adapter, user).await;
        create_role(&adapter, "AllReader").await;
        assign_role(&adapter, user, "AllReader").await;
        create_docperm(&adapter, "Employee", "AllReader", true, false, false, false).await;

        let rows = adapter
            .run(
                "RETURN fn::permissions::get_all($user, $doctype);",
                vec![
                    ("user".into(),    json!(user)),
                    ("doctype".into(), json!("Employee")),
                ],
            )
            .await
            .expect("fn::permissions::get_all");

        let obj = rows.into_iter().next().expect("non-null result");
        let get = |k: &str| obj.get(k).and_then(|v| v.as_bool());
        assert_eq!(get("read"),   Some(true),  "read must be true");
        assert_eq!(get("write"),  Some(false), "write must be false");
        assert_eq!(get("create"), Some(false), "create must be false");
        assert_eq!(get("delete"), Some(false), "delete must be false");
    }

    /// fn::permissions::get_roles() returns role list.
    #[tokio::test]
    async fn test_surreal_fn_get_roles() {
        let adapter = test_adapter("fn_roles").await;
        setup_tables(&adapter).await;

        let user = "fn.roles@test.example.com";
        create_user(&adapter, user).await;
        create_role(&adapter, "RoleA").await;
        create_role(&adapter, "RoleB").await;
        assign_role(&adapter, user, "RoleA").await;
        assign_role(&adapter, user, "RoleB").await;

        let rows = adapter
            .run(
                "RETURN fn::permissions::get_roles($user);",
                vec![("user".into(), json!(user))],
            )
            .await
            .expect("fn::permissions::get_roles");

        // The SurrealDB SDK may return the array either as a single Value::Array
        // row or as individual string rows depending on SDK version. Flatten both.
        let names: Vec<String> = rows
            .into_iter()
            .flat_map(|v| {
                if let Some(arr) = v.as_array() {
                    arr.iter()
                        .filter_map(|e| e.as_str().map(str::to_owned))
                        .collect::<Vec<_>>()
                } else if let Some(s) = v.as_str() {
                    vec![s.to_owned()]
                } else {
                    vec![]
                }
            })
            .collect();

        assert!(names.iter().any(|n| n == "RoleA"), "RoleA must be in result, got: {:?}", names);
        assert!(names.iter().any(|n| n == "RoleB"), "RoleB must be in result, got: {:?}", names);
    }

    /// Permissions for Employee don't bleed to Sales Invoice.
    #[tokio::test]
    async fn test_wrong_doctype_denied() {
        let adapter = test_adapter("bleed").await;
        setup_tables(&adapter).await;

        let user = "bleed@test.example.com";
        create_user(&adapter, user).await;
        create_role(&adapter, "EmpOnlyRole").await;
        assign_role(&adapter, user, "EmpOnlyRole").await;
        create_docperm(&adapter, "Employee", "EmpOnlyRole", true, false, false, false).await;

        assert!(
            has_permission(&adapter, user, "Employee", PermissionType::Read).await.unwrap(),
            "Should have read on Employee"
        );
        assert!(
            !has_permission(&adapter, user, "Sales Invoice", PermissionType::Read).await.unwrap(),
            "Should NOT have read on Sales Invoice"
        );
    }

    /// Two users have isolated permission sets.
    #[tokio::test]
    async fn test_two_users_isolated() {
        let adapter = test_adapter("isolated").await;
        setup_tables(&adapter).await;

        let user_a = "userA@test.example.com";
        let user_b = "userB@test.example.com";
        create_user(&adapter, user_a).await;
        create_user(&adapter, user_b).await;
        create_role(&adapter, "IsoRoleA").await;
        create_role(&adapter, "IsoRoleB").await;
        assign_role(&adapter, user_a, "IsoRoleA").await;
        assign_role(&adapter, user_b, "IsoRoleB").await;

        // IsoRoleA: read + write; IsoRoleB: read only
        create_docperm(&adapter, "Employee", "IsoRoleA", true, true,  false, false).await;
        create_docperm(&adapter, "Employee", "IsoRoleB", true, false, false, false).await;

        // user_a: read ✓, write ✓
        assert!(has_permission(&adapter, user_a, "Employee", PermissionType::Read).await.unwrap());
        assert!(has_permission(&adapter, user_a, "Employee", PermissionType::Write).await.unwrap());

        // user_b: read ✓, write ✗
        assert!(has_permission(&adapter, user_b, "Employee", PermissionType::Read).await.unwrap());
        assert!(!has_permission(&adapter, user_b, "Employee", PermissionType::Write).await.unwrap());
    }
}
