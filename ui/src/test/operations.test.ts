import { describe, it, expect, beforeEach, vi } from "vitest";
import type { LeaseHandle } from "../commands/barrier";

const flushMock = vi.hoisted(() => vi.fn(async () => {}));
const cancelAiMock = vi.hoisted(() => vi.fn());
vi.mock("../components/editor/editorRef", () => ({
  flushPendingEditorSave: flushMock,
  cancelActiveAiStreams: cancelAiMock,
}));

const openProjectMock = vi.hoisted(() =>
  vi.fn(async (...args: unknown[]) => {
    void args;
  })
);
const selectDocumentMock = vi.hoisted(() => vi.fn());
const enterFlowMock = vi.hoisted(() => vi.fn());
const storeState = vi.hoisted(() => ({
  project: { path: "/p", documents: { "doc-1": {}, "doc-2": {} } },
  activeDocId: "doc-1" as string | null,
  flowDocs: null as { docId: string; name: string; path: string }[] | null,
  openProject: openProjectMock,
  selectDocument: selectDocumentMock,
  enterFlow: enterFlowMock,
}));
vi.mock("../stores/projectStore", () => ({
  useProjectStore: { getState: () => storeState },
}));

import { runEpochOperation } from "../operations";
import {
  isBarrierActive,
  getReloadGeneration,
  resetBarrierForTests,
} from "../commands/barrier";

beforeEach(() => {
  resetBarrierForTests();
  flushMock.mockClear();
  cancelAiMock.mockClear();
  openProjectMock.mockClear();
  selectDocumentMock.mockClear();
  enterFlowMock.mockClear();
  storeState.activeDocId = "doc-1";
  storeState.flowDocs = null;
  storeState.project = { path: "/p", documents: { "doc-1": {}, "doc-2": {} } };
});

describe("epoch operation lifecycle", () => {
  it("freezes BEFORE draining, drains under the owner lease, and cancels AI", async () => {
    // Round-5 preflight-typing regression: typing during an unfrozen
    // drain schedules a save the completing flush marks clean; the lease
    // must be active before the flush starts.
    let barrierDuringFlush = false;
    let leaseDuringFlush: unknown = null;
    flushMock.mockImplementationOnce(async (lease?: unknown) => {
      barrierDuringFlush = isBarrierActive();
      leaseDuringFlush = lease;
    });
    await runEpochOperation(async () => "ok");
    expect(barrierDuringFlush).toBe(true);
    expect(leaseDuringFlush).not.toBeNull();
    expect(cancelAiMock).toHaveBeenCalledTimes(1);
  });

  it("reloads + rebuilds and releases on SUCCESS", async () => {
    const result = await runEpochOperation(async () => "done");
    expect(result).toBe("done");
    expect(openProjectMock).toHaveBeenCalledTimes(1);
    expect(openProjectMock.mock.calls[0][0]).toBe("/p");
    expect(openProjectMock.mock.calls[0][1]).toBeTruthy(); // owner-admitted
    expect(selectDocumentMock).toHaveBeenCalledWith("doc-1");
    expect(isBarrierActive()).toBe(false);
  });

  it("reloads + rebuilds and releases on FAILURE — the round-1 clobber window", async () => {
    // A guarded partial failure has already replaced the tree; without
    // the reload the stale buffer is one auto-save from clobbering it.
    const generationBefore = getReloadGeneration();
    await expect(
      runEpochOperation(async () => {
        throw new Error("injected post-mutation failure");
      })
    ).rejects.toThrow("injected post-mutation failure");
    expect(openProjectMock).toHaveBeenCalledTimes(1);
    expect(getReloadGeneration()).toBe(generationBefore + 1);
    expect(isBarrierActive()).toBe(false);
  });

  it("reloads on a CONFLICTS-shaped result — Ok(Conflicts) already rewrote the tree", async () => {
    const result = await runEpochOperation(async () => ({
      kind: "conflicts" as const,
      files: ["manuscript/one.md"],
    }));
    expect(result.kind).toBe("conflicts");
    expect(openProjectMock).toHaveBeenCalledTimes(1);
  });

  it("re-enters flow mode over surviving documents after reload", async () => {
    storeState.activeDocId = null;
    storeState.flowDocs = [
      { docId: "doc-1", name: "One", path: "manuscript/one.md" },
      { docId: "gone", name: "Gone", path: "manuscript/gone.md" },
    ];
    await runEpochOperation(async () => "ok");
    expect(enterFlowMock).toHaveBeenCalledWith([
      { docId: "doc-1", name: "One", path: "manuscript/one.md" },
    ]);
  });

  it("skips the drain for Abort/Force (the buffer holds discarded edits)", async () => {
    await runEpochOperation(async () => "ok", { skipDrain: true });
    expect(flushMock).not.toHaveBeenCalled();
  });

  it("keeps the barrier active across overlapping operations until the LAST release", async () => {
    // Round-5 overlap: the first completion must not resume editing
    // while the second still runs; the final state reflects the last
    // operation's reload.
    let releaseFirst!: () => void;
    const firstBlocked = new Promise<void>((r) => (releaseFirst = r));
    const first = runEpochOperation(async () => {
      await firstBlocked;
      return "first";
    });
    const second = runEpochOperation(async () => "second");
    await second;
    expect(isBarrierActive()).toBe(true); // first still holds its lease
    releaseFirst();
    await first;
    expect(isBarrierActive()).toBe(false);
    expect(openProjectMock).toHaveBeenCalledTimes(2);
  });

  it("releases the lease even when the reload itself fails", async () => {
    openProjectMock.mockImplementationOnce(async () => {
      throw new Error("reload failed");
    });
    await runEpochOperation(async () => "ok");
    expect(isBarrierActive()).toBe(false);
  });
});

describe("lease admission end-to-end shape", () => {
  it("the operation callback receives the same lease the gate admits", async () => {
    let seen: LeaseHandle | null = null;
    await runEpochOperation(async (lease) => {
      seen = lease;
      return "ok";
    });
    expect(seen).not.toBeNull();
  });
});
