/**
 * The single doorway between the UI and the Rust side.
 *
 * Inside the Tauri desktop app every call is Tauri IPC. In a plain browser
 * (the `chikn-web` shell, docs/plans/PLAN_WEB_SHELL.md) the same calls go to
 * same-origin JSON endpoints: `POST /api/cmd/<name>` with the identical
 * camelCase argument object, and events arrive over `GET /api/events` as a
 * server-sent-event stream.
 *
 * Contract kept identical to Tauri so no caller can tell the difference:
 *  - the resolved value is the command's JSON result;
 *  - a failed command rejects with the backend's plain error STRING (never
 *    an `Error` wrapper — callers render it with `String(e)`).
 *
 * This is the only module allowed to import `@tauri-apps/api/core` or
 * `@tauri-apps/api/event` (ESLint `no-restricted-imports`).
 */
import { invoke as tauriInvoke, isTauri } from "@tauri-apps/api/core";
import { listen as tauriListen, type UnlistenFn } from "@tauri-apps/api/event";
import { randomId } from "../utils/randomId";

export type { UnlistenFn };

export interface InvokeOptions {
  /** Let the request outlive page unload (`backup_on_close`). */
  keepalive?: boolean;
}

export interface TransportEvent<T> {
  payload: T;
}

/** Fired on `window` when the server answers 409: another tab or device holds the project. */
export const LEASE_CONFLICT_EVENT = "cs-web-lease-conflict";
export interface LeaseConflictDetail {
  command: string;
  args: Record<string, unknown> | undefined;
}
const SESSION_KEY = "cs-web-session";
const SESSION_HEADER = "X-CS-Session";

let sessionId: string | null = null;

/** A random id for this tab, kept across reloads of the same tab. */
export function getSessionId(): string {
  if (sessionId) return sessionId;
  try {
    const stored = window.sessionStorage.getItem(SESSION_KEY);
    if (stored) {
      sessionId = stored;
      return stored;
    }
  } catch {
    /* no sessionStorage */
  }
  sessionId = randomId();
  try {
    window.sessionStorage.setItem(SESSION_KEY, sessionId);
  } catch {
    /* lives for this page only */
  }
  return sessionId;
}

export function isWeb(): boolean {
  return !isTauri();
}

let webEventsAvailable = false;

/** Live-token AI output needs an event channel; the web shell reports whether it has one. */
export function supportsAiStreaming(): boolean {
  return isTauri() || webEventsAvailable;
}

export function setWebEventsAvailable(available: boolean): void {
  webEventsAvailable = available;
}

/** Headers every web request carries: just this tab's session id (for the single-writer lease). */
function sessionHeaders(): Record<string, string> {
  return { [SESSION_HEADER]: getSessionId() };
}

// ── Commands ─────────────────────────────────────────────────────────

export async function invoke<T>(
  command: string,
  args?: Record<string, unknown>,
  options?: InvokeOptions
): Promise<T> {
  if (isTauri()) {
    return tauriInvoke<T>(command, args);
  }
  return webInvoke<T>(command, args, options);
}

async function webInvoke<T>(
  command: string,
  args: Record<string, unknown> | undefined,
  options: InvokeOptions | undefined
): Promise<T> {
  const res = await fetch(`/api/cmd/${encodeURIComponent(command)}`, {
    method: "POST",
    headers: { "Content-Type": "application/json", ...sessionHeaders() },
    body: JSON.stringify(args ?? {}),
    keepalive: options?.keepalive,
  });
  if (res.ok) {
    if (res.status === 204) return undefined as T;
    return (await res.json()) as T;
  }
  if (res.status === 409) {
    const detail: LeaseConflictDetail = { command, args };
    window.dispatchEvent(new CustomEvent(LEASE_CONFLICT_EVENT, { detail }));
  }
  throw await failure(res);
}

/** GET a web-only endpoint (`/api/projects`, `/api/info`): same error contract as `invoke`. */
export async function webGet<T>(path: string): Promise<T> {
  const res = await webFetch(path, {});
  return (await res.json()) as T;
}

/**
 * Any other web-only request (downloads, uploads): adds the session header
 * and turns a non-2xx answer into the same plain-string rejection `invoke`
 * uses. The caller reads the successful response body itself.
 */
export async function webFetch(path: string, init: RequestInit): Promise<Response> {
  const res = await fetch(path, {
    ...init,
    headers: { ...(init.headers as Record<string, string> | undefined), ...sessionHeaders() },
  });
  if (!res.ok) throw await failure(res);
  return res;
}

/** The rejection value for a failed request: the backend's plain string, as Tauri rejects. */
async function failure(res: Response): Promise<string> {
  let body: unknown = null;
  try {
    body = await res.json();
  } catch {
    /* non-JSON error body */
  }
  return typeof body === "string" ? body : res.statusText || `HTTP ${res.status}`;
}

// ── Events ───────────────────────────────────────────────────────────

type Listener = (event: TransportEvent<unknown>) => void;
const webListeners = new Map<string, Set<Listener>>();
let streamConnecting: Promise<boolean> | null = null;

export async function listen<T>(
  event: string,
  handler: (event: TransportEvent<T>) => void
): Promise<UnlistenFn> {
  if (isTauri()) {
    return tauriListen<T>(event, handler);
  }
  const set = webListeners.get(event) ?? new Set<Listener>();
  const listener = handler as Listener;
  set.add(listener);
  webListeners.set(event, set);
  // Resolve only once the stream is open, so a command issued right after
  // `listen()` cannot emit before this page is receiving.
  await ensureEventStream();
  return () => {
    set.delete(listener);
    if (set.size === 0) webListeners.delete(event);
  };
}

/** One `text/event-stream` block → `{ event, payload }`, or null for keep-alives. */
export function parseSseBlock(block: string): { event: string; payload: unknown } | null {
  let event = "message";
  const data: string[] = [];
  for (const line of block.split("\n")) {
    if (line.startsWith("event:")) event = line.slice(6).trim();
    else if (line.startsWith("data:")) data.push(line.slice(5).replace(/^ /, ""));
  }
  if (data.length === 0) return null;
  const raw = data.join("\n");
  let payload: unknown;
  try {
    payload = JSON.parse(raw);
  } catch {
    payload = raw;
  }
  return { event, payload };
}

function dispatchSse(block: string): void {
  const parsed = parseSseBlock(block);
  if (!parsed) return;
  for (const listener of webListeners.get(parsed.event) ?? []) {
    try {
      listener({ payload: parsed.payload });
    } catch {
      /* a bad listener must not kill the stream */
    }
  }
}

/** Open (or reuse) the page's event stream. Resolves true once connected. */
function ensureEventStream(): Promise<boolean> {
  if (streamConnecting) return streamConnecting;
  let connected: (ok: boolean) => void = () => {};
  const ready = new Promise<boolean>((resolve) => {
    connected = resolve;
  });
  streamConnecting = ready;
  void (async () => {
    try {
      // Plain fetch rather than EventSource: EventSource cannot send the
      // session header.
      const res = await fetch("/api/events", { headers: sessionHeaders() });
      if (!res.ok || !res.body) {
        connected(false);
        return;
      }
      setWebEventsAvailable(true);
      connected(true);
      const reader = res.body.getReader();
      const decoder = new TextDecoder();
      let buffer = "";
      for (;;) {
        const { value, done } = await reader.read();
        if (done) break;
        buffer += decoder.decode(value, { stream: true });
        let idx = buffer.indexOf("\n\n");
        while (idx >= 0) {
          dispatchSse(buffer.slice(0, idx));
          buffer = buffer.slice(idx + 2);
          idx = buffer.indexOf("\n\n");
        }
      }
    } catch {
      /* connection lost; the next listen() reconnects */
    } finally {
      connected(false);
      streamConnecting = null;
      setWebEventsAvailable(false);
    }
  })();
  return ready;
}

/** Test hook: forget the session id and any registered listeners. */
export function resetTransportForTests(): void {
  sessionId = null;
  webListeners.clear();
  streamConnecting = null;
  webEventsAvailable = false;
}
