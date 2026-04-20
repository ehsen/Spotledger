-- ============================================================================
-- Spotledger Permission Functions
-- ============================================================================
--
-- Graph-based RBAC: each permission check is a single SurrealDB traversal
-- through the user → role → doctype permission graph.
--
-- Tables used:
--   tabUser        – user records (name, enabled)
--   tabHas_Role    – adjacency list: parent=user, role=role_name, parenttype='User'
--   tabDocPerm     – doctype role permissions: parent=doctype, role, permlevel, read, write, ...
--
-- Field-name map (tabDocPerm stores some with perm_ prefix):
--   canonical  DB field
--   ─────────  ────────
--   select     perm_select
--   read       read
--   write      write
--   create     perm_create
--   delete     perm_delete
--   submit     submit
--   cancel     perm_cancel
--   amend      amend
--   print      print
--   email      email
--   report     report
--   import     import
--   export     export
--   share      share
-- ============================================================================

-- ----------------------------------------------------------------------------
-- fn::permissions::get_roles($user) → array<string>
--
-- Returns the list of role names for a user by traversing tabHas_Role.
-- Administrator always returns the three built-in system roles.
-- ----------------------------------------------------------------------------
DEFINE FUNCTION OVERWRITE fn::permissions::get_roles($user: string) {
    IF $user = 'Administrator' {
        RETURN ['Administrator', 'System Manager', 'All'];
    };
    RETURN SELECT VALUE role
           FROM tabHas_Role
           WHERE parent = $user
             AND parenttype = 'User';
};

-- ----------------------------------------------------------------------------
-- fn::permissions::has($user, $doctype, $ptype) → bool
--
-- Single-query RBAC check:
--   1. Administrator bypass → true
--   2. Traverse tabHas_Role to collect user's roles
--   3. System Manager → true
--   4. Query tabDocPerm for any matching row that grants $ptype
-- ----------------------------------------------------------------------------
DEFINE FUNCTION OVERWRITE fn::permissions::has(
    $user:    string,
    $doctype: string,
    $ptype:   string
) {
    -- 1. Administrator always has permission
    IF $user = 'Administrator' { RETURN true; };

    -- 2. Collect user's roles (graph traversal)
    LET $roles = SELECT VALUE role
                 FROM tabHas_Role
                 WHERE parent = $user
                   AND parenttype = 'User';

    -- 3. System Manager has all doctype-level permissions
    IF 'System Manager' IN $roles { RETURN true; };

    -- No roles → no access
    IF array::len($roles) = 0 { RETURN false; };

    -- 4. Fetch matching DocPerm rows
    LET $perms = SELECT *
                 FROM tabDocPerm
                 WHERE parent = $doctype
                   AND role IN $roles
                   AND permlevel = 0;

    -- 5. Check the requested permission type
    IF $ptype = 'read'   { RETURN array::any($perms.map(|$p| $p.read        = true OR $p.read        = 1)); };
    IF $ptype = 'write'  { RETURN array::any($perms.map(|$p| $p.write       = true OR $p.write       = 1)); };
    IF $ptype = 'create' { RETURN array::any($perms.map(|$p| $p.perm_create = true OR $p.perm_create = 1)); };
    IF $ptype = 'delete' { RETURN array::any($perms.map(|$p| $p.perm_delete = true OR $p.perm_delete = 1)); };
    IF $ptype = 'submit' { RETURN array::any($perms.map(|$p| $p.submit      = true OR $p.submit      = 1)); };
    IF $ptype = 'cancel' { RETURN array::any($perms.map(|$p| $p.perm_cancel = true OR $p.perm_cancel = 1)); };
    IF $ptype = 'amend'  { RETURN array::any($perms.map(|$p| $p.amend       = true OR $p.amend       = 1)); };
    IF $ptype = 'print'  { RETURN array::any($perms.map(|$p| $p.print       = true OR $p.print       = 1)); };
    IF $ptype = 'email'  { RETURN array::any($perms.map(|$p| $p.email       = true OR $p.email       = 1)); };
    IF $ptype = 'report' { RETURN array::any($perms.map(|$p| $p.report      = true OR $p.report      = 1)); };
    IF $ptype = 'import' { RETURN array::any($perms.map(|$p| $p.import      = true OR $p.import      = 1)); };
    IF $ptype = 'export' { RETURN array::any($perms.map(|$p| $p.export      = true OR $p.export      = 1)); };
    IF $ptype = 'share'  { RETURN array::any($perms.map(|$p| $p.share       = true OR $p.share       = 1)); };
    IF $ptype = 'select' { RETURN array::any($perms.map(|$p| $p.perm_select = true OR $p.perm_select = 1)); };

    RETURN false;
};

-- ----------------------------------------------------------------------------
-- fn::permissions::get_all($user, $doctype) → object
--
-- Returns a full permission map for $user on $doctype in one DB round-trip.
-- Used by the API to compute docinfo.permissions without multiple calls.
-- ----------------------------------------------------------------------------
DEFINE FUNCTION OVERWRITE fn::permissions::get_all(
    $user:    string,
    $doctype: string
) {
    LET $all = {
        select: true, read: true, write: true, create: true, delete: true,
        submit: true, cancel: true, amend: true, print: true, email: true,
        report: true, import: true, export: true, share: true
    };

    -- Administrator
    IF $user = 'Administrator' { RETURN $all; };

    -- Collect roles
    LET $roles = SELECT VALUE role
                 FROM tabHas_Role
                 WHERE parent = $user
                   AND parenttype = 'User';

    -- System Manager
    IF 'System Manager' IN $roles { RETURN $all; };

    -- No roles → deny all
    IF array::len($roles) = 0 {
        RETURN {
            select: false, read: false, write: false, create: false, delete: false,
            submit: false, cancel: false, amend: false, print: false, email: false,
            report: false, import: false, export: false, share: false
        };
    };

    -- Fetch all matching DocPerm rows and OR them together
    LET $perms = SELECT *
                 FROM tabDocPerm
                 WHERE parent = $doctype
                   AND role IN $roles
                   AND permlevel = 0;

    RETURN {
        select: array::any($perms.map(|$p| $p.perm_select = true OR $p.perm_select = 1)),
        read:   array::any($perms.map(|$p| $p.read        = true OR $p.read        = 1)),
        write:  array::any($perms.map(|$p| $p.write       = true OR $p.write       = 1)),
        create: array::any($perms.map(|$p| $p.perm_create = true OR $p.perm_create = 1)),
        delete: array::any($perms.map(|$p| $p.perm_delete = true OR $p.perm_delete = 1)),
        submit: array::any($perms.map(|$p| $p.submit      = true OR $p.submit      = 1)),
        cancel: array::any($perms.map(|$p| $p.perm_cancel = true OR $p.perm_cancel = 1)),
        amend:  array::any($perms.map(|$p| $p.amend       = true OR $p.amend       = 1)),
        print:  array::any($perms.map(|$p| $p.print       = true OR $p.print       = 1)),
        email:  array::any($perms.map(|$p| $p.email       = true OR $p.email       = 1)),
        report: array::any($perms.map(|$p| $p.report      = true OR $p.report      = 1)),
        import: array::any($perms.map(|$p| $p.import      = true OR $p.import      = 1)),
        export: array::any($perms.map(|$p| $p.export      = true OR $p.export      = 1)),
        share:  array::any($perms.map(|$p| $p.share       = true OR $p.share       = 1)),
    };
};

-- ----------------------------------------------------------------------------
-- fn::permissions::get_boot_permissions($user) → object
--
-- Returns the six permission lists needed by the boot-info endpoint.
-- Replaces the multi-query Rust `build_permission_lists` function with a
-- single graph traversal.
--
-- Return shape:
--   {
--     can_read:   ["DocType", ...],
--     can_write:  [...],
--     can_create: [...],
--     can_delete: [...],
--     can_submit: [...],
--     can_cancel: [...],
--   }
-- ----------------------------------------------------------------------------
DEFINE FUNCTION OVERWRITE fn::permissions::get_boot_permissions($user: string) {
    -- Fetch all doctypes with their submittable flag
    LET $all_dt       = SELECT name, issubmittable FROM tabDocType LIMIT 5000;
    LET $all_names    = $all_dt.map(|$d| $d.name);
    LET $submittable  = $all_dt
                            .filter(|$d| $d.issubmittable = true OR $d.issubmittable = 1)
                            .map(|$d| $d.name);

    -- Administrator and System Manager get everything
    IF $user = 'Administrator' {
        RETURN {
            can_read:   $all_names, can_write:  $all_names,
            can_create: $all_names, can_delete: $all_names,
            can_submit: $submittable, can_cancel: $submittable,
        };
    };

    LET $roles = fn::permissions::get_roles($user);

    IF 'System Manager' IN $roles {
        RETURN {
            can_read:   $all_names, can_write:  $all_names,
            can_create: $all_names, can_delete: $all_names,
            can_submit: $submittable, can_cancel: $submittable,
        };
    };

    -- Regular users: check tabDocPerm for each doctype
    LET $perms = SELECT parent, role, read, write, perm_create, perm_delete, submit, perm_cancel
                 FROM tabDocPerm
                 WHERE role IN $roles AND permlevel = 0;

    LET $flag = |$n: string, $key: string| -> bool {
        array::any(
            $perms
                .filter(|$p| $p.parent = $n)
                .map(|$p| $p[$key] = true OR $p[$key] = 1)
        )
    };

    LET $can_read   = $all_names.filter(|$n| $flag($n, 'read'));
    LET $can_write  = $all_names.filter(|$n| $flag($n, 'write'));
    LET $can_create = $all_names.filter(|$n| $flag($n, 'perm_create'));
    LET $can_delete = $all_names.filter(|$n| $flag($n, 'perm_delete'));
    LET $can_submit = $submittable.filter(|$n| $flag($n, 'submit'));
    LET $can_cancel = $submittable.filter(|$n| $flag($n, 'perm_cancel'));

    RETURN {
        can_read:   $can_read,  can_write:  $can_write,
        can_create: $can_create, can_delete: $can_delete,
        can_submit: $can_submit, can_cancel: $can_cancel,
    };
};
