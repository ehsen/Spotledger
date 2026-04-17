//! Validation for DocType save inputs.
//!
//! Mirrors the checks Frappe performs in `DocType.validate()` and
//! `validate_fields()` / `validate_permissions()`, adapted for SurrealDB
//! reserved-word constraints and our type system.
//!
//! All checks run **before** any database write.  A non-empty
//! `Vec<ValidationError>` is returned as `DoctypeSaveError::Validation`.

use crate::doctype_save::{DocFieldInput, DocPermInput, DoctypeSaveInput};

// ── Public types ──────────────────────────────────────────────────────────────

/// One validation failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationError {
    /// `None` = DocType-level; `Some("fieldname")` = field-level;
    /// `Some("perm:N")` = permission row N.
    pub location: Option<String>,
    /// Short stable code, e.g. `"V3"`, `"F4"`.
    pub code:     &'static str,
    /// Human-readable message.
    pub message:  String,
}

impl ValidationError {
    fn doctype(code: &'static str, message: impl Into<String>) -> Self {
        Self { location: None, code, message: message.into() }
    }
    fn field(fieldname: impl Into<String>, code: &'static str, message: impl Into<String>) -> Self {
        Self { location: Some(fieldname.into()), code, message: message.into() }
    }
    fn perm(idx: usize, code: &'static str, message: impl Into<String>) -> Self {
        Self {
            location: Some(format!("perm:{idx}")),
            code,
            message: message.into(),
        }
    }
}

// ── Reserved words ────────────────────────────────────────────────────────────

/// SurrealDB reserved identifiers that must not be used as fieldnames.
/// Includes both v3 reserved words and Frappe system field names.
const RESERVED_FIELDNAMES: &[&str] = &[
    // SurrealDB keywords
    "id", "type", "return", "select", "create", "update", "delete",
    "insert", "upsert", "relate", "define", "remove", "use", "let",
    "begin", "commit", "cancel", "transaction", "set", "content",
    "merge", "patch", "where", "fetch", "limit", "start", "order",
    "group", "split", "with", "table", "field", "index", "event",
    "function", "scope", "token", "namespace", "database", "value",
    "assert", "permissions", "full", "none", "null", "true", "false",
    "if", "else", "then", "end", "for", "in", "is", "not", "and", "or",
    "contains", "containsall", "containsany", "containsnone",
    "inside", "notinside", "allinside", "anyinside", "noneinside",
    "outside", "intersects", "matches", "count", "math",
    "rand", "time", "duration", "geo", "array", "string", "number",
    "object", "bool", "bytes", "uuid", "record",
    // Frappe system fields — cannot be redefined as user fields
    "name", "doctype", "owner", "creation", "modified", "modified_by",
    "docstatus", "idx",
    // Child-table system fields
    "parent", "parenttype", "parentfield",
];

/// SurrealDB reserved words that must not be used as DocType names.
const RESERVED_DOCTYPE_NAMES: &[&str] = &[
    "DocType", "DocField", "DocPerm", "User", "Role", "UserPermission",
    "Site", "ModuleDef",
];

// ── Known field types ─────────────────────────────────────────────────────────

const KNOWN_FIELDTYPES: &[&str] = &[
    "Data", "Text", "Long Text", "Small Text", "Int", "Float",
    "Currency", "Percent", "Check", "Date", "Datetime", "Time",
    "Select", "Link", "Dynamic Link", "Table", "Table MultiSelect",
    "Attach", "Attach Image", "HTML", "Code", "Signature", "Password",
    "Color", "Rating", "JSON", "Section Break", "Column Break", "Tab Break",
    // lowercase variants accepted too — normalised before comparison
];

fn is_known_fieldtype(ft: &str) -> bool {
    let lower = ft.to_lowercase().replace('-', " ");
    KNOWN_FIELDTYPES.iter().any(|k| k.to_lowercase() == lower)
}

// ── Name helpers ──────────────────────────────────────────────────────────────

/// A valid DocType name: starts with a letter, only letters/digits/spaces/hyphens.
/// Max 61 chars (SurrealDB table name = "tab" + 61 = 64 total, well under limits).
fn is_valid_doctype_name(name: &str) -> bool {
    if name.is_empty() || name.len() > 61 {
        return false;
    }
    let mut chars = name.chars();
    let first = chars.next().unwrap();
    if !first.is_ascii_alphabetic() {
        return false;
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == ' ' || c == '-' || c == '_')
}

/// A valid fieldname: lowercase letters, digits, underscores; starts with a letter.
fn is_valid_fieldname(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }
    let mut chars = name.chars();
    let first = chars.next().unwrap();
    if !first.is_ascii_lowercase() {
        return false;
    }
    chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

/// A valid autoname value.
fn is_valid_autoname(autoname: &str) -> bool {
    if autoname.is_empty() {
        return true; // empty = use name field directly, valid
    }
    matches!(autoname, "hash" | "Prompt" | "autoincrement")
        || autoname.starts_with("field:")
        || autoname.starts_with("format:")
        || autoname.starts_with("naming_series:")
        || autoname.starts_with("UUID")
}

/// A valid `depends_on` expression — basic bracket balance check only.
fn is_balanced(expr: &str) -> bool {
    let mut depth = 0i32;
    for c in expr.chars() {
        match c {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth < 0 {
                    return false;
                }
            }
            _ => {}
        }
    }
    depth == 0
}

// ── Main entry point ──────────────────────────────────────────────────────────

/// Validate a [`DoctypeSaveInput`].
///
/// Returns `Ok(())` when all checks pass, or a list of [`ValidationError`]s.
pub fn validate_doctype_input(input: &DoctypeSaveInput) -> Result<(), Vec<ValidationError>> {
    let mut errors: Vec<ValidationError> = Vec::new();

    // ── DocType-level ─────────────────────────────────────────────────────────
    validate_doctype_meta(input, &mut errors);

    // ── Field-level ───────────────────────────────────────────────────────────
    let fieldnames = validate_fields(&input.fields, input.is_submittable, &mut errors);

    // ── Permission-level ──────────────────────────────────────────────────────
    validate_perms(&input.perms, input.is_submittable, &mut errors);

    // ── Cross-checks ─────────────────────────────────────────────────────────
    // F13: no duplicate fieldnames
    let mut seen_names: std::collections::HashSet<&str> = std::collections::HashSet::new();
    for fn_ in &fieldnames {
        if !seen_names.insert(fn_.as_str()) {
            errors.push(ValidationError::field(
                fn_,
                "F13",
                format!("Duplicate fieldname `{fn_}`"),
            ));
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

// ── DocType-level checks ──────────────────────────────────────────────────────

fn validate_doctype_meta(input: &DoctypeSaveInput, errors: &mut Vec<ValidationError>) {
    // V1 — name not empty
    if input.doctype.trim().is_empty() {
        errors.push(ValidationError::doctype("V1", "DocType name must not be empty"));
        return; // further name checks pointless
    }

    // V2/V3 — name format + length
    if !is_valid_doctype_name(&input.doctype) {
        errors.push(ValidationError::doctype(
            "V3",
            format!(
                "DocType name `{}` is invalid. Must start with a letter, \
                 contain only letters/digits/spaces/hyphens/underscores, max 61 chars.",
                input.doctype
            ),
        ));
    }

    // V4 — name not a reserved word
    if RESERVED_DOCTYPE_NAMES
        .iter()
        .any(|r| r.eq_ignore_ascii_case(&input.doctype))
    {
        errors.push(ValidationError::doctype(
            "V4",
            format!(
                "DocType name `{}` is reserved and cannot be used.",
                input.doctype
            ),
        ));
    }

    // V5 — module not empty
    if input.module.trim().is_empty() {
        errors.push(ValidationError::doctype("V5", "Module must not be empty"));
    }

    // V6 — autoname format valid
    if !is_valid_autoname(&input.autoname) {
        errors.push(ValidationError::doctype(
            "V6",
            format!(
                "Invalid autoname value `{}`. Must be one of: hash, Prompt, autoincrement, \
                 field:<fieldname>, format:<template>, naming_series:<prefix>.",
                input.autoname
            ),
        ));
    }
}

// ── Field-level checks ────────────────────────────────────────────────────────

/// Validate all fields and return collected fieldnames for duplicate-check.
fn validate_fields(
    fields: &[DocFieldInput],
    is_submittable: bool,
    errors: &mut Vec<ValidationError>,
) -> Vec<String> {
    let mut fieldnames = Vec::new();

    for f in fields {
        // F1 — fieldname not empty
        if f.fieldname.trim().is_empty() {
            errors.push(ValidationError::field(
                "<empty>",
                "F1",
                "fieldname must not be empty",
            ));
            continue;
        }

        let fn_ = f.fieldname.as_str();
        fieldnames.push(f.fieldname.clone());

        // F2 — fieldname valid identifier
        if !is_valid_fieldname(fn_) {
            errors.push(ValidationError::field(
                fn_,
                "F2",
                format!(
                    "fieldname `{fn_}` is invalid. Must be lowercase letters, digits, \
                     underscores, starting with a letter."
                ),
            ));
        }

        // F3 — fieldname not a reserved word
        if RESERVED_FIELDNAMES
            .iter()
            .any(|r| r.eq_ignore_ascii_case(fn_))
        {
            errors.push(ValidationError::field(
                fn_,
                "F3",
                format!("fieldname `{fn_}` is a reserved word and cannot be used"),
            ));
        }

        // F5 — fieldtype is known
        if !is_known_fieldtype(&f.fieldtype) {
            errors.push(ValidationError::field(
                fn_,
                "F5",
                format!("Unknown fieldtype `{}`", f.fieldtype),
            ));
        }

        // F6 — Link field must have options
        if f.fieldtype.eq_ignore_ascii_case("Link") {
            if f.options.as_deref().unwrap_or("").trim().is_empty() {
                errors.push(ValidationError::field(
                    fn_,
                    "F6",
                    format!("`{fn_}` is a Link field and must have options set to the linked DocType"),
                ));
            }
        }

        // F7 — Select/Autocomplete must have options
        if f.fieldtype.eq_ignore_ascii_case("Select")
            || f.fieldtype.eq_ignore_ascii_case("Autocomplete")
        {
            if f.options.as_deref().unwrap_or("").trim().is_empty() {
                errors.push(ValidationError::field(
                    fn_,
                    "F7",
                    format!("`{fn_}` is a Select field and must have options (newline-separated)"),
                ));
            }
        }

        // F8 — reqd=true + hidden=true conflict
        if f.reqd && f.hidden {
            errors.push(ValidationError::field(
                fn_,
                "F8",
                format!("`{fn_}` cannot be both required and hidden"),
            ));
        }

        // F9 — set_only_once + allow_on_submit conflict
        if f.set_only_once && f.allow_on_submit {
            errors.push(ValidationError::field(
                fn_,
                "F9",
                format!(
                    "`{fn_}` cannot have both set_only_once and allow_on_submit set"
                ),
            ));
        }

        // F10 — depends_on bracket balance
        if let Some(ref expr) = f.depends_on {
            if !expr.is_empty() && !is_balanced(expr) {
                errors.push(ValidationError::field(
                    fn_,
                    "F10",
                    format!("`{fn_}` depends_on expression has unbalanced parentheses"),
                ));
            }
        }

        // F11 — permlevel 0-9
        if f.permlevel > 9 {
            errors.push(ValidationError::field(
                fn_,
                "F11",
                format!("`{fn_}` permlevel must be 0–9, got {}", f.permlevel),
            ));
        }

        // F12 — fetch_from format: "OtherDocType.fieldname"
        if let Some(ref ff) = f.fetch_from {
            if !ff.is_empty() && !ff.contains('.') {
                errors.push(ValidationError::field(
                    fn_,
                    "F12",
                    format!(
                        "`{fn_}` fetch_from `{ff}` must be in the format \
                         `LinkedField.source_fieldname`"
                    ),
                ));
            }
        }

        // F13a — not a system field name (early check; duplicate check done outside)
        let lower_fn = fn_.to_lowercase();
        let is_system = RESERVED_FIELDNAMES
            .iter()
            .any(|r| r.to_lowercase() == lower_fn);
        // (already covered by F3 — reserved word check above)
        let _ = is_system;

        // F14 — submit-only flags on non-submittable doctype
        if !is_submittable && (f.allow_on_submit || f.set_only_once) {
            // This is a warning in Frappe, not an error — skip for now.
            // If we want to enforce, uncomment:
            // errors.push(ValidationError::field(fn_, "F14", "..."));
        }
    }

    fieldnames
}

// ── Permission-level checks ───────────────────────────────────────────────────

fn validate_perms(
    perms: &[DocPermInput],
    is_submittable: bool,
    errors: &mut Vec<ValidationError>,
) {
    for (i, p) in perms.iter().enumerate() {
        // P1 — role not empty
        if p.role.trim().is_empty() {
            errors.push(ValidationError::perm(i, "P1", "Permission role must not be empty"));
        }

        // P3 — permlevel 0-9
        if p.permlevel > 9 {
            errors.push(ValidationError::perm(
                i,
                "P3",
                format!("Permission permlevel must be 0–9, got {}", p.permlevel),
            ));
        }

        // P4 — at least read, write, or create
        if !p.read && !p.write && !p.perm_create {
            errors.push(ValidationError::perm(
                i,
                "P4",
                format!(
                    "Permission for role `{}` must have at least one of read/write/create",
                    p.role
                ),
            ));
        }

        // P5 — submit/cancel/amend only on submittable doctypes
        if !is_submittable && (p.submit || p.perm_cancel || p.amend) {
            errors.push(ValidationError::perm(
                i,
                "P5",
                format!(
                    "Permission for role `{}` sets submit/cancel/amend but DocType is not submittable",
                    p.role
                ),
            ));
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::doctype_save::{DocFieldInput, DocPermInput, DoctypeSaveInput};

    fn minimal() -> DoctypeSaveInput {
        DoctypeSaveInput {
            doctype:        "Airlines".into(),
            module:         "Custom".into(),
            autoname:       String::new(),
            is_child:       false,
            is_single:      false,
            is_submittable: false,
            is_tree:        false,
            custom:         true,
            fields:         vec![],
            perms:          vec![],
            extra_meta:     Default::default(),
        }
    }

    fn data_field(fieldname: &str) -> DocFieldInput {
        DocFieldInput {
            fieldname:      fieldname.into(),
            label:          fieldname.into(),
            fieldtype:      "Data".into(),
            options:        None,
            reqd:           false,
            hidden:         false,
            bold:           false,
            in_list_view:   false,
            in_standard_filter: false,
            read_only:      false,
            set_only_once:  false,
            allow_on_submit: false,
            permlevel:      0,
            unique:         false,
            not_nullable:   false,
            depends_on:     None,
            fetch_from:     None,
            default_value:  None,
            description:    None,
            idx:            0,
            extra_attrs:    Default::default(),
        }
    }

    fn sys_manager_perm() -> DocPermInput {
        DocPermInput {
            role:        "System Manager".into(),
            permlevel:   0,
            read:        true,
            write:       true,
            perm_create: true,
            perm_delete: true,
            perm_select: false,
            perm_cancel: false,
            submit:      false,
            cancel:      false,
            amend:       false,
            report:      true,
            import:      false,
            export:      true,
            print:       true,
            email:       false,
            share:       false,
            if_owner:    false,
            idx:         0,
        }
    }

    // ── V1: empty name ────────────────────────────────────────────────────────
    #[test]
    fn test_validate_doctype_rejects_empty_name() {
        let mut input = minimal();
        input.doctype = "".into();
        let errs = validate_doctype_input(&input).unwrap_err();
        assert!(errs.iter().any(|e| e.code == "V1"));
    }

    // ── V2/V3: name too long ──────────────────────────────────────────────────
    #[test]
    fn test_validate_doctype_rejects_long_name() {
        let mut input = minimal();
        input.doctype = "A".repeat(62);
        let errs = validate_doctype_input(&input).unwrap_err();
        assert!(errs.iter().any(|e| e.code == "V3"));
    }

    // ── V3: invalid characters ────────────────────────────────────────────────
    #[test]
    fn test_validate_doctype_rejects_invalid_chars() {
        let mut input = minimal();
        input.doctype = "1Invalid!".into();
        let errs = validate_doctype_input(&input).unwrap_err();
        assert!(errs.iter().any(|e| e.code == "V3"));
    }

    // ── V5: empty module ──────────────────────────────────────────────────────
    #[test]
    fn test_validate_doctype_rejects_empty_module() {
        let mut input = minimal();
        input.module = "".into();
        let errs = validate_doctype_input(&input).unwrap_err();
        assert!(errs.iter().any(|e| e.code == "V5"));
    }

    // ── V6: invalid autoname ──────────────────────────────────────────────────
    #[test]
    fn test_validate_doctype_rejects_invalid_autoname() {
        let mut input = minimal();
        input.autoname = "gibberish".into();
        let errs = validate_doctype_input(&input).unwrap_err();
        assert!(errs.iter().any(|e| e.code == "V6"));
    }

    // ── V6: valid autoname formats ────────────────────────────────────────────
    #[test]
    fn test_validate_doctype_accepts_valid_autonames() {
        let valid = &[
            "", "hash", "Prompt", "autoincrement",
            "field:airline_code", "format:AIR-{YYYY}-{###}",
            "naming_series:AIR-",
        ];
        for &an in valid {
            let mut input = minimal();
            input.autoname = an.into();
            assert!(
                validate_doctype_input(&input).is_ok(),
                "autoname `{an}` should be valid"
            );
        }
    }

    // ── F2: uppercase fieldname rejected ─────────────────────────────────────
    #[test]
    fn test_validate_field_rejects_uppercase() {
        let mut input = minimal();
        input.fields.push(data_field("AirlineName"));
        let errs = validate_doctype_input(&input).unwrap_err();
        assert!(errs.iter().any(|e| e.code == "F2"));
    }

    // ── F3: reserved fieldname ────────────────────────────────────────────────
    #[test]
    fn test_validate_field_rejects_reserved_words() {
        for &reserved in &["name", "doctype", "creation", "modified", "owner", "idx"] {
            let mut input = minimal();
            input.fields.push(data_field(reserved));
            let errs = validate_doctype_input(&input).unwrap_err();
            assert!(
                errs.iter().any(|e| e.code == "F3"),
                "Expected F3 for reserved fieldname `{reserved}`"
            );
        }
    }

    // ── F6: Link without options ──────────────────────────────────────────────
    #[test]
    fn test_validate_field_link_requires_options() {
        let mut input = minimal();
        let mut f = data_field("supplier");
        f.fieldtype = "Link".into();
        f.options = None;
        input.fields.push(f);
        let errs = validate_doctype_input(&input).unwrap_err();
        assert!(errs.iter().any(|e| e.code == "F6"));
    }

    // ── F7: Select without options ────────────────────────────────────────────
    #[test]
    fn test_validate_field_select_requires_options() {
        let mut input = minimal();
        let mut f = data_field("status");
        f.fieldtype = "Select".into();
        f.options = None;
        input.fields.push(f);
        let errs = validate_doctype_input(&input).unwrap_err();
        assert!(errs.iter().any(|e| e.code == "F7"));
    }

    // ── F8: reqd + hidden ─────────────────────────────────────────────────────
    #[test]
    fn test_validate_field_reqd_hidden_conflict() {
        let mut input = minimal();
        let mut f = data_field("airline_name");
        f.reqd = true;
        f.hidden = true;
        input.fields.push(f);
        let errs = validate_doctype_input(&input).unwrap_err();
        assert!(errs.iter().any(|e| e.code == "F8"));
    }

    // ── F9: set_only_once + allow_on_submit ───────────────────────────────────
    #[test]
    fn test_validate_field_set_only_once_allow_on_submit_conflict() {
        let mut input = minimal();
        let mut f = data_field("airline_code");
        f.set_only_once = true;
        f.allow_on_submit = true;
        input.fields.push(f);
        let errs = validate_doctype_input(&input).unwrap_err();
        assert!(errs.iter().any(|e| e.code == "F9"));
    }

    // ── F10: unbalanced depends_on ────────────────────────────────────────────
    #[test]
    fn test_validate_field_unbalanced_depends_on() {
        let mut input = minimal();
        let mut f = data_field("airline_name");
        f.depends_on = Some("eval:doc.status == 'Active' && (doc.x".into());
        input.fields.push(f);
        let errs = validate_doctype_input(&input).unwrap_err();
        assert!(errs.iter().any(|e| e.code == "F10"));
    }

    // ── F11: permlevel out of range ───────────────────────────────────────────
    #[test]
    fn test_validate_field_permlevel_range() {
        let mut input = minimal();
        let mut f = data_field("secret_field");
        f.permlevel = 10;
        input.fields.push(f);
        let errs = validate_doctype_input(&input).unwrap_err();
        assert!(errs.iter().any(|e| e.code == "F11"));
    }

    // ── F12: fetch_from without dot ───────────────────────────────────────────
    #[test]
    fn test_validate_field_fetch_from_format() {
        let mut input = minimal();
        let mut f = data_field("supplier_name");
        f.fetch_from = Some("just_field".into());
        input.fields.push(f);
        let errs = validate_doctype_input(&input).unwrap_err();
        assert!(errs.iter().any(|e| e.code == "F12"));
    }

    // ── F13: duplicate fieldname ──────────────────────────────────────────────
    #[test]
    fn test_validate_field_duplicate_fieldname() {
        let mut input = minimal();
        input.fields.push(data_field("airline_name"));
        input.fields.push(data_field("airline_name")); // duplicate
        let errs = validate_doctype_input(&input).unwrap_err();
        assert!(errs.iter().any(|e| e.code == "F13"));
    }

    // ── P1: empty role ────────────────────────────────────────────────────────
    #[test]
    fn test_validate_perm_empty_role() {
        let mut input = minimal();
        let mut p = sys_manager_perm();
        p.role = "".into();
        input.perms.push(p);
        let errs = validate_doctype_input(&input).unwrap_err();
        assert!(errs.iter().any(|e| e.code == "P1"));
    }

    // ── P3: permlevel out of range ────────────────────────────────────────────
    #[test]
    fn test_validate_perm_permlevel_range() {
        let mut input = minimal();
        let mut p = sys_manager_perm();
        p.permlevel = 10;
        input.perms.push(p);
        let errs = validate_doctype_input(&input).unwrap_err();
        assert!(errs.iter().any(|e| e.code == "P3"));
    }

    // ── P4: no permissions set ────────────────────────────────────────────────
    #[test]
    fn test_validate_perm_no_access() {
        let mut input = minimal();
        let mut p = sys_manager_perm();
        p.read = false;
        p.write = false;
        p.perm_create = false;
        input.perms.push(p);
        let errs = validate_doctype_input(&input).unwrap_err();
        assert!(errs.iter().any(|e| e.code == "P4"));
    }

    // ── P5: submit on non-submittable ─────────────────────────────────────────
    #[test]
    fn test_validate_perm_submit_on_non_submittable() {
        let mut input = minimal();
        let mut p = sys_manager_perm();
        p.submit = true;
        input.perms.push(p);
        let errs = validate_doctype_input(&input).unwrap_err();
        assert!(errs.iter().any(|e| e.code == "P5"));
    }

    // ── Happy path ────────────────────────────────────────────────────────────
    #[test]
    fn test_validate_valid_doctype_passes() {
        let mut input = minimal();
        input.fields.push(data_field("airline_name"));
        let mut f2 = data_field("iata_code");
        f2.unique = true;
        input.fields.push(f2);
        input.perms.push(sys_manager_perm());
        assert!(validate_doctype_input(&input).is_ok());
    }
}
