/**
 * Operation barrier for epoch-bumping operations (plan slice 3,
 * docs/plans/PLAN_TREE_REPLACE_EPOCH_GUARD.md step 4).
 *
 * Any operation that can bump the backend write epoch (restore, draft
 * switch/merge, sync pull/abort/force — and later merge completion) runs
 * under a counted lease. While one or more leases are held:
 *  - editing and programmatic editor mutation are frozen (the editor and
 *    dispatch sites subscribe via `subscribeBarrier`/`isBarrierActive`),
 *  - every project-mutating dispatch WITHOUT the owner's lease handle is
 *    refused — never deferred: a deferred dispatch would keep its captured
 *    pre-operation arguments and land them under a fresh token after
 *    release (review round 7),
 *  - the lease owner's own dispatches (pre-operation drain, the operation
 *    itself, the post-operation reload — `load_project` is permit-backed
 *    and conditionally disk-mutating, round 8) pass via the handle.
 *
 * Editing resumes only when the LAST lease releases (counted, round 4),
 * and the final release runs a generation check so an earlier operation's
 * rebuild cannot leave the buffer on a stale snapshot (round 5).
 */

export class BarrierRefusedError extends Error {
  constructor(command: string) {
    super(
      `"${command}" was blocked: a revision, draft, or sync operation is in progress. ` +
        `Retry after it finishes.`
    );
    this.name = "BarrierRefusedError";
  }
}

export interface LeaseHandle {
  readonly id: number;
  release(): void;
}

let nextLeaseId = 1;
const activeLeases = new Set<number>();
/** Bumped on every project reload/rebuild; stale-snapshot forms resync
 *  against this (rounds 5/8). */
let reloadGeneration = 0;

type BarrierListener = (active: boolean) => void;
const listeners = new Set<BarrierListener>();

function notify() {
  const active = activeLeases.size > 0;
  for (const listener of listeners) listener(active);
}

export function isBarrierActive(): boolean {
  return activeLeases.size > 0;
}

export function getReloadGeneration(): number {
  return reloadGeneration;
}

export function bumpReloadGeneration(): number {
  reloadGeneration += 1;
  return reloadGeneration;
}

/** Subscribe to barrier activation changes. Returns an unsubscribe fn. */
export function subscribeBarrier(listener: BarrierListener): () => void {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}

/** Acquire a lease. Editing/dispatch stay frozen until the LAST handle
 *  releases. Release is idempotent. */
export function acquireLease(): LeaseHandle {
  const id = nextLeaseId++;
  activeLeases.add(id);
  if (activeLeases.size === 1) notify();
  let released = false;
  return {
    id,
    release() {
      if (released) return;
      released = true;
      activeLeases.delete(id);
      if (activeLeases.size === 0) notify();
    },
  };
}

/**
 * Gate for project-mutating dispatches. Call before every mutating
 * `invoke` in the commands layer:
 *  - no lease held → proceed;
 *  - lease held + matching owner handle → proceed;
 *  - lease held, no/foreign handle → throw BarrierRefusedError (refuse,
 *    never defer — round 7).
 */
export function assertDispatchAllowed(command: string, lease?: LeaseHandle): void {
  if (activeLeases.size === 0) return;
  if (lease && activeLeases.has(lease.id)) return;
  throw new BarrierRefusedError(command);
}

/** Test seam: drop all barrier state between tests. */
export function resetBarrierForTests(): void {
  activeLeases.clear();
  listeners.clear();
  reloadGeneration = 0;
}

