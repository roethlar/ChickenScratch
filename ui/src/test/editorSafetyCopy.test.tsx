import { describe, it, expect, beforeEach, vi } from "vitest";
import { render, screen, waitFor, act } from "@testing-library/react";

/**
 * The editor keeps a safety copy of unsaved text in browser storage and puts
 * it back on the next load (PLAN_RELOAD_SAFETY_MOBILE slice 1). Autosave is
 * debounced and a browser does not wait for the save a reload interrupts, so
 * without this the last words typed were lost.
 */

const updateContentMock = vi.hoisted(() => vi.fn(async () => {}));
vi.mock("../commands/document", () => ({
  updateDocumentContent: updateContentMock,
}));
// Shortest autosave delay the editor allows (250 ms) so tests stay quick.
const settingsState = vi.hoisted(() => ({
  appSettings: { writing: { auto_save_seconds: 0.1 }, ai: { enabled: false } },
}));
vi.mock("../stores/settingsStore", () => ({
  useSettingsStore: (sel: (s: typeof settingsState) => unknown) => sel(settingsState),
}));
vi.mock("../commands/session", () => ({
  getSessionProgress: vi.fn(async () => null),
  updateSessionTarget: vi.fn(async () => {}),
}));
const toastMock = vi.hoisted(() => vi.fn());
vi.mock("../components/shared/Toast", () => ({
  toast: toastMock,
  toastError: vi.fn(),
}));
const confirmMock = vi.hoisted(() => vi.fn<(title: string) => Promise<boolean>>(async () => true));
vi.mock("../components/shared/Dialog", () => ({
  dialogConfirm: confirmMock,
}));

import { Editor } from "../components/editor/Editor";
import { getCurrentEditor } from "../components/editor/editorRef";
import { readSafetyCopy, writeSafetyCopy } from "../components/editor/safetyCopy";
import { useProjectStore } from "../stores/projectStore";
import { resetBarrierForTests } from "../commands/barrier";
import type { Document, Project } from "../types";

function doc(content: string): Document {
  return {
    id: "doc-one",
    name: "One",
    path: "manuscript/one.md",
    content,
    parent_id: null,
    created: "2025-01-01T00:00:00Z",
    modified: "2025-01-01T00:00:00Z",
    include_in_compile: true,
    word_count_target: 0,
    compile_order: 0,
  } as Document;
}

function project(content: string): Project {
  return {
    id: "prj",
    name: "Test",
    path: "/p",
    hierarchy: [{ type: "Document", id: "doc-one", name: "One", path: "manuscript/one.md" }],
    documents: { "doc-one": doc(content) },
    created: "2025-01-01T00:00:00Z",
    modified: "2025-01-01T00:00:00Z",
    metadata: {},
  } as unknown as Project;
}

function openWith(content: string) {
  const p = project(content);
  useProjectStore.setState({
    project: p,
    readOnly: false,
    readOnlyReasons: [],
    activeDocId: "doc-one",
    activeDoc: p.documents["doc-one"],
    flowDocs: null,
  });
}

beforeEach(() => {
  resetBarrierForTests();
  vi.clearAllMocks();
  confirmMock.mockResolvedValue(true);
  window.localStorage.clear();
});

describe("editor safety copy", () => {
  it("writes a copy while typing and drops it once the save lands", async () => {
    openWith("On disk.");
    render(<Editor />);
    await waitFor(() => expect(screen.getByText("On disk.")).toBeInTheDocument());

    // Hold the save so the copy is observable before the disk catches up.
    let finishSave: () => void = () => {};
    updateContentMock.mockImplementationOnce(
      () => new Promise<void>((resolve) => { finishSave = resolve; })
    );
    await act(async () => {
      getCurrentEditor()!.commands.insertContentAt(
        getCurrentEditor()!.state.doc.content.size,
        " More."
      );
    });
    await waitFor(() => {
      const copy = readSafetyCopy("/p", "doc-one");
      expect(copy?.content).toContain("More.");
      expect(copy?.base).toBe("On disk.");
    });
    await waitFor(() => expect(updateContentMock).toHaveBeenCalled());
    expect(readSafetyCopy("/p", "doc-one")).not.toBeNull();
    await act(async () => {
      finishSave();
    });
    await waitFor(() => expect(readSafetyCopy("/p", "doc-one")).toBeNull());
  });

  it("restores a copy the disk has not outrun, saves it and says so", async () => {
    writeSafetyCopy("/p", "doc-one", "On disk. Typed before the reload.", "On disk.");
    openWith("On disk.");
    render(<Editor />);

    await waitFor(() =>
      expect(screen.getByText("On disk. Typed before the reload.")).toBeInTheDocument()
    );
    expect(toastMock).toHaveBeenCalledWith(
      'Restored unsaved changes to "One" from this browser.',
      "info"
    );
    await waitFor(() => expect(updateContentMock).toHaveBeenCalled());
    expect(updateContentMock.mock.calls[0].slice(0, 3)).toEqual([
      "/p",
      "doc-one",
      "On disk. Typed before the reload.",
    ]);
    await waitFor(() => expect(readSafetyCopy("/p", "doc-one")).toBeNull());
    expect(confirmMock).not.toHaveBeenCalled();
  });

  it("removes a copy that already matches the disk without a notice", async () => {
    writeSafetyCopy("/p", "doc-one", "On disk.", "Older.");
    openWith("On disk.");
    render(<Editor />);
    await waitFor(() => expect(screen.getByText("On disk.")).toBeInTheDocument());
    await waitFor(() => expect(readSafetyCopy("/p", "doc-one")).toBeNull());
    expect(toastMock).not.toHaveBeenCalled();
    await act(async () => {
      await new Promise((r) => setTimeout(r, 300));
    });
    expect(updateContentMock).not.toHaveBeenCalled();
  });

  it("asks when the disk moved since the copy, and honours each answer", async () => {
    // Keep: the copy goes, the disk text stays, nothing is saved.
    confirmMock.mockResolvedValueOnce(false);
    writeSafetyCopy("/p", "doc-one", "Old disk. Typed here.", "Old disk.");
    openWith("Saved on the phone.");
    const first = render(<Editor />);
    await waitFor(() => expect(confirmMock).toHaveBeenCalledTimes(1));
    expect(confirmMock.mock.calls[0][0]).toContain('"One"');
    await waitFor(() => expect(readSafetyCopy("/p", "doc-one")).toBeNull());
    expect(screen.getByText("Saved on the phone.")).toBeInTheDocument();
    await act(async () => {
      await new Promise((r) => setTimeout(r, 300));
    });
    expect(updateContentMock).not.toHaveBeenCalled();
    first.unmount();

    // Use them: the copy replaces the buffer and is saved.
    confirmMock.mockResolvedValueOnce(true);
    writeSafetyCopy("/p", "doc-one", "Old disk. Typed here.", "Old disk.");
    openWith("Saved on the phone.");
    render(<Editor />);
    await waitFor(() => expect(screen.getByText("Old disk. Typed here.")).toBeInTheDocument());
    await waitFor(() => expect(updateContentMock).toHaveBeenCalled());
    expect(updateContentMock.mock.calls[0].slice(0, 3)).toEqual([
      "/p",
      "doc-one",
      "Old disk. Typed here.",
    ]);
  });

  it("writes the copy at once when the page is hidden", async () => {
    openWith("On disk.");
    render(<Editor />);
    await waitFor(() => expect(screen.getByText("On disk.")).toBeInTheDocument());
    // A save that never answers: the reload happens inside the window.
    updateContentMock.mockImplementation(() => new Promise<void>(() => {}));
    await act(async () => {
      getCurrentEditor()!.commands.insertContentAt(
        getCurrentEditor()!.state.doc.content.size,
        " Last words."
      );
    });
    act(() => {
      window.dispatchEvent(new Event("pagehide"));
    });
    expect(readSafetyCopy("/p", "doc-one")?.content).toContain("Last words.");
  });
});
