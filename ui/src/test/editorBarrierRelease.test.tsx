import { describe, it, expect, beforeEach, vi } from "vitest";
import { render, screen, waitFor, act } from "@testing-library/react";

/**
 * Releasing the epoch barrier must never write the pre-operation buffer
 * back to disk (PLAN_RESTORE_STALE_BUFFER.md). Tiptap's setEditable emits
 * an update by default; when the barrier released after a restore or a
 * draft switch, that update marked the stale buffer dirty and the reload
 * flush saved it over the freshly restored tree — the restore was
 * silently undone on disk (GUI validation 2026-09-25).
 */

const updateContentMock = vi.hoisted(() => vi.fn(async () => {}));
vi.mock("../commands/document", () => ({
  updateDocumentContent: updateContentMock,
}));
vi.mock("../stores/settingsStore", () => ({
  useSettingsStore: (sel: (s: { appSettings: null }) => unknown) =>
    sel({ appSettings: null }),
}));
vi.mock("../commands/session", () => ({
  getSessionProgress: vi.fn(async () => null),
  updateSessionTarget: vi.fn(async () => {}),
}));

import { Editor } from "../components/editor/Editor";
import { useProjectStore } from "../stores/projectStore";
import {
  acquireLease,
  bumpReloadGeneration,
  resetBarrierForTests,
} from "../commands/barrier";
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

beforeEach(() => {
  resetBarrierForTests();
  updateContentMock.mockClear();
  const p = project("Newer text.");
  useProjectStore.setState({
    project: p,
    readOnly: false,
    readOnlyReasons: [],
    activeDocId: "doc-one",
    activeDoc: p.documents["doc-one"],
    flowDocs: null,
  });
});

describe("editor across a barrier operation", () => {
  it("rebuilds from the reloaded tree without saving the stale buffer", async () => {
    render(<Editor />);
    await waitFor(() => expect(screen.getByText("Newer text.")).toBeInTheDocument());

    // The restore lifecycle: freeze, replace the tree, reload, release.
    let lease!: ReturnType<typeof acquireLease>;
    await act(async () => {
      lease = acquireLease();
    });
    await act(async () => {
      const restored = project("Older text.");
      useProjectStore.setState({
        project: restored,
        activeDocId: "doc-one",
        activeDoc: restored.documents["doc-one"],
      });
      bumpReloadGeneration();
    });
    await act(async () => {
      lease.release();
    });

    await waitFor(() => expect(screen.getByText("Older text.")).toBeInTheDocument());
    // Let any debounced autosave fire.
    await act(async () => {
      await new Promise((r) => setTimeout(r, 2600));
    });
    expect(updateContentMock).not.toHaveBeenCalled();
    expect(screen.getByText(/Saved/)).toBeInTheDocument();
  }, 10000);

  it("shows the word count as soon as a document loads", async () => {
    render(<Editor />);
    await waitFor(() => expect(screen.getByText("Newer text.")).toBeInTheDocument());
    await waitFor(() => expect(screen.getByText(/^2 words/)).toBeInTheDocument());
  });
});
