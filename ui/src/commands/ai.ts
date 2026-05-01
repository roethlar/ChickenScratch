import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export interface AiSettings {
  provider: string;
  model: string;
  endpoint?: string | null;
  api_key?: string | null;
}

export async function getAiSettings(): Promise<AiSettings> {
  return invoke("get_ai_settings");
}

export async function saveAiSettings(settings: AiSettings): Promise<void> {
  return invoke("save_ai_settings", { settings });
}

export async function aiSummarize(content: string): Promise<string> {
  return invoke("ai_summarize", { content });
}

export type AiOperation = "polish" | "expand" | "simplify" | "brainstorm";

export async function aiTransform(
  content: string,
  operation: AiOperation
): Promise<string> {
  return invoke("ai_transform", { content, operation });
}

/**
 * Stream a transform. Resolves when the backend reports the request finished.
 * Rejects on error events. Each delta is delivered to `onChunk` as it arrives.
 *
 * The returned promise is the lifecycle of one request only — if you need to
 * cancel, drop subscriptions on unmount; the backend thread will keep running
 * but its events will be ignored.
 */
export async function aiTransformStream(
  content: string,
  operation: AiOperation,
  onChunk: (delta: string) => void
): Promise<void> {
  const requestId = crypto.randomUUID();
  const unlisteners: UnlistenFn[] = [];

  return new Promise<void>((resolve, reject) => {
    let settled = false;
    const cleanup = () => {
      for (const u of unlisteners) {
        try { u(); } catch { /* ignore */ }
      }
    };
    const finish = (err?: string) => {
      if (settled) return;
      settled = true;
      cleanup();
      if (err) reject(new Error(err));
      else resolve();
    };

    Promise.all([
      listen<{ id: string; delta: string }>("ai:chunk", (e) => {
        if (e.payload.id === requestId) onChunk(e.payload.delta);
      }),
      listen<{ id: string }>("ai:done", (e) => {
        if (e.payload.id === requestId) finish();
      }),
      listen<{ id: string; message: string }>("ai:error", (e) => {
        if (e.payload.id === requestId) finish(e.payload.message);
      }),
    ])
      .then((unlistens) => {
        unlisteners.push(...unlistens);
        return invoke("ai_transform_stream", {
          content,
          operation,
          requestId,
        });
      })
      .catch((e) => finish(typeof e === "string" ? e : (e?.message ?? String(e))));
  });
}
