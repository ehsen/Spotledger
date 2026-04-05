//! Typed DocType schema definition — replaces Frappe's JSON meta.
//!
//! `DocTypeMeta` is the authoritative schema descriptor compiled into each
//! crate.  WASM plugins serialize this via MsgPack to the host, and the host
//! uses it for validation and form rendering.

use serde::{Deserialize, Serialize};

// ── FieldType ─────────────────────────────────────────────────────────────────

/// Frappe-compatible field types.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FieldType {
    Data,
    Text,
    LongText,
    SmallText,
    Int,
    Float,
    Currency,
    Percent,
    Check,
    Date,
    Datetime,
    Time,
    Select,
    Link,
    DynamicLink,
    Table,
    TableMultiSelect,
    Attach,
    AttachImage,
    Html,
    Code,
    Signature,
    Password,
    Color,
    Rating,
    Json,
    /// Section / column / tab — layout-only, no data stored.
    SectionBreak,
    ColumnBreak,
    TabBreak,
}

// ── FieldKind ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FieldKind {
    Data,
    Layout,
}

impl FieldType {
    pub fn kind(&self) -> FieldKind {
        match self {
            FieldType::SectionBreak | FieldType::ColumnBreak | FieldType::TabBreak
                => FieldKind::Layout,
            _ => FieldKind::Data,
        }
    }

    pub fn is_layout(&self) -> bool { self.kind() == FieldKind::Layout }
}

// ── LayoutKind ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LayoutKind {
    /// Regular single-document form.
    Form,
    /// Tree-structured doctype (e.g. Account, Territory).
    Tree,
    /// Submittable document with GL impact.
    Submittable,
}

// ── Permission ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Permission {
    pub role:    String,
    pub read:    bool,
    pub write:   bool,
    pub create:  bool,
    pub delete:  bool,
    pub submit:  bool,
    pub cancel:  bool,
    pub amend:   bool,
    pub report:  bool,
    pub import:  bool,
    pub export:  bool,
    pub print:   bool,
    pub email:   bool,
    pub share:   bool,
}

impl Permission {
    /// Full access for a role (typical System Manager grant).
    pub fn full(role: impl Into<String>) -> Self {
        Self {
            role: role.into(),
            read: true, write: true, create: true, delete: true,
            submit: true, cancel: true, amend: true,
            report: true, import: true, export: true,
            print: true, email: true, share: true,
        }
    }

    /// Read-only for a role.
    pub fn read_only(role: impl Into<String>) -> Self {
        Self {
            role: role.into(),
            read: true, write: false, create: false, delete: false,
            submit: false, cancel: false, amend: false,
            report: true, import: false, export: true,
            print: true, email: false, share: false,
        }
    }
}

// ── DocField ──────────────────────────────────────────────────────────────────

/// A single field in a DocType schema.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocField {
    pub fieldname:  String,
    pub label:      String,
    pub fieldtype:  FieldType,
    /// For `Link` fields — the target DocType name.
    /// For `Select` fields — use `select_options`.
    pub options:    Option<String>,
    /// For `Select` fields — `\n`-separated choices.
    pub select_options: Option<String>,
    pub reqd:       bool,
    pub in_list_view: bool,
    pub in_standard_filter: bool,
    pub bold:       bool,
    pub read_only:  bool,
    pub hidden:     bool,
    pub default_value: Option<String>,
    pub description:   Option<String>,
    pub precision:  Option<u8>,

    // ── Validation flags ──────────────────────────────────────────────────
    /// Cannot be changed once set (Frappe: `set_only_once`).
    pub set_only_once: bool,
    /// Field can be edited even after submission.
    pub allow_on_submit: bool,
    /// Skip XSS sanitization for this field.
    pub ignore_xss_filter: bool,
    /// Permission level — 0 = base, higher = sensitive.
    pub permlevel: u8,
    /// Unique constraint on this field.
    pub unique: bool,
    /// Field must not be NULL in DB.
    pub not_nullable: bool,
    /// Max length override (for Data/Text fields). 0 = use field-type default.
    pub length: Option<u32>,

    // ── Fetch-from (Link field auto-fill) ─────────────────────────────────
    /// `"linked_field.source_fieldname"` — copy value from linked doc.
    pub fetch_from: Option<String>,
    /// Only fetch when this field's current value is empty.
    pub fetch_if_empty: bool,
}

impl DocField {
    pub fn new(fieldname: impl Into<String>, label: impl Into<String>, fieldtype: FieldType) -> Self {
        Self {
            fieldname: fieldname.into(),
            label: label.into(),
            fieldtype,
            options: None,
            select_options: None,
            reqd: false,
            in_list_view: false,
            in_standard_filter: false,
            bold: false,
            read_only: false,
            hidden: false,
            default_value: None,
            description: None,
            precision: None,
            set_only_once: false,
            allow_on_submit: false,
            ignore_xss_filter: false,
            permlevel: 0,
            unique: false,
            not_nullable: false,
            length: None,
            fetch_from: None,
            fetch_if_empty: false,
        }
    }

    pub fn required(mut self) -> Self                  { self.reqd = true; self }
    pub fn in_list(mut self) -> Self                   { self.in_list_view = true; self }
    pub fn bold(mut self) -> Self                      { self.bold = true; self }
    pub fn read_only(mut self) -> Self                 { self.read_only = true; self }
    pub fn hidden(mut self) -> Self                    { self.hidden = true; self }
    pub fn unique(mut self) -> Self                    { self.unique = true; self }
    pub fn not_nullable(mut self) -> Self              { self.not_nullable = true; self }
    pub fn set_only_once(mut self) -> Self             { self.set_only_once = true; self }
    pub fn allow_on_submit(mut self) -> Self           { self.allow_on_submit = true; self }
    pub fn ignore_xss_filter(mut self) -> Self         { self.ignore_xss_filter = true; self }
    pub fn permlevel(mut self, lvl: u8) -> Self        { self.permlevel = lvl; self }
    pub fn length(mut self, n: u32) -> Self            { self.length = Some(n); self }
    pub fn fetch_from(mut self, s: impl Into<String>) -> Self {
        self.fetch_from = Some(s.into()); self
    }
    pub fn fetch_if_empty(mut self) -> Self            { self.fetch_if_empty = true; self }
    pub fn options(mut self, opts: impl Into<String>) -> Self {
        self.options = Some(opts.into()); self
    }
    pub fn select_options(mut self, opts: impl Into<String>) -> Self {
        self.select_options = Some(opts.into()); self
    }
    pub fn default(mut self, val: impl Into<String>) -> Self {
        self.default_value = Some(val.into()); self
    }
    pub fn precision(mut self, p: u8) -> Self          { self.precision = Some(p); self }
}

// ── DocTypeMeta ───────────────────────────────────────────────────────────────

/// Full typed schema for a DocType.
///
/// Compiled DocTypes provide this statically; WASM plugins serialize it via
/// `sl_register_doctype()` at load time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocTypeMeta {
    pub name:        String,
    pub module:      String,
    pub is_single:   bool,
    pub is_tree:     bool,
    pub is_child:    bool,
    pub is_submittable: bool,
    pub track_changes: bool,
    pub fields:      Vec<DocField>,
    pub permissions: Vec<Permission>,
    pub title_field: Option<String>,
    pub search_fields: Vec<String>,
    pub sort_field: Option<String>,
    pub sort_order: Option<String>,
    /// Frappe `autoname` — e.g. `"naming_series:"`, `"field:fieldname"`, `"hash"`, `"Prompt"`.
    pub autoname: Option<String>,
    /// Default naming series template shown in `naming_series` field.
    pub naming_series: Option<String>,
}

impl DocTypeMeta {
    pub fn builder(name: impl Into<String>, module: impl Into<String>) -> DocTypeMetaBuilder {
        DocTypeMetaBuilder::new(name, module)
    }
}

// ── DocTypeMetaBuilder ────────────────────────────────────────────────────────

pub struct DocTypeMetaBuilder {
    meta: DocTypeMeta,
}

impl DocTypeMetaBuilder {
    fn new(name: impl Into<String>, module: impl Into<String>) -> Self {
        Self {
            meta: DocTypeMeta {
                name: name.into(),
                module: module.into(),
                is_single: false,
                is_tree: false,
                is_child: false,
                is_submittable: false,
                track_changes: false,
                fields: Vec::new(),
                permissions: Vec::new(),
                title_field: None,
                search_fields: Vec::new(),
                sort_field: None,
                sort_order: None,
                autoname: None,
                naming_series: None,
            },
        }
    }

    pub fn autoname(mut self, s: impl Into<String>) -> Self {
        self.meta.autoname = Some(s.into()); self
    }

    pub fn naming_series(mut self, s: impl Into<String>) -> Self {
        self.meta.naming_series = Some(s.into()); self
    }

    pub fn single(mut self) -> Self           { self.meta.is_single = true; self }
    pub fn tree(mut self) -> Self             { self.meta.is_tree = true; self }
    pub fn child(mut self) -> Self            { self.meta.is_child = true; self }
    pub fn submittable(mut self) -> Self      { self.meta.is_submittable = true; self }
    pub fn track_changes(mut self) -> Self    { self.meta.track_changes = true; self }

    pub fn field(mut self, f: DocField) -> Self {
        self.meta.fields.push(f);
        self
    }

    pub fn permission(mut self, p: Permission) -> Self {
        self.meta.permissions.push(p);
        self
    }

    pub fn title_field(mut self, f: impl Into<String>) -> Self {
        self.meta.title_field = Some(f.into()); self
    }

    pub fn search_field(mut self, f: impl Into<String>) -> Self {
        self.meta.search_fields.push(f.into()); self
    }

    pub fn sort(mut self, field: impl Into<String>, order: impl Into<String>) -> Self {
        self.meta.sort_field = Some(field.into());
        self.meta.sort_order = Some(order.into());
        self
    }

    pub fn build(self) -> DocTypeMeta { self.meta }
}

// ── Frappe JSON Conversion ────────────────────────────────────────────────────

impl DocTypeMeta {
    /// Convert to Frappe REST API JSON response format.
    pub fn to_frappe_json(&self) -> serde_json::Value {
        use serde_json::{json, Value};

        let fields: Vec<Value> = self.fields
            .iter()
            .map(|f| f.to_frappe_json())
            .collect();

        let permissions: Vec<Value> = self.permissions
            .iter()
            .map(|p| p.to_frappe_json())
            .collect();

        let mut obj = json!({
            "name": self.name,
            "module": self.module,
            "doctype": "DocType",
            "issingle": self.is_single,
            "istree": self.is_tree,
            "issubmittable": self.is_submittable,
            "track_changes": self.track_changes,
            "is_child_table": self.is_child,
            "fields": fields,
            "permissions": permissions,
        });

        if let Value::Object(ref mut map) = obj {
            if let Some(tf) = &self.title_field {
                map.insert("title_field".into(), Value::String(tf.clone()));
            }
            if !self.search_fields.is_empty() {
                map.insert("search_fields".into(), Value::String(self.search_fields.join(", ")));
            }
            if let Some(sf) = &self.sort_field {
                map.insert("sort_field".into(), Value::String(sf.clone()));
            }
            if let Some(so) = &self.sort_order {
                map.insert("sort_order".into(), Value::String(so.clone()));
            }
            if let Some(an) = &self.autoname {
                map.insert("autoname".into(), Value::String(an.clone()));
            }
            if let Some(ns) = &self.naming_series {
                map.insert("naming_series".into(), Value::String(ns.clone()));
            }
            // Standard defaults for compatibility with Frappe desk
            map.entry("links".to_string()).or_insert(Value::Array(vec![]));
            map.entry("actions".to_string()).or_insert(Value::Array(vec![]));
            map.entry("states".to_string()).or_insert(Value::Array(vec![]));
        }

        obj
    }
}

impl DocField {
    /// Convert to Frappe REST API JSON format.
    pub fn to_frappe_json(&self) -> serde_json::Value {
        use serde_json::{json, Value};

        let fieldtype = match self.fieldtype {
            FieldType::Data => "Data",
            FieldType::SmallText => "Small Text",
            FieldType::Text => "Text",
            FieldType::LongText => "Long Text",
            FieldType::Code => "Code",
            FieldType::Password => "Password",
            FieldType::Int => "Int",
            FieldType::Float => "Float",
            FieldType::Currency => "Currency",
            FieldType::Percent => "Percent",
            FieldType::Check => "Check",
            FieldType::Date => "Date",
            FieldType::Datetime => "Datetime",
            FieldType::Time => "Time",
            FieldType::Select => "Select",
            FieldType::Link => "Link",
            FieldType::DynamicLink => "Dynamic Link",
            FieldType::Table => "Table",
            FieldType::TableMultiSelect => "Table MultiSelect",
            FieldType::Attach => "Attach",
            FieldType::AttachImage => "Attach Image",
            FieldType::Html => "HTML",
            FieldType::Signature => "Signature",
            FieldType::Color => "Color",
            FieldType::Rating => "Rating",
            FieldType::Json => "JSON",
            FieldType::SectionBreak => "Section Break",
            FieldType::ColumnBreak => "Column Break",
            FieldType::TabBreak => "Tab Break",
        };

        let mut obj = json!({
            "fieldname": self.fieldname,
            "label": self.label,
            "fieldtype": fieldtype,
            "reqd": self.reqd as i32,
            "in_list_view": self.in_list_view as i32,
            "in_standard_filter": self.in_standard_filter as i32,
            "bold": self.bold as i32,
            "read_only": self.read_only as i32,
            "hidden": self.hidden as i32,
            "set_only_once": self.set_only_once as i32,
            "allow_on_submit": self.allow_on_submit as i32,
            "ignore_xss_filter": self.ignore_xss_filter as i32,
            "permlevel": self.permlevel,
            "unique": self.unique as i32,
        });

        if let Value::Object(ref mut map) = obj {
            if let Some(opts) = &self.options {
                map.insert("options".into(), Value::String(opts.clone()));
            }
            if let Some(sel_opts) = &self.select_options {
                map.insert("options".into(), Value::String(sel_opts.clone()));
            }
            if let Some(dv) = &self.default_value {
                map.insert("default".into(), Value::String(dv.clone()));
            }
            if let Some(desc) = &self.description {
                map.insert("description".into(), Value::String(desc.clone()));
            }
            if let Some(prec) = self.precision {
                map.insert("precision".into(), Value::Number(prec.into()));
            }
            if let Some(len) = self.length {
                map.insert("length".into(), Value::Number(len.into()));
            }
            if let Some(ff) = &self.fetch_from {
                map.insert("fetch_from".into(), Value::String(ff.clone()));
            }
            if self.fetch_if_empty {
                map.insert("fetch_if_empty".into(), Value::Number(1.into()));
            }
        }

        obj
    }
}

impl Permission {
    /// Convert to Frappe REST API JSON format.
    pub fn to_frappe_json(&self) -> serde_json::Value {
        use serde_json::json;

        json!({
            "role": self.role,
            "read": self.read as i32,
            "write": self.write as i32,
            "create": self.create as i32,
            "delete": self.delete as i32,
            "submit": self.submit as i32,
            "cancel": self.cancel as i32,
            "amend": self.amend as i32,
            "report": self.report as i32,
            "import": self.import as i32,
            "export": self.export as i32,
            "print": self.print as i32,
            "email": self.email as i32,
            "share": self.share as i32,
            "permlevel": 0,
            "if_owner": 0,
        })
    }
}
