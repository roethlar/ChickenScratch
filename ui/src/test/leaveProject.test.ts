import { describe, it, expect, beforeEach, vi } from "vitest";

/**
 * The Projects button and the reload reopen (PLAN_RELOAD_SAFETY_MOBILE
 * slice 1): leaving saves, backs up, hands the lease back and forgets the
 * place; a page load in the browser shell goes back to the remembered
 * project and note.
 */

const flushMock = vi.hoisted(() => vi.fn(async () => {}));
vi.mock("../components/editor/editorRef", () => ({ flushPendingEditorSave: flushMock }));
const invokeMock = vi.hoisted(() => vi.fn(async () => undefined));
const webState = vi.hoisted(() => ({ web: true }));
vi.mock("../commands/transport", () => ({
  invoke: invokeMock,
  isWeb: () => webState.web,
}));
const releaseMock = vi.hoisted(() => vi.fn(async () => {}));
vi.mock("../commands/web", () => ({ releaseProject: releaseMock }));
const toastErrorMock = vi.hoisted(() => vi.fn());
vi.mock("../components/shared/Toast", () => ({ toastError: toastErrorMock }));
const barrier = vi.hoisted(() => ({ active: false }));
vi.mock("../commands/barrier", () => ({ isBarrierActive: () => barrier.active }));

const closeProjectMock = vi.hoisted(() => vi.fn());
const selectDocumentMock = vi.hoisted(() => vi.fn());
const storeState = vi.hoisted(() => ({
  project: null as null | { path: string; documents: Record<string, unknown> },
  openProject: vi.fn(async () => {}),
  selectDocument: selectDocumentMock,
  closeProject: closeProjectMock,
}));
vi.mock("../stores/projectStore", () => ({
  useProjectStore: { getState: () => storeState },
}));

import { leaveProject, reopenLastProject } from "../leaveProject";
import { getLastProject, rememberDocument, rememberProject } from "../lastOpen";

beforeEach(() => {
  vi.clearAllMocks();
  window.localStorage.clear();
  webState.web = true;
  barrier.active = false;
  storeState.project = { path: "/srv/root/Notes.chikn", documents: { "doc-1": {}, "doc-2": {} } };
  storeState.openProject = vi.fn(async () => {});
});

describe("leaveProject", () => {
  it("saves, backs up, releases the lease, forgets the place and closes", async () => {
    rememberProject("/srv/root/Notes.chikn");
    expect(await leaveProject()).toBe(true);
    expect(flushMock).toHaveBeenCalledTimes(1);
    expect(invokeMock).toHaveBeenCalledWith("backup_on_close", {
      projectPath: "/srv/root/Notes.chikn",
    });
    expect(releaseMock).toHaveBeenCalledWith("/srv/root/Notes.chikn");
    expect(getLastProject()).toBeNull();
    expect(closeProjectMock).toHaveBeenCalledTimes(1);
    // Order: the flush and backup finish before the store empties.
    expect(flushMock.mock.invocationCallOrder[0]).toBeLessThan(
      closeProjectMock.mock.invocationCallOrder[0]
    );
    expect(invokeMock.mock.invocationCallOrder[0]).toBeLessThan(
      closeProjectMock.mock.invocationCallOrder[0]
    );
  });

  it("keeps the project open when the latest changes cannot be saved", async () => {
    flushMock.mockRejectedValueOnce(new Error("disk full"));
    rememberProject("/srv/root/Notes.chikn");
    expect(await leaveProject()).toBe(false);
    expect(closeProjectMock).not.toHaveBeenCalled();
    expect(invokeMock).not.toHaveBeenCalled();
    expect(getLastProject()).toBe("/srv/root/Notes.chikn");
    expect(toastErrorMock).toHaveBeenCalled();
  });

  it("refuses while an operation holds the barrier", async () => {
    barrier.active = true;
    expect(await leaveProject()).toBe(false);
    expect(flushMock).not.toHaveBeenCalled();
    expect(closeProjectMock).not.toHaveBeenCalled();
  });

  it("still leaves when the close-time backup fails, and skips the lease on the desktop", async () => {
    invokeMock.mockRejectedValueOnce(new Error("no backup folder"));
    webState.web = false;
    expect(await leaveProject()).toBe(true);
    expect(releaseMock).not.toHaveBeenCalled();
    expect(closeProjectMock).toHaveBeenCalledTimes(1);
  });
});

describe("reopenLastProject", () => {
  it("opens the remembered project and selects the remembered note", async () => {
    rememberProject("/srv/root/Notes.chikn");
    rememberDocument("/srv/root/Notes.chikn", "doc-2");
    await reopenLastProject();
    expect(storeState.openProject).toHaveBeenCalledWith("/srv/root/Notes.chikn");
    expect(selectDocumentMock).toHaveBeenCalledWith("doc-2");
  });

  it("does not select a note the project no longer has", async () => {
    rememberProject("/srv/root/Notes.chikn");
    rememberDocument("/srv/root/Notes.chikn", "gone");
    await reopenLastProject();
    expect(selectDocumentMock).not.toHaveBeenCalled();
  });

  it("forgets a project that fails to open so Welcome shows and does not loop", async () => {
    rememberProject("/srv/root/Gone.chikn");
    storeState.openProject = vi.fn(async () => {
      storeState.project = null;
    });
    await reopenLastProject();
    expect(getLastProject()).toBeNull();
    expect(selectDocumentMock).not.toHaveBeenCalled();
  });

  it("does nothing when nothing is remembered", async () => {
    await reopenLastProject();
    expect(storeState.openProject).not.toHaveBeenCalled();
  });
});
