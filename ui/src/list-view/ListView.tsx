// ListView — DocType list view using Syncfusion DataGrid (Phase 5)
// For now renders a basic server-side paginated table.

import { useEffect, useState, useCallback } from "react";
import { call } from "../naja/call";
import { meta as metaModule } from "../naja/meta";
import { useShell } from "../store/shell-store";
import type { DocField } from "../types/doctype";

interface Props { doctype: string }

const PAGE_SIZE = 50;

export function ListView({ doctype }: Props) {
  const openTab = useShell((s) => s.openTab);
  const [cols, setCols] = useState<DocField[]>([]);
  const [rows, setRows] = useState<Record<string, unknown>[]>([]);
  const [total, setTotal] = useState(0);
  const [page, setPage] = useState(0);
  const [search, setSearch] = useState("");
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");

  // Load metadata and first page
  useEffect(() => {
    setLoading(true);
    setError("");
    setPage(0);
    metaModule.get_doctype(doctype).then((meta) => {
      const visible = meta.fields
        .filter(
          (f) =>
            f.in_list_view &&
            !["Section Break", "Column Break", "Tab Break", "HTML"].includes(f.fieldtype) &&
            !f.hidden,
        )
        .slice(0, 8);
      setCols(visible.length > 0 ? visible : meta.fields.filter((f) =>
        !["Section Break", "Column Break", "Tab Break"].includes(f.fieldtype) && !f.hidden
      ).slice(0, 5));
    }).catch(console.error);
  }, [doctype]);

  const load = useCallback(
    async (pageNum: number, searchVal: string) => {
      if (cols.length === 0) return;
      setLoading(true);
      setError("");
      try {
        const fieldnames = ["name", ...cols.map((c) => c.fieldname)];
        const filters: unknown[] = searchVal
          ? [["name", "like", `%${searchVal}%`]]
          : [];
        const [rows_res, count_res] = await Promise.all([
          call<Record<string, unknown>[]>("frappe.client.get_list", {
            doctype,
            fields: JSON.stringify(fieldnames),
            filters: JSON.stringify(filters),
            limit: PAGE_SIZE,
            limit_start: pageNum * PAGE_SIZE,
            order_by: "modified desc",
          }),
          call<number>("frappe.client.get_count", {
            doctype,
            filters: JSON.stringify(filters),
          }),
        ]);
        setRows(rows_res as Record<string, unknown>[]);
        setTotal(count_res as number);
      } catch (err) {
        setError((err as Error).message);
      } finally {
        setLoading(false);
      }
    },
    [cols, doctype],
  );

  useEffect(() => { load(page, search); }, [cols, page, search, load]);

  const total_pages = Math.ceil(total / PAGE_SIZE);

  return (
    <div className="list-view">
      <div className="list-header">
        <span className="list-title">{doctype}</span>
        <input
          className="list-search"
          placeholder="Search…"
          value={search}
          onChange={(e) => { setSearch(e.target.value); setPage(0); }}
        />
        <button
          className="btn btn-primary btn-sm"
          onClick={() => openTab({ kind: "form", doctype, title: `New ${doctype}` })}
        >
          New
        </button>
      </div>

      {loading && <div className="list-state">Loading…</div>}
      {!loading && error && <div className="list-state error">{error}</div>}
      {!loading && !error && (
        <>
          <div className="list-table-wrap">
            <table className="list-table">
              <thead>
                <tr>
                  <th>Name</th>
                  {cols.map((c) => <th key={c.fieldname}>{c.label || c.fieldname}</th>)}
                </tr>
              </thead>
              <tbody>
                {rows.length === 0 ? (
                  <tr><td colSpan={cols.length + 1} className="list-empty">No records</td></tr>
                ) : rows.map((row) => (
                  <tr
                    key={String(row.name)}
                    className="list-row"
                    onClick={() =>
                      openTab({
                        kind: "form",
                        doctype,
                        docname: String(row.name),
                        title: String(row.name),
                      })
                    }
                  >
                    <td className="list-name">{String(row.name ?? "")}</td>
                    {cols.map((c) => (
                      <td key={c.fieldname}>{String(row[c.fieldname] ?? "")}</td>
                    ))}
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
          {total_pages > 1 && (
            <div className="list-pagination">
              <button
                className="btn btn-ghost btn-sm"
                disabled={page === 0}
                onClick={() => setPage((p) => p - 1)}
              >‹ Prev</button>
              <span className="page-info">{page + 1} / {total_pages} ({total} total)</span>
              <button
                className="btn btn-ghost btn-sm"
                disabled={page >= total_pages - 1}
                onClick={() => setPage((p) => p + 1)}
              >Next ›</button>
            </div>
          )}
        </>
      )}
    </div>
  );
}
