// naja.model — document state operations
// Reactive state backed by per-tab Zustand stores.
// Reading returns current state; setting triggers reactive UI updates.

import type { DocRecord } from "../types/form";

// The model module delegates to the active store registry — each form tab
// registers its own store. Model operations on a (doctype, name) pair
// look up the right store and mutate it directly.

export interface ModelStore {
  get_value(field: string): unknown;
  set_value(field: string, value: unknown): void;
  set_values(values: Record<string, unknown>): void;
  get_doc(): DocRecord;
  is_dirty(): boolean;
  mark_dirty(): void;
  // Child table ops
  add_child(table_field: string, row_data?: DocRecord): DocRecord;
  remove_child(table_field: string, row_name: string): void;
  get_children(table_field: string): DocRecord[];
  set_child_value(table_field: string, row_name: string, field: string, value: unknown): void;
  clear_table(table_field: string): void;
}

type StoreKey = string; // `${doctype}::${name}`
const registry = new Map<StoreKey, ModelStore>();

function key(doctype: string, name: string): StoreKey {
  return `${doctype}::${name}`;
}

export const model = {
  /** Register a form store for a tab. Called by the form engine on mount. */
  _register(doctype: string, name: string, store: ModelStore) {
    registry.set(key(doctype, name), store);
  },

  /** Unregister when a tab closes. */
  _unregister(doctype: string, name: string) {
    registry.delete(key(doctype, name));
  },

  _get_store(doctype: string, name: string): ModelStore | null {
    return registry.get(key(doctype, name)) ?? null;
  },

  get_value(doctype: string, name: string, field: string): unknown {
    return registry.get(key(doctype, name))?.get_value(field) ?? null;
  },

  get_doc(doctype: string, name: string): DocRecord | null {
    return registry.get(key(doctype, name))?.get_doc() ?? null;
  },

  set_value(doctype: string, name: string, field: string, value: unknown) {
    registry.get(key(doctype, name))?.set_value(field, value);
  },

  set_values(doctype: string, name: string, values: Record<string, unknown>) {
    registry.get(key(doctype, name))?.set_values(values);
  },

  // --- Child tables ---
  add_child(doc: DocRecord, table_field: string, row_data?: DocRecord): DocRecord {
    const { doctype, name } = doc as { doctype: string; name: string };
    return registry.get(key(doctype, name))?.add_child(table_field, row_data) ?? {};
  },

  remove_child(doc: DocRecord, table_field: string, row_name: string) {
    const { doctype, name } = doc as { doctype: string; name: string };
    registry.get(key(doctype, name))?.remove_child(table_field, row_name);
  },

  get_children(doc: DocRecord, table_field: string): DocRecord[] {
    const { doctype, name } = doc as { doctype: string; name: string };
    return registry.get(key(doctype, name))?.get_children(table_field) ?? [];
  },

  set_child_value(doc: DocRecord, table_field: string, row_name: string, field: string, value: unknown) {
    const { doctype, name } = doc as { doctype: string; name: string };
    registry.get(key(doctype, name))?.set_child_value(table_field, row_name, field, value);
  },

  clear_table(doc: DocRecord, table_field: string) {
    const { doctype, name } = doc as { doctype: string; name: string };
    registry.get(key(doctype, name))?.clear_table(table_field);
  },

  is_dirty(doctype: string, name: string): boolean {
    return registry.get(key(doctype, name))?.is_dirty() ?? false;
  },
};
