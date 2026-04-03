import { useEffect, useState } from "react";
import { call } from "../../naja/call";
import { useShell } from "../../store/shell-store";

interface ModuleEntry {
  module: string;
  doctypes: string[];
}

export function Sidebar() {
  const openTab = useShell((s) => s.openTab);
  const [modules, setModules] = useState<ModuleEntry[]>([]);
  const [collapsed, setCollapsed] = useState<Set<string>>(new Set());
  const [sidebarOpen, setSidebarOpen] = useState(true);

  useEffect(() => {
    call<{ name: string; module: string }[]>("frappe.client.get_list", {
      doctype: "DocType",
      fields: JSON.stringify(["name", "module"]),
      filters: JSON.stringify([["istable", "!=", 1], ["issingle", "!=", 1]]),
      limit: 500,
      order_by: "module, name asc",
    }, { cache: 300, background: true })
      .then((rows) => {
        const map = new Map<string, string[]>();
        for (const r of rows as { name: string; module: string }[]) {
          if (!map.has(r.module)) map.set(r.module, []);
          map.get(r.module)!.push(r.name);
        }
        setModules(
          Array.from(map.entries())
            .map(([module, doctypes]) => ({ module, doctypes }))
            .sort((a, b) => a.module.localeCompare(b.module)),
        );
      })
      .catch(console.error);
  }, []);

  function toggleModule(mod: string) {
    setCollapsed((prev) => {
      const next = new Set(prev);
      if (next.has(mod)) next.delete(mod); else next.add(mod);
      return next;
    });
  }

  if (!sidebarOpen) {
    return (
      <div className="sidebar sidebar-collapsed">
        <button
          className="sidebar-toggle"
          onClick={() => setSidebarOpen(true)}
          aria-label="Open sidebar"
        >›</button>
      </div>
    );
  }

  return (
    <div className="sidebar">
      <div className="sidebar-header">
        <span className="sidebar-title">Modules</span>
        <button
          className="sidebar-toggle"
          onClick={() => setSidebarOpen(false)}
          aria-label="Collapse sidebar"
        >‹</button>
      </div>
      <div className="sidebar-body">
        {modules.map(({ module, doctypes }) => (
          <div key={module} className="sidebar-module">
            <button
              className="sidebar-module-header"
              onClick={() => toggleModule(module)}
            >
              <span className={`sidebar-chevron ${collapsed.has(module) ? "collapsed" : ""}`}>
                ›
              </span>
              {module}
            </button>
            {!collapsed.has(module) && (
              <ul className="sidebar-doctype-list">
                {doctypes.map((dt) => (
                  <li key={dt}>
                    <button
                      className="sidebar-doctype"
                      onClick={() => openTab({ kind: "list", doctype: dt, title: dt })}
                    >
                      {dt}
                    </button>
                  </li>
                ))}
              </ul>
            )}
          </div>
        ))}
      </div>
    </div>
  );
}
