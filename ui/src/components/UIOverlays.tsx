// Global UI overlays — driven by naja.ui event subscriptions
// Rendered once at the app root level.

import { useEffect, useState } from "react";
import { ui } from "../naja/ui";
import type { ToastEntry, DialogRequest } from "../naja/ui";

export function UIOverlays() {
  return (
    <>
      <ToastContainer />
      <ConfirmDialog />
      <FreezeOverlay />
    </>
  );
}

function ToastContainer() {
  const [toasts, setToasts] = useState<ToastEntry[]>([]);
  useEffect(() => ui.subscribeToasts(setToasts), []);

  if (toasts.length === 0) return null;

  return (
    <div className="toast-container" aria-live="polite">
      {toasts.map((t) => (
        <div key={t.id} className={`toast toast-${t.type}`} role="alert">
          {t.message}
        </div>
      ))}
    </div>
  );
}

function ConfirmDialog() {
  const [req, setReq] = useState<DialogRequest | null>(null);
  useEffect(() => ui.subscribeDialog(setReq), []);

  if (!req) return null;

  return (
    <div className="dialog-overlay">
      <div className="dialog-box" role="alertdialog" aria-modal="true">
        {req.title && <div className="dialog-title">{req.title}</div>}
        <div className="dialog-message">{req.message}</div>
        <div className="dialog-actions">
          <button
            className="btn btn-ghost"
            onClick={() => ui._resolveDialog(false)}
            autoFocus
          >
            Cancel
          </button>
          <button
            className="btn btn-primary"
            onClick={() => ui._resolveDialog(true)}
          >
            Confirm
          </button>
        </div>
      </div>
    </div>
  );
}

function FreezeOverlay() {
  const [state, setState] = useState({ active: false, message: "" });
  useEffect(() => ui.subscribeFreeze(setState), []);

  if (!state.active) return null;

  return (
    <div className="freeze-overlay" aria-busy="true">
      <div className="freeze-content">
        <span className="spinner" />
        <span>{state.message}</span>
      </div>
    </div>
  );
}
