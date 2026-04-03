// naja.form.on — form lifecycle hook registry and dispatcher
// Hooks are registered per DocType and fired at lifecycle events.

import type { FormInstance } from "../types/form";
import type { DocRecord } from "../types/form";

export type FormHookFn = (form: FormInstance, row?: DocRecord) => void | Promise<void>;
export type FormCancelableFn = (form: FormInstance) => boolean | void | Promise<boolean | void>;

export type FormEventMap = {
  onload?: FormHookFn;
  refresh?: FormHookFn;
  before_save?: FormCancelableFn;
  after_save?: FormHookFn;
  before_submit?: FormCancelableFn;
  after_submit?: FormHookFn;
  before_cancel?: FormCancelableFn;
  after_cancel?: FormHookFn;
  // Field-level hooks keyed by fieldname
  [fieldname: string]: FormHookFn | FormCancelableFn | undefined;
};

// Registry: doctype -> list of hook objects (merged in registration order)
const registry = new Map<string, FormEventMap[]>();

export const formEvents = {
  /** Register hooks for a DocType. Multiple calls stack. */
  on(doctype: string, hooks: FormEventMap): () => void {
    if (!registry.has(doctype)) registry.set(doctype, []);
    registry.get(doctype)!.push(hooks);
    return () => {
      const list = registry.get(doctype);
      if (list) {
        const idx = list.indexOf(hooks);
        if (idx >= 0) list.splice(idx, 1);
      }
    };
  },

  /** Dispatch a lifecycle event, returning false if any hook cancelled it. */
  async dispatch_lifecycle(
    doctype: string,
    event: "onload" | "refresh" | "before_save" | "after_save" |
           "before_submit" | "after_submit" | "before_cancel" | "after_cancel",
    form: FormInstance,
  ): Promise<boolean> {
    const hooks = registry.get(doctype) ?? [];
    for (const hookSet of hooks) {
      const handler = hookSet[event];
      if (typeof handler === "function") {
        const result = await (handler as (f: FormInstance) => boolean | void | Promise<boolean | void>)(form);
        if (result === false) return false;
      }
    }
    return true;
  },

  /** Dispatch a field change event. */
  async dispatch_field(
    doctype: string,
    fieldname: string,
    form: FormInstance,
    row?: DocRecord,
  ): Promise<void> {
    const hooks = registry.get(doctype) ?? [];
    for (const hookSet of hooks) {
      const handler = hookSet[fieldname];
      if (typeof handler === "function") {
        await (handler as (f: FormInstance, r?: DocRecord) => void | Promise<void>)(form, row);
      }
    }
  },
};
