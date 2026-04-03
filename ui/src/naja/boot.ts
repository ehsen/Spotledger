// naja.boot — application boot data, loaded once at app startup

import { session } from "./session";

export interface BootInfo {
  user: string;
  user_info: Record<string, unknown>;
  user_roles: string[];
  home_page: string;
  lang: string;
  time_zone?: string;
  sysdefaults: Record<string, string>;
  all_doctypes: string[];
  can_read: string[];
  can_write: string[];
  can_create: string[];
  can_delete: string[];
  can_submit: string[];
  sitename: string;
  version: string;
}

let _boot: BootInfo | null = null;
const _listeners = new Set<(boot: BootInfo) => void>();

export const boot = {
  get data(): BootInfo | null { return _boot; },

  async load(): Promise<BootInfo> {
    if (_boot) return _boot;
    try {
      const res = await fetch("/api/method/frappe.client.get_boot_info", {
        method: "POST",
        credentials: "include",
      });
      if (!res.ok) throw new Error("boot_info unavailable");
      const json = await res.json() as { message: BootInfo };
      _boot = json.message;
    } catch {
      // Spotledger doesn't implement get_boot_info yet — build minimal boot
      const userRes = await fetch("/api/method/frappe.auth.get_logged_user", {
        method: "POST",
        credentials: "include",
      }).catch(() => null);
      const user = userRes ? ((await userRes.json() as { message: string }).message ?? "Administrator") : "Administrator";
      _boot = _makeMinimalBoot(user);
    }
    session._load({
      user: _boot.user,
      user_roles: _boot.user_roles,
      can_read: _boot.can_read,
      can_write: _boot.can_write,
      can_create: _boot.can_create,
      can_delete: _boot.can_delete,
      can_submit: _boot.can_submit,
    });

    _listeners.forEach((fn) => fn(_boot!));
    return _boot!;
  },

  onLoaded(fn: (boot: BootInfo) => void): () => void {
    if (_boot) { fn(_boot); return () => {}; }
    _listeners.add(fn);
    return () => _listeners.delete(fn);
  },
};

function _makeMinimalBoot(user: string): BootInfo {
  return {
    user,
    user_info: {},
    user_roles: user === "Administrator" ? ["Administrator", "System Manager"] : [],
    home_page: "/app",
    lang: "en",
    time_zone: "UTC",
    sysdefaults: {},
    all_doctypes: [],
    can_read: [],
    can_write: [],
    can_create: [],
    can_delete: [],
    can_submit: [],
    sitename: window.location.hostname,
    version: "1.0.0",
  };
}
