// DocType metadata types — matches Frappe JSON schema conventions

export type FieldType =
  | "Data" | "Small Text" | "Text" | "Long Text" | "Text Editor" | "Code" | "HTML"
  | "Int" | "Float" | "Currency" | "Percent" | "Rating"
  | "Check" | "Select" | "MultiSelect"
  | "Link" | "Dynamic Link"
  | "Date" | "Datetime" | "Time"
  | "Table" | "Table MultiSelect"
  | "Attach" | "Attach Image"
  | "Color" | "Signature" | "Geolocation"
  | "Section Break" | "Column Break" | "Tab Break"
  | "Read Only" | "Button" | "Heading" | "Password"
  | "Barcode" | "Duration";

export type VisibilityOperator =
  | "eq" | "ne" | "gt" | "lt" | "gte" | "lte"
  | "in" | "not_in" | "is_set" | "is_empty";

export interface SimpleRule {
  field: string;
  op: VisibilityOperator;
  value?: unknown;
}

export interface CompoundRule {
  all?: VisibilityRule[];
  any?: VisibilityRule[];
}

export type VisibilityRule = SimpleRule | CompoundRule;

export interface DocField {
  fieldname: string;
  fieldtype: FieldType;
  label: string;
  options?: string;           // Link target doctype, Select options (newline-separated), child doctype
  reqd?: 0 | 1;
  in_list_view?: 0 | 1;
  in_filter?: 0 | 1;
  read_only?: 0 | 1;
  hidden?: 0 | 1;
  bold?: 0 | 1;
  collapsible?: 0 | 1;
  collapsible_depends_on?: string;
  depends_on?: string;        // Frappe eval: string — parsed for backward compat
  mandatory_depends_on?: string;
  read_only_depends_on?: string;
  visible_if?: VisibilityRule; // NajaJS typed rule (preferred over depends_on)
  description?: string;
  default?: string;
  precision?: string;
  columns?: number;           // 1-4 grid column span
  fetch_from?: string;
  fetch_if_empty?: 0 | 1;
  in_global_search?: 0 | 1;
  search_index?: 0 | 1;
  allow_bulk_edit?: 0 | 1;
  translatable?: 0 | 1;
  permlevel?: number;
  // Child table / Link display
  show_title_field_in_link?: 0 | 1;
}

export interface DocTypeMeta {
  name: string;
  module?: string;
  doctype?: string;
  autoname?: string;
  title_field?: string;
  search_fields?: string;
  sort_field?: string;
  sort_order?: "ASC" | "DESC";
  is_submittable?: 0 | 1;
  istable?: 0 | 1;
  issingle?: 0 | 1;
  is_tree?: 0 | 1;
  quick_entry?: 0 | 1;
  track_changes?: 0 | 1;
  track_seen?: 0 | 1;
  track_views?: 0 | 1;
  show_title_field_in_link?: 0 | 1;
  description?: string;
  fields: DocField[];
  permissions?: DocPerm[];
  // NajaJS extensions
  _loaded_at?: number;        // cache timestamp
}

export interface DocPerm {
  role: string;
  read?: 0 | 1;
  write?: 0 | 1;
  create?: 0 | 1;
  delete?: 0 | 1;
  submit?: 0 | 1;
  cancel?: 0 | 1;
  amend?: 0 | 1;
  permlevel?: number;
}
