// naja.meta — DocType metadata cache
// Fetched once, cached indefinitely. All reads are synchronous after first load.

import type { DocTypeMeta } from "../types/doctype";
import { events } from "./events";

const cache = new Map<string, DocTypeMeta>();
const pending = new Map<string, Promise<DocTypeMeta>>();

async function fetch_meta(doctype: string): Promise<DocTypeMeta> {
  const res = await fetch(
    `/api/method/frappe.desk.form.load.getdoctype?doctype=${encodeURIComponent(doctype)}`,
    { credentials: "include" }
  );
  if (!res.ok) throw new Error(`getdoctype failed: HTTP ${res.status}`);
  const json = await res.json() as { docs?: DocTypeMeta[] };
  const docs = json.docs ?? [];
  const meta = docs.find((d) => d.name === doctype) ?? docs[0];
  if (!meta) throw new Error(`DocType not found: ${doctype}`);
  return { ...meta, _loaded_at: Date.now() };
}

export const meta = {
  /** Load a DocType, returning cached copy if available. */
  async get_doctype(doctype: string): Promise<DocTypeMeta> {
    if (cache.has(doctype)) return cache.get(doctype)!;
    if (pending.has(doctype)) return pending.get(doctype)!;
    const p = fetch_meta(doctype).then((m) => {
      cache.set(doctype, m);
      pending.delete(doctype);
      events.emit("meta:loaded", { doctype });
      return m;
    }).catch((e) => { pending.delete(doctype); throw e; });
    pending.set(doctype, p);
    return p;
  },

  /** Synchronous read — only valid after get_doctype() has resolved. */
  get_doctype_sync(doctype: string): DocTypeMeta | null {
    return cache.get(doctype) ?? null;
  },

  get_field(doctype: string, fieldname: string) {
    return cache.get(doctype)?.fields.find((f) => f.fieldname === fieldname) ?? null;
  },

  get_fields(doctype: string) {
    return cache.get(doctype)?.fields ?? [];
  },

  get_table_fields(doctype: string) {
    return (cache.get(doctype)?.fields ?? []).filter(
      (f) => f.fieldtype === "Table" || f.fieldtype === "Table MultiSelect",
    );
  },

  get_link_fields(doctype: string) {
    return (cache.get(doctype)?.fields ?? []).filter(
      (f) => f.fieldtype === "Link" || f.fieldtype === "Dynamic Link",
    );
  },

  has_field(doctype: string, fieldname: string): boolean {
    return !!(cache.get(doctype)?.fields ?? []).find((f) => f.fieldname === fieldname);
  },

  is_submittable(doctype: string): boolean {
    return !!(cache.get(doctype)?.is_submittable);
  },

  get_title_field(doctype: string): string {
    return cache.get(doctype)?.title_field ?? "name";
  },

  /** Force-reload a doctype (clears cache entry). */
  invalidate(doctype: string) {
    cache.delete(doctype);
    pending.delete(doctype);
  },
};
