import { useState } from "react";
import type { Editor } from "@tiptap/react";
import {
  Bold,
  Italic,
  Underline,
  Strikethrough,
  Heading1,
  Heading2,
  Heading3,
  List,
  ListOrdered,
  Quote,
  Code,
  Code2,
  Minus,
  Link,
  Unlink,
  Undo,
  Redo,
  Sparkles,
  MessageSquare,
  Asterisk,
  X,
} from "lucide-react";
import { useCallback } from "react";
import { dialogPrompt } from "../shared/Dialog";
import {
  aiTransformStream,
  cancelAiTransformStream,
  createAiStreamId,
  type AiOperation,
} from "../../commands/ai";
import { toastError, toastSuccess } from "../shared/Toast";
import { useSettingsStore } from "../../stores/settingsStore";
import { useProjectStore } from "../../stores/projectStore";
import * as docCmd from "../../commands/document";
import { getEditorMarkdown, registerAiStreamCanceller } from "./editorRef";
import { exitFlowWithEditorFlush } from "./navigationGuards";
import { useBarrierActive } from "../../hooks/useBarrier";

interface ToolbarProps {
  editor: Editor | null;
}

function ToolbarButton({
  onClick,
  active,
  disabled,
  title,
  children,
}: {
  onClick: () => void;
  active?: boolean;
  disabled?: boolean;
  title: string;
  children: React.ReactNode;
}) {
  // Every toolbar action dispatches TipTap commands programmatically,
  // which setEditable(false) does NOT block — so the single shared
  // button is where the barrier freezes formatting, links, footnotes,
  // comments, and AI while an epoch-bumping operation runs
  // (plan slice 3, round 4).
  const barrierActive = useBarrierActive();
  return (
    <button
      className={`toolbar-btn ${active ? "active" : ""}`}
      onClick={onClick}
      disabled={disabled || barrierActive}
      title={title}
      onMouseDown={(e) => e.preventDefault()} // prevent editor blur
    >
      {children}
    </button>
  );
}

function ToolbarSep() {
  return <div className="toolbar-sep" />;
}

function selectedRangeFingerprint(editor: Editor, from: number, to: number): string {
  return JSON.stringify(editor.state.doc.slice(from, to).toJSON());
}

export function Toolbar({ editor }: ToolbarProps) {
  const setLink = useCallback(async () => {
    if (!editor) return;
    const prev = editor.getAttributes("link").href || "";
    const url = await dialogPrompt("URL:", prev);
    if (url === null) return;
    if (url === "") {
      editor.chain().focus().extendMarkRange("link").unsetLink().run();
      return;
    }
    editor.chain().focus().extendMarkRange("link").setLink({ href: url }).run();
  }, [editor]);

  const addFootnote = useCallback(async () => {
    if (!editor) return;
    const body = await dialogPrompt("Footnote text:");
    if (body === null || body.trim() === "") return;
    editor.chain().focus().insertContent({
      type: "footnote",
      attrs: { body: body.trim() },
    }).run();
  }, [editor]);

  const addComment = useCallback(async () => {
    if (!editor) return;
    const { from, to } = editor.state.selection;
    if (from === to) {
      toastError("Select text first to comment on");
      return;
    }
    const body = await dialogPrompt("Comment:");
    if (body === null) return;
    const project = useProjectStore.getState().project;
    const activeDoc = useProjectStore.getState().activeDoc;
    if (!project || !activeDoc) return;
    const commentId = "c_" + Math.random().toString(36).slice(2, 10);
    // Apply the mark locally first
    editor.chain().focus().setMark("comment", { id: commentId }).run();
    try {
      // tiptap-markdown serializes in-process
      const newContent = getEditorMarkdown(editor);
      const updated = await docCmd.addComment(
        project.path, activeDoc.id, commentId, body, newContent
      );
      // Use setProject so activeDoc.comments updates — the comments
      // panel reads it for its render.
      useProjectStore.getState().setProject(updated);
      toastSuccess("Comment added");
    } catch (e) {
      toastError(`Failed: ${e}`);
    }
  }, [editor]);

  const aiEnabled = useSettingsStore((s) => s.appSettings?.ai.enabled) ?? false;

  if (!editor) return null;

  const s = 15; // icon size

  return (
    <div className="editor-toolbar">
      <ToolbarButton
        onClick={() => editor.chain().focus().undo().run()}
        disabled={!editor.can().undo()}
        title="Undo (Ctrl+Z)"
      >
        <Undo size={s} />
      </ToolbarButton>
      <ToolbarButton
        onClick={() => editor.chain().focus().redo().run()}
        disabled={!editor.can().redo()}
        title="Redo (Ctrl+Shift+Z)"
      >
        <Redo size={s} />
      </ToolbarButton>

      <ToolbarSep />

      <ToolbarButton
        onClick={() => editor.chain().focus().toggleBold().run()}
        active={editor.isActive("bold")}
        title="Bold (Ctrl+B)"
      >
        <Bold size={s} />
      </ToolbarButton>
      <ToolbarButton
        onClick={() => editor.chain().focus().toggleItalic().run()}
        active={editor.isActive("italic")}
        title="Italic (Ctrl+I)"
      >
        <Italic size={s} />
      </ToolbarButton>
      <ToolbarButton
        onClick={() => editor.chain().focus().toggleUnderline().run()}
        active={editor.isActive("underline")}
        title="Underline (Ctrl+U)"
      >
        <Underline size={s} />
      </ToolbarButton>
      <ToolbarButton
        onClick={() => editor.chain().focus().toggleStrike().run()}
        active={editor.isActive("strike")}
        title="Strikethrough"
      >
        <Strikethrough size={s} />
      </ToolbarButton>

      <ToolbarSep />

      <ToolbarButton
        onClick={() => editor.chain().focus().toggleHeading({ level: 1 }).run()}
        active={editor.isActive("heading", { level: 1 })}
        title="Heading 1"
      >
        <Heading1 size={s} />
      </ToolbarButton>
      <ToolbarButton
        onClick={() => editor.chain().focus().toggleHeading({ level: 2 }).run()}
        active={editor.isActive("heading", { level: 2 })}
        title="Heading 2"
      >
        <Heading2 size={s} />
      </ToolbarButton>
      <ToolbarButton
        onClick={() => editor.chain().focus().toggleHeading({ level: 3 }).run()}
        active={editor.isActive("heading", { level: 3 })}
        title="Heading 3"
      >
        <Heading3 size={s} />
      </ToolbarButton>

      <ToolbarSep />

      <ToolbarButton
        onClick={() => editor.chain().focus().toggleBulletList().run()}
        active={editor.isActive("bulletList")}
        title="Bullet List"
      >
        <List size={s} />
      </ToolbarButton>
      <ToolbarButton
        onClick={() => editor.chain().focus().toggleOrderedList().run()}
        active={editor.isActive("orderedList")}
        title="Numbered List"
      >
        <ListOrdered size={s} />
      </ToolbarButton>
      <ToolbarButton
        onClick={() => editor.chain().focus().toggleBlockquote().run()}
        active={editor.isActive("blockquote")}
        title="Blockquote"
      >
        <Quote size={s} />
      </ToolbarButton>

      <ToolbarSep />

      <ToolbarButton
        onClick={() => editor.chain().focus().toggleCode().run()}
        active={editor.isActive("code")}
        title="Inline Code"
      >
        <Code size={s} />
      </ToolbarButton>
      <ToolbarButton
        onClick={() => editor.chain().focus().toggleCodeBlock().run()}
        active={editor.isActive("codeBlock")}
        title="Code Block"
      >
        <Code2 size={s} />
      </ToolbarButton>
      <ToolbarButton
        onClick={() => editor.chain().focus().setHorizontalRule().run()}
        title="Horizontal Rule"
      >
        <Minus size={s} />
      </ToolbarButton>

      <ToolbarSep />

      <ToolbarButton
        onClick={setLink}
        active={editor.isActive("link")}
        title="Insert Link"
      >
        <Link size={s} />
      </ToolbarButton>
      {editor.isActive("link") && (
        <ToolbarButton
          onClick={() => editor.chain().focus().unsetLink().run()}
          title="Remove Link"
        >
          <Unlink size={s} />
        </ToolbarButton>
      )}

      <ToolbarSep />

      <ToolbarButton
        onClick={addComment}
        active={editor.isActive("comment")}
        title="Add Comment"
      >
        <MessageSquare size={s} />
      </ToolbarButton>
      <ToolbarButton
        onClick={addFootnote}
        title="Insert Footnote"
      >
        <Asterisk size={s} />
      </ToolbarButton>

      <FlowButton />

      {aiEnabled && (
        <>
          <ToolbarSep />
          <AiMenu editor={editor} />
        </>
      )}
    </div>
  );
}

function FlowButton() {
  const flowDocs = useProjectStore((s) => s.flowDocs);

  if (!flowDocs) return null;

  // Drain pending edits BEFORE clearing `flowDocs`. Without the await,
  // the editor's load effect runs against a store where `flowDocs` is
  // already null and the flow buffer's edits would be saved to the
  // wrong target (or dropped entirely).
  const handleExit = async () => {
    await exitFlowWithEditorFlush();
  };

  return (
    <>
      <ToolbarSep />
      <ToolbarButton
        onClick={handleExit}
        active={true}
        title="Exit Flow mode"
      >
        <X size={14} />
      </ToolbarButton>
      <span className="toolbar-flow-label">{flowDocs.length} docs</span>
    </>
  );
}

function AiMenu({ editor }: { editor: Editor }) {
  const [open, setOpen] = useState(false);
  const [working, setWorking] = useState(false);

  const handleOp = useCallback(async (op: AiOperation) => {
    const aiContextKey = (state: ReturnType<typeof useProjectStore.getState>) => {
      if (state.activeDocId) return `doc:${state.activeDocId}`;
      if (state.flowDocs) {
        return `flow:${state.flowDocs.map((doc) => doc.docId).join(",")}`;
      }
      return null;
    };

    const origin = useProjectStore.getState();
    const originProjectPath = origin.project?.path ?? null;
    const originContextKey = aiContextKey(origin);
    if (!originContextKey || !originProjectPath) {
      toastError("Open a document before using AI");
      setOpen(false);
      return;
    }

    const { from, to } = editor.state.selection;
    if (from === to) {
      toastError("Select some text first");
      setOpen(false);
      return;
    }
    const selectedText = editor.state.doc.textBetween(from, to, " ");
    const selectedFingerprint = selectedRangeFingerprint(editor, from, to);
    setWorking(true);
    setOpen(false);

    const requestId = createAiStreamId();
    const abortController = new AbortController();
    let cancelledForContext = false;
    const stillOnOriginDoc = () => {
      if (cancelledForContext) return false;
      const current = useProjectStore.getState();
      const matches =
        aiContextKey(current) === originContextKey &&
        current.project?.path === originProjectPath;
      if (!matches) cancelledForContext = true;
      return matches;
    };

    let unsubscribe: (() => void) | null = null;
    unsubscribe = useProjectStore.subscribe((state) => {
      if (
        aiContextKey(state) !== originContextKey ||
        state.project?.path !== originProjectPath
      ) {
        cancelledForContext = true;
        cancelAiTransformStream(requestId).catch(() => {});
        abortController.abort();
        unsubscribe?.();
      }
    });
    // The barrier lifecycle cancels in-flight streams at barrier entry —
    // deltas landing during/after a tree operation would mutate a buffer
    // the rebuild is about to replace (plan slice 3, round 4).
    const unregisterCanceller = registerAiStreamCanceller(() => {
      cancelledForContext = true;
      cancelAiTransformStream(requestId).catch(() => {});
      abortController.abort();
    });

    try {
      if (!stillOnOriginDoc()) return;

      if (op === "brainstorm") {
        // Brainstorm: stream results into a blockquote AFTER the selection
        // so the user keeps their original passage.
        editor
          .chain()
          .focus()
          .setTextSelection(to)
          .insertContent("\n\n> ")
          .run();
        const insertPos = editor.state.selection.from;
        let currentEnd = insertPos;
        await aiTransformStream(
          selectedText,
          op,
          (delta) => {
            const chunk = delta.replace(/\n/g, "\n> ");
            editor.commands.insertContentAt(currentEnd, chunk);
            currentEnd += chunk.length;
          },
          {
            requestId,
            shouldContinue: stillOnOriginDoc,
            abortSignal: abortController.signal,
          }
        );
      } else {
        // Replacement ops are all-or-nothing: keep the selected prose intact
        // while the stream is in flight, then replace it in one transaction
        // only if the user is still on the same document and the selected
        // range still contains the original text.
        let replacement = "";
        await aiTransformStream(
          selectedText,
          op,
          (delta) => {
            replacement += delta;
          },
          {
            requestId,
            shouldContinue: stillOnOriginDoc,
            abortSignal: abortController.signal,
          }
        );
        if (
          !abortController.signal.aborted &&
          !cancelledForContext &&
          replacement.length > 0 &&
          stillOnOriginDoc() &&
          editor.state.doc.textBetween(from, to, " ") === selectedText &&
          selectedRangeFingerprint(editor, from, to) === selectedFingerprint
        ) {
          editor
            .chain()
            .focus()
            .insertContentAt({ from, to }, replacement)
            .run();
        }
      }
    } catch (e) {
      if (!cancelledForContext) {
        toastError(`AI failed: ${e}`);
      }
    } finally {
      unregisterCanceller();
      unsubscribe?.();
      setWorking(false);
    }
  }, [editor]);

  return (
    <div className="ai-menu-wrapper">
      <ToolbarButton
        onClick={() => setOpen(!open)}
        active={open}
        disabled={working}
        title="AI writing tools (select text first)"
      >
        {working ? <span className="ai-spinner">...</span> : <Sparkles size={15} />}
      </ToolbarButton>
      {open && (
        <div className="ai-menu-dropdown">
          <button onClick={() => handleOp("polish")}>Polish</button>
          <button onClick={() => handleOp("expand")}>Expand</button>
          <button onClick={() => handleOp("simplify")}>Simplify</button>
          <button onClick={() => handleOp("brainstorm")}>Brainstorm</button>
        </div>
      )}
    </div>
  );
}
