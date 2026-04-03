import type { DocTypeMeta } from "./doctype";

// The raw document object — all field values are stored here
export type DocRecord = Record<string, unknown>;

// Per-field UI state — separate from doc values
export interface FieldState {
  hidden?: boolean;
  disabled?: boolean;
  required?: boolean;
  description?: string;
  options?: string[];   // override Select options
  error?: string;
}

// A definition for a toolbar button added by custom scripts / hooks
export interface ButtonDefinition {
  label: string;
  fn: (form: FormInstance) => void | Promise<void>;
  group?: string;
  variant?: "default" | "primary" | "danger";
}

// The reactive state store per form tab (Zustand)
export interface FormState {
  doctype: string;
  docname: string;
  doc: DocRecord;
  original_doc: DocRecord;
  meta: DocTypeMeta | null;
  field_states: Record<string, FieldState>;
  intro: { message: string; type: "info" | "warning" | "error" | "success" } | null;
  buttons: ButtonDefinition[];
  loading: boolean;
  saving: boolean;
  dirty: boolean;
  is_new: boolean;
}

// Public form API exposed to userland code and hooks
export interface FieldProxy {
  hide(): void;
  show(): void;
  enable(): void;
  disable(): void;
  require(): void;
  unrequire(): void;
  set_description(text: string): void;
  set_options(options: string[]): void;
  focus(): void;
  scroll_to(): void;
}

export interface TableProxy {
  add_row(data?: DocRecord): void;
  remove_row(row_name: string): void;
  clear(): void;
  get_rows(): DocRecord[];
  refresh(): void;
}

export interface FormInstance {
  // Identity
  doctype: string;
  docname: string;
  doc: DocRecord;

  // Values
  get_value(field: string): unknown;
  get_values(): DocRecord;
  set_value(field: string, value: unknown): void;
  set_values(values: Record<string, unknown>): void;

  // Field proxies
  field(fieldname: string): FieldProxy;
  toggle(fieldname: string, show: boolean): void;
  toggle_reqd(fieldname: string, required: boolean): void;
  toggle_enable(fieldname: string, enabled: boolean): void;

  // Actions
  save(): Promise<void>;
  submit(): Promise<void>;
  cancel(): Promise<void>;
  reload(): Promise<void>;
  mark_dirty(): void;

  // Toolbar
  add_button(label: string, fn: (form: FormInstance) => void, group?: string): void;
  remove_button(label: string): void;
  clear_buttons(): void;

  // Header messages
  set_intro(message: string, type?: "info" | "warning" | "error" | "success"): void;
  clear_intro(): void;

  // Child tables
  table(fieldname: string): TableProxy;

  // Status
  is_dirty(): boolean;
  is_new(): boolean;
  is_submitted(): boolean;
  is_cancelled(): boolean;
}
