import { useEditor, useEditorState, EditorContent, type Editor as TipTapEditor } from "@tiptap/react";
import StarterKit from "@tiptap/starter-kit";
import Placeholder from "@tiptap/extension-placeholder";
import CharacterCount from "@tiptap/extension-character-count";
import { Underline } from "@tiptap/extension-underline";
import { TextStyle } from "@tiptap/extension-text-style";
import { Color } from "@tiptap/extension-color";
import { Link } from "@tiptap/extension-link";
import { useState, useEffect, useRef, useCallback } from "react";
import { useProjectStore } from "../../stores/projectStore";
import { useSettingsStore } from "../../stores/settingsStore";
import * as sessionCmd from "../../commands/session";
import { Toolbar } from "./Toolbar";
import { FindReplace } from "./FindReplace";
import { CommentMark } from "../comments/CommentMark";
import { FootnoteNode } from "./FootnoteNode";
import { AnchorNode } from "./AnchorNode";
import { ImageNode } from "./ImageNode";
import { classifyDocumentTarget, isAllowedLinkUri } from "./documentLinks";
import {
  setCurrentEditor,
  setPendingFlush,
  getEditorMarkdown,
  takeEditorFocusRequest,
} from "./editorRef";
import { getReloadGeneration, isBarrierActive, type LeaseHandle } from "../../commands/barrier";
import { useBarrierActive } from "../../hooks/useBarrier";
import { Markdown } from "tiptap-markdown";
import * as docCmd from "../../commands/document";
import { toast, toastError } from "../shared/Toast";
import { dialogConfirm } from "../shared/Dialog";
import { useProfile } from "../../profiles";
import { FlowBoundary, buildFlowBoundary, splitFlowSections } from "./FlowBoundary";
import {
  clearSafetyCopy,
  decideRestore,
  readSafetyCopy,
  writeSafetyCopy,
} from "./safetyCopy";

/** How often, at most, the dirty buffer is mirrored into browser storage while typing. */
const SAFETY_WRITE_MS = 500;

/**
 * Mirror a just-saved markdown payload into `project.documents` *and*
 * `activeDoc` (when it points at the same id). This keeps the editor's
 * load effect from reading stale store content if the user switches to
 * another doc and back before the next full project reload — a common
 * way to silently revert recent typing.
 */
function applyContentToStore(docId: string, markdown: string) {
  useProjectStore.setState((state) => {
    const project = state.project;
    if (!project) return state;
    const existing = project.documents[docId];
    if (!existing) return state;
    const updatedDoc = { ...existing, content: markdown };
    return {
      ...state,
      project: {
        ...project,
        documents: { ...project.documents, [docId]: updatedDoc },
      },
      activeDoc:
        state.activeDoc?.id === docId
          ? { ...state.activeDoc, content: markdown }
          : state.activeDoc,
    };
  });
}

export function Editor() {
  const profile = useProfile();
  const activeDoc = useProjectStore((s) => s.activeDoc);
  const flowDocs = useProjectStore((s) => s.flowDocs);
  const saving = useProjectStore((s) => s.saving);
  const readOnly = useProjectStore((s) => s.readOnly);
  const saveTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const docIdRef = useRef<string | null>(null);
  const flowIdsRef = useRef<string | null>(null);
  // Snapshot of the FlowDocs we built the editor buffer from. Captured at
  // load time and read by flushPendingSave; reading from the store at save
  // time would race with `exitFlow` / `selectDocument`, which clear
  // `flowDocs` *before* the editor's effect runs the flush.
  const flowDocsRef = useRef<{ docId: string; name: string; path: string }[] | null>(null);
  const [dirty, setDirty] = useState(false);
  // Mirror of the dirty state for synchronous reads. flushPendingSave needs
  // to no-op when nothing has changed (otherwise idle calls from periodic
  // backup / auto-commit re-stamp .meta files and create timestamp-only
  // revisions). React state isn't readable synchronously inside the
  // useCallback closure.
  const dirtyRef = useRef(false);
  const setDirtyTracked = useCallback((v: boolean) => {
    dirtyRef.current = v;
    setDirty(v);
  }, []);
  const [findOpen, setFindOpen] = useState(false);
  const [findReplace, setFindReplace] = useState(false);
  const editorRef = useRef<TipTapEditor | null>(null);

  // Safety copy (PLAN_RELOAD_SAFETY_MOBILE slice 1, `safetyCopy.ts`): the
  // disk text each loaded document was built from, so a copy records what it
  // diverged from, and the throttle timer for writing copies while typing.
  const baseRef = useRef<Map<string, string>>(new Map());
  const safetyTimer = useRef<ReturnType<typeof setTimeout> | null>(null);

  /** Mirror the dirty buffer into browser storage, one copy per document. */
  const writeSafetyNow = useCallback(() => {
    if (safetyTimer.current) {
      clearTimeout(safetyTimer.current);
      safetyTimer.current = null;
    }
    if (!dirtyRef.current) return;
    const ed = editorRef.current;
    const project = useProjectStore.getState().project;
    if (!ed || !project) return;
    const markdown = getEditorMarkdown(ed);
    if (flowIdsRef.current && flowDocsRef.current) {
      for (const sec of splitFlowSections(markdown)) {
        writeSafetyCopy(project.path, sec.docId, sec.content, baseRef.current.get(sec.docId) ?? "");
      }
      return;
    }
    const docId = docIdRef.current;
    if (!docId) return;
    writeSafetyCopy(project.path, docId, markdown, baseRef.current.get(docId) ?? "");
  }, []);

  const scheduleSafetyWrite = useCallback(() => {
    if (safetyTimer.current) return;
    safetyTimer.current = setTimeout(writeSafetyNow, SAFETY_WRITE_MS);
  }, [writeSafetyNow]);

  /**
   * A save of `saved` landed for `docId`: the disk holds it now. The copy
   * goes unless the buffer has already moved on (then the next save settles
   * it; a reload in between restores the newer text).
   */
  const settleSafety = useCallback(
    (projectPath: string, docId: string, saved: string, bufferNow: string | null) => {
      baseRef.current.set(docId, saved);
      if (bufferNow === null || bufferNow === saved) clearSafetyCopy(projectPath, docId);
    },
    []
  );

  /** Flow-mode counterpart: settle every section that was saved. */
  const settleFlowSafety = useCallback(
    (projectPath: string, saved: { docId: string; content: string }[]) => {
      const ed = editorRef.current;
      const now = new Map<string, string>();
      if (ed) {
        for (const sec of splitFlowSections(getEditorMarkdown(ed))) now.set(sec.docId, sec.content);
      }
      for (const sec of saved) {
        settleSafety(projectPath, sec.docId, sec.content, now.get(sec.docId) ?? null);
      }
    },
    [settleSafety]
  );

  const saveCurrent = useCallback(async () => {
    const editor = editorRef.current;
    const p = useProjectStore.getState().project;
    const flow = useProjectStore.getState().flowDocs;
    if (!editor || !p) return;
    // Suspended while an epoch-bumping operation holds the barrier: the
    // buffer is about to be rebuilt from disk, and a save here would land
    // pre-operation content under a fresh token (plan slice 3).
    if (isBarrierActive()) return;

    useProjectStore.setState({ saving: true });
    let anyFailure = false;
    try {
      const markdown = getEditorMarkdown(editor);
      if (flow) {
        // Flow mode: split at boundary markers, save each section back.
        // Track per-section failures — if any section fails, leave the
        // dirty flag set so the user knows their work isn't fully
        // persisted. Previous behavior swallowed failures inside the
        // loop and unconditionally cleared `dirty`, hiding partial-save
        // states behind a "Saved" indicator.
        const sections = splitFlowSections(markdown);
        const saved: { docId: string; content: string }[] = [];
        for (const sec of sections) {
          try {
            await docCmd.updateDocumentContent(p.path, sec.docId, sec.content);
            saved.push(sec);
          } catch (e) {
            anyFailure = true;
            toastError(`Failed to save ${sec.docId}: ${e}`);
          }
        }
        settleFlowSafety(p.path, saved);
        // Reload project so docs pick up updated content (even partial —
        // the on-disk truth is what we mirror).
        const Project = await import("../../commands/project");
        const reloaded = (await Project.loadProject(p.path)).project;
        useProjectStore.setState({ project: reloaded });
      } else {
        // Single-doc mode
        const d = useProjectStore.getState().activeDoc;
        if (!d) return;
        await docCmd.updateDocumentContent(p.path, d.id, markdown);
        // Disk first, then store update. Without the map update,
        // switching to another doc and back makes the editor load the
        // stale `project.documents[d.id].content` from before this save —
        // silently reverting whatever the user just typed.
        applyContentToStore(d.id, markdown);
        settleSafety(p.path, d.id, markdown, getEditorMarkdown(editor));
      }
      if (!anyFailure) setDirtyTracked(false);
    } catch (e) {
      toastError(`Save failed: ${e}`);
    } finally {
      useProjectStore.setState({ saving: false });
    }
  }, [setDirtyTracked, settleSafety, settleFlowSafety]);

  const autoSaveSeconds = useSettingsStore(
    (s) => s.appSettings?.writing.auto_save_seconds
  );

  const debouncedSave = useCallback(() => {
    // Inert while the barrier is up — rebuild setContent runs with
    // emitUpdate=false, and the editor is non-editable, so any update
    // reaching here mid-operation is programmatic noise, not typing.
    if (isBarrierActive()) return;
    setDirtyTracked(true);
    // The safety copy covers the debounce window below: a reload before the
    // save fires finds the text in browser storage.
    scheduleSafetyWrite();
    if (saveTimer.current) clearTimeout(saveTimer.current);
    // Settings exposes the delay; fall back to 2s if settings haven't
    // hydrated yet. Convert seconds → ms.
    const delayMs = Math.max(250, (autoSaveSeconds ?? 2) * 1000);
    saveTimer.current = setTimeout(() => {
      saveCurrent();
    }, delayMs);
  }, [saveCurrent, autoSaveSeconds, setDirtyTracked, scheduleSafetyWrite]);

  /**
   * Put back a safety copy left by an earlier page of this browser, once the
   * document's disk text is in the buffer. Disk unchanged since the copy →
   * apply it and let autosave write it, with one notice. Disk changed since
   * (another tab or device saved) → ask; never overwrite either side quietly.
   */
  const restoreSafetyCopy = useCallback(
    async (docId: string, name: string, disk: string, stale: () => boolean) => {
      const ed = editorRef.current;
      const project = useProjectStore.getState().project;
      if (!ed || !project || useProjectStore.getState().readOnly) return;
      const copy = readSafetyCopy(project.path, docId);
      const decision = decideRestore(copy, disk);
      if (decision.kind === "none") {
        if (copy) clearSafetyCopy(project.path, docId);
        return;
      }
      if (decision.kind === "conflict") {
        const useCopy = await dialogConfirm(
          `"${name}" has unsaved changes from an earlier session in this browser, but it was also changed elsewhere since. Use the unsaved changes from this browser? Cancel keeps the note as it is now.`
        );
        // The writer moved on while the question was open: keep the copy
        // for the next time this note opens.
        if (stale() || docIdRef.current !== docId) return;
        if (!useCopy) {
          clearSafetyCopy(project.path, docId);
          return;
        }
      }
      ed.commands.setContent(decision.content, { emitUpdate: false });
      debouncedSave();
      toast(`Restored unsaved changes to "${name}" from this browser.`, "info");
    },
    [debouncedSave]
  );

  /**
   * Flush a pending debounced save synchronously, BEFORE the editor swaps
   * to a new document or flow set. We can't call `saveCurrent` here because
   * it reads `activeDoc` dynamically — by the time the effect that calls us
   * runs, `activeDoc` has already moved on, so a naive flush would save the
   * outgoing doc's text into the incoming doc's id. Instead, we read the
   * id the editor was bound to (`docIdRef`) plus the editor's current
   * markdown and write that explicitly.
   */
  const flushPendingSave = useCallback(async (lease?: LeaseHandle): Promise<void> => {
    // No-op when the editor has nothing pending. Without this guard,
    // periodic auto-commit / backup intervals (which call us before
    // checking git status) would re-stamp `.meta` and create
    // timestamp-only revisions on every idle tick.
    if (!dirtyRef.current) return;
    // Non-owner flushes are suspended while the barrier is up; the
    // lifecycle's own drain passes its lease and proceeds (round 7).
    if (!lease && isBarrierActive()) return;

    if (saveTimer.current) {
      clearTimeout(saveTimer.current);
      saveTimer.current = null;
    }
    const ed = editorRef.current;
    const project = useProjectStore.getState().project;
    if (!ed || !project) return;

    const markdown = getEditorMarkdown(ed);
    const flowKey = flowIdsRef.current;
    const flowDocs = flowDocsRef.current;

    if (flowKey && flowDocs) {
      // Flow-mode flush. Splits the editor buffer at boundaries and
      // saves each section. Reads `flowDocsRef` (captured at load time)
      // instead of the store, because callers — `exitFlow` and
      // `selectDocument` — clear `flowDocs` BEFORE the editor effect
      // runs the flush, and reading the store here would either
      // attribute the entire flow buffer to a single doc or skip the
      // save altogether.
      const sections = splitFlowSections(markdown);
      // Memory first so a quick switch-back loads the just-typed text
      // from `project.documents[id]` instead of stale content.
      for (const sec of sections) {
        applyContentToStore(sec.docId, sec.content);
      }
      let anyFailure = false;
      const saved: { docId: string; content: string }[] = [];
      for (const sec of sections) {
        try {
          await docCmd.updateDocumentContent(project.path, sec.docId, sec.content, lease);
          saved.push(sec);
        } catch (e) {
          anyFailure = true;
          toastError(`Failed to save ${sec.docId}: ${e}`);
        }
      }
      settleFlowSafety(project.path, saved);
      if (!anyFailure) setDirtyTracked(false);
      if (anyFailure) throw new Error("Flow save partially failed");
      return;
    }

    const oldDocId = docIdRef.current;
    if (!oldDocId) return;
    // Memory-first: the store update has to happen synchronously so a
    // user who switches to another doc and back during the disk write
    // doesn't reload stale content. The disk-write promise still
    // resolves/rejects so beforeunload / auto-commit can wait on it
    // and skip downstream steps on failure.
    applyContentToStore(oldDocId, markdown);
    try {
      await docCmd.updateDocumentContent(project.path, oldDocId, markdown, lease);
      settleSafety(project.path, oldDocId, markdown, getEditorMarkdown(ed));
      setDirtyTracked(false);
    } catch (e) {
      toastError(`Save failed: ${e}`);
      throw e;
    }
  }, [setDirtyTracked, settleSafety, settleFlowSafety]);

  const editor = useEditor({
    extensions: [
      StarterKit.configure({
        heading: { levels: [1, 2, 3, 4] },
      }),
      Placeholder.configure({
        placeholder: "Start writing...",
      }),
      CharacterCount,
      Underline,
      TextStyle,
      Color,
      Link.configure({
        openOnClick: false,
        // Links between documents are relative or root-anchored paths
        // (`two.md#second`, `/manuscript/two.md`); the default rule would
        // drop them on parse. Only the syntax is checked here, in every
        // mode: where a click leads is decided when it happens, from the
        // document that holds it (PLAN_SCRIVENER_UPLOAD slice 3, plan-4
        // R4-4, R16-4, R17-3).
        isAllowedUri: (url) => isAllowedLinkUri(url),
        HTMLAttributes: {
          rel: "noopener noreferrer nofollow",
        },
      }),
      CommentMark,
      FootnoteNode,
      AnchorNode,
      ImageNode,
      FlowBoundary,
      Markdown.configure({
        html: true,            // allow inline HTML to pass through untouched
        tightLists: true,
        bulletListMarker: "-",
        linkify: false,
        breaks: false,
        transformPastedText: false,
        transformCopiedText: false,
      }),
    ],
    content: "",
    editorProps: {
      attributes: {
        class: "editor-content",
        spellcheck: "true",
      },
      handleDOMEvents: {
        // A click on a link between documents opens that document. In
        // Flow mode the editor cannot tell which document a click belongs
        // to, so a relative target opens only when every document in the
        // flow resolves it to the same one; otherwise a notice says to
        // open the document on its own (plan-4 R17-3, R18-1).
        click: (_view, event) => {
          const anchor = (event.target as HTMLElement | null)?.closest?.("a[href]");
          if (!anchor) return false;
          const href = anchor.getAttribute("href") ?? "";
          const state = useProjectStore.getState();
          const project = state.project;
          if (!project) return false;
          const docs = Object.values(project.documents);
          const origins = state.flowDocs
            ? state.flowDocs
                .map((fd) => project.documents[fd.docId])
                .filter((d): d is NonNullable<typeof d> => Boolean(d))
            : state.activeDoc
              ? [state.activeDoc]
              : [];
          const target = classifyDocumentTarget(href, origins, docs);
          if (target.kind === "document") {
            event.preventDefault();
            state.selectDocument(target.doc.id);
            return true;
          }
          if (target.kind === "ambiguous") {
            event.preventDefault();
            toast("Open this document on its own to follow the link", "info");
            return true;
          }
          return false;
        },
      },
    },
    onUpdate: () => {
      debouncedSave();
    },
  });

  useEffect(() => { editorRef.current = editor; }, [editor]);

  // Read-only project: the buffer must never diverge from disk, so the
  // editor itself is non-editable (no debounced saves can ever fire).
  // Same while an epoch-bumping operation holds the barrier: keystrokes
  // during an awaited restore/pull would live only in a buffer the
  // rebuild is about to replace (plan slice 3, round 3).
  //
  // emitUpdate=false: Tiptap's setEditable emits an update by default,
  // which reached debouncedSave and marked the buffer dirty the moment
  // the barrier released — the reload flush then wrote the pre-operation
  // text over the restored/switched tree (PLAN_RESTORE_STALE_BUFFER.md).
  // An editability change is never an edit.
  const barrierActive = useBarrierActive();
  useEffect(() => {
    if (editor) editor.setEditable(!readOnly && !barrierActive, false);
  }, [editor, readOnly, barrierActive]);

  // Ctrl+F / Ctrl+H shortcuts
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      const mod = e.metaKey || e.ctrlKey;
      if (mod && e.key === "f" && !e.shiftKey) {
        e.preventDefault();
        setFindOpen(true);
        setFindReplace(false);
      }
      if (mod && e.key === "h") {
        e.preventDefault();
        setFindOpen(true);
        setFindReplace(true);
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, []);

  // Load document content when active doc changes, or enter flow mode.
  // Also keyed on the reload generation (via barrierActive re-runs): after
  // a barrier lifecycle reloads the project, the buffer must rebuild from
  // the reloaded documents even when the visible doc/flow ids are
  // unchanged — a same-id restore otherwise keeps stale text on screen
  // (plan slice 3, rounds 5/8).
  const loadedGenerationRef = useRef(getReloadGeneration());
  useEffect(() => {
    if (!editor) return;
    let cancelled = false;

    const loadBuffer = async () => {
      const flow = useProjectStore.getState().flowDocs;
      const generation = getReloadGeneration();

      if (flow) {
        // Flow mode: concatenate documents with boundary markers.
        const flowKey = flow.map((d) => d.docId).join("|");
        if (flowIdsRef.current === flowKey && loadedGenerationRef.current === generation)
          return;
        // Persist any pending edits from the previous buffer (single-doc or
        // a different flow set) before we replace the editor content.
        try {
          await flushPendingSave();
        } catch {
          return;
        }
        if (cancelled) return;
        flowIdsRef.current = flowKey;
        loadedGenerationRef.current = generation;
        // Capture the flow set so flushPendingSave can save against it
        // even after the store's `flowDocs` is cleared by exitFlow or by
        // selectDocument.
        flowDocsRef.current = flow.map((d) => ({ docId: d.docId, name: d.name, path: d.path }));
        docIdRef.current = null;

        const project = useProjectStore.getState().project;
        if (!project) return;
        const canRestore = !useProjectStore.getState().readOnly;
        const restored: string[] = [];
        const parts: string[] = [];
        for (const fd of flow) {
          const doc = project.documents[fd.docId];
          if (!doc) continue;
          const disk = doc.content || "";
          baseRef.current.set(fd.docId, disk);
          let text = disk;
          if (canRestore) {
            // A safety copy whose disk text has not moved comes back here;
            // one the disk has outrun stays in storage and is decided when
            // the note opens on its own (restoreSafetyCopy asks).
            const copy = readSafetyCopy(project.path, fd.docId);
            const decision = decideRestore(copy, disk);
            if (decision.kind === "restore") {
              text = decision.content;
              restored.push(fd.name);
            } else if (decision.kind === "none" && copy) {
              clearSafetyCopy(project.path, fd.docId);
            }
          }
          // Emit a leading boundary for *every* doc — including the first.
          // splitFlowSections only outputs sections delimited by markers, so
          // skipping the leading marker silently drops every edit to the
          // first document on save.
          parts.push(buildFlowBoundary(fd.docId, fd.name));
          parts.push(text);
        }
        // emitUpdate=false: Tiptap 3 fires onUpdate by default on programmatic
        // setContent, which would route through debouncedSave and immediately
        // re-stamp every loaded doc's `modified` even though the user hasn't
        // typed. Document loads must be inert.
        editor.commands.setContent(parts.join(""), { emitUpdate: false });
        setDirtyTracked(false);
        if (restored.length > 0) {
          debouncedSave();
          toast(
            `Restored unsaved changes to ${restored.map((n) => `"${n}"`).join(", ")} from this browser.`,
            "info"
          );
        }
        return;
      }

      // Single-doc mode
      if (flowIdsRef.current !== null) {
        // Coming back from flow mode — flush whatever the flow buffer holds.
        // The flush still has the captured flow set in `flowDocsRef`, so
        // each section gets saved to the right doc.
        try {
          await flushPendingSave();
        } catch {
          return;
        }
        if (cancelled) return;
      }
      flowIdsRef.current = null;
      flowDocsRef.current = null;
      if (!activeDoc) {
        // Switching to "no document" — flush so the previous doc's edits
        // don't sit on the timer until the next mount.
        if (docIdRef.current) {
          try {
            await flushPendingSave();
          } catch {
            return;
          }
          if (cancelled) return;
        }
        editor.commands.clearContent(false);
        docIdRef.current = null;
        return;
      }
      if (docIdRef.current !== activeDoc.id || loadedGenerationRef.current !== generation) {
        // Critical: persist edits to the OUTGOING doc *before* we overwrite
        // the buffer with the incoming doc's content. Without this, any
        // typing from the past 2s of debounce window is silently dropped.
        try {
          await flushPendingSave();
        } catch {
          return;
        }
        if (cancelled) return;
        docIdRef.current = activeDoc.id;
        loadedGenerationRef.current = generation;
        const md = activeDoc.content || "";
        baseRef.current.set(activeDoc.id, md);
        // emitUpdate=false so loading a doc doesn't trigger autosave (Tiptap
        // 3 emits update on setContent by default).
        editor.commands.setContent(md, { emitUpdate: false });
        setDirtyTracked(false);
        if (takeEditorFocusRequest()) editor.commands.focus("end");
        await restoreSafetyCopy(activeDoc.id, activeDoc.name, md, () => cancelled);
      }
    };

    void loadBuffer();
    return () => { cancelled = true; };
    // barrierActive re-runs this after each lifecycle release so the
    // generation check above can rebuild same-id buffers post-reload.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [activeDoc?.id, editor, flowDocs, barrierActive]);

  // Search highlight: find and select first match when navigating from search
  const searchHighlight = useProjectStore((s) => s.searchHighlight);
  useEffect(() => {
    if (!editor || !searchHighlight || !activeDoc) return;
    setTimeout(() => {
      const text = editor.state.doc.textContent;
      const idx = text.toLowerCase().indexOf(searchHighlight.toLowerCase());
      if (idx >= 0) {
        let found = false;
        editor.state.doc.descendants((node, nodePos) => {
          if (found || !node.isText || !node.text) return;
          const nodeIdx = node.text.toLowerCase().indexOf(searchHighlight.toLowerCase());
          if (nodeIdx >= 0) {
            const from = nodePos + nodeIdx;
            const to = from + searchHighlight.length;
            editor.chain().focus().setTextSelection({ from, to }).run();
            found = true;
          }
        });
      }
      useProjectStore.setState({ searchHighlight: null });
    }, 100);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [searchHighlight, activeDoc?.id, editor]);

  // On unmount: flush a pending debounced save before tearing down. Just
  // clearing the timer (the original behavior) was a silent data-loss
  // path — typing stops, the user navigates away, and the 2s debounce
  // never fires. flushPendingSave is fire-and-forget; if the editor
  // unmounts as part of app close, see App.tsx's beforeunload handler
  // which awaits the same flush before backup_on_close runs.
  useEffect(() => {
    return () => {
      writeSafetyNow();
      flushPendingSave();
    };
  }, [flushPendingSave, writeSafetyNow]);

  // The last keystrokes must survive a reload: a storage write is
  // synchronous where the save request started from `beforeunload` is not
  // awaited by the browser (PLAN_RELOAD_SAFETY_MOBILE slice 1).
  useEffect(() => {
    const onHide = () => writeSafetyNow();
    window.addEventListener("pagehide", onHide);
    window.addEventListener("beforeunload", onHide);
    return () => {
      window.removeEventListener("pagehide", onHide);
      window.removeEventListener("beforeunload", onHide);
    };
  }, [writeSafetyNow]);

  useEffect(() => {
    setCurrentEditor(editor);
    return () => setCurrentEditor(null);
  }, [editor]);

  // Publish the pending-save flush to the global ref so non-Editor code
  // (App.tsx's beforeunload handler) can wait on it before the window
  // tears down.
  useEffect(() => {
    setPendingFlush(flushPendingSave);
    return () => setPendingFlush(null);
  }, [flushPendingSave]);

  const project = useProjectStore((s) => s.project);
  const sessionStartWords = useProjectStore((s) => s.sessionStartWords);

  // Subscribe to the editor's own transactions: document loads use
  // emitUpdate=false and leave React state untouched, so reading the
  // count during render alone would show a stale total until typing.
  const words =
    useEditorState({
      editor,
      selector: ({ editor: ed }) => ed?.storage.characterCount.words() ?? 0,
    }) ?? 0;

  // Flow mode: render even without activeDoc
  if (!activeDoc && !flowDocs) {
    return (
      <div className="editor-empty">
        <p>Select a document to start writing</p>
      </div>
    );
  }


  const saveLabel = saving ? "Saving..." : dirty ? "Modified" : "Saved";
  const totalProjectWords = project
    ? Object.values(project.documents).reduce((sum, doc) => {
        return sum + (doc.content || "").split(/\s+/).filter(Boolean).length;
      }, 0)
    : 0;
  const sessionWords = Math.max(0, totalProjectWords - sessionStartWords);

  return (
    <div className="editor-pane">
      <Toolbar editor={editor} />
      <FindReplace
        editor={editor}
        open={findOpen}
        showReplace={findReplace}
        onClose={() => setFindOpen(false)}
      />
      <div className="editor-scroll">
        <EditorContent editor={editor} />
        {profile.features.sessionTargets && <SessionBadge />}
      </div>
      <div className="editor-status">
        <span>
          {flowDocs ? `Editing ${flowDocs.length} documents` : ""}
          {flowDocs && ` · `}
          {words.toLocaleString()} words
          {sessionWords > 0 && ` · +${sessionWords.toLocaleString()} this session`}
        </span>
        <span>{saveLabel}</span>
      </div>
    </div>
  );
}

function SessionBadge() {
  const project = useProjectStore((s) => s.project);
  const [progress, setProgress] = useState<sessionCmd.SessionProgress | null>(null);
  const [hidden, setHidden] = useState(false);
  const hideTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const sessionErrorShown = useRef(false);

  useEffect(() => {
    if (!project) return;
    let cancelled = false;
    sessionErrorShown.current = false;
    const refresh = () => {
      sessionCmd
        .getSessionProgress(project.path)
        .then((p) => { if (!cancelled) setProgress(p); })
        .catch((e) => {
          if (cancelled) return;
          setProgress(null);
          if (!sessionErrorShown.current) {
            sessionErrorShown.current = true;
            toastError(`Session tracking unavailable: ${e}`);
          }
        });
    };
    refresh();
    const interval = setInterval(refresh, 30_000);
    return () => { cancelled = true; clearInterval(interval); };
  }, [project]);

  // Auto-hide on idle, reappear on any keypress in the editor
  useEffect(() => {
    const reveal = () => {
      setHidden(false);
      if (hideTimer.current) clearTimeout(hideTimer.current);
      hideTimer.current = setTimeout(() => setHidden(true), 4000);
    };
    reveal();
    window.addEventListener("keydown", reveal);
    return () => {
      window.removeEventListener("keydown", reveal);
      if (hideTimer.current) clearTimeout(hideTimer.current);
    };
  }, [project?.path]);

  if (!progress) return null;
  const hasTarget =
    progress.words_per_session != null ||
    progress.total_target != null ||
    progress.deadline != null;
  if (!hasTarget) return null;

  const goal = progress.words_per_session ?? 0;
  const pct = goal > 0 ? Math.min(100, Math.round((progress.today_words / goal) * 100)) : 0;
  const reached = goal > 0 && progress.today_words >= goal;

  const parts: string[] = [];
  if (goal > 0) parts.push(`Today ${progress.today_words.toLocaleString()}/${goal.toLocaleString()}`);
  if (progress.days_remaining != null) {
    if (progress.days_remaining > 0) parts.push(`${progress.days_remaining}d left`);
    else if (progress.days_remaining === 0) parts.push("deadline today");
    else parts.push("deadline passed");
  }
  if (progress.needed_per_day != null) {
    parts.push(`${progress.needed_per_day.toLocaleString()}/day needed`);
  }

  return (
    <div className={`session-badge ${hidden ? "hidden" : ""} ${reached ? "reached" : ""}`}>
      <div className="session-badge-text">{parts.join(" · ")}</div>
      {goal > 0 && (
        <div className="session-badge-bar">
          <div className="session-badge-fill" style={{ width: `${pct}%` }} />
        </div>
      )}
    </div>
  );
}
