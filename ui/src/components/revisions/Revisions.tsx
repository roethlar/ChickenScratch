import { useState, useEffect, useCallback, useId, useRef } from "react";
import { dialogPrompt, dialogConfirm, useModalFocusTrap } from "../shared/Dialog";
import { flushPendingEditorSave } from "../editor/editorRef";
import { selectDocumentWithEditorFlush } from "../editor/navigationGuards";
import {
  Save,
  History,
  RotateCcw,
  GitBranch,
  GitMerge,
  HardDrive,
  Cloud,
  Upload,
  Download,
  GitCompare,
} from "lucide-react";
import { DraftCompare } from "./DraftCompare";
import { useProjectStore } from "../../stores/projectStore";
import { toastSuccess, toastError } from "../shared/Toast";
import * as gitCmd from "../../commands/git";
import * as threadCmd from "../../commands/threads";
import type { Revision, DraftVersion, FileDiff, SyncStatus } from "../../commands/git";
import { runEpochOperation } from "../../operations";
import { useBarrierActive } from "../../hooks/useBarrier";

export function Revisions() {
  const project = useProjectStore((s) => s.project);
  const [revisions, setRevisions] = useState<Revision[]>([]);
  const [drafts, setDrafts] = useState<DraftVersion[]>([]);
  const [saving, setSaving] = useState(false);
  const [message, setMessage] = useState("");
  const [tab, setTab] = useState<"history" | "drafts" | "threads">("history");
  const [diffId, setDiffId] = useState<string | null>(null);
  const [diffFiles, setDiffFiles] = useState<FileDiff[]>([]);
  const [wordDiffData, setWordDiffData] = useState<[string, string][] | null>(null);
  const [wordDiffFile, setWordDiffFile] = useState<string | null>(null);
  const [syncStatus, setSyncStatus] = useState<SyncStatus | null>(null);
  const [syncBusy, setSyncBusy] = useState(false);
  const [showCompare, setShowCompare] = useState(false);
  // Surfaced from both pull (remote conflict) and draft merge (local
  // conflict); declared up here so handleMergeDraft can dispatch to it.
  // Carries the merge attestation captured when the conflict was
  // surfaced: the force confirmation binds to it, so a merge swapped or
  // drifted underneath an open dialog refuses instead of discarding a
  // state the writer never saw (finding s4-1).
  const [conflict, setConflict] = useState<{
    files: string[];
    attestation: string | null;
  } | null>(null);
  // Every tree-replacing trigger disables while ANY barrier lease is held
  // (plan slice 3, round 4): overlapping operations would queue behind the
  // backend project lock and the second would replay against the first's
  // rebuilt state.
  const barrierActive = useBarrierActive();

  const refresh = useCallback(async () => {
    if (!project) return;
    const [revs, drs, st] = await Promise.all([
      gitCmd.listRevisions(project.path),
      gitCmd.listDrafts(project.path),
      gitCmd.syncStatus(project.path).catch(() => null),
    ]);
    setRevisions(revs);
    setDrafts(drs);
    setSyncStatus(st);
  }, [project]);

  // Loading revisions and drafts from an external system (git) — an effect is
  // the correct boundary for this async side-effect that sets multiple states.
  useEffect(() => {
    // eslint-disable-next-line react-hooks/set-state-in-effect
    refresh();
  }, [refresh]);

  /**
   * Run a git operation only after the editor's pending debounced save has
   * landed on disk. Without this gate, "type, immediately click Save Revision"
   * (or switch draft, restore, push, pull, force-pull) commits/operates on the
   * pre-debounce on-disk state and the typed words go into the next revision —
   * or get clobbered when a destructive op overwrites disk while the live
   * Tiptap buffer still holds newer memory-only text. (F-007)
   *
   * If the flush throws, we surface the error and abort — better to let the
   * user retry than silently commit stale content.
   */
  const runWithEditorFlush = useCallback(
    async <T,>(opName: string, fn: () => Promise<T>): Promise<T | undefined> => {
      try {
        await flushPendingEditorSave();
      } catch (e) {
        toastError(`${opName} aborted — editor save failed: ${e}`);
        return undefined;
      }
      return await fn();
    },
    []
  );

  const handleSave = async () => {
    if (!project || !message.trim()) return;
    setSaving(true);
    try {
      const ok = await runWithEditorFlush("Save revision", async () => {
        await gitCmd.saveRevision(project.path, message.trim());
        return true;
      });
      if (ok) {
        setMessage("");
        toastSuccess("Revision saved.");
        await refresh();
      }
    } catch (e) {
      toastError(`Save failed: ${e}`);
    }
    setSaving(false);
  };

  const handleRestore = async (commitId: string) => {
    if (!project) return;
    if (!(await dialogConfirm("Restore to this revision? Your current work will be preserved as a new revision.")))
      return;
    // Epoch-bumping: the barrier lifecycle freezes editing, drains the
    // buffer, and reloads + rebuilds on every result kind — including
    // failure, where the old success-only reload left a stale buffer one
    // auto-save away from clobbering the restored tree (plan slice 3).
    try {
      await runEpochOperation((lease) =>
        gitCmd.restoreRevision(project.path, commitId, lease)
      );
    } catch (e) {
      toastError(`Restore failed: ${e}`);
    }
    await refresh();
  };

  const handleNewDraft = async () => {
    if (!project) return;
    const name = await dialogPrompt("Draft version name:");
    if (!name) return;
    await runWithEditorFlush("Create draft", async () => {
      await gitCmd.createDraft(project.path, name);
      await useProjectStore.getState().openProject(project.path);
      await refresh();
    });
  };

  const handleSwitchDraft = async (name: string) => {
    if (!project) return;
    try {
      await runEpochOperation((lease) =>
        gitCmd.switchDraft(project.path, name, lease)
      );
    } catch (e) {
      toastError(`Switch failed: ${e}`);
    }
    await refresh();
  };

  const handleMergeDraft = async (name: string) => {
    if (!project) return;
    if (!(await dialogConfirm(`Merge "${name}" into the current draft?`))) return;
    try {
      // F-009: merge_draft returns a tagged result. Ok(Conflicts) has
      // ALREADY rewritten the working tree, so it gets the same
      // reload+rebuild as success and failure (the lifecycle does this on
      // every result kind); the dialog then offers the exits.
      const result = await runEpochOperation((lease) =>
        gitCmd.mergeDraft(project.path, name, lease)
      );
      if (result.kind === "conflicts") {
        const ms = await gitCmd.mergeState(project.path).catch(() => null);
        setConflict({
          files: result.files,
          attestation: ms?.attestation ?? null,
        });
      }
    } catch (e) {
      toastError(`Merge failed: ${e}`);
    }
    await refresh();
  };

  const handleBackup = async () => {
    if (!project) return;
    const dir = await dialogPrompt(
      "Backup directory:",
      localStorage.getItem("chickenscratch-backup-dir") || ""
    );
    if (!dir) return;
    localStorage.setItem("chickenscratch-backup-dir", dir);
    try {
      await runWithEditorFlush("Backup", async () => {
        await gitCmd.manualBackup(project.path, dir);
        toastSuccess("Backup complete.");
        await refresh();
      });
    } catch (e) {
      toastError(`Backup failed: ${e}`);
    }
  };

  const handlePush = async () => {
    if (!project) return;
    setSyncBusy(true);
    try {
      await runWithEditorFlush("Push", async () => {
        await gitCmd.syncPush(project.path);
        toastSuccess("Pushed to remote.");
        await refresh();
      });
    } catch (e) {
      toastError(`Push failed: ${e}`);
    }
    setSyncBusy(false);
  };

  // Fetch reads remote refs but doesn't touch the working tree, so the editor
  // flush is unnecessary for correctness. Wrapping anyway keeps every git
  // entry point uniform — easier to reason about than "most ops gate, fetch
  // doesn't" — and the cost is one no-op flush when the buffer is clean.
  const handleFetch = async () => {
    if (!project) return;
    setSyncBusy(true);
    try {
      await runWithEditorFlush("Fetch", async () => {
        await gitCmd.syncFetch(project.path);
        toastSuccess("Fetched from remote.");
        await refresh();
      });
    } catch (e) {
      toastError(`Fetch failed: ${e}`);
    }
    setSyncBusy(false);
  };

  const handlePull = async () => {
    if (!project) return;
    setSyncBusy(true);
    try {
      // F-008/slice 3: every pull result kind runs under the barrier
      // lifecycle — fast_forward and merged rewrite working-tree files,
      // conflicts leaves markers on disk, and a guarded partial failure
      // has already mutated the tree. All of them reload + rebuild.
      const result = await runEpochOperation((lease) =>
        gitCmd.syncPull(project.path, lease)
      );
      switch (result.kind) {
        case "up_to_date":
          toastSuccess("Already up to date.");
          break;
        case "fast_forward":
          toastSuccess("Pulled (fast-forward).");
          break;
        case "merged":
          toastSuccess("Pulled and merged.");
          break;
        case "conflicts": {
          const ms = await gitCmd.mergeState(project.path).catch(() => null);
          setConflict({
            files: result.files,
            attestation: ms?.attestation ?? null,
          });
          break;
        }
      }
      await refresh();
    } catch (e) {
      toastError(`Pull failed: ${e}`);
    }
    setSyncBusy(false);
  };

  const handleAbortPull = async () => {
    if (!project) return;
    setSyncBusy(true);
    try {
      // skipDrain: the buffer holds local edits the user chose to discard;
      // flushing would save them right before the reset threw them away.
      await runEpochOperation(
        (lease) => gitCmd.syncAbortPull(project.path, lease),
        { skipDrain: true }
      );
      toastSuccess("Merge aborted; local restored.");
      setConflict(null);
      await refresh();
    } catch (e) {
      toastError(`Abort failed: ${e}`);
    }
    setSyncBusy(false);
  };

  const handleForceResolve = async () => {
    // The captured attestation is required: without it the backend cannot
    // verify the writer is discarding the merge state they were shown, so
    // the Force exit stays disabled (fail closed) rather than guessing.
    if (!project || !conflict?.attestation) return;
    if (!(await dialogConfirm(
      "Discard your version of these files and take the incoming one? This cannot be undone."
    ))) return;
    setSyncBusy(true);
    try {
      // Resolves the in-progress merge by taking MERGE_HEAD wholesale —
      // the incoming remote commit after a pull conflict, the draft tip
      // after a draft-merge conflict. The old sync_pull_force route could
      // never run here: its dirty-worktree check fires on every conflicted
      // tree (plan slice 4 — this was a live bug). skipDrain: the buffer
      // holds the local edits the user just chose to discard.
      await runEpochOperation(
        (lease) =>
          gitCmd.forceResolveMerge(project.path, conflict.attestation!, lease),
        { skipDrain: true }
      );
      toastSuccess("Conflicts resolved with the incoming version.");
      setConflict(null);
      await refresh();
    } catch (e) {
      toastError(`Overwrite failed: ${e}`);
      // A fail-closed refusal means the merge drifted since the dialog
      // appeared. Re-capture the live state so the dialog shows what a
      // fresh confirmation would actually act on — otherwise the stale
      // attestation would refuse every retry (plan round 12: "requiring
      // fresh authority/confirmation").
      const ms = await gitCmd.mergeState(project.path).catch(() => null);
      if (ms?.in_progress) {
        setConflict({
          files: ms.conflicted_files,
          attestation: ms.attestation,
        });
      } else {
        setConflict(null);
      }
    }
    setSyncBusy(false);
  };

  if (!project) return null;

  const activeDraft = drafts.find((d) => d.is_active)?.name || "main";

  return (
    <div className="revisions">
      <div className="revisions-header">
        <span>Revisions</span>
        <span className="revisions-branch">{activeDraft}</span>
      </div>

      <div className="revisions-save">
        <input
          value={message}
          onChange={(e) => setMessage(e.target.value)}
          placeholder="Describe this revision..."
          onKeyDown={(e) => e.key === "Enter" && handleSave()}
        />
        <button onClick={handleSave} disabled={saving || !message.trim()}>
          <Save size={14} />
        </button>
      </div>

      <div className="revisions-tabs">
        <button
          className={tab === "history" ? "active" : ""}
          onClick={() => setTab("history")}
        >
          <History size={14} /> History
        </button>
        <button
          className={tab === "drafts" ? "active" : ""}
          onClick={() => setTab("drafts")}
        >
          <GitBranch size={14} /> Drafts
        </button>
        <button
          className={tab === "threads" ? "active" : ""}
          onClick={() => setTab("threads")}
        >
          <GitBranch size={14} /> Threads
        </button>
      </div>

      <div className="revisions-body">
        {tab === "history" && (
          <div className="revisions-list">
            {revisions.map((rev) => (
              <div key={rev.id}>
                <div className="revision-item">
                  <div
                    className="revision-info revision-clickable"
                    onClick={async () => {
                      if (diffId === rev.id) { setDiffId(null); setDiffFiles([]); return; }
                      setDiffId(rev.id);
                      try {
                        const files = await gitCmd.revisionDiff(project!.path, rev.id);
                        setDiffFiles(files);
                      } catch { setDiffFiles([]); }
                    }}
                    title="Click to see changes"
                  >
                    <span className="revision-msg">{rev.message}</span>
                    <span className="revision-time">
                      {new Date(rev.timestamp).toLocaleDateString(undefined, {
                        month: "short",
                        day: "numeric",
                        hour: "numeric",
                        minute: "2-digit",
                      })}
                    </span>
                  </div>
                  <button
                    className="revision-restore"
                    onClick={() => handleRestore(rev.id)}
                    disabled={barrierActive}
                    title="Restore to this revision"
                  >
                    <RotateCcw size={12} />
                  </button>
                </div>
                {diffId === rev.id && diffFiles.length > 0 && (
                  <div className="revision-diff">
                    {diffFiles.map((f, i) => (
                      <button
                        key={i}
                        className={`revision-diff-file diff-${f.status} ${wordDiffFile === f.path ? "active" : ""}`}
                        onClick={async () => {
                          if (wordDiffFile === f.path) {
                            setWordDiffFile(null);
                            setWordDiffData(null);
                            return;
                          }
                          setWordDiffFile(f.path);
                          try {
                            const data = await gitCmd.wordDiff(project!.path, rev.id, f.path);
                            setWordDiffData(data);
                          } catch { setWordDiffData(null); }
                        }}
                      >
                        <span className="diff-badge">{f.status[0].toUpperCase()}</span>
                        {f.path}
                      </button>
                    ))}
                    {wordDiffData && wordDiffFile && (
                      <div className="word-diff-view">
                        {wordDiffData.map(([kind, text], i) => (
                          <span key={i} className={`word-diff-${kind}`}>{text} </span>
                        ))}
                      </div>
                    )}
                  </div>
                )}
              </div>
            ))}
            {revisions.length === 0 && (
              <div className="revisions-empty">No revisions yet</div>
            )}
          </div>
        )}

        {tab === "drafts" && (
          <div className="revisions-list">
            {drafts.map((draft) => (
              <div
                key={draft.name}
                className={`revision-item ${draft.is_active ? "active-draft" : ""}`}
              >
                <div className="revision-info">
                  <span className="revision-msg">
                    {draft.name}
                    {draft.is_active && " (active)"}
                  </span>
                </div>
                {!draft.is_active && (
                  <div className="draft-actions">
                    <button
                      onClick={() => handleSwitchDraft(draft.name)}
                      disabled={barrierActive}
                      title="Switch to this draft"
                    >
                      <GitBranch size={12} />
                    </button>
                    <button
                      onClick={() => handleMergeDraft(draft.name)}
                      disabled={barrierActive}
                      title="Merge into current draft"
                    >
                      <GitMerge size={12} />
                    </button>
                  </div>
                )}
              </div>
            ))}
            <button className="drafts-new-btn" onClick={handleNewDraft}>
              <GitBranch size={14} /> New Draft Version
            </button>
            {drafts.length >= 2 && (
              <button className="drafts-new-btn" onClick={() => setShowCompare(true)}>
                <GitCompare size={14} /> Compare Drafts
              </button>
            )}
          </div>
        )}

        {tab === "threads" && project && (
          <ThreadsList
            project={project}
            onChange={(p) => useProjectStore.getState().setProject(p)}
          />
        )}
      </div>

      <DraftCompare open={showCompare} onClose={() => setShowCompare(false)} />

      <div className="revisions-footer">
        <button className="revisions-backup-btn" onClick={handleBackup}>
          <HardDrive size={14} /> Backup
        </button>
        <SyncControls
          status={syncStatus}
          busy={syncBusy || barrierActive}
          onPush={handlePush}
          onFetch={handleFetch}
          onPull={handlePull}
        />
      </div>

      {conflict && (
        <ConflictDialog
          files={conflict.files}
          busy={syncBusy || barrierActive}
          forceReady={!!conflict.attestation}
          onAbort={handleAbortPull}
          onForce={handleForceResolve}
          onResolveManually={() => setConflict(null)}
        />
      )}
    </div>
  );
}

function ConflictDialog({
  files,
  busy,
  forceReady,
  onAbort,
  onForce,
  onResolveManually,
}: {
  files: string[];
  busy: boolean;
  /** False when no attestation was captured for the confirmation binding —
   *  the force exit fails closed by staying disabled (finding s4-1). */
  forceReady: boolean;
  onAbort: () => void;
  onForce: () => void;
  onResolveManually: () => void;
}) {
  const titleId = useId();
  const manualButtonRef = useRef<HTMLButtonElement>(null);
  const { dialogRef, onDialogKeyDown } = useModalFocusTrap<HTMLDivElement>(
    true,
    onResolveManually,
    manualButtonRef
  );

  return (
    <div className="conflict-overlay">
      <div
        ref={dialogRef}
        className="conflict-dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby={titleId}
        tabIndex={-1}
        onKeyDown={onDialogKeyDown}
      >
        <h3 id={titleId}>Merge conflicts</h3>
        <p>
          The incoming changes touch the same files you did. The files now
          show both versions with conflict markers. Pick one:
        </p>
        <ul className="conflict-files">
          {files.slice(0, 10).map((f) => <li key={f}><code>{f}</code></li>)}
          {files.length > 10 && <li>…and {files.length - 10} more</li>}
        </ul>
        <div className="conflict-actions">
          <button
            ref={manualButtonRef}
            onClick={onResolveManually}
            disabled={busy}
            title="Close this dialog and edit the marked files; the merge bar stays until you complete or abort"
          >
            Resolve manually
          </button>
          <button onClick={onAbort} disabled={busy}>
            Abort merge (keep your version)
          </button>
          <button
            onClick={onForce}
            disabled={busy || !forceReady}
            className="conflict-danger"
          >
            Take the incoming version
          </button>
        </div>
      </div>
    </div>
  );
}

function SyncControls({
  status,
  busy,
  onPush,
  onFetch,
  onPull,
}: {
  status: SyncStatus | null;
  busy: boolean;
  onPush: () => void;
  onFetch: () => void;
  onPull: () => void;
}) {
  const enabled = !!status?.has_remote;
  const summary = (() => {
    if (!enabled) return "Configure a remote in Settings › Remote";
    if (status!.ahead === 0 && status!.behind === 0) return "Up to date with remote";
    const parts: string[] = [];
    if (status!.ahead) parts.push(`${status!.ahead} to push`);
    if (status!.behind) parts.push(`${status!.behind} to pull`);
    return parts.join(" · ");
  })();

  return (
    <div className="revisions-sync">
      <div className="revisions-sync-summary">
        <Cloud size={12} />
        <span>{summary}</span>
      </div>
      <div className="revisions-sync-actions">
        <button onClick={onFetch} disabled={!enabled || busy} title="Fetch from remote (no merge)">
          <Download size={12} /> Fetch
        </button>
        <button
          onClick={onPull}
          disabled={!enabled || busy}
          title="Pull from remote (fetch + merge)"
        >
          <Download size={12} /> Pull
        </button>
        <button onClick={onPush} disabled={!enabled || busy} title="Push to remote">
          <Upload size={12} /> Push
        </button>
      </div>
    </div>
  );
}

/** ── Threads list (Tier 1 #3) ─────────────────────────────────────────── */
import type { Project, Document, Thread } from "../../types";
import { useMemo } from "react";

const DEFAULT_THREAD_COLORS = [
  "#3b82f6", "#ef4444", "#f59e0b", "#10b981",
  "#a855f7", "#06b6d4", "#ec4899", "#84cc16",
];

function ThreadsList({
  project,
  onChange,
}: {
  project: Project;
  onChange: (project: Project) => void;
}) {
  const threads = project.threads ?? [];
  const [dangling, setDangling] = useState<threadCmd.DanglingRef[]>([]);

  useEffect(() => {
    let cancelled = false;
    threadCmd
      .validateReferences(project.path)
      .then((refs) => { if (!cancelled) setDangling(refs); })
      .catch(() => { if (!cancelled) setDangling([]); });
    return () => { cancelled = true; };
  }, [project]);

  const scenesByThread = useMemo(() => {
    const map = new Map<string, Document[]>();
    for (const doc of Object.values(project.documents)) {
      const list = doc.fields?.threads;
      if (!Array.isArray(list)) continue;
      for (const id of list) {
        if (typeof id !== "string") continue;
        if (!map.has(id)) map.set(id, []);
        map.get(id)!.push(doc);
      }
    }
    return map;
  }, [project.documents]);

  const handleNew = useCallback(async () => {
    const name = await dialogPrompt("New thread name:");
    if (!name?.trim()) return;
    try {
      const color = DEFAULT_THREAD_COLORS[threads.length % DEFAULT_THREAD_COLORS.length];
      const updated = await threadCmd.createThread(project.path, name.trim(), color);
      onChange(updated);
    } catch (e) {
      toastError(`Failed to create thread: ${e}`);
    }
  }, [project, threads.length, onChange]);

  const handleColorChange = useCallback(
    async (id: string, color: string) => {
      try {
        const updated = await threadCmd.updateThread(project.path, id, { color });
        onChange(updated);
      } catch (e) {
        toastError(`Failed: ${e}`);
      }
    },
    [project, onChange]
  );

  const handleRename = useCallback(
    async (thread: Thread) => {
      const name = await dialogPrompt("Rename thread:", thread.name);
      if (!name?.trim() || name.trim() === thread.name) return;
      try {
        const updated = await threadCmd.updateThread(project.path, thread.id, {
          name: name.trim(),
        });
        onChange(updated);
      } catch (e) {
        toastError(`Failed: ${e}`);
      }
    },
    [project, onChange]
  );

  const handleDelete = useCallback(
    async (thread: Thread) => {
      const refCount = scenesByThread.get(thread.id)?.length ?? 0;
      const confirmMsg =
        refCount > 0
          ? `Delete "${thread.name}"? It's used by ${refCount} scene${refCount === 1 ? "" : "s"} — references will be stripped.`
          : `Delete thread "${thread.name}"?`;
      if (!(await dialogConfirm(confirmMsg))) return;
      try {
        const updated = await threadCmd.deleteThread(project.path, thread.id);
        onChange(updated);
      } catch (e) {
        toastError(`Failed: ${e}`);
      }
    },
    [project, scenesByThread, onChange]
  );

  return (
    <div className="threads-panel">
      {dangling.length > 0 && (
        <div className="dangling-refs">
          <strong>{dangling.length}</strong> dangling reference
          {dangling.length === 1 ? "" : "s"} — scenes pointing at deleted
          characters/locations/threads. Open the scene's inspector to clear them.
          <details>
            <summary>Show</summary>
            <ul>
              {dangling.slice(0, 12).map((d, i) => (
                <li key={i}>
                  <em>{d.doc_name}</em> — {d.field}: <code>{d.missing_id}</code>
                </li>
              ))}
              {dangling.length > 12 && <li>…and {dangling.length - 12} more</li>}
            </ul>
          </details>
        </div>
      )}
      {threads.length === 0 ? (
        <div className="threads-empty">
          No plot threads yet. Tag scenes via the Inspector "Threads" field, or
          create one here to start tracking storylines.
        </div>
      ) : (
        threads.map((t) => {
          const scenes = scenesByThread.get(t.id) ?? [];
          const wordCount = scenes.reduce(
            (sum, doc) =>
              sum +
              (doc.content || "").replace(/<[^>]*>/g, " ").split(/\s+/).filter(Boolean).length,
            0
          );
          return (
            <details key={t.id} className="thread-card">
              <summary className="thread-card-summary">
                <ThreadColorSwatch
                  color={t.color ?? "#888"}
                  onChange={(c) => handleColorChange(t.id, c)}
                />
                <span className="thread-card-name" onClick={(e) => { e.preventDefault(); handleRename(t); }}>
                  {t.name}
                </span>
                <span className="thread-card-stats">
                  {scenes.length} scene{scenes.length === 1 ? "" : "s"} ·{" "}
                  {wordCount.toLocaleString()}w
                </span>
                <button
                  className="thread-card-delete"
                  onClick={(e) => { e.preventDefault(); handleDelete(t); }}
                  title="Delete thread"
                >
                  ×
                </button>
              </summary>
              <div className="thread-card-scenes">
                {scenes.length === 0 ? (
                  <div className="thread-card-empty">No scenes tagged.</div>
                ) : (
                  scenes.map((doc) => (
                    <button
                      key={doc.id}
                      className="thread-card-scene"
                      onClick={() => { void selectDocumentWithEditorFlush(doc.id); }}
                      title={doc.synopsis ?? ""}
                    >
                      {doc.name}
                    </button>
                  ))
                )}
              </div>
            </details>
          );
        })
      )}
      <button className="drafts-new-btn" onClick={handleNew}>
        <GitBranch size={14} /> New Thread
      </button>
    </div>
  );
}

function ThreadColorSwatch({
  color,
  onChange,
}: {
  color: string;
  onChange: (c: string) => void;
}) {
  const inputRef = useRef<HTMLInputElement>(null);
  return (
    <>
      <button
        type="button"
        className="thread-card-swatch"
        style={{ backgroundColor: color }}
        onClick={(e) => { e.preventDefault(); inputRef.current?.click(); }}
        title="Change colour"
      />
      <input
        ref={inputRef}
        type="color"
        value={color}
        onChange={(e) => onChange(e.target.value)}
        style={{ position: "absolute", width: 0, height: 0, opacity: 0 }}
      />
    </>
  );
}
