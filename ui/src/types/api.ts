// API request/response types matching Frappe REST conventions

export interface FrappeMethodResponse<T = unknown> {
  message: T;
  _server_messages?: string;
  exc?: string;
  exc_type?: string;
}

export interface GetDocResponse {
  docs: Record<string, unknown>[];
  docinfo: DocInfo;
  _link_titles?: Record<string, string>;
}

export interface DocInfo {
  name: string;
  doctype: string;
  user_info: Record<string, UserInfo>;
  perm: DocPermSet;
  comments: unknown[];
  communications: unknown[];
  assignments: unknown[];
  is_document_followed: boolean;
  shared: unknown[];
  attachments: unknown[];
  versions: unknown[];
  workflow_logs: unknown[];
}

export interface UserInfo {
  name: string;
  fullname: string;
  email: string;
  image?: string;
  time_zone?: string;
}

export interface DocPermSet {
  read: boolean;
  write: boolean;
  create: boolean;
  delete: boolean;
  submit: boolean;
  cancel: boolean;
  amend: boolean;
}

export interface ReportViewResponse {
  keys: string[];
  values: (string | number | null)[][];
}

export interface CallOptions {
  debounce?: number;           // ms to debounce repeated same-method calls
  cache?: number;              // seconds to cache result (uses TanStack Query)
  background?: boolean;        // suppress loading indicator
  signal?: AbortSignal;
}
