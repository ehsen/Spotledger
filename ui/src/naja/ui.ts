// naja.ui — dialogs, alerts, notifications, freeze/unfreeze
// All dialogs are React portals rendered via a global dialog manager.

export type AlertType = "info" | "success" | "warning" | "error";

// Toast notification queue
interface ToastEntry {
  id: string;
  message: string;
  type: AlertType;
  duration: number;
}

const toastListeners = new Set<(toasts: ToastEntry[]) => void>();
let toasts: ToastEntry[] = [];

function emitToasts() {
  toastListeners.forEach((fn) => fn([...toasts]));
}

// Confirm/Prompt dialog state
interface DialogRequest {
  id: string;
  type: "confirm" | "alert";
  message: string;
  title?: string;
  resolve: (value: boolean) => void;
}

const dialogListeners = new Set<(req: DialogRequest | null) => void>();
let currentDialog: DialogRequest | null = null;

let freezeCount = 0;
const freezeListeners = new Set<(state: { active: boolean; message: string }) => void>();
let freezeMessage = "";

export const ui = {
  // --- Toasts ---
  toast(message: string, seconds = 3, type: AlertType = "info") {
    const id = Math.random().toString(36).slice(2);
    const entry: ToastEntry = { id, message, type, duration: seconds * 1000 };
    toasts = [...toasts, entry];
    emitToasts();
    setTimeout(() => {
      toasts = toasts.filter((t) => t.id !== id);
      emitToasts();
    }, entry.duration);
  },

  alert(message: string, type: AlertType = "info") {
    ui.toast(message, 5, type);
  },

  // --- Confirm dialog ---
  async confirm(message: string, options?: { title?: string }): Promise<boolean> {
    return new Promise<boolean>((resolve) => {
      currentDialog = {
        id: Math.random().toString(36).slice(2),
        type: "confirm",
        message,
        title: options?.title,
        resolve,
      };
      dialogListeners.forEach((fn) => fn(currentDialog));
    });
  },

  // --- Freeze ---
  freeze(message = "Loading…") {
    freezeMessage = message;
    freezeCount++;
    if (freezeCount === 1) freezeListeners.forEach((fn) => fn({ active: true, message }));
  },

  unfreeze() {
    freezeCount = Math.max(0, freezeCount - 1);
    if (freezeCount === 0) freezeListeners.forEach((fn) => fn({ active: false, message: "" }));
  },

  // --- Subscription hooks (used by UI components) ---
  subscribeToasts(fn: (toasts: ToastEntry[]) => void): () => void {
    toastListeners.add(fn);
    fn([...toasts]);
    return () => toastListeners.delete(fn);
  },

  subscribeDialog(fn: (req: DialogRequest | null) => void): () => void {
    dialogListeners.add(fn);
    return () => dialogListeners.delete(fn);
  },

  subscribeFreeze(fn: (state: { active: boolean; message: string }) => void): () => void {
    freezeListeners.add(fn);
    return () => freezeListeners.delete(fn);
  },

  /** Called by the dialog component when user responds. */
  _resolveDialog(value: boolean) {
    currentDialog?.resolve(value);
    currentDialog = null;
    dialogListeners.forEach((fn) => fn(null));
  },
};

export type { ToastEntry, DialogRequest };
