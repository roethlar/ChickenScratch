/**
 * The remembered place: which project and note were open in this browser
 * (docs/plans/PLAN_RELOAD_SAFETY_MOBILE.md, slice 1).
 *
 * The web shell reopens it after a reload so the writer lands where they
 * were; the Projects button forgets it so the list shows after an explicit
 * leave. The desktop shell never reads it (a launch still shows Welcome).
 */

const PROJECT_KEY = "cs-last-project";
const docKey = (projectPath: string) => `cs-last-doc:${projectPath}`;

function storage(): Storage | null {
  try {
    return window.localStorage ?? null;
  } catch {
    return null;
  }
}

function put(key: string, value: string): void {
  try {
    storage()?.setItem(key, value);
  } catch {
    /* storage unavailable: nothing is remembered */
  }
}

function take(key: string): string | null {
  try {
    return storage()?.getItem(key) ?? null;
  } catch {
    return null;
  }
}

export function rememberProject(projectPath: string): void {
  put(PROJECT_KEY, projectPath);
}

export function rememberDocument(projectPath: string, docId: string): void {
  put(docKey(projectPath), docId);
}

export function getLastProject(): string | null {
  return take(PROJECT_KEY);
}

export function getLastDocument(projectPath: string): string | null {
  return take(docKey(projectPath));
}

/** Forget the project; its remembered note stays so a later reopen lands on it. */
export function forgetProject(): void {
  try {
    storage()?.removeItem(PROJECT_KEY);
  } catch {
    /* nothing to forget */
  }
}
