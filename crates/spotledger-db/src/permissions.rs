//! Permission system — implements `has_permission` via SurrealDB graph traversal.
//!
//! Permission check logic mirrors `frappe.permissions.has_permission()`.
//!
//! Graph model (defined in bootstrap schema):
//!   User ->tabHas_Role-> Role
//!   Role ->tabDocPerm -> (DocType, ptype)
//!
//! tabDocPerm fields: parent (DocType), role, read, write, create, delete, submit, cancel, amend
//!
//! For Phase 2 we implement the basic role-based check.
//! User-level sharing (shared_doc edges) is a Phase 3 enhancement.

use serde_json::Value;

use crate::adapter::DbAdapter;
use crate::error::DbError;

/// Permission type names matching Frappe's ptype strings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ptype {
    Read,
    Write,
    Create,
    Delete,
    Submit,
    Cancel,
    Amend,
}

impl Ptype {
    pub fn as_field(&self) -> &'static str {
        match self {
            Ptype::Read => "read",
            Ptype::Write => "write",
            Ptype::Create => "create",
            Ptype::Delete => "delete",
            Ptype::Submit => "submit",
            Ptype::Cancel => "cancel",
            Ptype::Amend => "amend",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "read" => Some(Ptype::Read),
            "write" => Some(Ptype::Write),
            "create" => Some(Ptype::Create),
            "delete" => Some(Ptype::Delete),
            "submit" => Some(Ptype::Submit),
            "cancel" => Some(Ptype::Cancel),
            "amend" => Some(Ptype::Amend),
            _ => None,
        }
    }
}

/// Check whether `user` has `ptype` permission on `doctype`.
///
/// Algorithm:
/// 1. System Manager role always has all permissions.
/// 2. Fetch roles assigned to the user from `tabHas_Role`.
/// 3. Check `tabDocPerm` for any row matching (parent=doctype, role IN roles, <ptype>=1).
/// 4. Return true if any matching row is found.
pub async fn has_permission(
    adapter: &DbAdapter,
    user: &str,
    doctype: &str,
    ptype: &str,
) -> Result<bool, DbError> {
    // Administrator always has permission
    if user == "Administrator" {
        return Ok(true);
    }

    // Fetch user's roles
    let roles = get_user_roles(adapter, user).await?;

    if roles.contains(&"System Manager".to_string()) {
        return Ok(true);
    }

    if roles.is_empty() {
        return Ok(false);
    }

    // Validate ptype
    let ptype_field = Ptype::from_str(ptype)
        .map(|p| p.as_field())
        .unwrap_or("read");

    // Query DocPerm for matching role + doctype + ptype
    // tabDocPerm.parent = doctype, tabDocPerm.role IN roles, tabDocPerm.<ptype> = 1
    let roles_json: Vec<Value> = roles.iter().map(|r| Value::String(r.clone())).collect();

    let surql = format!(
        "SELECT count() > 0 AS ok FROM tabDocPerm \
         WHERE parent = $doctype AND role IN $roles AND `{ptype_field}` = 1 \
         LIMIT 1"
    );

    let rows = adapter
        .run(
            &surql,
            vec![
                ("doctype".into(), doctype.into()),
                ("roles".into(),   Value::Array(roles_json)),
            ],
        )
        .await?;
    let ok = rows
        .first()
        .and_then(|r| r.get("ok"))
        .and_then(Value::as_bool)
        .unwrap_or(false);

    Ok(ok)
}

/// Fetch all role names assigned to a user from `tabHas_Role`.
pub async fn get_user_roles(adapter: &DbAdapter, user: &str) -> Result<Vec<String>, DbError> {
    let rows = adapter
        .run(
            "SELECT role FROM tabHas_Role WHERE parent = $user AND parenttype = 'User'",
            vec![("user".into(), user.into())],
        )
        .await?;
    let roles: Vec<String> = rows
        .into_iter()
        .filter_map(|r| r.get("role").and_then(Value::as_str).map(str::to_owned))
        .collect();
    Ok(roles)
}

/// Get all permissions for a doctype for a given user.
/// Returns a map of ptype → bool.
pub async fn get_doc_permissions(
    adapter: &DbAdapter,
    user: &str,
    doctype: &str,
) -> Result<DocPermissions, DbError> {
    if user == "Administrator" {
        return Ok(DocPermissions::all());
    }

    let roles = get_user_roles(adapter, user).await?;

    if roles.contains(&"System Manager".to_string()) {
        return Ok(DocPermissions::all());
    }

    if roles.is_empty() {
        return Ok(DocPermissions::none());
    }

    let roles_json: Vec<Value> = roles.iter().map(|r| Value::String(r.clone())).collect();

    let rows = adapter
        .run(
            "SELECT read, write, create, delete, submit, cancel, amend \
             FROM tabDocPerm \
             WHERE parent = $doctype AND role IN $roles",
            vec![
                ("doctype".into(), doctype.into()),
                ("roles".into(),   Value::Array(roles_json)),
            ],
        )
        .await?;

    let mut perms = DocPermissions::none();
    for row in rows {
        perms.read |= bool_field(&row, "read");
        perms.write |= bool_field(&row, "write");
        perms.create |= bool_field(&row, "create");
        perms.delete |= bool_field(&row, "delete");
        perms.submit |= bool_field(&row, "submit");
        perms.cancel |= bool_field(&row, "cancel");
        perms.amend |= bool_field(&row, "amend");
    }

    Ok(perms)
}

fn bool_field(row: &Value, field: &str) -> bool {
    row.get(field).map(|v| {
        v.as_bool().unwrap_or(false) || v.as_i64().map(|n| n != 0).unwrap_or(false)
    }).unwrap_or(false)
}

/// Aggregate permissions for a doctype for a user (union of all matching roles).
#[derive(Debug, Clone, Default)]
pub struct DocPermissions {
    pub read: bool,
    pub write: bool,
    pub create: bool,
    pub delete: bool,
    pub submit: bool,
    pub cancel: bool,
    pub amend: bool,
}

impl DocPermissions {
    pub fn all() -> Self {
        Self {
            read: true,
            write: true,
            create: true,
            delete: true,
            submit: true,
            cancel: true,
            amend: true,
        }
    }

    pub fn none() -> Self {
        Self::default()
    }

    /// Returns true if the user has the given ptype.
    pub fn check(&self, ptype: &str) -> bool {
        match ptype {
            "read" => self.read,
            "write" => self.write,
            "create" => self.create,
            "delete" => self.delete,
            "submit" => self.submit,
            "cancel" => self.cancel,
            "amend" => self.amend,
            _ => false,
        }
    }

    /// Serialize to a JSON object (used by getdoc docinfo.permissions).
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "read":   self.read as i32,
            "write":  self.write as i32,
            "create": self.create as i32,
            "delete": self.delete as i32,
            "submit": self.submit as i32,
            "cancel": self.cancel as i32,
            "amend":  self.amend as i32,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ptype_roundtrip() {
        assert_eq!(Ptype::from_str("read").unwrap().as_field(), "read");
        assert_eq!(Ptype::from_str("submit").unwrap().as_field(), "submit");
        assert!(Ptype::from_str("unknown").is_none());
    }

    #[test]
    fn doc_permissions_check() {
        let p = DocPermissions {
            read: true,
            write: false,
            create: false,
            delete: false,
            submit: false,
            cancel: false,
            amend: false,
        };
        assert!(p.check("read"));
        assert!(!p.check("write"));
        assert!(!p.check("create"));
    }

    #[test]
    fn doc_permissions_all() {
        let p = DocPermissions::all();
        assert!(p.check("read"));
        assert!(p.check("write"));
        assert!(p.check("submit"));
    }
}
