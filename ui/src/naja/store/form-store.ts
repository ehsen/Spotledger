// Per-tab Zustand store factory
// Each form tab gets its own isolated store. No globals, no shared state.

import { create } from "zustand";
import { immer } from "zustand/middleware/immer";
import type { DocRecord, FieldState, ButtonDefinition, FormState } from "../../types/form";
import type { DocTypeMeta } from "../../types/doctype";
import type { ModelStore } from "../model";
import { model } from "../model";

interface FormStoreActions {
  // Lifecycle
  load_doc(doc: DocRecord, meta: DocTypeMeta, is_new: boolean): void;
  reset_original(): void;

  // Values — reactive
  set_value(field: string, value: unknown): void;
  set_values(values: Record<string, unknown>): void;

  // Field state
  set_field_state(field: string, patch: Partial<FieldState>): void;
  set_error(field: string, error: string | null): void;
  clear_field_states(): void;

  // Intro
  set_intro(message: string, type: FormState["intro"] extends null ? never : NonNullable<FormState["intro"]>["type"]): void;
  clear_intro(): void;

  // Buttons
  add_button(btn: ButtonDefinition): void;
  remove_button(label: string): void;
  clear_buttons(): void;

  // Lifecycle flags
  set_loading(v: boolean): void;
  set_saving(v: boolean): void;
  mark_dirty(): void;
  reset_dirty(): void;

  // Child tables
  add_child(table_field: string, row_data?: DocRecord): DocRecord;
  remove_child(table_field: string, row_name: string): void;
  get_children(table_field: string): DocRecord[];
  set_child_value(table_field: string, row_name: string, field: string, value: unknown): void;
  clear_table(table_field: string): void;
}

export type FormStore = FormState & FormStoreActions;

function generate_row_name(): string {
  return "new-row-" + Math.random().toString(36).slice(2, 10);
}

export function create_form_store(doctype: string, docname: string) {
  return create<FormStore>()(
    immer((set, get) => ({
      // Initial state
      doctype,
      docname,
      doc: {},
      original_doc: {},
      meta: null,
      field_states: {},
      intro: null,
      buttons: [],
      loading: false,
      saving: false,
      dirty: false,
      is_new: false,

      load_doc(doc, meta, is_new) {
        set((s) => {
          s.doc = doc;
          s.original_doc = structuredClone(doc);
          s.meta = meta;
          s.is_new = is_new;
          s.dirty = is_new; // new docs start dirty
          s.loading = false;
          s.field_states = {};
        });
      },

      reset_original() {
        set((s) => { s.original_doc = structuredClone(s.doc); s.dirty = false; });
      },

      set_value(field, value) {
        set((s) => {
          s.doc[field] = value;
          s.dirty = true;
        });
      },

      set_values(values) {
        set((s) => {
          Object.assign(s.doc, values);
          s.dirty = true;
        });
      },

      set_field_state(field, patch) {
        set((s) => {
          s.field_states[field] = { ...(s.field_states[field] ?? {}), ...patch };
        });
      },

      set_error(field, error) {
        set((s) => {
          if (!s.field_states[field]) s.field_states[field] = {};
          if (error === null) {
            delete s.field_states[field].error;
          } else {
            s.field_states[field].error = error;
          }
        });
      },

      clear_field_states() {
        set((s) => { s.field_states = {}; });
      },

      set_intro(message, type) {
        set((s) => { s.intro = { message, type }; });
      },

      clear_intro() {
        set((s) => { s.intro = null; });
      },

      add_button(btn) {
        set((s) => {
          s.buttons = s.buttons.filter((b) => b.label !== btn.label);
          s.buttons.push(btn);
        });
      },

      remove_button(label) {
        set((s) => { s.buttons = s.buttons.filter((b) => b.label !== label); });
      },

      clear_buttons() {
        set((s) => { s.buttons = []; });
      },

      set_loading(v) { set((s) => { s.loading = v; }); },
      set_saving(v) { set((s) => { s.saving = v; }); },
      mark_dirty() { set((s) => { s.dirty = true; }); },
      reset_dirty() { set((s) => { s.dirty = false; }); },

      // Child tables
      add_child(table_field, row_data = {}) {
        const row: DocRecord = {
          name: generate_row_name(),
          doctype: get().meta?.fields.find((f) => f.fieldname === table_field)?.options ?? "",
          parenttype: get().doctype,
          parent: get().docname,
          parentfield: table_field,
          idx: ((get().doc[table_field] as DocRecord[] | undefined)?.length ?? 0) + 1,
          ...row_data,
        };
        set((s) => {
          const existing = s.doc[table_field] as DocRecord[] | undefined;
          s.doc[table_field] = [...(existing ?? []), row];
          s.dirty = true;
        });
        return row;
      },

      remove_child(table_field, row_name) {
        set((s) => {
          const rows = (s.doc[table_field] as DocRecord[] | undefined) ?? [];
          s.doc[table_field] = rows.filter((r) => r.name !== row_name);
          s.dirty = true;
        });
      },

      get_children(table_field) {
        return (get().doc[table_field] as DocRecord[] | undefined) ?? [];
      },

      set_child_value(table_field, row_name, field, value) {
        set((s) => {
          const rows = (s.doc[table_field] as DocRecord[] | undefined) ?? [];
          const row = rows.find((r) => r.name === row_name);
          if (row) row[field] = value;
          s.dirty = true;
        });
      },

      clear_table(table_field) {
        set((s) => { s.doc[table_field] = []; s.dirty = true; });
      },
    }))
  );
}

/** Build a ModelStore adapter from a form store instance. */
export function make_model_adapter(store: ReturnType<typeof create_form_store>): ModelStore {
  const s = () => store.getState();
  return {
    get_value: (field) => s().doc[field] ?? null,
    set_value: (field, value) => s().set_value(field, value),
    set_values: (values) => s().set_values(values),
    get_doc: () => s().doc,
    is_dirty: () => s().dirty,
    mark_dirty: () => s().mark_dirty(),
    add_child: (tf, row) => s().add_child(tf, row),
    remove_child: (tf, name) => s().remove_child(tf, name),
    get_children: (tf) => s().get_children(tf),
    set_child_value: (tf, rn, f, v) => s().set_child_value(tf, rn, f, v),
    clear_table: (tf) => s().clear_table(tf),
  };
}

/** Register the store with naja.model on mount, unregister on unmount. */
export function use_model_registration(
  store: ReturnType<typeof create_form_store>,
  doctype: string,
  docname: string,
) {
  model._register(doctype, docname, make_model_adapter(store));
  return () => model._unregister(doctype, docname);
}
