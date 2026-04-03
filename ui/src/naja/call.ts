// naja.call — typed promise-based API client
// Single source for all HTTP communication with the Spotledger backend.

import type { CallOptions } from "../types/api";

// In-flight debounce timers per method key
const debounceTimers = new Map<string, ReturnType<typeof setTimeout>>();
const debounceResolvers = new Map<string, ((v: unknown) => void)[]>();

// Simple TTL cache (TanStack Query used for React-component caching separately)
interface CacheEntry { value: unknown; expiresAt: number }
const callCache = new Map<string, CacheEntry>();

let loadingCount = 0;
const loadingListeners = new Set<(active: boolean) => void>();

function setLoading(active: boolean) {
  loadingListeners.forEach((fn) => fn(active));
}

export function subscribeLoading(fn: (active: boolean) => void): () => void {
  loadingListeners.add(fn);
  return () => loadingListeners.delete(fn);
}

async function _execute(method: string, args: Record<string, unknown>, signal?: AbortSignal): Promise<unknown> {
  const body = new URLSearchParams();
  for (const [k, v] of Object.entries(args)) {
    body.append(k, typeof v === "object" ? JSON.stringify(v) : String(v ?? ""));
  }

  let res: Response;
  try {
    res = await fetch(`/api/method/${method}`, {
      method: "POST",
      body,
      credentials: "include",
      signal,
    });
  } catch (e) {
    if ((e as Error).name === "AbortError") throw e;
    throw new Error("Cannot reach server — is Spotledger running?");
  }

  const text = await res.text();
  let json: Record<string, unknown> = {};
  if (text) {
    try { json = JSON.parse(text); } catch {
      if (!res.ok) throw new Error(`HTTP ${res.status}: ${text.slice(0, 200)}`);
    }
  }

  if (!res.ok) {
    const msgs = json._server_messages as string | undefined;
    let msg = `HTTP ${res.status}`;
    if (msgs) {
      try { msg = (JSON.parse(JSON.parse(msgs)[0]) as { message: string }).message; } catch { /* ignore */ }
    } else if (json.message) {
      msg = String(json.message);
    }
    throw new Error(msg);
  }
  return json.message ?? json;
}

export async function call<T = unknown>(
  method: string,
  args: Record<string, unknown> = {},
  opts: CallOptions = {},
): Promise<T> {
  const { debounce, cache, background, signal } = opts;

  // Cache check
  if (cache) {
    const key = `${method}:${JSON.stringify(args)}`;
    const entry = callCache.get(key);
    if (entry && entry.expiresAt > Date.now()) {
      return entry.value as T;
    }
  }

  if (!background) {
    loadingCount++;
    if (loadingCount === 1) setLoading(true);
  }

  const execute = (): Promise<T> => {
    const cacheKey = `${method}:${JSON.stringify(args)}`;
    return _execute(method, args, signal).then((v) => {
      if (cache) {
        callCache.set(cacheKey, { value: v, expiresAt: Date.now() + cache * 1000 });
      }
      return v as T;
    }).finally(() => {
      if (!background) {
        loadingCount = Math.max(0, loadingCount - 1);
        if (loadingCount === 0) setLoading(false);
      }
    });
  };

  if (!debounce) return execute();

  // Debounce: collect callers and resolve all when the timer fires
  return new Promise<T>((resolve, reject) => {
    const timerKey = `${method}:${JSON.stringify(args)}`;
    if (!debounceResolvers.has(timerKey)) {
      debounceResolvers.set(timerKey, []);
    }
    debounceResolvers.get(timerKey)!.push(resolve as (v: unknown) => void);

    const existing = debounceTimers.get(timerKey);
    if (existing) clearTimeout(existing);

    debounceTimers.set(timerKey, setTimeout(() => {
      debounceTimers.delete(timerKey);
      const resolvers = debounceResolvers.get(timerKey) ?? [];
      debounceResolvers.delete(timerKey);
      _execute(method, args, signal)
        .then((v) => resolvers.forEach((r) => r(v)))
        .catch(reject);
    }, debounce));
  });
}
