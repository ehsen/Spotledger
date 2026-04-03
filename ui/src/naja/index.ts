// naja — public API surface
// All userland code, custom scripts, and hooks interact exclusively through this object.
// Nothing else from the naja/ directory is imported by outside code.

import type { ComponentType } from "react";
import { call } from "./call";
import { model } from "./model";
import { meta } from "./meta";
import { session } from "./session";
import { ui } from "./ui";
import { events } from "./events";
import { boot } from "./boot";
import { formEvents } from "./form-events";

export const naja = {
  call,
  model,
  meta,
  session,
  ui,
  events,
  boot,

  /** naja.form — lifecycle hooks and custom actions */
  form: {
    on: formEvents.on.bind(formEvents),

    /** Add a toolbar action that appears on all forms of a DocType. */
    add_action(doctype: string, action: {
      label: string;
      fn: (form: import("../types/form").FormInstance) => void | Promise<void>;
      group?: string;
    }) {
      formEvents.on(doctype, {
        onload: (form) => { form.add_button(action.label, action.fn, action.group); },
      });
    },
  },

  /** naja.controls — custom field type registration. */
  controls: {
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    _registry: new Map<string, ComponentType<any>>(),
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    register(fieldtype: string, component: ComponentType<any>) {
      naja.controls._registry.set(fieldtype, component);
    },
  },
};

// Make naja available globally for custom scripts (like frappe is in Frappe)
if (typeof window !== "undefined") {
  (window as unknown as Record<string, unknown>).naja = naja;
}

export default naja;
export { call } from "./call";
export { model } from "./model";
export { meta } from "./meta";
export { session } from "./session";
export { ui } from "./ui";
export { events } from "./events";
export { boot } from "./boot";
export { formEvents } from "./form-events";
export { create_form_store, make_model_adapter, use_model_registration } from "./store/form-store";
export { FormStoreContext, useFormStore } from "./store/store-context";
