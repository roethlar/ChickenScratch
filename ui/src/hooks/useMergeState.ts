import { useCallback, useEffect, useRef, useState } from "react";
import * as gitCmd from "../commands/git";
import type { MergeState } from "../commands/git";
import { subscribeBarrier } from "../commands/barrier";

/**
 * Merge-in-progress state, queried from the backend so it survives app
 * restart (plan slice 4): the banner must reappear after a crash or quit
 * mid-merge, when the conflicted project may only open read-only through
 * the recovery path. Re-queries when the project path changes and after
 * every barrier release (each epoch-bumping operation — pull, merge,
 * complete, abort — can create or clear merge state).
 *
 * Two guards keep responses from crossing project boundaries (finding
 * s4-4, both rounds):
 *  - a request sequence drops LATE responses (project A's answer arriving
 *    after a switch to B must not land);
 *  - the stored snapshot is scoped to the path that produced it and
 *    derived at RENDER time, so an already-resolved answer for A cannot
 *    render even for the one frame before effects run — App would pair
 *    A's banner with B's path and its Complete/Abort would dispatch
 *    against B.
 */
export function useMergeState(projectPath: string | null): {
  mergeState: MergeState | null;
  refreshMergeState: () => void;
} {
  const [snapshot, setSnapshot] = useState<{
    path: string;
    state: MergeState;
  } | null>(null);
  const seqRef = useRef(0);

  const refreshMergeState = useCallback(() => {
    const seq = ++seqRef.current;
    if (!projectPath) {
      setSnapshot(null);
      return;
    }
    gitCmd
      .mergeState(projectPath)
      .then((s) => {
        if (seqRef.current === seq) setSnapshot({ path: projectPath, state: s });
      })
      .catch(() => {
        if (seqRef.current === seq) setSnapshot(null);
      });
  }, [projectPath]);

  useEffect(() => {
    // Merge state lives in the repository on disk — an external system;
    // querying it on mount/path-change is the correct effect boundary.
    // eslint-disable-next-line react-hooks/set-state-in-effect
    refreshMergeState();
    const unsubscribe = subscribeBarrier((active) => {
      if (!active) refreshMergeState();
    });
    return () => {
      // Invalidate every in-flight response for the old path/mount. The
      // ref is a request counter, not a DOM node: reading its LATEST
      // value at cleanup time is exactly the intended semantics.
      // eslint-disable-next-line react-hooks/exhaustive-deps
      seqRef.current++;
      unsubscribe();
    };
  }, [refreshMergeState]);

  // Render-scoped: a snapshot captured for another project never renders,
  // not even in the pre-effect frame right after a project switch.
  const mergeState =
    snapshot && snapshot.path === projectPath ? snapshot.state : null;
  return { mergeState, refreshMergeState };
}
