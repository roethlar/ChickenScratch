import { describe, it, expect, beforeEach, vi } from "vitest";

/**
 * Quick note (PLAN_RELOAD_SAFETY_MOBILE slice 3): one tap opens the project,
 * adds a dated note at the root and selects it.
 */

const createDocumentMock = vi.hoisted(() =>
  vi.fn(async (_path: string, name: string) => ({
    documents: { "doc-1": {}, "new-1": { name } },
  }))
);
vi.mock("../commands/document", () => ({ createDocument: createDocumentMock }));
const focusMock = vi.hoisted(() => vi.fn());
vi.mock("../components/editor/editorRef", () => ({ requestEditorFocus: focusMock }));
const toastErrorMock = vi.hoisted(() => vi.fn());
vi.mock("../components/shared/Toast", () => ({ toastError: toastErrorMock }));

const storeState = vi.hoisted(() => ({
  project: null as null | { path: string; documents: Record<string, unknown> },
  readOnly: false,
  openProject: vi.fn(async () => {}),
  setProject: vi.fn(),
  selectDocument: vi.fn(),
}));
vi.mock("../stores/projectStore", () => ({
  useProjectStore: { getState: () => storeState },
}));

import { quickNote, quickNoteName } from "../quickNote";

beforeEach(() => {
  vi.clearAllMocks();
  storeState.readOnly = false;
  storeState.project = null;
  storeState.openProject = vi.fn(async () => {
    storeState.project = { path: "/srv/root/Notes.chikn", documents: { "doc-1": {} } };
  });
});

describe("quickNoteName", () => {
  it("names the note by local date and time", () => {
    expect(quickNoteName(new Date(2026, 9, 25, 14, 3))).toBe("Note 2026-10-25 14:03");
    expect(quickNoteName(new Date(2026, 0, 2, 9, 7))).toBe("Note 2026-01-02 09:07");
  });
});

describe("quickNote", () => {
  it("opens the project, creates a dated root note and selects it", async () => {
    expect(await quickNote("/srv/root/Notes.chikn")).toBe(true);
    expect(storeState.openProject).toHaveBeenCalledWith("/srv/root/Notes.chikn");
    expect(createDocumentMock).toHaveBeenCalledTimes(1);
    const [path, name] = createDocumentMock.mock.calls[0];
    expect(path).toBe("/srv/root/Notes.chikn");
    expect(name).toMatch(/^Note \d{4}-\d{2}-\d{2} \d{2}:\d{2}$/);
    expect(storeState.setProject).toHaveBeenCalled();
    expect(storeState.selectDocument).toHaveBeenCalledWith("new-1");
    expect(focusMock).toHaveBeenCalled();
  });

  it("reports a failed open through the store and adds nothing", async () => {
    storeState.openProject = vi.fn(async () => {
      storeState.project = null;
    });
    expect(await quickNote("/srv/root/Gone.chikn")).toBe(false);
    expect(createDocumentMock).not.toHaveBeenCalled();
  });

  it("leaves a read-only project open without adding a note, and says why", async () => {
    storeState.openProject = vi.fn(async () => {
      storeState.project = { path: "/srv/root/Old.chikn", documents: {} };
      storeState.readOnly = true;
    });
    expect(await quickNote("/srv/root/Old.chikn")).toBe(true);
    expect(createDocumentMock).not.toHaveBeenCalled();
    expect(toastErrorMock).toHaveBeenCalled();
  });

  it("keeps the project open when the note cannot be created", async () => {
    createDocumentMock.mockRejectedValueOnce(new Error("disk full"));
    expect(await quickNote("/srv/root/Notes.chikn")).toBe(true);
    expect(storeState.selectDocument).not.toHaveBeenCalled();
    expect(toastErrorMock).toHaveBeenCalledWith(expect.stringContaining("disk full"));
  });
});
