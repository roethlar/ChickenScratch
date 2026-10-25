import { describe, it, expect, beforeEach, afterEach, vi } from "vitest";
import { render, screen, fireEvent, act } from "@testing-library/react";

/**
 * The Inspector shows a document's `fields.notes` as a Notes box and saves
 * edits through the generic fields update (PLAN_SCRIVENER_FIDELITY slice 3:
 * Scrivener document notes land there, so they must be visible and
 * editable rather than buried in the sidecar).
 */

const aliceDoc = vi.hoisted(() => ({
  id: "d1",
  name: "Alice",
  path: "manuscript/alice.md",
  content: "Scene text.",
  fields: { notes: "Remember the egg." },
}));
// The Inspector loads a document's values when the active document
// changes, as in the app (it mounts before a document is selected), so the
// store starts empty and the document is selected after the first render.
const storeState = vi.hoisted(() => ({
  project: { path: "/p", name: "P", documents: {}, hierarchy: [] },
  activeDoc: null as typeof aliceDoc | null,
  setProject: vi.fn(),
}));
vi.mock("../stores/projectStore", () => ({
  useProjectStore: (sel: (s: typeof storeState) => unknown) => sel(storeState),
}));

const updateMetaMock = vi.hoisted(() =>
  vi.fn(async () => ({ path: "/p", name: "P", documents: {}, hierarchy: [] }))
);
vi.mock("../commands/document", () => ({
  updateDocumentMetadata: updateMetaMock,
  renameNode: vi.fn(async () => ({})),
  createEntity: vi.fn(async () => ({})),
}));
vi.mock("../commands/threads", () => ({
  listThreads: vi.fn(async () => []),
  createThread: vi.fn(async () => ({})),
}));
vi.mock("../commands/barrier", () => ({ getReloadGeneration: () => 0 }));
vi.mock("../hooks/useBarrier", () => ({ useBarrierActive: () => false }));
vi.mock("../profiles", () => ({
  useProfile: () => ({
    labelPresets: ["Scene", "Chapter"],
    features: { sceneFields: false },
  }),
}));
vi.mock("../components/shared/Toast", () => ({ toastError: vi.fn() }));

import { Inspector } from "../components/inspector/Inspector";

beforeEach(() => {
  vi.clearAllMocks();
  vi.useFakeTimers();
  storeState.activeDoc = null;
});
afterEach(() => {
  vi.useRealTimers();
});

function renderWithAlice() {
  const view = render(<Inspector />);
  storeState.activeDoc = aliceDoc;
  view.rerender(<Inspector />);
  return view;
}

describe("Inspector notes", () => {
  it("shows the document notes and saves an edit through fields.notes", async () => {
    renderWithAlice();
    const box = screen.getByLabelText("Notes") as HTMLTextAreaElement;
    expect(box.value).toBe("Remember the egg.");

    fireEvent.change(box, { target: { value: "Remember the egg and the key." } });
    await act(async () => {
      vi.advanceTimersByTime(1600);
    });

    expect(updateMetaMock).toHaveBeenCalled();
    const calls = updateMetaMock.mock.calls as unknown as [string, string, { fields?: Record<string, unknown> }][];
    const last = calls[calls.length - 1];
    expect(last[0]).toBe("/p");
    expect(last[1]).toBe("d1");
    expect(last[2].fields?.notes).toBe("Remember the egg and the key.");
  });

  it("clears the stored notes when the box is emptied", async () => {
    renderWithAlice();
    const box = screen.getByLabelText("Notes") as HTMLTextAreaElement;
    expect(box.value).toBe("Remember the egg.");
    fireEvent.change(box, { target: { value: "" } });
    await act(async () => {
      vi.advanceTimersByTime(1600);
    });
    const calls = updateMetaMock.mock.calls as unknown as [string, string, { fields?: Record<string, unknown> }][];
    const last = calls[calls.length - 1];
    expect(last[2].fields?.notes).toBeNull();
  });
});
