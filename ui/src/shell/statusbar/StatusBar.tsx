import { useEffect, useState } from "react";
import { useShell } from "../../store/shell-store";
import { subscribeLoading } from "../../naja/call";

export function StatusBar() {
  const user = useShell((s) => s.user);
  const fullName = useShell((s) => s.fullName);
  const tabs = useShell((s) => s.tabs);
  const activeTabId = useShell((s) => s.activeTabId);
  const [loading, setLoading] = useState(false);
  const [wsStatus, setWsStatus] = useState<"connected" | "disconnected">("connected");

  useEffect(() => subscribeLoading(setLoading), []);

  const active = tabs.find((t) => t.id === activeTabId);

  return (
    <div className="statusbar">
      <div className="statusbar-left">
        <span className={`statusbar-ws ${wsStatus}`} title={`WebSocket ${wsStatus}`}>
          {wsStatus === "connected" ? "⬤" : "⬤"}
        </span>
        {active?.doctype && (
          <span className="statusbar-doctype">{active.doctype}</span>
        )}
        {active?.docname && (
          <>
            <span className="statusbar-sep">/</span>
            <span className="statusbar-docname">{active.docname}</span>
          </>
        )}
        {active?.dirty && (
          <span className="statusbar-dirty" title="Unsaved changes">●</span>
        )}
      </div>

      <div className="statusbar-right">
        {loading && <span className="statusbar-loading">Syncing…</span>}
        <span className="statusbar-user" title={user ?? ""}>
          {fullName ?? user ?? ""}
        </span>
        <span className="statusbar-hint"><kbd>Ctrl+K</kbd></span>
      </div>
    </div>
  );
}
