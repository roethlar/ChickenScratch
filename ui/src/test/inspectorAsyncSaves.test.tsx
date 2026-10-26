import { describe, it, expect, beforeEach, afterEach, vi } from "vitest";
import { render, screen, fireEvent, act } from "@testing-library/react";
import type { Document, Project } from "../types";
import type { updateDocumentMetadata } from "../commands/document";

/**
 * The Inspector's saves settle later than they are dispatched. These cases
 * use the real project store and a persistence mock whose answer can be
 * held back (review finding s3-1, third re-review; the reviewer's own
 * reproductions, kept as guards):
 *
 * - an edit back to the old text while a save is pending is still saved;
 * - a previous document's late answer never becomes the current document's
 *   baseline.
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

/** Holds the next save's answer until the returned function is called. */
function delayNextSave() {
  let finish: () => void = () => {
    throw new Error("save not started");
  };
  updateMetaMock.mockImplementationOnce(
    (_path, docId, meta) =>
      new Promise<Project>((resolve) => {
        finish = () => resolve(write(docId, meta));
      })
  );
  return () => finish();
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

describe("Inspector saves that settle late", () => {
  it.each(["debounced", "immediate"])(
    "keeps an edit back to A made while a %s save of B is pending",
    async (mode) => {
      const finish = delayNextSave();
      const view = render(<Inspector />);
      const box = screen.getByLabelText("Notes");
      fireEvent.change(box, { target: { value: "B" } });
      if (mode === "debounced") await advance(1600);
      else fireEvent.click(screen.getByRole("checkbox"));
      expect(updateMetaMock).toHaveBeenCalledTimes(1);
      fireEvent.change(box, { target: { value: "A" } });
      await act(async () => {
        finish();
      });
      await advance(2000);
      expect(box).toHaveValue("A");
      view.unmount();
      render(<Inspector />);
      expect(screen.getByLabelText("Notes")).toHaveValue("A");
      expect(stored.documents.d1.fields?.notes).toBe("A");
      expect(updateMetaMock).toHaveBeenCalledTimes(2);
    }
  );

  it("does not take a previous document's late save as the new document's baseline", async () => {
    const finish = delayNextSave();
    render(<Inspector />);
    fireEvent.change(screen.getByLabelText("Notes"), { target: { value: "B" } });
    await advance(1600);
    act(() => useProjectStore.getState().selectDocument("d2"));
    expect(screen.getByLabelText("Notes")).toHaveValue("C");
    await act(async () => {
      finish();
    });
    fireEvent.change(screen.getByLabelText("Notes"), { target: { value: "B" } });
    await advance(2000);
    expect(stored.documents.d2.fields?.notes).toBe("B");
    expect(updateMetaMock).toHaveBeenCalledTimes(2);
  });
});
