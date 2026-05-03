import { useEditor, EditorContent, type Editor as TipTapEditor } from "@tiptap/react";
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
import { setCurrentEditor, setPendingFlush, getEditorMarkdown } from "./editorRef";
import { Markdown } from "tiptap-markdown";
import * as docCmd from "../../commands/document";
import { toastError } from "../shared/Toast";
import { FlowBoundary, buildFlowBoundary, splitFlowSections } from "./FlowBoundary";

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
  const activeDoc = useProjectStore((s) => s.activeDoc);
  const flowDocs = useProjectStore((s) => s.flowDocs);
  const saving = useProjectStore((s) => s.saving);
  const saveTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const docIdRef = useRef<string | null>(null);
  const flowIdsRef = useRef<string | null>(null);
  const [dirty, setDirty] = useState(false);
  const [findOpen, setFindOpen] = useState(false);
  const [findReplace, setFindReplace] = useState(false);
  const editorRef = useRef<TipTapEditor | null>(null);

  const saveCurrent = useCallback(async () => {
    const editor = editorRef.current;
    const p = useProjectStore.getState().project;
    const flow = useProjectStore.getState().flowDocs;
    if (!editor || !p) return;

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
        for (const sec of sections) {
          try {
            await docCmd.updateDocumentContent(p.path, sec.docId, sec.content);
          } catch (e) {
            anyFailure = true;
            toastError(`Failed to save ${sec.docId}: ${e}`);
          }
        }
        // Reload project so docs pick up updated content (even partial —
        // the on-disk truth is what we mirror).
        const Project = await import("../../commands/project");
        const reloaded = await Project.loadProject(p.path);
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
      }
      if (!anyFailure) setDirty(false);
    } catch (e) {
      toastError(`Save failed: ${e}`);
    } finally {
      useProjectStore.setState({ saving: false });
    }
  }, []);

  const autoSaveSeconds = useSettingsStore(
    (s) => s.appSettings?.writing.auto_save_seconds
  );

  const debouncedSave = useCallback(() => {
    setDirty(true);
    if (saveTimer.current) clearTimeout(saveTimer.current);
    // Settings exposes the delay; fall back to 2s if settings haven't
    // hydrated yet. Convert seconds → ms.
    const delayMs = Math.max(250, (autoSaveSeconds ?? 2) * 1000);
    saveTimer.current = setTimeout(() => {
      saveCurrent();
    }, delayMs);
  }, [saveCurrent, autoSaveSeconds]);

  /**
   * Flush a pending debounced save synchronously, BEFORE the editor swaps
   * to a new document or flow set. We can't call `saveCurrent` here because
   * it reads `activeDoc` dynamically — by the time the effect that calls us
   * runs, `activeDoc` has already moved on, so a naive flush would save the
   * outgoing doc's text into the incoming doc's id. Instead, we read the
   * id the editor was bound to (`docIdRef`) plus the editor's current
   * markdown and write that explicitly.
   */
  const flushPendingSave = useCallback(async (): Promise<void> => {
    if (saveTimer.current) {
      clearTimeout(saveTimer.current);
      saveTimer.current = null;
    }
    const ed = editorRef.current;
    const project = useProjectStore.getState().project;
    if (!ed || !project) return;

    const flowKey = flowIdsRef.current;
    if (flowKey) {
      // Flow mode: defer to saveCurrent which handles multi-doc dispatch
      // and the post-write project reload.
      await saveCurrent();
      return;
    }
    const oldDocId = docIdRef.current;
    if (!oldDocId) return;
    const markdown = getEditorMarkdown(ed);
    // Disk first, store second, error propagates. The earlier shape
    // (memory before disk + try/catch swallow) made
    // `flushPendingEditorSave()` resolve "successfully" even when the
    // backend write failed — beforeunload's backup_on_close would then
    // commit stale on-disk state while the in-memory store claimed it
    // was saved. Now the store update only runs after the disk write
    // confirms, and a failed write rejects the promise so callers
    // (App.tsx beforeunload, periodic auto-commit) can surface or skip.
    try {
      await docCmd.updateDocumentContent(project.path, oldDocId, markdown);
      applyContentToStore(oldDocId, markdown);
    } catch (e) {
      toastError(`Save failed: ${e}`);
      throw e;
    }
  }, [saveCurrent]);

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
        HTMLAttributes: {
          rel: "noopener noreferrer nofollow",
        },
      }),
      CommentMark,
      FootnoteNode,
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
    },
    onUpdate: () => {
      debouncedSave();
    },
  });

  useEffect(() => { editorRef.current = editor; }, [editor]);

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
  useEffect(() => {
    if (!editor) return;
    const flow = useProjectStore.getState().flowDocs;

    if (flow) {
      // Flow mode: concatenate documents with boundary markers.
      const flowKey = flow.map((d) => d.docId).join("|");
      if (flowIdsRef.current === flowKey) return;
      // Persist any pending edits from the previous buffer (single-doc or
      // a different flow set) before we replace the editor content.
      flushPendingSave();
      flowIdsRef.current = flowKey;
      docIdRef.current = null;

      const project = useProjectStore.getState().project;
      if (!project) return;
      const parts: string[] = [];
      for (const fd of flow) {
        const doc = project.documents[fd.docId];
        if (!doc) continue;
        // Emit a leading boundary for *every* doc — including the first.
        // splitFlowSections only outputs sections delimited by markers, so
        // skipping the leading marker silently drops every edit to the
        // first document on save.
        parts.push(buildFlowBoundary(fd.docId, fd.name));
        parts.push(doc.content || "");
      }
      editor.commands.setContent(parts.join(""));
      // eslint-disable-next-line react-hooks/set-state-in-effect
      setDirty(false);
      return;
    }

    // Single-doc mode
    if (flowIdsRef.current !== null) {
      // Coming back from flow mode — flush whatever the flow buffer holds.
      flushPendingSave();
    }
    flowIdsRef.current = null;
    if (!activeDoc) {
      // Switching to "no document" — flush so the previous doc's edits
      // don't sit on the timer until the next mount.
      if (docIdRef.current) flushPendingSave();
      editor.commands.clearContent();
      docIdRef.current = null;
      return;
    }
    if (docIdRef.current !== activeDoc.id) {
      // Critical: persist edits to the OUTGOING doc *before* we overwrite
      // the buffer with the incoming doc's content. Without this, any
      // typing from the past 2s of debounce window is silently dropped.
      flushPendingSave();
      docIdRef.current = activeDoc.id;
      const md = activeDoc.content || "";
      editor.commands.setContent(md);
      setDirty(false);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [activeDoc?.id, editor, flowDocs]);

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
      flushPendingSave();
    };
  }, [flushPendingSave]);

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

  // Flow mode: render even without activeDoc
  if (!activeDoc && !flowDocs) {
    return (
      <div className="editor-empty">
        <p>Select a document to start writing</p>
      </div>
    );
  }

  const words = editor?.storage.characterCount.words() ?? 0;
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
        <SessionBadge />
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

  useEffect(() => {
    if (!project) return;
    let cancelled = false;
    const refresh = () => {
      sessionCmd
        .getSessionProgress(project.path)
        .then((p) => { if (!cancelled) setProgress(p); })
        .catch(() => { if (!cancelled) setProgress(null); });
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
