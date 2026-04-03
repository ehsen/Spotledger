// FormEngine — root form component, one per tab
// Creates its own Zustand store, loads the document, fires hooks, renders layout.

import { useEffect, useMemo, useRef, useCallback } from "react";
import { FormStoreContext } from "../naja/store/store-context";
import { create_form_store, use_model_registration } from "../naja/store/form-store";
import { meta as metaModule } from "../naja/meta";
import { call } from "../naja/call";
import { events } from "../naja/events";
import { formEvents } from "../naja/form-events";
import { ui } from "../naja/ui";
import { useShell } from "../store/shell-store";
import { FormLayout } from "./FormLayout";
import { FormToolbar } from "./FormToolbar";
import type { FormInstance, DocRecord, FieldProxy, TableProxy } from "../types/form";
import type { GetDocResponse } from "../types/api";

interface Props {
  doctype: string;
  docname?: string;   // undefined = new document
  tabId: string;
}

export function FormEngine({ doctype, docname, tabId }: Props) {
  // Create a per-tab store. useMemo ensures it's created once per tab mount.
  const store = useMemo(
    () => create_form_store(doctype, docname ?? `new-${doctype.toLowerCase().replace(/\s+/g, "-")}-1`),
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [tabId]
  );

  const useStore = store;
  const s = () => store.getState();
  const markShellDirty = useShell((sh) => sh.markDirty);

  // Register with naja.model
  const actualDocname = useStore((st) => st.docname);
  useEffect(
    () => use_model_registration(store, doctype, actualDocname),
    [store, doctype, actualDocname]
  );

  // Mark shell tab dirty indicator when form dirty flag changes
  const dirty = useStore((st) => st.dirty);
  useEffect(() => { markShellDirty(tabId, dirty); }, [dirty, tabId, markShellDirty]);

  // Initial load
  useEffect(() => {
    let cancelled = false;
    s().set_loading(true);

    const load = async () => {
      try {
        const meta = await metaModule.get_doctype(doctype);
        if (cancelled) return;

        let doc: DocRecord;
        const is_new = !docname;

        if (!is_new) {
          const res = await call<GetDocResponse>("frappe.desk.form.load.getdoc", {
            doctype,
            name: docname,
          });
          doc = (res as GetDocResponse).docs?.[0] ?? {};
        } else {
          doc = { doctype, name: s().docname, docstatus: 0 };
        }

        if (cancelled) return;
        s().load_doc(doc, meta, is_new);

        const form = build_form_instance(store, doctype, s().docname);
        await formEvents.dispatch_lifecycle(doctype, "onload", form);
        if (!cancelled) await formEvents.dispatch_lifecycle(doctype, "refresh", form);
      } catch (err) {
        if (!cancelled) {
          s().set_loading(false);
          ui.toast((err as Error).message, 5, "error");
        }
      }
    };

    load();
    return () => { cancelled = true; };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [doctype, docname, tabId]);

  const formInstance = useMemo(
    () => build_form_instance(store, doctype, actualDocname),
    // Rebuild when docname changes (after a new doc is saved)
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [store, doctype, actualDocname]
  );

  const handleFieldChange = useCallback(
    async (fieldname: string, value: unknown) => {
      s().set_value(fieldname, value);
      await formEvents.dispatch_field(doctype, fieldname, formInstance);
    },
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [doctype, formInstance]
  );

  const handleSave = useCallback(async () => {
    const form = formInstance;
    const ok = await formEvents.dispatch_lifecycle(doctype, "before_save", form);
    if (!ok) return;
    s().set_saving(true);
    try {
      const doc = s().doc;
      const res = await call<{ doc: DocRecord }>("frappe.desk.form.save.savedocs", {
        doc: JSON.stringify(doc),
        action: "Save",
      });
      const saved = (res as { doc: DocRecord }).doc ?? doc;
      s().load_doc(saved, s().meta!, false);
      s().reset_dirty();
      ui.toast("Saved", 2, "success");
      events.emit("doc:saved", { doctype, name: s().docname });
      await formEvents.dispatch_lifecycle(doctype, "after_save", form);
    } catch (err) {
      ui.toast((err as Error).message, 6, "error");
    } finally {
      s().set_saving(false);
    }
  }, [doctype, formInstance]);

  const handleSubmit = useCallback(async () => {
    const form = formInstance;
    const ok = await formEvents.dispatch_lifecycle(doctype, "before_submit", form);
    if (!ok) return;
    const confirmed = await ui.confirm(`Submit ${s().docname}?`, { title: "Confirm Submit" });
    if (!confirmed) return;
    s().set_saving(true);
    try {
      const res = await call<{ doc: DocRecord }>("frappe.desk.form.save.savedocs", {
        doc: JSON.stringify({ ...s().doc, docstatus: 1 }),
        action: "Submit",
      });
      const saved = (res as { doc: DocRecord }).doc ?? s().doc;
      s().load_doc(saved, s().meta!, false);
      s().reset_dirty();
      ui.toast("Submitted", 2, "success");
      events.emit("doc:submitted", { doctype, name: s().docname });
      await formEvents.dispatch_lifecycle(doctype, "after_submit", form);
    } catch (err) {
      ui.toast((err as Error).message, 6, "error");
    } finally {
      s().set_saving(false);
    }
  }, [doctype, formInstance]);

  const handleCancel = useCallback(async () => {
    const form = formInstance;
    const ok = await formEvents.dispatch_lifecycle(doctype, "before_cancel", form);
    if (!ok) return;
    const confirmed = await ui.confirm(`Cancel ${s().docname}?`, { title: "Confirm Cancel" });
    if (!confirmed) return;
    s().set_saving(true);
    try {
      await call("frappe.client.cancel", { doctype, name: s().docname });
      const res = await call<GetDocResponse>("frappe.desk.form.load.getdoc", {
        doctype, name: s().docname,
      });
      const doc = (res as GetDocResponse).docs?.[0] ?? s().doc;
      s().load_doc(doc, s().meta!, false);
      s().reset_dirty();
      ui.toast("Cancelled", 2, "success");
      events.emit("doc:cancelled", { doctype, name: s().docname });
      await formEvents.dispatch_lifecycle(doctype, "after_cancel", form);
    } catch (err) {
      ui.toast((err as Error).message, 6, "error");
    } finally {
      s().set_saving(false);
    }
  }, [doctype, formInstance]);

  const handleReload = useCallback(async () => {
    if (s().dirty) {
      const ok = await ui.confirm("Discard unsaved changes and reload?");
      if (!ok) return;
    }
    s().set_loading(true);
    try {
      const res = await call<GetDocResponse>("frappe.desk.form.load.getdoc", {
        doctype, name: s().docname,
      });
      const doc = (res as GetDocResponse).docs?.[0] ?? {};
      s().load_doc(doc, s().meta!, false);
      s().reset_dirty();
    } catch (err) {
      ui.toast((err as Error).message, 5, "error");
    }
  }, [doctype]);

  // Split into individual selectors — inline objects cause useSyncExternalStore infinite loops
  const loading = useStore((st) => st.loading);
  const meta    = useStore((st) => st.meta);
  const intro   = useStore((st) => st.intro);
  const fields = meta?.fields ?? [];

  // Keyboard shortcut Ctrl+S to save
  useEffect(() => {
    function handler(e: KeyboardEvent) {
      if ((e.ctrlKey || e.metaKey) && e.key === "s") {
        e.preventDefault();
        if (s().dirty || s().is_new) handleSave();
      }
    }
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [handleSave]);

  if (loading) {
    return (
      <div className="form-loading">
        <span className="spinner" />
        <span>Loading {doctype}…</span>
      </div>
    );
  }

  return (
    <FormStoreContext.Provider value={store}>
      <div className="form-engine">
        <FormToolbar
          formInstance={formInstance}
          tabId={tabId}
          onSave={handleSave}
          onSubmit={handleSubmit}
          onCancel={handleCancel}
          onReload={handleReload}
        />
        {intro && (
          <div className={`form-intro form-intro-${intro.type}`}>
            {intro.message}
          </div>
        )}
        <div className="form-body">
          <FormLayout fields={fields} onFieldChange={handleFieldChange} />
        </div>
      </div>
    </FormStoreContext.Provider>
  );
}

/** Build the FormInstance API wrapper around a store. */
function build_form_instance(
  store: ReturnType<typeof create_form_store>,
  doctype: string,
  docname: string,
): FormInstance {
  const s = () => store.getState();

  const field = (fieldname: string): FieldProxy => ({
    hide:   () => s().set_field_state(fieldname, { hidden: true }),
    show:   () => s().set_field_state(fieldname, { hidden: false }),
    enable: () => s().set_field_state(fieldname, { disabled: false }),
    disable:() => s().set_field_state(fieldname, { disabled: true }),
    require:() => s().set_field_state(fieldname, { required: true }),
    unrequire: () => s().set_field_state(fieldname, { required: false }),
    set_description: (text) => s().set_field_state(fieldname, { description: text }),
    set_options: (opts) => s().set_field_state(fieldname, { options: opts }),
    focus:  () => { const el = document.getElementById(`field-${fieldname}`); el?.focus(); },
    scroll_to: () => { const el = document.getElementById(`field-${fieldname}`); el?.scrollIntoView({ behavior: "smooth" }); },
  });

  const table = (fieldname: string): TableProxy => ({
    add_row:    (data) => { s().add_child(fieldname, data); },
    remove_row: (name) => s().remove_child(fieldname, name),
    clear:      () => s().clear_table(fieldname),
    get_rows:   () => s().get_children(fieldname),
    refresh:    () => { /* triggers via reactive state automatically */ },
  });

  return {
    doctype,
    docname,
    get doc() { return s().doc; },
    get_value: (field) => s().doc[field] ?? null,
    get_values: () => ({ ...s().doc }),
    set_value: (f, v) => s().set_value(f, v),
    set_values: (vals) => s().set_values(vals),
    field,
    toggle: (f, show) => s().set_field_state(f, { hidden: !show }),
    toggle_reqd: (f, req) => s().set_field_state(f, { required: req }),
    toggle_enable: (f, en) => s().set_field_state(f, { disabled: !en }),
    save: async () => { /* handled by FormEngine */ },
    submit: async () => { /* handled by FormEngine */ },
    cancel: async () => { /* handled by FormEngine */ },
    reload: async () => { /* handled by FormEngine */ },
    mark_dirty: () => s().mark_dirty(),
    add_button: (label, fn, group) => s().add_button({ label, fn, group }),
    remove_button: (label) => s().remove_button(label),
    clear_buttons: () => s().clear_buttons(),
    set_intro: (message, type = "info") => s().set_intro(message, type),
    clear_intro: () => s().clear_intro(),
    table,
    is_dirty: () => s().dirty,
    is_new: () => s().is_new,
    is_submitted: () => (s().doc?.docstatus as number) === 1,
    is_cancelled: () => (s().doc?.docstatus as number) === 2,
  };
}
