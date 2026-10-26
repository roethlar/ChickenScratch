import { describe, it, expect, beforeEach, afterEach, vi } from "vitest";
import { render, screen, fireEvent, act } from "@testing-library/react";
import type { Document, Project } from "../types";
import type { updateDocumentMetadata } from "../commands/document";

/**
 * Saves still outstanding when the writer acts (review finding s3-1, fifth
 * re-review; the reviewer's reproductions, kept as guards):
 *
 * - an immediate control on an unchanged form still saves;
 * - an edit back to the old text during a pending save is kept when the
 *   writer leaves the document before the answer;
 * - reopening the Inspector during a pending save shows the saved-to-be
 *   values and never writes the store's older values back.
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
  updateMetaMock.mockImplementationOnce((_path, docId, meta) => {
    const reply = write(docId, meta);
    return new Promise<Project>((resolve) => {
      finish = () => resolve(reply);
    });
  });
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

describe("immediate controls on an unchanged form", () => {
  it("saves Include in Compile and Compile Order without any other edit", async () => {
    render(<Inspector />);
    await act(async () => {
      fireEvent.click(screen.getByRole("checkbox"));
    });
    expect(updateMetaMock).toHaveBeenCalledTimes(1);
    expect(stored.documents.d1.include_in_compile).toBe(false);
    await act(async () => {
      fireEvent.change(screen.getByPlaceholderText("0 = binder order"), { target: { value: "3" } });
    });
    expect(updateMetaMock).toHaveBeenCalledTimes(2);
    expect(stored.documents.d1.compile_order).toBe(3);
    expect(stored.documents.d1.fields?.notes).toBe("A");
  });
});

describe("leaving during a pending save", () => {
  it.each(["select another document", "hide the Inspector"])(
    "keeps an edit back to the old text when the writer chooses to %s before the answer",
    async (mode) => {
      const finish = delayNextSave();
      const view = render(<Inspector />);
      const box = screen.getByLabelText("Notes");
      fireEvent.change(box, { target: { value: "B" } });
      await advance(1600);
      expect(updateMetaMock).toHaveBeenCalledTimes(1);
      fireEvent.change(box, { target: { value: "A" } });
      if (mode === "select another document") {
        act(() => useProjectStore.getState().selectDocument("d2"));
      } else {
        view.unmount();
      }
      await act(async () => {
        finish();
      });
      await advance(5000);
      expect(stored.documents.d1.fields?.notes).toBe("A");
      expect(updateMetaMock).toHaveBeenCalledTimes(2);
    }
  );
});

describe("a queued draft made obsolete by a later edit", () => {
  it.each(["select another document", "hide the Inspector"])(
    "does not overwrite the final edit when the writer chooses to %s before the answer",
    async (mode) => {
      // Review finding s3-1, sixth re-review: A→B held, C queued past the
      // debounce, then back to B; leaving must not let the queued C win.
      const finish = delayNextSave();
      const view = render(<Inspector />);
      const box = screen.getByLabelText("Notes");
      fireEvent.change(box, { target: { value: "B" } });
      await advance(1600);
      expect(updateMetaMock).toHaveBeenCalledTimes(1);
      fireEvent.change(box, { target: { value: "C" } });
      await advance(1600);
      fireEvent.change(box, { target: { value: "B" } });
      if (mode === "select another document") {
        act(() => useProjectStore.getState().selectDocument("d2"));
      } else {
        view.unmount();
      }
      await act(async () => {
        finish();
      });
      await advance(5000);
      expect(stored.documents.d1.fields?.notes).toBe("B");
    }
  );
});

describe("reopening during a pending save", () => {
  it("shows the values being saved and never writes the older ones back", async () => {
    const finish = delayNextSave();
    const view = render(<Inspector />);
    fireEvent.change(screen.getByLabelText("Notes"), { target: { value: "B" } });
    await advance(1600);
    expect(updateMetaMock).toHaveBeenCalledTimes(1);
    view.unmount();
    render(<Inspector />);
    expect(screen.getByLabelText("Notes")).toHaveValue("B");
    await act(async () => {
      finish();
    });
    await advance(5000);
    expect(stored.documents.d1.fields?.notes).toBe("B");
    expect(updateMetaMock).toHaveBeenCalledTimes(1);
  });

  it("keeps a newer queued draft over the store's older values", async () => {
    const finish = delayNextSave();
    const view = render(<Inspector />);
    const box = screen.getByLabelText("Notes");
    fireEvent.change(box, { target: { value: "B" } });
    await advance(1600);
    fireEvent.change(box, { target: { value: "newer" } });
    await advance(1600);
    view.unmount();
    render(<Inspector />);
    expect(screen.getByLabelText("Notes")).toHaveValue("newer");
    await act(async () => {
      finish();
    });
    await advance(5000);
    expect(stored.documents.d1.fields?.notes).toBe("newer");
    expect(updateMetaMock).toHaveBeenCalledTimes(2);
  });
});
