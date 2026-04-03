// Shell-level Zustand store — tabs, palette, session
// Replaces the old src/store.ts

import { create } from "zustand";
import type { TabKind } from "../shell/tabbar/tab-types";

export interface TabEntry {
  id: string;
  kind: TabKind;
  doctype?: string;
  docname?: string;
  title: string;
  dirty?: boolean;
}

/** Generate a stable deterministic tab ID — ensures one-tab per document. */
function tab_id(kind: TabKind, doctype?: string, docname?: string): string {
  return `${kind}:${doctype ?? ""}:${docname ?? ""}`;
}

interface SessionState {
  user: string | null;
  fullName: string | null;
  setUser(user: string, fullName: string): void;
  clearUser(): void;
}

interface ShellState extends SessionState {
  tabs: TabEntry[];
  activeTabId: string | null;
  paletteOpen: boolean;

  openTab(tab: Omit<TabEntry, "id">): void;
  closeTab(id: string): void;
  setActiveTab(id: string): void;
  markDirty(id: string, dirty: boolean): void;
  setPaletteOpen(open: boolean): void;
}

export const useShell = create<ShellState>((set, get) => ({
  // Session
  user: null,
  fullName: null,
  setUser: (user, fullName) => set({ user, fullName }),
  clearUser: () => set({ user: null, fullName: null }),

  // Tabs
  tabs: [],
  activeTabId: null,
  paletteOpen: false,

  openTab(tab) {
    const id = tab_id(tab.kind, tab.doctype, tab.docname);
    const existing = get().tabs.find((t) => t.id === id);
    if (existing) {
      set({ activeTabId: id });
      return;
    }
    const entry: TabEntry = { ...tab, id };
    set((s) => ({ tabs: [...s.tabs, entry], activeTabId: id }));
  },

  closeTab(id) {
    set((s) => {
      const tabs = s.tabs.filter((t) => t.id !== id);
      const activeTabId = s.activeTabId === id
        ? (tabs[tabs.length - 1]?.id ?? null)
        : s.activeTabId;
      return { tabs, activeTabId };
    });
  },

  setActiveTab(id) { set({ activeTabId: id }); },

  markDirty(id, dirty) {
    set((s) => ({
      tabs: s.tabs.map((t) => t.id === id ? { ...t, dirty } : t),
    }));
  },

  setPaletteOpen(open) { set({ paletteOpen: open }); },
}));
