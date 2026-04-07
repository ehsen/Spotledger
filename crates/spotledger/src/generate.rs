//! `spotledger generate doctype --from <path.json>` — convert a Frappe-compatible
//! DocType JSON file into a Rust source file ready to drop into any app crate.
//!
//! ## Usage
//!
//! ```powershell
//! # Write to stdout
//! spotledger generate doctype --from apps/stock/item.json
//!
//! # Write to a file
//! spotledger generate doctype --from apps/stock/item.json --out apps/stock/src/item.rs
//! ```
//!
//! ## What it reads from Frappe JSON
//!
//! Top-level keys: `name`, `module`, `autoname`, `is_single`, `is_tree`,
//! `is_child_table`, `is_submittable`, `track_changes`, `title_field`,
//! `sort_field`, `sort_order`, `fields[]`, `permissions[]`.
//!
//! Per-field keys: `fieldname`, `fieldtype`, `label`, `options`, `reqd`,
//! `in_list_view`, `in_standard_filter`, `bold`, `read_only`, `hidden`,
//! `default`, `description`, `unique`, `set_only_once`, `allow_on_submit`,
//! `permlevel`, `length`, `fetch_from`, `fetch_if_empty`.
//!
//! Per-permission keys: `role`, `read`, `write`, `create`, `delete`,
//! `submit`, `cancel`, `amend`, `report`, `import`, `export`, `print`,
//! `email`, `share`.

use anyhow::{bail, Context};
use serde::Deserialize;

use crate::cli::GenerateArgs;

// ── Frappe JSON shapes ────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct FrappeDocType {
    name: String,
    #[serde(default)]
    module: String,
    #[serde(default)]
    autoname: Option<String>,
    #[serde(default, deserialize_with = "de_bool_int")]
    is_single: bool,
    #[serde(default, deserialize_with = "de_bool_int")]
    is_tree: bool,
    #[serde(default, deserialize_with = "de_bool_int")]
    is_child_table: bool,
    #[serde(default, deserialize_with = "de_bool_int")]
    is_submittable: bool,
    #[serde(default, deserialize_with = "de_bool_int")]
    track_changes: bool,
    #[serde(default)]
    title_field: Option<String>,
    #[serde(default)]
    sort_field: Option<String>,
    #[serde(default)]
    sort_order: Option<String>,
    #[serde(default)]
    fields: Vec<FrappeField>,
    #[serde(default)]
    permissions: Vec<FrappePermission>,
    /// The `field_order` array, if present, gives the desired display order.
    #[serde(default)]
    field_order: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct FrappeField {
    fieldname: String,
    fieldtype: String,
    #[serde(default)]
    label: Option<String>,
    #[serde(default)]
    options: Option<String>,
    #[serde(default, deserialize_with = "de_bool_int")]
    reqd: bool,
    #[serde(default, deserialize_with = "de_bool_int")]
    in_list_view: bool,
    #[serde(default, deserialize_with = "de_bool_int")]
    in_standard_filter: bool,
    #[serde(default, deserialize_with = "de_bool_int")]
    bold: bool,
    #[serde(default, deserialize_with = "de_bool_int")]
    read_only: bool,
    #[serde(default, deserialize_with = "de_bool_int")]
    hidden: bool,
    #[serde(default)]
    default: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default, deserialize_with = "de_bool_int")]
    unique: bool,
    #[serde(default, deserialize_with = "de_bool_int")]
    set_only_once: bool,
    #[serde(default, deserialize_with = "de_bool_int")]
    allow_on_submit: bool,
    #[serde(default)]
    permlevel: Option<u8>,
    #[serde(default)]
    length: Option<u32>,
    #[serde(default)]
    fetch_from: Option<String>,
    #[serde(default, deserialize_with = "de_bool_int")]
    fetch_if_empty: bool,
}

#[derive(Debug, Deserialize)]
struct FrappePermission {
    role: String,
    #[serde(default, deserialize_with = "de_bool_int")]
    read: bool,
    #[serde(default, deserialize_with = "de_bool_int")]
    write: bool,
    #[serde(default, deserialize_with = "de_bool_int")]
    create: bool,
    #[serde(default, deserialize_with = "de_bool_int")]
    delete: bool,
    #[serde(default, deserialize_with = "de_bool_int")]
    submit: bool,
    #[serde(default, deserialize_with = "de_bool_int")]
    cancel: bool,
    #[serde(default, deserialize_with = "de_bool_int")]
    amend: bool,
    #[serde(default, deserialize_with = "de_bool_int")]
    report: bool,
    #[serde(default, deserialize_with = "de_bool_int")]
    import: bool,
    #[serde(default, deserialize_with = "de_bool_int")]
    export: bool,
    #[serde(default, deserialize_with = "de_bool_int")]
    print: bool,
    #[serde(default, deserialize_with = "de_bool_int")]
    email: bool,
    #[serde(default, deserialize_with = "de_bool_int")]
    share: bool,
}

// ── Entry point ───────────────────────────────────────────────────────────────

pub async fn generate(args: GenerateArgs) -> anyhow::Result<()> {
    let json_path = args.from.canonicalize()
        .with_context(|| format!("Cannot find input file: {}", args.from.display()))?;

    let raw = tokio::fs::read_to_string(&json_path)
        .await
        .with_context(|| format!("Reading {}", json_path.display()))?;

    let dt: FrappeDocType = serde_json::from_str(&raw)
        .with_context(|| format!("Parsing {} as Frappe DocType JSON", json_path.display()))?;

    let rust_src = render(&dt)?;

    match args.out {
        Some(ref out_path) => {
            // Create parent directories if needed
            if let Some(parent) = out_path.parent() {
                tokio::fs::create_dir_all(parent)
                    .await
                    .with_context(|| format!("Creating directories for {}", out_path.display()))?;
            }
            tokio::fs::write(out_path, &rust_src)
                .await
                .with_context(|| format!("Writing {}", out_path.display()))?;
            println!("Wrote {}", out_path.display());
            println!();
            println!(
                "Next: add `pub mod {};` to your crate's lib.rs",
                out_path.file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("???")
            );
        }
        None => {
            println!("{rust_src}");
        }
    }

    Ok(())
}

// ── Renderer ──────────────────────────────────────────────────────────────────

fn render(dt: &FrappeDocType) -> anyhow::Result<String> {
    // Determine field order: use field_order if present, otherwise declaration order.
    let ordered_fields: Vec<&FrappeField> = if dt.field_order.is_empty() {
        dt.fields.iter().collect()
    } else {
        let mut ordered = Vec::with_capacity(dt.fields.len());
        for name in &dt.field_order {
            if let Some(f) = dt.fields.iter().find(|f| &f.fieldname == name) {
                ordered.push(f);
            }
        }
        // Append any fields not listed in field_order at the end
        for f in &dt.fields {
            if !dt.field_order.contains(&f.fieldname) {
                ordered.push(f);
            }
        }
        ordered
    };

    let fn_name = doctype_fn_name(&dt.name);
    let registry_name = &dt.name;

    let mut out = String::new();

    // File header
    out.push_str(&format!(
        "//! `{name}` DocType — generated from Frappe JSON by `spotledger generate doctype`.\n\
         //! Edit freely; this file is not regenerated automatically.\n\n",
        name = dt.name,
    ));

    // Imports
    out.push_str(
        "use spotledger_core::meta::{DocField, DocTypeMeta, FieldType, Permission};\n\
         use spotledger_core::registry::MetaEntry;\n\n",
    );

    // Meta function
    out.push_str(&format!("pub fn {fn_name}() -> DocTypeMeta {{\n"));
    out.push_str("    DocTypeMeta {\n");
    out.push_str(&format!("        name: \"{name}\".into(),\n", name = dt.name));
    out.push_str(&format!("        module: \"{module}\".into(),\n", module = dt.module));

    // Structural flags — only emit non-default values
    out.push_str(&format!(
        "        is_single:      {},\n\
                 is_tree:        {},\n\
                 is_child:       {},\n\
                 is_submittable: {},\n\
                 track_changes:  {},\n",
        dt.is_single,
        dt.is_tree,
        dt.is_child_table,
        dt.is_submittable,
        dt.track_changes,
    ));

    // Fields
    out.push_str("        fields: vec![\n");
    for f in &ordered_fields {
        out.push_str(&render_field(f)?);
    }
    out.push_str("        ],\n");

    // Permissions
    out.push_str("        permissions: vec![\n");
    if dt.permissions.is_empty() {
        out.push_str("            Permission::full(\"System Manager\"),\n");
    } else {
        for p in &dt.permissions {
            out.push_str(&render_permission(p));
        }
    }
    out.push_str("        ],\n");

    // Display metadata
    match &dt.title_field {
        Some(f) => out.push_str(&format!("        title_field:   Some(\"{f}\".into()),\n")),
        None => out.push_str("        title_field:   None,\n"),
    }
    out.push_str("        search_fields: vec![],\n");
    match &dt.sort_field {
        Some(f) => out.push_str(&format!("        sort_field:    Some(\"{f}\".into()),\n")),
        None => out.push_str("        sort_field:    None,\n"),
    }
    match &dt.sort_order {
        Some(o) => out.push_str(&format!("        sort_order:    Some(\"{o}\".into()),\n")),
        None => out.push_str("        sort_order:    None,\n"),
    }

    // Autoname
    match &dt.autoname {
        Some(a) => out.push_str(&format!(
            "        autoname:      Some(\"{a}\".into()),\n"
        )),
        None => out.push_str("        autoname:      None,\n"),
    }
    out.push_str("        naming_series: None,\n");

    out.push_str("    }\n}\n\n");

    // inventory::submit!
    out.push_str(&format!(
        "inventory::submit!(MetaEntry {{\n\
         \x20\x20\x20\x20name: \"{registry_name}\",\n\
         \x20\x20\x20\x20meta: {fn_name},\n\
         }});\n",
    ));

    Ok(out)
}

fn render_field(f: &FrappeField) -> anyhow::Result<String> {
    let ft = map_fieldtype(&f.fieldtype).with_context(|| {
        format!(
            "Unknown fieldtype '{}' on field '{}'",
            f.fieldtype, f.fieldname
        )
    })?;

    let label = f.label.as_deref().unwrap_or(&f.fieldname);
    let mut line = format!(
        "            DocField::new(\"{fn}\", \"{label}\", FieldType::{ft})",
        fn = f.fieldname,
    );

    // Options (Link target or Select choices)
    if let Some(ref opts) = f.options {
        let escaped = opts.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n");
        line.push_str(&format!("\n                .options(\"{escaped}\")"));
    }

    // Boolean flags — emit only when true
    if f.reqd            { line.push_str("\n                .required()"); }
    if f.unique          { line.push_str("\n                .unique()"); }
    if f.in_list_view    { line.push_str("\n                .in_list()"); }
    if f.in_standard_filter { line.push_str("\n                .in_standard_filter()"); }
    if f.bold            { line.push_str("\n                .bold()"); }
    if f.read_only       { line.push_str("\n                .read_only()"); }
    if f.hidden          { line.push_str("\n                .hidden()"); }
    if f.set_only_once   { line.push_str("\n                .set_only_once()"); }
    if f.allow_on_submit { line.push_str("\n                .allow_on_submit()"); }
    if f.fetch_if_empty  { line.push_str("\n                .fetch_if_empty()"); }

    if let Some(ref d) = f.default {
        let escaped = d.replace('\\', "\\\\").replace('"', "\\\"");
        line.push_str(&format!("\n                .default_value(\"{escaped}\")"));
    }
    if let Some(ref desc) = f.description {
        let escaped = desc.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', " ");
        line.push_str(&format!("\n                .description(\"{escaped}\")"));
    }
    if let Some(ref ff) = f.fetch_from {
        let escaped = ff.replace('"', "\\\"");
        line.push_str(&format!("\n                .fetch_from(\"{escaped}\")"));
    }
    if let Some(lvl) = f.permlevel {
        if lvl > 0 {
            line.push_str(&format!("\n                .permlevel({lvl})"));
        }
    }
    if let Some(len) = f.length {
        if len > 0 {
            line.push_str(&format!("\n                .length({len})"));
        }
    }

    line.push_str(",\n");
    Ok(line)
}

fn render_permission(p: &FrappePermission) -> String {
    // Detect common shortcuts first
    let all_write = p.read && p.write && p.create && p.delete
        && p.report && p.export && p.print && p.email;
    let all_elevated = all_write && p.submit && p.cancel && p.amend && p.import && p.share;

    if all_elevated {
        return format!("            Permission::full(\"{}\"),\n", p.role);
    }

    if p.read && !p.write && !p.create && !p.delete && !p.submit {
        return format!("            Permission::read_only(\"{}\"),\n", p.role);
    }

    // Full custom permission
    let mut out = format!("            Permission {{\n                role: \"{}\".into(),\n", p.role);
    for (name, val) in [
        ("read",   p.read),
        ("write",  p.write),
        ("create", p.create),
        ("delete", p.delete),
        ("submit", p.submit),
        ("cancel", p.cancel),
        ("amend",  p.amend),
        ("report", p.report),
        ("import", p.import),
        ("export", p.export),
        ("print",  p.print),
        ("email",  p.email),
        ("share",  p.share),
    ] {
        out.push_str(&format!("                {name}: {val},\n"));
    }
    out.push_str("            },\n");
    out
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Map a Frappe fieldtype string to the Rust `FieldType` variant name.
fn map_fieldtype(s: &str) -> anyhow::Result<&'static str> {
    Ok(match s {
        "Data"                          => "Data",
        "Text"                          => "Text",
        "Long Text"                     => "LongText",
        "Small Text"                    => "SmallText",
        "Int" | "Integer"               => "Int",
        "Float"                         => "Float",
        "Currency"                      => "Currency",
        "Percent"                       => "Percent",
        "Check"                         => "Check",
        "Date"                          => "Date",
        "Datetime"                      => "Datetime",
        "Time"                          => "Time",
        "Select"                        => "Select",
        "Link"                          => "Link",
        "Dynamic Link"                  => "DynamicLink",
        "Table"                         => "Table",
        "Table MultiSelect"             => "TableMultiSelect",
        "Attach"                        => "Attach",
        "Attach Image"                  => "AttachImage",
        "HTML" | "HTML Editor"          => "Html",
        "Code"                          => "Code",
        "Signature"                     => "Signature",
        "Password"                      => "Password",
        "Color"                         => "Color",
        "Rating"                        => "Rating",
        "JSON"                          => "Json",
        "Section Break"                 => "SectionBreak",
        "Column Break"                  => "ColumnBreak",
        "Tab Break"                     => "TabBreak",
        // Frappe aliases / old names
        "Read Only"                     => "Data",   // display-only, treated as Data
        "Barcode"                       => "Data",
        "Geolocation"                   => "Json",
        "Duration"                      => "Int",
        "Phone"                         => "Data",
        "Autocomplete"                  => "Data",
        other => bail!("Unrecognised Frappe fieldtype: '{other}'"),
    })
}

/// Convert "Sales Invoice" → `sales_invoice_meta` (function name).
fn doctype_fn_name(name: &str) -> String {
    let snake = to_snake(name);
    format!("{snake}_meta")
}

/// Convert "Sales Invoice" → `sales_invoice` (module / file name stem).
pub fn doctype_mod_name(name: &str) -> String {
    to_snake(name)
}

fn to_snake(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_alphanumeric() { c.to_ascii_lowercase() } else { '_' })
        .collect::<String>()
        // collapse consecutive underscores
        .split('_')
        .filter(|p| !p.is_empty())
        .collect::<Vec<_>>()
        .join("_")
}

/// Deserialize Frappe boolean fields that may be 0/1 integers or true/false booleans.
fn de_bool_int<'de, D: serde::Deserializer<'de>>(d: D) -> Result<bool, D::Error> {
    use serde::de::{self, Visitor};
    struct BoolIntVisitor;
    impl<'de> Visitor<'de> for BoolIntVisitor {
        type Value = bool;
        fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            write!(f, "bool or 0/1 integer")
        }
        fn visit_bool<E: de::Error>(self, v: bool) -> Result<bool, E> { Ok(v) }
        fn visit_u64<E: de::Error>(self, v: u64) -> Result<bool, E>   { Ok(v != 0) }
        fn visit_i64<E: de::Error>(self, v: i64) -> Result<bool, E>   { Ok(v != 0) }
    }
    d.deserialize_any(BoolIntVisitor)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snake_case_conversion() {
        assert_eq!(to_snake("SalesInvoice"), "salesinvoice");
        assert_eq!(to_snake("Sales Invoice"), "sales_invoice");
        assert_eq!(to_snake("Sales  Invoice"), "sales_invoice");
        assert_eq!(to_snake("Currency"), "currency");
    }

    #[test]
    fn fn_name_from_doctype() {
        assert_eq!(doctype_fn_name("Sales Invoice"), "sales_invoice_meta");
        assert_eq!(doctype_fn_name("Currency"), "currency_meta");
    }

    #[test]
    fn fieldtype_mapping_roundtrip() {
        for ft in [
            "Data", "Text", "Long Text", "Small Text", "Int", "Float",
            "Currency", "Percent", "Check", "Date", "Datetime", "Time",
            "Select", "Link", "Dynamic Link", "Table", "Table MultiSelect",
            "Attach", "Attach Image", "HTML", "Code", "Signature", "Password",
            "Color", "Rating", "JSON", "Section Break", "Column Break", "Tab Break",
        ] {
            assert!(map_fieldtype(ft).is_ok(), "fieldtype '{ft}' should map");
        }
    }

    #[test]
    fn render_currency_minimal() {
        let json = r#"{
            "name": "Currency",
            "module": "Geo",
            "autoname": "field:currency_name",
            "fields": [
                {"fieldname": "currency_name", "fieldtype": "Data", "label": "Currency Name",
                 "reqd": 1, "unique": 1},
                {"fieldname": "enabled", "fieldtype": "Check", "label": "Enabled",
                 "in_list_view": 1, "default": "0"}
            ],
            "permissions": [
                {"role": "System Manager", "read": 1, "write": 1, "create": 1, "delete": 1,
                 "submit": 1, "cancel": 1, "amend": 1, "report": 1, "import": 1,
                 "export": 1, "print": 1, "email": 1, "share": 1}
            ]
        }"#;
        let dt: FrappeDocType = serde_json::from_str(json).unwrap();
        let src = render(&dt).unwrap();
        assert!(src.contains("pub fn currency_meta()"));
        assert!(src.contains("FieldType::Data"));
        assert!(src.contains(".required()"));
        assert!(src.contains(".unique()"));
        assert!(src.contains(".in_list()"));
        assert!(src.contains("Permission::full(\"System Manager\")"));
        assert!(src.contains("inventory::submit!"));
    }
}
