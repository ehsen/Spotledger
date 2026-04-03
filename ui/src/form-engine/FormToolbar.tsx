// FormToolbar — Save, Submit, Cancel, and custom buttons

import { useShell } from "../store/shell-store";
import { useFormStore } from "../naja/store/store-context";
import type { FormInstance } from "../types/form";

interface Props {
  formInstance: FormInstance;
  tabId: string;
  onSave: () => Promise<void>;
  onSubmit: () => Promise<void>;
  onCancel: () => Promise<void>;
  onReload: () => Promise<void>;
}

export function FormToolbar({ formInstance, tabId, onSave, onSubmit, onCancel, onReload }: Props) {
  const useStore = useFormStore();
  const dirty = useStore((s) => s.dirty);
  const saving = useStore((s) => s.saving);
  const is_new = useStore((s) => s.is_new);
  const meta = useStore((s) => s.meta);
  const doc = useStore((s) => s.doc);
  const custom_buttons = useStore((s) => s.buttons);
  const markDirty = useShell((s) => s.markDirty);

  const docstatus = (doc?.docstatus as number) ?? 0;
  const is_submittable = !!meta?.is_submittable;
  const is_submitted = docstatus === 1;
  const is_cancelled = docstatus === 2;

  // Group custom buttons
  const ungrouped = custom_buttons.filter((b) => !b.group);
  const grouped = new Map<string, typeof custom_buttons>();
  for (const b of custom_buttons.filter((b) => b.group)) {
    if (!grouped.has(b.group!)) grouped.set(b.group!, []);
    grouped.get(b.group!)!.push(b);
  }

  return (
    <div className="form-toolbar">
      {/* Left side — status breadcrumb */}
      <div className="form-toolbar-left">
        <span className="form-meta-name">{meta?.name ?? ""}</span>
        {is_submitted && <span className="badge badge-submitted">Submitted</span>}
        {is_cancelled && <span className="badge badge-cancelled">Cancelled</span>}
        {is_new && <span className="badge badge-new">New</span>}
        {dirty && !saving && <span className="form-dirty-dot" title="Unsaved changes">●</span>}
      </div>

      {/* Right side — actions */}
      <div className="form-toolbar-right">
        {/* Custom ungrouped buttons */}
        {ungrouped.map((btn) => (
          <button
            key={btn.label}
            className={`btn btn-${btn.variant ?? "default"}`}
            onClick={() => btn.fn(formInstance)}
          >
            {btn.label}
          </button>
        ))}

        {/* Custom grouped button dropdowns */}
        {Array.from(grouped.entries()).map(([group, btns]) => (
          <div key={group} className="btn-group-dropdown">
            <span className="btn btn-grouped">{group}</span>
            <div className="btn-group-menu">
              {btns.map((b) => (
                <button key={b.label} className="btn-group-item" onClick={() => b.fn(formInstance)}>
                  {b.label}
                </button>
              ))}
            </div>
          </div>
        ))}

        {/* Reload */}
        <button
          className="btn btn-ghost"
          onClick={onReload}
          disabled={saving}
          title="Reload from server"
        >
          ↺
        </button>

        {/* Submit / Cancel */}
        {is_submittable && !is_submitted && !is_new && docstatus === 0 && (
          <button className="btn btn-primary" onClick={onSubmit} disabled={saving || dirty}>
            Submit
          </button>
        )}
        {is_submitted && (
          <button className="btn btn-danger" onClick={onCancel} disabled={saving}>
            Cancel
          </button>
        )}

        {/* Save */}
        {!is_submitted && !is_cancelled && (
          <button
            className="btn btn-primary"
            onClick={onSave}
            disabled={saving || (!dirty && !is_new)}
          >
            {saving ? "Saving…" : "Save"}
          </button>
        )}
      </div>
    </div>
  );
}
