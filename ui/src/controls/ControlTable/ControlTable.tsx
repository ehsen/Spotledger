// ControlTable — inline child table editor
// Uses the form store to read/write rows reactively.
// Syncfusion DataGrid will replace this in Phase 4.

import { useEffect, useState, useCallback } from "react";
import { meta as metaModule } from "../../naja/meta";
import { useFormStore } from "../../naja/store/store-context";
import type { ControlProps } from "../registry";
import type { DocField } from "../../types/doctype";
import type { DocRecord } from "../../types/form";

export function ControlTable({ field }: ControlProps) {
  const useStore = useFormStore();
  const rows = useStore((s) => (s.doc[field.fieldname] as DocRecord[]) ?? []);
  const addChild = useStore((s) => s.add_child);
  const removeChild = useStore((s) => s.remove_child);
  const setChildValue = useStore((s) => s.set_child_value);

  const child_doctype = field.options ?? "";
  const [child_fields, set_child_fields] = useState<DocField[]>([]);

  useEffect(() => {
    if (!child_doctype) return;
    metaModule.get_doctype(child_doctype)
      .then((meta) => {
        set_child_fields(
          meta.fields.filter(
            (f) =>
              !["Section Break", "Column Break", "Tab Break", "Button"].includes(f.fieldtype) &&
              f.in_list_view,
          ),
        );
      })
      .catch(console.error);
  }, [child_doctype]);

  const handleAddRow = useCallback(() => {
    addChild(field.fieldname);
  }, [addChild, field.fieldname]);

  const handleRemoveRow = useCallback((name: string) => {
    removeChild(field.fieldname, name);
  }, [removeChild, field.fieldname]);

  const handleCellChange = useCallback(
    (row_name: string, fieldname: string, value: unknown) => {
      setChildValue(field.fieldname, row_name, fieldname, value);
    },
    [setChildValue, field.fieldname],
  );

  return (
    <div className="form-table">
      <div className="form-table-wrap">
        <table className="form-table-grid">
          <thead>
            <tr>
              <th className="table-idx">#</th>
              {child_fields.map((f) => (
                <th key={f.fieldname} className="table-header-cell">{f.label}</th>
              ))}
              <th className="table-actions-header" />
            </tr>
          </thead>
          <tbody>
            {rows.length === 0 && (
              <tr>
                <td colSpan={child_fields.length + 2} className="table-empty">
                  No rows. Click Add Row below.
                </td>
              </tr>
            )}
            {rows.map((row, idx) => (
              <tr key={String(row.name)} className="table-row">
                <td className="table-idx">{idx + 1}</td>
                {child_fields.map((f) => (
                  <td key={f.fieldname} className="table-cell">
                    <TableCell
                      field={f}
                      value={row[f.fieldname] ?? null}
                      row_name={String(row.name)}
                      onChange={handleCellChange}
                    />
                  </td>
                ))}
                <td className="table-actions">
                  <button
                    className="table-row-delete"
                    onClick={() => handleRemoveRow(String(row.name))}
                    title="Remove row"
                    type="button"
                  >
                    ×
                  </button>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      <div className="form-table-footer">
        <button className="btn btn-ghost btn-sm" type="button" onClick={handleAddRow}>
          + Add Row
        </button>
      </div>
    </div>
  );
}

function TableCell({
  field,
  value,
  row_name,
  onChange,
}: {
  field: DocField;
  value: unknown;
  row_name: string;
  onChange: (row_name: string, fieldname: string, value: unknown) => void;
}) {
  const { fieldtype, fieldname } = field;

  if (fieldtype === "Check") {
    return (
      <input
        type="checkbox"
        checked={value === 1 || value === true}
        onChange={(e) => onChange(row_name, fieldname, e.target.checked ? 1 : 0)}
      />
    );
  }

  if (fieldtype === "Select") {
    const opts = (field.options ?? "").split("\n").filter(Boolean);
    return (
      <select
        className="table-cell-input"
        value={String(value ?? "")}
        onChange={(e) => onChange(row_name, fieldname, e.target.value)}
      >
        <option value="">—</option>
        {opts.map((o) => <option key={o} value={o}>{o}</option>)}
      </select>
    );
  }

  const inputType =
    fieldtype === "Int" || fieldtype === "Float" || fieldtype === "Currency" || fieldtype === "Percent"
      ? "number"
      : fieldtype === "Date"
      ? "date"
      : "text";

  return (
    <input
      type={inputType}
      className="table-cell-input"
      value={value === null || value === undefined ? "" : String(value)}
      onChange={(e) => {
        const v = e.target.value;
        onChange(row_name, fieldname,
          inputType === "number" ? (v === "" ? null : parseFloat(v)) : (v || null),
        );
      }}
    />
  );
}
