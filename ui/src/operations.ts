import {
  acquireLease,
  bumpReloadGeneration,
  type LeaseHandle,
} from "./commands/barrier";
import { flushPendingEditorSave, cancelActiveAiStreams } from "./components/editor/editorRef";
import { useProjectStore } from "./stores/projectStore";

/**
 * Run one epoch-bumping operation (restore, draft switch/merge, sync
 * pull/abort/force) under the full barrier lifecycle (plan slice 3):
 *
 *  1. acquire the lease — editing, programmatic editor mutation, and every
 *     non-owner project-mutating dispatch freeze FIRST (freeze-before-drain,
 *     round 5); in-flight AI streams are cancelled (round 4);
 *  2. drain the editor under the owner handle (the pre-operation flush is
 *     owner-admitted through the gate);
 *  3. run the operation with the lease;
 *  4. reload the project and rebuild the visible buffer on EVERY result
 *     kind — success, thrown error, and Ok(Conflicts) (rounds 1/2/6) — the
 *     reload dispatch is owner-admitted (round 8), and the reload
 *     generation bump forces stale-snapshot forms and the editor's
 *     load-effect to resync even for same-id documents (rounds 5/8);
 *  5. release the lease (editing resumes only when the LAST lease
 *     releases — counted, round 4).
 *
 * `skipDrain` exists for Abort/Force, which deliberately discard the buffer
 * (flushing would save edits the user chose to throw away).
 */
export async function runEpochOperation<T>(
  operation: (lease: LeaseHandle) => Promise<T>,
  options: { skipDrain?: boolean } = {}
): Promise<T> {
  const lease = acquireLease();
  const store = useProjectStore.getState();
  const projectPath = store.project?.path ?? null;
  const viewDocId = store.activeDocId;
  const viewFlow = store.flowDocs;
  try {
    cancelActiveAiStreams();
    if (!options.skipDrain) {
      await flushPendingEditorSave(lease);
    }
    return await operation(lease);
  } finally {
    if (projectPath) {
      try {
        await useProjectStore.getState().openProject(projectPath, lease);
        bumpReloadGeneration();
        const reloaded = useProjectStore.getState();
        if (viewFlow && reloaded.project) {
          // Rebuild flow mode over the documents that survived the
          // operation; the generation bump makes the editor reload the
          // buffer even though the flow ids are unchanged.
          const survivors = viewFlow.filter(
            (d) => reloaded.project && reloaded.project.documents[d.docId]
          );
          if (survivors.length > 0) {
            useProjectStore.getState().enterFlow(survivors);
          }
        } else if (viewDocId && reloaded.project?.documents[viewDocId]) {
          useProjectStore.getState().selectDocument(viewDocId);
        }
      } catch {
        // The reload itself failing must not mask the operation's own
        // result; the store carries the load error for the UI to surface.
      }
    }
    lease.release();
  }
}

