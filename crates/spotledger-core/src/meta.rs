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
        }
    }

    pub fn required(mut self) -> Self { self.reqd = true; self }
    pub fn in_list(mut self) -> Self  { self.in_list_view = true; self }
    pub fn bold(mut self) -> Self     { self.bold = true; self }
    pub fn read_only(mut self) -> Self { self.read_only = true; self }
    pub fn options(mut self, opts: impl Into<String>) -> Self {
        self.options = Some(opts.into()); self
    }
    pub fn default(mut self, val: impl Into<String>) -> Self {
        self.default_value = Some(val.into()); self
    }
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
            },
        }
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
