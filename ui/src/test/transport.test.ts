import { describe, it, expect, beforeEach, afterEach, vi } from "vitest";

// Browser mode: no Tauri runtime.
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(async () => {
    throw new Error("Tauri invoke must not be used in web mode");
  }),
  isTauri: () => false,
}));
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(async () => () => {}),
}));

import {
  invoke,
  listen,
  getWebToken,
  setWebToken,
  clearWebToken,
  parseSseBlock,
  resetTransportForTests,
  AUTH_REQUIRED_EVENT,
} from "../commands/transport";
import { aiTransformStream } from "../commands/ai";

type FetchArgs = [input: RequestInfo | URL, init?: RequestInit];

function jsonResponse(body: unknown, status = 200): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "Content-Type": "application/json" },
  });
}

const fetchMock = vi.fn<(...args: FetchArgs) => Promise<Response>>();

beforeEach(() => {
  resetTransportForTests();
  clearWebToken();
  fetchMock.mockReset();
  vi.stubGlobal("fetch", fetchMock);
  window.history.replaceState(null, "", "/");
});

afterEach(() => {
  vi.unstubAllGlobals();
});

describe("web transport: commands", () => {
  it("posts the command name and the exact args object with the bearer token", async () => {
    setWebToken("secret-token");
    fetchMock.mockResolvedValueOnce(jsonResponse({ ok: true }));

    const result = await invoke("load_project", { path: "/root/Notes.chikn" });

    expect(result).toEqual({ ok: true });
    expect(fetchMock).toHaveBeenCalledTimes(1);
    const [url, init] = fetchMock.mock.calls[0];
    expect(url).toBe("/api/cmd/load_project");
    expect(init?.method).toBe("POST");
    expect(init?.headers).toMatchObject({
      Authorization: "Bearer secret-token",
      "Content-Type": "application/json",
    });
    expect(JSON.parse(String(init?.body))).toEqual({ path: "/root/Notes.chikn" });
  });

  it("rejects with the backend's plain error string, exactly like Tauri", async () => {
    fetchMock.mockResolvedValueOnce(jsonResponse("Not found: Document not found: d1", 422));

    await expect(invoke("get_document", { projectPath: "/p", docId: "d1" })).rejects.toBe(
      "Not found: Document not found: d1"
    );
  });

  it("on 401 clears the stored token and announces that a token is needed", async () => {
    setWebToken("stale");
    fetchMock.mockResolvedValueOnce(jsonResponse("Unauthorized", 401));
    const announced = vi.fn();
    window.addEventListener(AUTH_REQUIRED_EVENT, announced);

    await expect(invoke("get_app_settings")).rejects.toBe("Unauthorized");

    expect(getWebToken()).toBeNull();
    expect(announced).toHaveBeenCalledTimes(1);
    window.removeEventListener(AUTH_REQUIRED_EVENT, announced);
  });

  it("passes keepalive through for close-time backups", async () => {
    fetchMock.mockResolvedValueOnce(jsonResponse(null));

    await invoke("backup_on_close", { projectPath: "/p" }, { keepalive: true });

    expect(fetchMock.mock.calls[0][1]?.keepalive).toBe(true);
  });
});

describe("web transport: token from the address bar", () => {
  it("stores ?token= on first use and strips it from the URL", () => {
    window.history.replaceState(null, "", "/?token=from-link&x=1");

    expect(getWebToken()).toBe("from-link");
    expect(window.location.search).toBe("?x=1");
    // Second read comes from storage, not the (now clean) URL.
    expect(getWebToken()).toBe("from-link");
  });
});

describe("web transport: events", () => {
  it("parses an SSE block into its event name and JSON payload", () => {
    expect(parseSseBlock('event: ai:chunk\ndata: {"id":"r1","delta":"Hel"}')).toEqual({
      event: "ai:chunk",
      payload: { id: "r1", delta: "Hel" },
    });
    expect(parseSseBlock(": keep-alive")).toBeNull();
  });
});

describe("AI streaming without an event channel", () => {
  it("falls back to the one-shot transform and delivers it as a single chunk", async () => {
    fetchMock.mockResolvedValueOnce(jsonResponse("Polished prose."));
    const chunks: string[] = [];

    await aiTransformStream("rough prose", "polish", (delta) => chunks.push(delta));

    expect(chunks).toEqual(["Polished prose."]);
    expect(fetchMock.mock.calls[0][0]).toBe("/api/cmd/ai_transform");
  });

  it("applies nothing once the caller says stop", async () => {
    fetchMock.mockResolvedValueOnce(jsonResponse("Too late."));
    const chunks: string[] = [];

    await aiTransformStream("rough", "polish", (delta) => chunks.push(delta), {
      shouldContinue: () => false,
    });

    expect(chunks).toEqual([]);
  });
});

describe("web transport: live event stream", () => {
  it("resolves listen() once connected and dispatches parsed events", async () => {
    let push: (chunk: string) => void = () => {};
    const body = new ReadableStream<Uint8Array>({
      start(controller) {
        push = (chunk) => controller.enqueue(new TextEncoder().encode(chunk));
      },
    });
    fetchMock.mockResolvedValueOnce(
      new Response(body, { status: 200, headers: { "Content-Type": "text/event-stream" } })
    );
    const received: unknown[] = [];

    const unlisten = await listen<{ id: string; delta: string }>("ai:chunk", (e) => received.push(e.payload));
    expect(fetchMock.mock.calls[0][0]).toBe("/api/events");

    push('event: ai:chunk\ndata: {"id":"r1","delta":"Hel"}\n\n');
    push(': keep-alive\n\nevent: ai:done\ndata: {"id":"r1"}\n\n');
    await vi.waitFor(() => expect(received).toEqual([{ id: "r1", delta: "Hel" }]));
    unlisten();
  });
});
