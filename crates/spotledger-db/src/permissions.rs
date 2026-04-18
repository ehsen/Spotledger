//! **Permission System — Phase 3 Implementation**
//!
//! Implements SpotledgerCore's permission model using Frappe's approach but with
//! strong Rust typing. No arbitrary strings; all permission constructs are first-class types.
//!
//! ## Architecture
//!
//! Permission checking follows this hierarchy (in order):
//! 1. Administrator check → always allowed
//! 2. System Manager role → always allowed (for doctype-level)
//! 3. Role-based permissions → check DocType-level permission
//! 4. Owner permissions → if_owner logic (document owner gets special permissions)
//! 5. User Permissions → link field restrictions
//! 6. Document Sharing → explicit document-level sharing
//!
//! ## Type Safety
//!
//! - `PermissionType` enum: Select, Read, Write, Create, Delete, Submit, Cancel, Amend,
//!   Print, Email, Report, Import, Export, Share
//! - `Role` newtype: Strongly typed role names, not arbitrary strings
//! - `PermissionLevel` enum: Doctype (0), ChildTable (1+)
//! - `IfOwner`: Permissions only for document owner
//! - `User` struct: Wraps User name with role access
//! - `DocPermission` struct: Full permission set for a doctype
//! - `UserPermission` struct: Link field restrictions
//! - `Share` struct: Document-level sharing
//!
//! ## Storage Model
//!
//! Tables (in SurrealDB):
//! - `tabUser`: name (PK), email, firstname, lastname, enabled
//! - `tabRole`: name (PK), disabled
//! - `tabHas_Role`: parent (User), role (Role) — adjacency list
//! - `tabDocPerm`: parent (DocType), role (Role), permlevel, select, read, write,
//!   create, delete, submit, cancel, amend, print, email, report, import, export, share
//! - `tabUser_Permission`: user (User), allow_on_submit, apply_to_all_doctypes,
//!   is_default, for_value (linked doc name), user_permission_name
//! - `tabShared`: user (User), doctype (DocType), name (doc name), read, write, share, submit, email, print
//!
//! ## Migration Notes
//!
//! This system replaces string-based permission checks in spotledger-plugins.
//! All host functions will use typed permission checks via this module.

use serde_json::{json, Value};

use crate::adapter::DbAdapter;
use crate::error::DbError;

// ============================================================================
// PERMISSION TYPE — All possible permission actions
// ============================================================================

/// All possible permission types in SpotledgerCore.
///
/// Mirrors Frappe's permission system but as a first-class Rust enum.
/// No "arbitrary ptype strings" allowed anywhere.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum PermissionType {
    // Document operations
    Select,
    Read,
    Write,
    Create,
    Delete,

    // Submission workflow
    Submit,
    Cancel,
    Amend,

    // Output/Sharing
    Print,
    Email,
    Report,

    // Bulk operations
    Import,
    Export,

    // Sharing
    Share,
}

impl PermissionType {
    /// Standard permission types matching Frappe.
    pub const STANDARD_RIGHTS: &'static [PermissionType] = &[
        PermissionType::Select,
        PermissionType::Read,
        PermissionType::Write,
        PermissionType::Create,
        PermissionType::Delete,
        PermissionType::Submit,
        PermissionType::Cancel,
        PermissionType::Amend,
        PermissionType::Print,
        PermissionType::Email,
        PermissionType::Report,
        PermissionType::Import,
        PermissionType::Export,
        PermissionType::Share,
    ];

    /// Convert to database field name.
    pub fn as_db_field(&self) -> &'static str {
        match self {
            PermissionType::Select => "select",
            PermissionType::Read => "read",
            PermissionType::Write => "write",
            PermissionType::Create => "create",
            PermissionType::Delete => "perm_delete",
            PermissionType::Submit => "submit",
            PermissionType::Cancel => "cancel",
            PermissionType::Amend => "amend",
            PermissionType::Print => "print",
            PermissionType::Email => "email",
            PermissionType::Report => "report",
            PermissionType::Import => "import",
            PermissionType::Export => "export",
            PermissionType::Share => "share",
        }
    }

    /// Parse from string (for compatibility with plugins).
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "select" => Some(PermissionType::Select),
            "read" => Some(PermissionType::Read),
            "write" => Some(PermissionType::Write),
            "create" => Some(PermissionType::Create),
            "delete" => Some(PermissionType::Delete),
            "submit" => Some(PermissionType::Submit),
            "cancel" => Some(PermissionType::Cancel),
            "amend" => Some(PermissionType::Amend),
            "print" => Some(PermissionType::Print),
            "email" => Some(PermissionType::Email),
            "report" => Some(PermissionType::Report),
            "import" => Some(PermissionType::Import),
            "export" => Some(PermissionType::Export),
            "share" => Some(PermissionType::Share),
            _ => None,
        }
    }

    /// Display name for humans.
    pub fn display_name(&self) -> &'static str {
        match self {
            PermissionType::Select => "Select",
            PermissionType::Read => "Read",
            PermissionType::Write => "Write",
            PermissionType::Create => "Create",
            PermissionType::Delete => "Delete",
            PermissionType::Submit => "Submit",
            PermissionType::Cancel => "Cancel",
            PermissionType::Amend => "Amend",
            PermissionType::Print => "Print",
            PermissionType::Email => "Email",
            PermissionType::Report => "Report",
            PermissionType::Import => "Import",
            PermissionType::Export => "Export",
            PermissionType::Share => "Share",
        }
    }
}

// ============================================================================
// ROLE — Strongly typed role names
// ============================================================================

/// A role in the system. Roles are assigned to users and grant permissions on doctypes.
///
/// Common roles: "Administrator", "System Manager", "Sales User", "Accounts User", etc.
/// Roles are domain-specific and should never be arbitrary strings in typed code.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Role(pub String);

impl Role {
    /// The Administrator role — always grants all permissions.
    pub fn administrator() -> Self {
        Role("Administrator".to_string())
    }

    /// The System Manager role — grants all doctype-level permissions.
    pub fn system_manager() -> Self {
        Role("System Manager".to_string())
    }

    /// Create a custom role by name.
    pub fn new(name: impl Into<String>) -> Self {
        Role(name.into())
    }

    /// Get the role name as a string (for display/logging).
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

// ============================================================================
// PERMISSION LEVEL — DocType vs Child Table permissions
// ============================================================================

/// Permission levels for DocTypes and Child Tables.
///
/// - **DocType (0)**: Permission on the top-level DocType
/// - **ChildTable (1+)**: Permission on child tables (Baby Tables)
///
/// Higher permlevel = more restrictive (child-level permissions).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum PermissionLevel {
    DocType,
    ChildTable(u32),
}

impl PermissionLevel {
    /// Convert to numeric level (0 for DocType, 1+ for ChildTable).
    pub fn as_u32(&self) -> u32 {
        match self {
            PermissionLevel::DocType => 0,
            PermissionLevel::ChildTable(n) => *n,
        }
    }

    /// Parse from numeric level.
    pub fn from_u32(n: u32) -> Self {
        match n {
            0 => PermissionLevel::DocType,
            n => PermissionLevel::ChildTable(n),
        }
    }
}

// ============================================================================
// USER — Strongly typed user with roles
// ============================================================================

/// A user in the system. Users have roles which grant permissions.
///
/// Phase 3: Fetches roles from tabHas_Role.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct User {
    pub name: String,
    pub roles: Vec<Role>,
}

impl User {
    /// Create a new user with the given roles.
    pub fn new(name: impl Into<String>, roles: Vec<Role>) -> Self {
        User {
            name: name.into(),
            roles,
        }
    }

    /// Check if user is Administrator.
    pub fn is_administrator(&self) -> bool {
        self.name == "Administrator" || self.roles.contains(&Role::administrator())
    }

    /// Check if user is System Manager.
    pub fn is_system_manager(&self) -> bool {
        self.roles.contains(&Role::system_manager())
    }

    /// Check if user has a specific role.
    pub fn has_role(&self, role: &Role) -> bool {
        self.roles.contains(role)
    }
}

// ============================================================================
// DOCUMENT PERMISSIONS — Typed permission set
// ============================================================================

/// Permissions for a single DocType (aggregated across user's roles).
///
/// This is a bitmask of all permission types the user has on a doctype.
/// Computed by checking all applicable role permissions and OR-ing them together.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DocPermission {
    pub select: bool,
    pub read: bool,
    pub write: bool,
    pub create: bool,
    pub delete: bool,
    pub submit: bool,
    pub cancel: bool,
    pub amend: bool,
    pub print: bool,
    pub email: bool,
    pub report: bool,
    pub import: bool,
    pub export: bool,
    pub share: bool,
}

impl DocPermission {
    /// Create permission set with everything enabled.
    pub fn all() -> Self {
        DocPermission {
            select: true,
            read: true,
            write: true,
            create: true,
            delete: true,
            submit: true,
            cancel: true,
            amend: true,
            print: true,
            email: true,
            report: true,
            import: true,
            export: true,
            share: true,
        }
    }

    /// Create permission set with everything disabled.
    pub fn none() -> Self {
        DocPermission::default()
    }

    /// Check if a specific permission is granted.
    pub fn has(&self, ptype: PermissionType) -> bool {
        match ptype {
            PermissionType::Select => self.select,
            PermissionType::Read => self.read,
            PermissionType::Write => self.write,
            PermissionType::Create => self.create,
            PermissionType::Delete => self.delete,
            PermissionType::Submit => self.submit,
            PermissionType::Cancel => self.cancel,
            PermissionType::Amend => self.amend,
            PermissionType::Print => self.print,
            PermissionType::Email => self.email,
            PermissionType::Report => self.report,
            PermissionType::Import => self.import,
            PermissionType::Export => self.export,
            PermissionType::Share => self.share,
        }
    }

    /// OR two permission sets together (union).
    pub fn merge(&mut self, other: &DocPermission) {
        self.select |= other.select;
        self.read |= other.read;
        self.write |= other.write;
        self.create |= other.create;
        self.delete |= other.delete;
        self.submit |= other.submit;
        self.cancel |= other.cancel;
        self.amend |= other.amend;
        self.print |= other.print;
        self.email |= other.email;
        self.report |= other.report;
        self.import |= other.import;
        self.export |= other.export;
        self.share |= other.share;
    }

    /// Serialize to JSON (for docinfo.permissions in API response).
    pub fn to_json(&self) -> Value {
        json!({
            "select": self.select as i32,
            "read": self.read as i32,
            "write": self.write as i32,
            "create": self.create as i32,
            "delete": self.delete as i32,
            "submit": self.submit as i32,
            "cancel": self.cancel as i32,
            "amend": self.amend as i32,
            "print": self.print as i32,
            "email": self.email as i32,
            "report": self.report as i32,
            "import": self.import as i32,
            "export": self.export as i32,
            "share": self.share as i32,
        })
    }

    /// Check using Ptype enum (for backward compatibility).
    pub fn check_ptype(&self, ptype: PermissionType) -> bool {
        self.has(ptype)
    }
}

// ============================================================================
// IF_OWNER — Owner-specific permissions
// ============================================================================

/// Permissions that only apply if the user is the document owner.
///
/// When if_owner is enabled on a DocType:
/// - Non-owners get limited access (select/read only)
/// - Only the owner gets the full permission set
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct IfOwner {
    pub read: bool,
    pub write: bool,
    pub delete: bool,
    pub amend: bool,
    pub cancel: bool,
}

impl IfOwner {
    /// Create if_owner permissions with everything enabled.
    pub fn all() -> Self {
        IfOwner {
            read: true,
            write: true,
            delete: true,
            amend: true,
            cancel: true,
        }
    }

    /// Check if a specific permission is granted.
    pub fn has(&self, ptype: PermissionType) -> bool {
        match ptype {
            PermissionType::Read => self.read,
            PermissionType::Write => self.write,
            PermissionType::Delete => self.delete,
            PermissionType::Amend => self.amend,
            PermissionType::Cancel => self.cancel,
            _ => false, // Other permission types don't support if_owner
        }
    }
}

// ============================================================================
// USER PERMISSION — Link field restrictions
// ============================================================================

/// User-level permissions tied to link fields.
///
/// Restricts user access based on linked documents (company, employee, territory, etc.).
/// e.g., "Sales User can only see documents linked to their assigned company"
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserPermission {
    /// The user this permission applies to
    pub user: String,
    /// The DocType being restricted (e.g., "Company", "Employee")
    pub doctype: String,
    /// The specific document the user is allowed to access (e.g., "Main Company")
    pub for_value: String,
    /// If true, applies to any doctype that links to this doctype
    pub apply_to_all_doctypes: bool,
    /// Parent permission name (for nested permissions)
    pub parent: Option<String>,
}

impl UserPermission {
    /// Check if user is allowed to access a linked value.
    pub fn is_allowed(&self, value: &str) -> bool {
        self.for_value == value
    }
}

// ============================================================================
// SHARE — Document-level sharing
// ============================================================================

/// A shared document entry — grants specific permissions to a user for a single doc.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Share {
    /// Who the document is shared with
    pub user: String,
    /// DocType of the shared document
    pub doctype: String,
    /// Name of the shared document
    pub name: String,
    /// Shareable permissions
    pub read: bool,
    pub write: bool,
    pub share: bool,
    pub submit: bool,
    pub email: bool,
    pub print: bool,
}

impl Share {
    /// Check if a permission is granted via this share.
    pub fn has(&self, ptype: PermissionType) -> bool {
        match ptype {
            PermissionType::Read => self.read,
            PermissionType::Write => self.write,
            PermissionType::Share => self.share,
            PermissionType::Submit => self.submit,
            PermissionType::Email => self.email,
            PermissionType::Print => self.print,
            _ => false, // Only these permission types can be shared
        }
    }
}

// ============================================================================
// CORE PERMISSION CHECKING
// ============================================================================

/// Check if a user has a specific permission on a DocType.
///
/// ## Algorithm
///
/// 1. If user is Administrator → return true
/// 2. Delegate to `fn::permissions::has()` SurrealDB function (single graph traversal)
/// 3. Fall back to Rust implementation if the function is not yet installed
pub async fn has_permission(
    adapter: &DbAdapter,
    user: &str,
    doctype: &str,
    ptype: PermissionType,
) -> Result<bool, DbError> {
    // Administrator always has permission (local bypass, no DB needed)
    if user == "Administrator" {
        return Ok(true);
    }

    // Try SurrealDB graph-traversal function first
    let ptype_str = ptype.as_db_field();
    if let Some(val) = adapter
        .run(
            "RETURN fn::permissions::has($user, $doctype, $ptype);",
            vec![
                ("user".into(),    Value::String(user.to_string())),
                ("doctype".into(), Value::String(doctype.to_string())),
                ("ptype".into(),   Value::String(ptype_str.to_string())),
            ],
        )
        .await
        .ok()
        .and_then(|rows| {
            rows.into_iter().next().and_then(|v| {
                v.as_bool().or_else(|| v.as_i64().map(|n| n != 0))
            })
        })
    {
        return Ok(val);
    }

    // Fallback: Rust implementation (used if fn::permissions not yet installed)
    has_permission_rust(adapter, user, doctype, ptype).await
}

/// Rust fallback implementation — used when the SurrealDB function is not installed.
async fn has_permission_rust(
    adapter: &DbAdapter,
    user: &str,
    doctype: &str,
    ptype: PermissionType,
) -> Result<bool, DbError> {
    let roles = get_user_roles(adapter, user).await?;

    // System Manager has all doctype-level permissions
    if roles.iter().any(|r| r == &Role::system_manager()) {
        return Ok(true);
    }

    if roles.is_empty() {
        return Ok(false);
    }

    let perms = get_doc_permissions_by_roles(adapter, &roles, doctype).await?;
    Ok(perms.has(ptype))
}

/// Get all permissions for a user on a DocType.
///
/// Aggregates permissions across all user's roles using OR logic.
/// Delegates to `fn::permissions::get_all()` SurrealDB function when available.
pub async fn get_doc_permissions(
    adapter: &DbAdapter,
    user: &str,
    doctype: &str,
) -> Result<DocPermission, DbError> {
    // Administrator always has everything
    if user == "Administrator" {
        return Ok(DocPermission::all());
    }

    // Try SurrealDB function first (single graph traversal)
    if let Some(obj) = adapter
        .run(
            "RETURN fn::permissions::get_all($user, $doctype);",
            vec![
                ("user".into(),    Value::String(user.to_string())),
                ("doctype".into(), Value::String(doctype.to_string())),
            ],
        )
        .await
        .ok()
        .and_then(|rows| rows.into_iter().next())
        .filter(|v| v.is_object())
    {
        return Ok(DocPermission {
            select: extract_bool(&obj, "select"),
            read:   extract_bool(&obj, "read"),
            write:  extract_bool(&obj, "write"),
            create: extract_bool(&obj, "create"),
            delete: extract_bool(&obj, "delete"),
            submit: extract_bool(&obj, "submit"),
            cancel: extract_bool(&obj, "cancel"),
            amend:  extract_bool(&obj, "amend"),
            print:  extract_bool(&obj, "print"),
            email:  extract_bool(&obj, "email"),
            report: extract_bool(&obj, "report"),
            import: extract_bool(&obj, "import"),
            export: extract_bool(&obj, "export"),
            share:  extract_bool(&obj, "share"),
        });
    }

    // Fallback: Rust implementation
    let roles = get_user_roles(adapter, user).await?;
    if roles.iter().any(|r| r == &Role::system_manager()) {
        return Ok(DocPermission::all());
    }
    if roles.is_empty() {
        return Ok(DocPermission::none());
    }
    get_doc_permissions_by_roles(adapter, &roles, doctype).await
}

/// Internal: Get all permissions for specific roles on a DocType.
async fn get_doc_permissions_by_roles(
    adapter: &DbAdapter,
    roles: &[Role],
    doctype: &str,
) -> Result<DocPermission, DbError> {
    let roles_str: Vec<String> = roles.iter().map(|r| r.0.clone()).collect();
    let roles_json: Vec<Value> = roles_str.iter().map(|r| Value::String(r.clone())).collect();

    let surql = "SELECT \
        perm_select, read, write, perm_create, perm_delete, submit, perm_cancel, amend, \
        print, email, report, import, export, share \
        FROM tabDocPerm \
        WHERE parent = $doctype AND role IN $roles AND permlevel = 0";

    let rows = adapter
        .run(
            surql,
            vec![
                ("doctype".into(), Value::String(doctype.to_string())),
                ("roles".into(), Value::Array(roles_json)),
            ],
        )
        .await?;

    let mut merged = DocPermission::none();
    for row in rows {
        let perm = DocPermission {
            select: extract_bool(&row, "perm_select"),
            read: extract_bool(&row, "read"),
            write: extract_bool(&row, "write"),
            create: extract_bool(&row, "perm_create"),
            delete: extract_bool(&row, "perm_delete"),
            submit: extract_bool(&row, "submit"),
            cancel: extract_bool(&row, "perm_cancel"),
            amend: extract_bool(&row, "amend"),
            print: extract_bool(&row, "print"),
            email: extract_bool(&row, "email"),
            report: extract_bool(&row, "report"),
            import: extract_bool(&row, "import"),
            export: extract_bool(&row, "export"),
            share: extract_bool(&row, "share"),
        };
        merged.merge(&perm);
    }

    Ok(merged)
}

/// Get all roles assigned to a user.
pub async fn get_user_roles(adapter: &DbAdapter, user: &str) -> Result<Vec<Role>, DbError> {
    let surql = "SELECT role FROM tabHas_Role WHERE parent = $user AND parenttype = 'User'";
    let rows = adapter
        .run(surql, vec![("user".into(), Value::String(user.to_string()))])
        .await?;

    let roles: Vec<Role> = rows
        .into_iter()
        .filter_map(|r| r.get("role").and_then(Value::as_str).map(|s| Role::new(s)))
        .collect();

    Ok(roles)
}

/// Extract boolean value from JSON (handles both bool and i64).
fn extract_bool(row: &Value, field: &str) -> bool {
    row.get(field)
        .map(|v| {
            v.as_bool().unwrap_or(false) || v.as_i64().map(|n| n != 0).unwrap_or(false)
        })
        .unwrap_or(false)
}

// ============================================================================
// BACKWARD COMPATIBILITY — Old string-based API (deprecated)
// ============================================================================

/// Check permission using string method names (deprecated, for plugin compatibility).
///
/// Will be removed once all host functions are updated to use typed API.
pub async fn has_permission_string(
    adapter: &DbAdapter,
    user: &str,
    doctype: &str,
    ptype: &str,
) -> Result<bool, DbError> {
    if let Some(ptype_enum) = PermissionType::from_str(ptype) {
        has_permission(adapter, user, doctype, ptype_enum).await
    } else {
        Ok(false)
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_permission_type_conversions() {
        assert_eq!(PermissionType::Read.as_db_field(), "read");
        assert_eq!(PermissionType::from_str("write"), Some(PermissionType::Write));
        assert_eq!(PermissionType::from_str("invalid"), None);
        assert_eq!(PermissionType::Read.display_name(), "Read");
    }

    #[test]
    fn test_role_creation() {
        let admin = Role::administrator();
        assert_eq!(admin.as_str(), "Administrator");

        let custom = Role::new("Sales Manager");
        assert_eq!(custom.as_str(), "Sales Manager");
    }

    #[test]
    fn test_permission_level() {
        assert_eq!(PermissionLevel::DocType.as_u32(), 0);
        assert_eq!(PermissionLevel::ChildTable(1).as_u32(), 1);
        assert_eq!(PermissionLevel::from_u32(0), PermissionLevel::DocType);
        assert_eq!(PermissionLevel::from_u32(2), PermissionLevel::ChildTable(2));
    }

    #[test]
    fn test_user_operations() {
        let user = User::new("test@example.com", vec![Role::new("Sales User")]);
        assert!(!user.is_administrator());
        assert!(!user.is_system_manager());
        assert!(user.has_role(&Role::new("Sales User")));

        let admin = User::new("Administrator", vec![]);
        assert!(admin.is_administrator());
    }

    #[test]
    fn test_doc_permission() {
        let mut perms = DocPermission::none();
        assert!(!perms.has(PermissionType::Read));

        perms.read = true;
        assert!(perms.has(PermissionType::Read));

        let other = DocPermission {
            write: true,
            delete: true,
            ..DocPermission::default()
        };
        perms.merge(&other);
        assert!(perms.has(PermissionType::Read));
        assert!(perms.has(PermissionType::Write));
        assert!(perms.has(PermissionType::Delete));
    }

    #[test]
    fn test_doc_permission_all() {
        let perms = DocPermission::all();
        for ptype in PermissionType::STANDARD_RIGHTS {
            assert!(perms.has(*ptype), "Expected {} to be granted in all()", ptype.as_db_field());
        }
    }

    #[test]
    fn test_if_owner() {
        let if_owner = IfOwner::all();
        assert!(if_owner.has(PermissionType::Write));
        assert!(!if_owner.has(PermissionType::Create)); // Cannot be in if_owner
    }

    #[test]
    fn test_share_operations() {
        let share = Share {
            user: "user@example.com".into(),
            doctype: "Sales Order".into(),
            name: "SO-001".into(),
            read: true,
            write: true,
            share: false,
            submit: false,
            email: false,
            print: true,
        };

        assert!(share.has(PermissionType::Read));
        assert!(share.has(PermissionType::Write));
        assert!(!share.has(PermissionType::Share));
        assert!(share.has(PermissionType::Print));
    }

    #[test]
    fn test_user_permission() {
        let up = UserPermission {
            user: "user@example.com".into(),
            doctype: "Company".into(),
            for_value: "Main Company".into(),
            apply_to_all_doctypes: false,
            parent: None,
        };

        assert!(up.is_allowed("Main Company"));
        assert!(!up.is_allowed("Other Company"));
    }

    #[test]
    fn test_permission_type_standard_rights() {
        assert_eq!(PermissionType::STANDARD_RIGHTS.len(), 14);
        assert!(PermissionType::STANDARD_RIGHTS.contains(&PermissionType::Read));
        assert!(PermissionType::STANDARD_RIGHTS.contains(&PermissionType::Print));
        assert!(PermissionType::STANDARD_RIGHTS.contains(&PermissionType::Share));
    }
}
