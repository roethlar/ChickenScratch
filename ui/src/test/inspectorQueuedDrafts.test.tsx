import { describe, it, expect, beforeEach, afterEach, vi } from "vitest";
import { render, screen, fireEvent, act } from "@testing-library/react";
import type { Document, Project } from "../types";
import type { updateDocumentMetadata } from "../commands/document";

/**
 * Drafts queued behind a slow or refused save are kept outside the mounted
 * form and sent when that save settles (review finding s3-1, fourth
 * re-review; the reviewer's own reproductions, kept as guards):
 *
 * - an edit made while a request is pending is saved after that request
 *   fails, in the same document or another;
 * - a newer draft deferred behind a slow save is saved even when the writer
 *   selects another document or hides the Inspector meanwhile.
 */

const updateMetaMock = vi.hoisted(() => vi.fn<typeof updateDocumentMetadata>());
vi.mock("../commands/document", () => ({
  updateDocumentMetadata: updateMetaMock,
  renameNode: vi.fn(),
  createEntity: vi.fn(),
}));
vi.mock("../commands/barrier", () => ({ getReloadGeneration: () => 0 }));
vi.mock("../hooks/useBarrier", () => ({ useBarrierActive: () => false }));
vi.mock("../profiles", () => ({
  useProfile: () => ({ labelPresets: [], features: { sceneFields: false } }),
}));
vi.mock("../components/shared/Toast", () => ({ toastError: vi.fn() }));

import { useProjectStore } from "../stores/projectStore";
import { Inspector } from "../components/inspector/Inspector";

function document(id: string, notes: string): Document {
  return {
    id,
    name: id,
    path: `manuscript/${id}.md`,
    content: "Body",
    parent_id: null,
    created: "2026-01-01",
    modified: "2026-01-01",
    include_in_compile: true,
    word_count_target: 0,
    compile_order: 0,
    fields: { notes },
  };
}

let stored: Project;
type Meta = Parameters<typeof updateDocumentMetadata>[2];
function write(docId: string, meta: Meta): Project {
  const old = stored.documents[docId];
  const updated: Document = {
    ...old,
    synopsis: meta.synopsis,
    label: meta.label,
    status: meta.status,
    keywords: meta.keywords,
    include_in_compile: meta.include_in_compile ?? old.include_in_compile,
    compile_order: meta.compile_order ?? old.compile_order,
    word_count_target: meta.word_count_target ?? old.word_count_target,
    fields: { ...old.fields, ...meta.fields },
  };
  stored = { ...stored, documents: { ...stored.documents, [docId]: updated } };
  return structuredClone(stored);
}

async function advance(ms: number) {
  await act(async () => {
    vi.advanceTimersByTime(ms);
  });
}

beforeEach(() => {
  vi.useFakeTimers();
  updateMetaMock.mockReset();
  stored = {
    id: "p",
    path: "/p",
    name: "P",
    hierarchy: [],
    metadata: {},
    created: "2026-01-01",
    modified: "2026-01-01",
    documents: { d1: document("d1", "A"), d2: document("d2", "C") },
  };
  updateMetaMock.mockImplementation(async (_path, docId, meta) => write(docId, meta));
  useProjectStore.setState({
    project: structuredClone(stored),
    activeDocId: "d1",
    activeDoc: structuredClone(stored.documents.d1),
  });
});
afterEach(() => vi.useRealTimers());

describe("drafts queued behind a failed save", () => {
  it.each(["same document", "different document"])(
    "saves an edit made in the %s while a failed request was pending",
    async (mode) => {
      let rejectSave!: (reason: Error) => void;
      updateMetaMock.mockImplementationOnce(
        () =>
          new Promise<Project>((_resolve, reject) => {
            rejectSave = reject;
          })
      );
      render(<Inspector />);
      fireEvent.change(screen.getByLabelText("Notes"), { target: { value: "B" } });
      await advance(1600);
      expect(updateMetaMock).toHaveBeenCalledTimes(1);
      const targetId = mode === "same document" ? "d1" : "d2";
      if (targetId === "d2") act(() => useProjectStore.getState().selectDocument("d2"));
      fireEvent.change(screen.getByLabelText("Notes"), { target: { value: "latest edit" } });
      await advance(500);
      await act(async () => {
        rejectSave(new Error("temporary network error"));
      });
      await advance(5000);
      expect(screen.getByLabelText("Notes")).toHaveValue("latest edit");
      expect(stored.documents[targetId].fields?.notes).toBe("latest edit");
      expect(updateMetaMock).toHaveBeenCalledTimes(2);
    }
  );
});

describe("drafts deferred behind a slow save", () => {
  it.each(["select another document", "hide the Inspector"])(
    "keeps a newer draft past the debounce window when the writer chooses to %s",
    async (mode) => {
      let finish!: () => void;
      // The backend writes B; its answer takes time to reach the client.
      updateMetaMock.mockImplementationOnce((_path, docId, meta) => {
        const reply = write(docId, meta);
        return new Promise<Project>((resolve) => {
          finish = () => resolve(reply);
        });
      });
      const view = render(<Inspector />);
      fireEvent.change(screen.getByLabelText("Notes"), { target: { value: "B" } });
      await advance(1600);
      expect(updateMetaMock).toHaveBeenCalledTimes(1);
      fireEvent.change(screen.getByLabelText("Notes"), { target: { value: "latest edit" } });
      await advance(2000);
      if (mode === "select another document") {
        act(() => useProjectStore.getState().selectDocument("d2"));
      } else {
        view.unmount();
      }
      await act(async () => {
        finish();
      });
      await advance(5000);
      expect(stored.documents.d1.fields?.notes).toBe("latest edit");
      expect(updateMetaMock).toHaveBeenCalledTimes(2);
    }
  );

  it("hands a dirty form over at once when the writer leaves inside the debounce window", async () => {
    render(<Inspector />);
    fireEvent.change(screen.getByLabelText("Notes"), { target: { value: "quick" } });
    await advance(300);
    expect(updateMetaMock).not.toHaveBeenCalled();
    act(() => useProjectStore.getState().selectDocument("d2"));
    await advance(100);
    expect(updateMetaMock).toHaveBeenCalledTimes(1);
    expect(stored.documents.d1.fields?.notes).toBe("quick");
  });
});
