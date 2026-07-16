import { useEffect, useRef, useSyncExternalStore } from "react";
import {
  getReloadGeneration,
  isBarrierActive,
  subscribeBarrier,
} from "../commands/barrier";

/** True while any epoch-bumping operation holds a barrier lease. Inputs,
 *  Save buttons, and programmatic editor mutation sites disable on it. */
export function useBarrierActive(): boolean {
  return useSyncExternalStore(
    (onChange) => subscribeBarrier(() => onChange()),
    isBarrierActive
  );
}

/**
 * Stale-snapshot form resync (plan rounds 5–8): calls `resync(wasDirty)`
 * after every project reload performed by the barrier lifecycle. Keyed on
 * the reload generation — never on a document id or project path, which
 * miss same-target reloads — and driven by the barrier subscription
 * itself, so it cannot miss a reload to render batching. The caller
 * resyncs its local form state from the reloaded store; when `wasDirty`
 * is true it must drop the draft LOUDLY (explicit notice), never
 * silently.
 */
export function useReloadResync(
  isDirty: () => boolean,
  resync: (wasDirty: boolean) => void
): void {
  const seenRef = useRef(getReloadGeneration());
  const isDirtyRef = useRef(isDirty);
  const resyncRef = useRef(resync);
  useEffect(() => {
    isDirtyRef.current = isDirty;
    resyncRef.current = resync;
  });
  useEffect(() => {
    const check = () => {
      const current = getReloadGeneration();
      if (current !== seenRef.current) {
        seenRef.current = current;
        resyncRef.current(isDirtyRef.current());
      }
    };
    // Catch a reload that landed between first render and subscription.
    check();
    return subscribeBarrier(check);
  }, []);
}
