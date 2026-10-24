import { describe, it, expect, beforeEach, afterEach, vi } from "vitest";

// Browser mode: the web-only upload goes through the shared transport.
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
  isTauri: () => false,
}));
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn(async () => () => {}) }));

import { importUpload } from "../commands/web";
import { setWebToken, resetTransportForTests } from "../commands/transport";

const fetchMock = vi.fn<(input: RequestInfo | URL, init?: RequestInit) => Promise<Response>>();

beforeEach(() => {
  resetTransportForTests();
  setWebToken("tok");
  fetchMock.mockReset();
  vi.stubGlobal("fetch", fetchMock);
});

afterEach(() => {
  vi.unstubAllGlobals();
});

describe("import by upload", () => {
  it("posts the file, project path and parent as multipart with the token", async () => {
    fetchMock.mockResolvedValueOnce(
      new Response(JSON.stringify({ id: "p", documents: {} }), {
        status: 200,
        headers: { "Content-Type": "application/json" },
      })
    );
    const file = new File(["# Hi"], "note.md", { type: "text/markdown" });

    const project = await importUpload("/srv/root/Notes.chikn", file, "folder-1");

    expect(project).toEqual({ id: "p", documents: {} });
    const [url, init] = fetchMock.mock.calls[0];
    expect(url).toBe("/api/import-upload");
    expect(init?.method).toBe("POST");
    expect((init?.headers as Record<string, string>).Authorization).toBe("Bearer tok");
    const body = init?.body as FormData;
    expect(body).toBeInstanceOf(FormData);
    expect(body.get("projectPath")).toBe("/srv/root/Notes.chikn");
    expect(body.get("parentId")).toBe("folder-1");
    expect((body.get("file") as File).name).toBe("note.md");
  });

  it("rejects with the server's plain string on failure", async () => {
    fetchMock.mockResolvedValueOnce(
      new Response(JSON.stringify("Unknown error: Pandoc conversion failed"), {
        status: 422,
        headers: { "Content-Type": "application/json" },
      })
    );
    await expect(
      importUpload("/srv/root/Notes.chikn", new File(["x"], "a.docx"))
    ).rejects.toBe("Unknown error: Pandoc conversion failed");
  });
});
