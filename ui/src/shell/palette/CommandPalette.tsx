import { useEffect, useRef, useState } from "react";
import { Command } from "cmdk";
import { call } from "../../naja/call";
import { useShell } from "../../store/shell-store";

interface Result {
  id: string;
  label: string;
  category: string;
  action: () => void;
}

export function CommandPalette() {
  const open = useShell((s) => s.paletteOpen);
  const setPaletteOpen = useShell((s) => s.setPaletteOpen);
  const openTab = useShell((s) => s.openTab);
  const tabs = useShell((s) => s.tabs);
  const [search, setSearch] = useState("");
  const [doctypes, setDoctypes] = useState<string[]>([]);
  const inputRef = useRef<HTMLInputElement>(null);

  // Load all non-child-table DocTypes once
  useEffect(() => {
    call<{ name: string; istable: number }[]>("frappe.client.get_list", {
      doctype: "DocType",
      fields: JSON.stringify(["name", "istable"]),
      filters: JSON.stringify([["istable", "!=", 1]]),
      limit: 500,
      order_by: "name asc",
    }, { cache: 300, background: true })
      .then((rows) => setDoctypes((rows as { name: string }[]).map((r) => r.name)))
      .catch(console.error);
  }, []);

  // Ctrl+K / ⌘+K
  useEffect(() => {
    function handler(e: KeyboardEvent) {
      if ((e.ctrlKey || e.metaKey) && e.key === "k") {
        e.preventDefault();
        setPaletteOpen(true);
      }
    }
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [setPaletteOpen]);

  function close() {
    setPaletteOpen(false);
    setSearch("");
  }

  if (!open) return null;

  // Build result groups
  const q = search.toLowerCase();
  const filteredDoctypes = q
    ? doctypes.filter((d) => d.toLowerCase().includes(q))
    : doctypes;

  const openTabResults: Result[] = tabs
    .filter((t) => q ? t.title.toLowerCase().includes(q) : true)
    .map((t) => ({
      id: `tab:${t.id}`,
      label: t.title,
      category: "Open Tabs",
      action: () => { openTab(t); close(); },
    }));

  const doctypeListResults: Result[] = filteredDoctypes.slice(0, 20).map((dt) => ({
    id: `list:${dt}`,
    label: dt,
    category: "Open List",
    action: () => { openTab({ kind: "list", doctype: dt, title: dt }); close(); },
  }));

  const newDocResults: Result[] = filteredDoctypes.slice(0, 10).map((dt) => ({
    id: `new:${dt}`,
    label: `New ${dt}`,
    category: "Create New",
    action: () => { openTab({ kind: "form", doctype: dt, title: `New ${dt}` }); close(); },
  }));

  const allResults = [...openTabResults, ...doctypeListResults, ...newDocResults];
  const grouped = new Map<string, Result[]>();
  for (const r of allResults) {
    if (!grouped.has(r.category)) grouped.set(r.category, []);
    grouped.get(r.category)!.push(r);
  }

  return (
    <div className="palette-overlay" onClick={close}>
      <div className="palette-container" onClick={(e) => e.stopPropagation()}>
        <Command className="palette-command" shouldFilter={false} loop>
          <div className="palette-input-wrap">
            <span className="palette-icon">⌘</span>
            <Command.Input
              ref={inputRef}
              className="palette-input"
              placeholder="Open DocType, search records, run actions…"
              value={search}
              onValueChange={setSearch}
              autoFocus
            />
            <kbd className="palette-esc" onClick={close}>Esc</kbd>
          </div>
          <Command.List className="palette-list">
            {allResults.length === 0 && (
              <Command.Empty className="palette-empty">No results for "{search}"</Command.Empty>
            )}
            {Array.from(grouped.entries()).map(([category, items]) => (
              <Command.Group key={category} heading={category} className="palette-group">
                {items.map((r) => (
                  <Command.Item
                    key={r.id}
                    value={r.id}
                    className="palette-item"
                    onSelect={r.action}
                  >
                    {r.label}
                  </Command.Item>
                ))}
              </Command.Group>
            ))}
          </Command.List>
        </Command>
      </div>
    </div>
  );
}
