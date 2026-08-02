import { describe, it, expect, beforeEach, vi } from "vitest";
import { render, screen, waitFor, act } from "@testing-library/react";

/**
 * Merge-banner action wiring (plan slice 4, round 9): Complete is an
 * epoch-bumping operation and MUST run the full barrier lifecycle WITH
 * the editor drain — a writer who resolves markers in the editor and
 * clicks Complete before the debounce fires would otherwise commit the
 * still-marker-laden disk state as the permanent two-parent merge
 * commit. Abort deliberately skips the drain: the buffer holds edits
 * being discarded.
 */

const flushMock = vi.hoisted(() => vi.fn(async () => {}));
const cancelAiMock = vi.hoisted(() => vi.fn());
vi.mock("../components/editor/editorRef", () => ({
  flushPendingEditorSave: flushMock,
  cancelActiveAiStreams: cancelAiMock,
}));

const openProjectMock = vi.hoisted(() => vi.fn(async () => {}));
const storeState = vi.hoisted(() => ({
  project: { path: "/p", documents: {} },
  activeDocId: null,
  flowDocs: null,
  openProject: openProjectMock,
  selectDocument: vi.fn(),
  enterFlow: vi.fn(),
}));
vi.mock("../stores/projectStore", () => ({
  useProjectStore: { getState: () => storeState },
}));

const completeMergeMock = vi.hoisted(() =>
  vi.fn(async (...args: unknown[]) => {
    void args;
    return {} as unknown;
  })
);
const abortMock = vi.hoisted(() => vi.fn(async () => {}));
type MergeStateShape = {
  in_progress: boolean;
  conflicted_files: string[];
  attestation: string | null;
};
const mergeStateMock = vi.hoisted(() =>
  vi.fn(
    async (): Promise<{
      in_progress: boolean;
      conflicted_files: string[];
      attestation: string | null;
    }> => ({ in_progress: true, conflicted_files: [], attestation: "abc:01" })
  )
);
vi.mock("../commands/git", () => ({
  completeMerge: completeMergeMock,
  syncAbortPull: abortMock,
  mergeState: mergeStateMock,
}));

const promptMock = vi.hoisted(() => vi.fn(async () => "Merged"));
const confirmMock = vi.hoisted(() => vi.fn(async () => true));
vi.mock("../components/shared/Dialog", () => ({
  dialogPrompt: promptMock,
  dialogConfirm: confirmMock,
}));
vi.mock("../components/shared/Toast", () => ({
  toastSuccess: vi.fn(),
  toastError: vi.fn(),
}));

import { MergeBanner } from "../components/revisions/MergeBanner";
import { useMergeState } from "../hooks/useMergeState";
import {
  acquireLease,
  isBarrierActive,
  resetBarrierForTests,
} from "../commands/barrier";
import { renderHook } from "@testing-library/react";

beforeEach(() => {
  resetBarrierForTests();
  vi.clearAllMocks();
});

const bannerProps = {
  projectPath: "/p",
  state: {
    in_progress: true,
    conflicted_files: ["manuscript/one.md"],
    attestation: "abc:01",
  },
  onChanged: vi.fn(),
};

describe("MergeBanner actions", () => {
  it("Complete drains the editor under the barrier BEFORE the commit dispatch", async () => {
    let barrierDuringFlush = false;
    let flushedBeforeCommit = false;
    flushMock.mockImplementationOnce(async () => {
      barrierDuringFlush = isBarrierActive();
    });
    completeMergeMock.mockImplementationOnce(async () => {
      flushedBeforeCommit = flushMock.mock.calls.length === 1;
      return {};
    });

    render(<MergeBanner {...bannerProps} />);
    await act(async () => {
      screen.getByText("Complete merge").click();
    });

    expect(completeMergeMock).toHaveBeenCalledTimes(1);
    // The lifecycle passed its lease to the dispatch (owner admission).
    expect(completeMergeMock.mock.calls[0][2]).toBeTruthy();
    expect(barrierDuringFlush).toBe(true);
    expect(flushedBeforeCommit).toBe(true);
    expect(openProjectMock).toHaveBeenCalled(); // reload ran
    expect(isBarrierActive()).toBe(false);
    expect(bannerProps.onChanged).toHaveBeenCalled();
  });

  it("Abort skips the drain — the buffer holds discarded edits", async () => {
    render(<MergeBanner {...bannerProps} />);
    await act(async () => {
      screen.getByText("Abort merge").click();
    });

    expect(abortMock).toHaveBeenCalledTimes(1);
    expect(flushMock).not.toHaveBeenCalled();
    expect(openProjectMock).toHaveBeenCalled();
    expect(isBarrierActive()).toBe(false);
  });

  it("a cancelled prompt completes nothing", async () => {
    promptMock.mockImplementationOnce(async () => null as unknown as string);
    render(<MergeBanner {...bannerProps} />);
    await act(async () => {
      screen.getByText("Complete merge").click();
    });
    expect(completeMergeMock).not.toHaveBeenCalled();
  });

  it("both exits disable while any barrier lease is held", async () => {
    let lease!: ReturnType<typeof acquireLease>;
    act(() => {
      lease = acquireLease();
    });
    render(<MergeBanner {...bannerProps} />);
    expect(screen.getByText("Complete merge")).toBeDisabled();
    expect(screen.getByText("Abort merge")).toBeDisabled();
    act(() => {
      lease.release();
    });
    expect(screen.getByText("Complete merge")).toBeEnabled();
  });
});

describe("useMergeState", () => {
  it("queries on mount and re-queries after every barrier release", async () => {
    const { result } = renderHook(() => useMergeState("/p"));
    await waitFor(() =>
      expect(result.current.mergeState?.in_progress).toBe(true)
    );
    expect(mergeStateMock).toHaveBeenCalledTimes(1);

    // An epoch operation ends (e.g. abort cleared the merge): the banner
    // must find out without a restart.
    mergeStateMock.mockImplementationOnce(async () => ({
      in_progress: false,
      conflicted_files: [],
      attestation: null,
    }));
    await act(async () => {
      const lease = acquireLease();
      lease.release();
    });
    await waitFor(() =>
      expect(result.current.mergeState?.in_progress).toBe(false)
    );
  });

  it("yields null without a project", () => {
    const { result } = renderHook(() => useMergeState(null));
    expect(result.current.mergeState).toBeNull();
    expect(mergeStateMock).not.toHaveBeenCalled();
  });

  it("a stale response from a previous project cannot overwrite the current state", async () => {
    // Finding s4-4: project A's query resolving after a switch to project
    // B must not land — a late `in_progress: false` would hide B's
    // recovery banner; a late `true` would describe A while the banner's
    // actions dispatch against B.
    let resolveA!: (s: MergeStateShape) => void;
    mergeStateMock.mockImplementationOnce(
      () => new Promise<MergeStateShape>((r) => (resolveA = r))
    );
    const { result, rerender } = renderHook(
      (path: string | null) => useMergeState(path),
      { initialProps: "/a" as string | null }
    );

    mergeStateMock.mockImplementationOnce(async () => ({
      in_progress: true,
      conflicted_files: [],
      attestation: "merge-of-b",
    }));
    rerender("/b");
    await waitFor(() =>
      expect(result.current.mergeState?.attestation).toBe("merge-of-b")
    );

    // A's answer arrives late — it must be dropped, not applied.
    await act(async () => {
      resolveA({ in_progress: false, conflicted_files: [], attestation: null });
    });
    expect(result.current.mergeState?.attestation).toBe("merge-of-b");
    expect(result.current.mergeState?.in_progress).toBe(true);
  });

  it("an already-resolved previous project's state never renders against the new path — not even pre-effect", async () => {
    // Finding s4-4, second reopen: an effect-based clear runs only AFTER
    // the first post-switch render commits, so that frame still pairs A's
    // resolved state with B's path — App would show A's banner with
    // actions dispatching against B. The snapshot must be scoped to its
    // path at RENDER time. The probe records what each render actually
    // saw, which renderHook's effect flushing would hide.
    const seen: string[] = [];
    function Probe({ path }: { path: string }) {
      const { mergeState } = useMergeState(path);
      seen.push(`${path}=${mergeState?.attestation ?? "none"}`);
      return null;
    }

    mergeStateMock.mockImplementationOnce(async () => ({
      in_progress: true,
      conflicted_files: [],
      attestation: "merge-of-a",
    }));
    const { rerender } = render(<Probe path="/a" />);
    await waitFor(() => expect(seen).toContain("/a=merge-of-a"));

    // B's query stays pending; no render may pair /b with A's state.
    let resolveB!: (s: MergeStateShape) => void;
    mergeStateMock.mockImplementationOnce(
      () => new Promise<MergeStateShape>((r) => (resolveB = r))
    );
    rerender(<Probe path="/b" />);
    await act(async () => {
      resolveB({
        in_progress: true,
        conflicted_files: [],
        attestation: "merge-of-b",
      });
    });

    expect(seen).not.toContain("/b=merge-of-a");
    expect(seen).toContain("/b=merge-of-b");
  });
});
