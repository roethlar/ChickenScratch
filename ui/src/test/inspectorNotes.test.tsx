import { StrictMode } from "react";
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
  keywords: [] as string[],
  include_in_compile: true,
  compile_order: 0,
  word_count_target: 0,
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

  it("mounting on an already selected document shows its notes and saves nothing by itself", async () => {
    // Review finding s3-1: opening the Inspector used to leave the form
    // empty and then save the empty form, erasing the stored metadata.
    storeState.activeDoc = aliceDoc;
    render(<Inspector />);
    const box = screen.getByLabelText("Notes") as HTMLTextAreaElement;
    expect(box.value).toBe("Remember the egg.");
    await act(async () => {
      vi.advanceTimersByTime(2000);
    });
    expect(updateMetaMock).not.toHaveBeenCalled();

    fireEvent.change(box, { target: { value: "Edited." } });
    await act(async () => {
      vi.advanceTimersByTime(1600);
    });
    expect(updateMetaMock).toHaveBeenCalledTimes(1);
    const calls = updateMetaMock.mock.calls as unknown as [string, string, { fields?: Record<string, unknown>; synopsis?: unknown }][];
    expect(calls[0][2].fields?.notes).toBe("Edited.");
  });

  it("under StrictMode, mounting on a selected document still saves nothing by itself", async () => {
    // Review finding s3-1, re-review: StrictMode runs every effect twice in
    // development; the replay used to look like an edit and save the
    // hydrated form, re-splitting a keyword that contains a comma.
    storeState.activeDoc = { ...aliceDoc, keywords: ["family, friends"] };
    render(
      <StrictMode>
        <Inspector />
      </StrictMode>
    );
    const box = screen.getByLabelText("Notes") as HTMLTextAreaElement;
    expect(box.value).toBe("Remember the egg.");
    await act(async () => {
      vi.advanceTimersByTime(2000);
    });
    expect(updateMetaMock).not.toHaveBeenCalled();

    // An edit saves once; editing back to the stored text saves again,
    // because the stored value has moved on.
    fireEvent.change(box, { target: { value: "Edited." } });
    await act(async () => {
      vi.advanceTimersByTime(1600);
    });
    expect(updateMetaMock).toHaveBeenCalledTimes(1);
    fireEvent.change(box, { target: { value: "Remember the egg." } });
    await act(async () => {
      vi.advanceTimersByTime(1600);
    });
    expect(updateMetaMock).toHaveBeenCalledTimes(2);
    const calls = updateMetaMock.mock.calls as unknown as [string, string, { fields?: Record<string, unknown> }][];
    expect(calls[1][2].fields?.notes).toBe("Remember the egg.");
  });

  it("an immediate control saving the form keeps a later edit back to the old text savable", async () => {
    // Review finding s3-1, second re-review: Include in Compile saved the
    // form with notes B at once, but the baseline still said A, so editing
    // back to A before the debounce was skipped and B stayed on disk.
    type Call = [string, string, { fields?: Record<string, unknown>; include_in_compile?: boolean }];
    renderWithAlice();
    const box = screen.getByLabelText("Notes") as HTMLTextAreaElement;
    fireEvent.change(box, { target: { value: "B" } });
    await act(async () => {
      fireEvent.click(screen.getByRole("checkbox"));
    });
    expect(updateMetaMock).toHaveBeenCalledTimes(1);
    let calls = updateMetaMock.mock.calls as unknown as Call[];
    expect(calls[0][2].include_in_compile).toBe(false);
    expect(calls[0][2].fields?.notes).toBe("B");

    fireEvent.change(box, { target: { value: "Remember the egg." } });
    await act(async () => {
      vi.advanceTimersByTime(1600);
    });
    expect(updateMetaMock).toHaveBeenCalledTimes(2);
    calls = updateMetaMock.mock.calls as unknown as Call[];
    expect(calls[1][2].fields?.notes).toBe("Remember the egg.");
  });

  it("a refused save leaves the form dirty so the next edit tries again", async () => {
    // Review finding s3-1, second re-review: the baseline moved before the
    // write succeeded, so after a refusal an edit back to the same text
    // matched the baseline and was never retried.
    type Call = [string, string, { fields?: Record<string, unknown> }];
    updateMetaMock.mockRejectedValueOnce(new Error("locked"));
    renderWithAlice();
    const box = screen.getByLabelText("Notes") as HTMLTextAreaElement;
    fireEvent.change(box, { target: { value: "B" } });
    await act(async () => {
      vi.advanceTimersByTime(1600);
    });
    expect(updateMetaMock).toHaveBeenCalledTimes(1);

    fireEvent.change(box, { target: { value: "B." } });
    fireEvent.change(box, { target: { value: "B" } });
    await act(async () => {
      vi.advanceTimersByTime(1600);
    });
    expect(updateMetaMock).toHaveBeenCalledTimes(2);
    const calls = updateMetaMock.mock.calls as unknown as Call[];
    expect(calls[1][2].fields?.notes).toBe("B");
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
