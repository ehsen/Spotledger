// naja.session — current user, roles, permissions
// Populated from naja.boot at startup.

export interface SessionData {
  user: string;
  user_fullname: string;
  user_email: string;
  user_image?: string;
  user_roles: string[];
  home_page: string;
  lang: string;
  time_zone: string;
  user_info: Record<string, { fullname: string; email: string; image?: string }>;
  sysdefaults: Record<string, string>;
  can_read: string[];
  can_write: string[];
  can_create: string[];
  can_delete: string[];
  can_submit: string[];
}

const _data: Partial<SessionData> = {};

export const session = {
  get user(): string { return _data.user ?? ""; },
  get user_fullname(): string { return _data.user_fullname ?? ""; },
  get user_email(): string { return _data.user_email ?? ""; },
  get user_image(): string | undefined { return _data.user_image; },
  get user_roles(): string[] { return _data.user_roles ?? []; },

  has_role(role: string | string[]): boolean {
    const roles = _data.user_roles ?? [];
    if (Array.isArray(role)) return role.some((r) => roles.includes(r));
    return roles.includes(role);
  },

  can_read(doctype: string): boolean {
    return (_data.can_read ?? []).includes(doctype) || _data.user === "Administrator";
  },

  can_write(doctype: string): boolean {
    return (_data.can_write ?? []).includes(doctype) || _data.user === "Administrator";
  },

  can_create(doctype: string): boolean {
    return (_data.can_create ?? []).includes(doctype) || _data.user === "Administrator";
  },

  can_delete(doctype: string): boolean {
    return (_data.can_delete ?? []).includes(doctype) || _data.user === "Administrator";
  },

  can_submit(doctype: string): boolean {
    return (_data.can_submit ?? []).includes(doctype) || _data.user === "Administrator";
  },

  /** Called by naja.boot once boot data is loaded. */
  _load(data: Partial<SessionData>) {
    Object.assign(_data, data);
  },
};
