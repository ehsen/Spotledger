// naja.events — global event bus
// Decouples modules: AI panel listens to doc:saved without knowing about the form engine.

type Handler<T = unknown> = (payload: T) => void;

interface EventMap {
  "doc:saved": { doctype: string; name: string };
  "doc:submitted": { doctype: string; name: string };
  "doc:cancelled": { doctype: string; name: string };
  "tab:opened": { tabId: string; doctype?: string };
  "tab:closed": { tabId: string };
  "session:expired": void;
  "meta:loaded": { doctype: string };
  [key: string]: unknown;
}

type Unsubscribe = () => void;

class EventBus {
  private listeners = new Map<string, Set<Handler>>();

  on<K extends keyof EventMap>(event: K, handler: Handler<EventMap[K]>): Unsubscribe {
    if (!this.listeners.has(event as string)) {
      this.listeners.set(event as string, new Set());
    }
    this.listeners.get(event as string)!.add(handler as Handler);
    return () => this.listeners.get(event as string)?.delete(handler as Handler);
  }

  emit<K extends keyof EventMap>(event: K, payload: EventMap[K]): void {
    this.listeners.get(event as string)?.forEach((h) => {
      try { h(payload as unknown); } catch (e) { console.error(`Event handler error [${String(event)}]`, e); }
    });
  }
}

export const events = new EventBus();
