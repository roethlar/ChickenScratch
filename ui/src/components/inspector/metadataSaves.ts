import * as docCmd from "../../commands/document";
import type { Project } from "../../types";
import { toastError } from "../shared/Toast";

/**
 * Metadata saves for the Inspector, kept outside the mounted form (review
 * finding s3-1, fourth re-review: drafts deferred behind a slow or failing
 * save were lost on navigation, unmount or after a refusal).
 *
 * One draft per document is kept pending and one save per document runs at
 * a time. When a save settles, the newest pending draft for that document
 * goes next, whatever happened to the form on screen. A draft identical to
 * the last acknowledged form is not sent; a draft identical to the last
 * refused one waits for an edit, so a refusal never loops. A draft from an
 * immediate control (Include in Compile, Compile Order, Word Target) is
 * always sent, and its field survives a newer form draft queued over it
 * (fifth re-review).
 */

export type MetadataPayload = Parameters<typeof docCmd.updateDocumentMetadata>[2];

export interface Draft {
  projectPath: string;
  docId: string;
  /** The form the payload was built from, for comparison with the baseline. */
  snapshot: string;
  payload: MetadataPayload;
  /** Applies the project the backend returns (the store's `setProject`). */
  apply: (project: Project) => void;
  /** Sent even when the form equals the baseline (an immediate control). */
  force?: boolean;
}

const pending = new Map<string, Draft>();
const inFlight = new Map<string, Draft>();
const acknowledged = new Map<string, string>();
const refused = new Map<string, string>();
const listeners = new Set<() => void>();

export function draftKey(projectPath: string, docId: string): string {
  return `${projectPath}|${docId}`;
}

/** Called whenever a save settles; the Inspector re-checks its form then. */
export function subscribe(listener: () => void): () => void {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}

/**
 * The form just loaded from the project is the baseline, unless a save for
 * the document is still running or waiting (its answer is the newer truth).
 */
export function hydrated(key: string, snapshot: string): void {
  if (inFlight.has(key) || pending.has(key)) return;
  acknowledged.set(key, snapshot);
  refused.delete(key);
}

/**
 * The draft still to be acknowledged for a document, if any: the one waiting
 * behind the running save, else the running one. A form hydrating while one
 * exists shows it instead of the store's older values (fifth re-review).
 */
export function outstanding(key: string): Draft | undefined {
  return pending.get(key) ?? inFlight.get(key);
}

/**
 * True when `snapshot` is not what the document will end up saved as: the
 * draft waiting to go next if there is one (sixth re-review: a form edited
 * back to the running save's text while a newer draft waited was not dirty,
 * and the waiting draft then overwrote it), else the form being saved (an
 * edit back to the old text during a save still counts, fifth re-review),
 * else the acknowledged form, a refused form excepted.
 */
export function isDirty(key: string, snapshot: string): boolean {
  const waiting = pending.get(key);
  if (waiting) return snapshot !== waiting.snapshot;
  const flying = inFlight.get(key);
  if (flying) return snapshot !== flying.snapshot;
  return snapshot !== acknowledged.get(key) && snapshot !== refused.get(key);
}

/**
 * The writer edited the form since the last refusal: the refused form may
 * be tried again (editing away and back is a deliberate retry), while an
 * untouched refused form is never re-sent on its own.
 */
export function edited(key: string): void {
  refused.delete(key);
}

export function submit(draft: Draft): void {
  const key = draftKey(draft.projectPath, draft.docId);
  const previous = pending.get(key);
  // A newer draft replaces the waiting one but keeps the fields only the
  // older one carried (an immediate control's value).
  const merged: Draft = previous
    ? {
        ...draft,
        payload: { ...previous.payload, ...draft.payload },
        force: previous.force || draft.force,
      }
    : draft;
  pending.set(key, merged);
  void pump(key);
}

async function pump(key: string): Promise<void> {
  if (inFlight.has(key)) return;
  const draft = pending.get(key);
  if (!draft) return;
  pending.delete(key);
  if (!draft.force && !isDirty(key, draft.snapshot)) return;
  inFlight.set(key, draft);
  try {
    const updated = await docCmd.updateDocumentMetadata(draft.projectPath, draft.docId, draft.payload);
    acknowledged.set(key, draft.snapshot);
    refused.delete(key);
    draft.apply(updated);
  } catch (e) {
    // The dispatch gate refuses metadata writes while an epoch-bumping
    // operation runs; the draft is dropped loudly and the form stays dirty,
    // so the next edit tries again. No automatic retry of the same form.
    refused.set(key, draft.snapshot);
    toastError(`Details not saved: ${e}`);
  } finally {
    inFlight.delete(key);
    for (const listener of listeners) listener();
    // A newer draft arrived meanwhile: it goes next.
    void pump(key);
  }
}
