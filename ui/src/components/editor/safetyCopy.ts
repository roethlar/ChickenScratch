/**
 * Browser-side safety copy of unsaved editor text
 * (docs/plans/PLAN_RELOAD_SAFETY_MOBILE.md, slice 1).
 *
 * Autosave is debounced, and a browser does not wait for the request a
 * `beforeunload` handler starts, so a reload inside that window used to lose
 * the last words typed. The editor now mirrors its dirty buffer into
 * `localStorage` (throttled while typing, synchronously on `pagehide`), and
 * the next load of the same document puts that text back.
 *
 * Each copy records `base`: the on-disk text the buffer was loaded from.
 * That is what lets the restore tell "the disk did not move, apply my copy"
 * from "someone else saved since" — the second case asks the writer rather
 * than overwriting either side (I6).
 *
 * Storage is a net, never a requirement: a browser without it, or one that
 * throws (private window, quota), degrades to today's behaviour.
 */

const PREFIX = "cs-draft:";

export interface SafetyCopy {
  /** The editor buffer as markdown. */
  content: string;
  /** The disk text the buffer was loaded from. */
  base: string;
  /** `Date.now()` when written; informational. */
  at: number;
}

export type RestoreDecision =
  | { kind: "none" }
  /** The disk still holds `base`: the copy is simply newer work. */
  | { kind: "restore"; content: string }
  /** The disk changed since the copy was made: someone else saved. */
  | { kind: "conflict"; content: string };

export function safetyKey(projectPath: string, docId: string): string {
  return `${PREFIX}${projectPath}:${docId}`;
}

function storage(): Storage | null {
  try {
    return window.localStorage ?? null;
  } catch {
    return null;
  }
}

export function writeSafetyCopy(
  projectPath: string,
  docId: string,
  content: string,
  base: string
): void {
  const s = storage();
  if (!s) return;
  const copy: SafetyCopy = { content, base, at: Date.now() };
  try {
    s.setItem(safetyKey(projectPath, docId), JSON.stringify(copy));
  } catch {
    /* quota or blocked storage: the autosave path is unaffected */
  }
}

export function readSafetyCopy(projectPath: string, docId: string): SafetyCopy | null {
  const s = storage();
  if (!s) return null;
  try {
    const raw = s.getItem(safetyKey(projectPath, docId));
    if (!raw) return null;
    const parsed = JSON.parse(raw) as Partial<SafetyCopy>;
    if (typeof parsed.content !== "string" || typeof parsed.base !== "string") return null;
    return { content: parsed.content, base: parsed.base, at: Number(parsed.at) || 0 };
  } catch {
    return null;
  }
}

export function clearSafetyCopy(projectPath: string, docId: string): void {
  const s = storage();
  if (!s) return;
  try {
    s.removeItem(safetyKey(projectPath, docId));
  } catch {
    /* nothing to clear */
  }
}

/**
 * What to do with a copy when its document loads with `disk` as the current
 * on-disk text. The caller clears the copy itself on `none` (it may be a
 * stale copy equal to disk) and after a `conflict` the writer resolved.
 */
export function decideRestore(copy: SafetyCopy | null, disk: string): RestoreDecision {
  if (!copy) return { kind: "none" };
  if (copy.content === disk) return { kind: "none" };
  if (copy.base === disk) return { kind: "restore", content: copy.content };
  return { kind: "conflict", content: copy.content };
}
