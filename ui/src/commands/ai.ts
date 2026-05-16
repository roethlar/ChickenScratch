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
 * The returned promise is the lifecycle of one request only. `shouldContinue`
 * is checked before every stream event is applied; returning false cancels the
 * backend request and resolves without applying that event.
 */
export function createAiStreamId(): string {
  return crypto.randomUUID();
}

export async function cancelAiTransformStream(requestId: string): Promise<void> {
  return invoke("cancel_ai_transform_stream", { requestId });
}

export async function aiTransformStream(
  content: string,
  operation: AiOperation,
  onChunk: (delta: string) => void,
  options: {
    requestId?: string;
    shouldContinue?: () => boolean;
    abortSignal?: AbortSignal;
  } = {}
): Promise<void> {
  const requestId = options.requestId ?? createAiStreamId();
  const unlisteners: UnlistenFn[] = [];

  return new Promise<void>((resolve, reject) => {
    let settled = false;
    let cancelling = false;

    function cleanup() {
      options.abortSignal?.removeEventListener("abort", cancelAndIgnore);
      for (const u of unlisteners) {
        try { u(); } catch { /* ignore */ }
      }
    }
    function finish(err?: string) {
      if (settled) return;
      settled = true;
      cleanup();
      if (err) reject(new Error(err));
      else resolve();
    }
    function cancelAndIgnore() {
      if (!cancelling) {
        cancelling = true;
        cancelAiTransformStream(requestId).catch(() => {});
      }
      finish();
      return false;
    }
    function shouldHandleEvent() {
      if (settled) return false;
      return options.shouldContinue?.() === false ? cancelAndIgnore() : true;
    }

    if (options.abortSignal?.aborted) {
      cancelAndIgnore();
      return;
    }
    options.abortSignal?.addEventListener("abort", cancelAndIgnore, { once: true });

    Promise.all([
      listen<{ id: string; delta: string }>("ai:chunk", (e) => {
        if (e.payload.id === requestId && shouldHandleEvent()) {
          onChunk(e.payload.delta);
        }
      }),
      listen<{ id: string }>("ai:done", (e) => {
        if (e.payload.id === requestId && shouldHandleEvent()) finish();
      }),
      listen<{ id: string; message: string }>("ai:error", (e) => {
        if (e.payload.id === requestId && shouldHandleEvent()) finish(e.payload.message);
      }),
    ])
      .then((unlistens) => {
        if (settled) {
          for (const u of unlistens) {
            try { u(); } catch { /* ignore */ }
          }
          return undefined;
        }
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
